//! A Debug Adapter Protocol (DAP) server for FreeMat-rs `.m` scripts.
//!
//! The [Debug Adapter Protocol][dap] is the JSON-RPC-ish wire format VS Code and
//! other editors speak to debuggers. This crate drives the [`fm_interp`]
//! interpreter through that protocol: it sets line breakpoints, single-steps,
//! reports the call stack, and reads (and evaluates against) the live variable
//! scopes — all by hooking the interpreter's single statement chokepoint
//! ([`fm_interp::debug::DebugHook`]).
//!
//! [dap]: https://microsoft.github.io/debug-adapter-protocol/
//!
//! # Architecture
//!
//! A single [`Session`] owns the transport (any [`BufRead`]/[`Write`] pair —
//! stdio in production, a TCP loopback in tests) and all debug state. Because
//! the interpreter must *own* its [`DebugHook`] during a run while the session
//! also drives the protocol around the run, the session lives behind an
//! `Rc<RefCell<…>>`. A tiny [`Hook`] adapter holding a clone of that handle is
//! what gets installed into the interpreter; when a statement is about to run it
//! borrows the session back and does the real work.
//!
//! # Limitations (single-file model)
//!
//! Breakpoints are matched by line against the **launched program**'s source.
//! Stepping (`next`/`stepIn`/`stepOut`) works across function-call frames, and
//! the call stack / variables of every frame are reported, but line breakpoints
//! inside separate function `.m` files are out of scope for this first cut.

use std::cell::RefCell;
use std::collections::HashSet;
use std::io::{BufRead, Write};
use std::rc::Rc;

use fm_interp::Interpreter;
use fm_interp::debug::{DebugControl, DebugHook};
use serde_json::{Value, json};

pub mod proto;
mod wire;

use proto::MAIN_THREAD;
pub use wire::{read_message, write_message};

/// What the interpreter should do next, as chosen by a `continue`/step request
/// while stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunMode {
    /// Run freely; stop only when a breakpoint line is reached.
    Continue,
    /// Stop at the very next statement (step-in / pause / stop-on-entry).
    StepIn,
    /// Stop at the next statement at this call depth or shallower (step-over).
    StepOver(usize),
    /// Stop at the next statement strictly shallower than this depth (step-out).
    StepOut(usize),
}

/// All debug-session state plus the transport. Lives behind `Rc<RefCell<…>>`
/// (see the module docs) so the protocol driver and the interpreter hook can
/// both reach it.
pub struct Session {
    reader: Box<dyn BufRead>,
    writer: Box<dyn Write>,
    /// Outgoing message sequence counter (DAP requires monotonic `seq`).
    seq: i64,
    /// Breakpoint lines (1-based) set in the launched program.
    breakpoints: HashSet<usize>,
    /// The launched program's source text (identity-compared against the seam's
    /// `src` to keep breakpoints from firing inside called functions).
    program_src: String,
    /// The program's source path, echoed back in `stackTrace` frames.
    program_path: String,
    /// How execution should proceed (updated by continue/step requests).
    mode: RunMode,
    /// `true` once a `terminate`/`disconnect` was requested — unwinds the run.
    terminate: bool,
    /// Per-stop `variablesReference` allocator (frame map), rebuilt on every stop.
    frames: proto::Frames,
}

impl Session {
    /// Build a session over the given transport halves.
    #[must_use]
    pub fn new(reader: Box<dyn BufRead>, writer: Box<dyn Write>) -> Self {
        Session {
            reader,
            writer,
            seq: 0,
            breakpoints: HashSet::new(),
            program_src: String::new(),
            program_path: String::new(),
            mode: RunMode::Continue,
            terminate: false,
            frames: proto::Frames::new(),
        }
    }

    // ---- low-level send helpers --------------------------------------------

    fn next_seq(&mut self) -> i64 {
        self.seq += 1;
        self.seq
    }

    /// Send a successful response to `request`, with an optional `body`.
    fn respond(&mut self, request: &Value, body: Value) {
        let seq = self.next_seq();
        let msg = json!({
            "seq": seq,
            "type": "response",
            "request_seq": request.get("seq").and_then(Value::as_i64).unwrap_or(0),
            "success": true,
            "command": request.get("command").and_then(Value::as_str).unwrap_or(""),
            "body": body,
        });
        let _ = write_message(&mut self.writer, &msg);
    }

    /// Send a failed response carrying a human-readable `message`.
    fn respond_err(&mut self, request: &Value, message: &str) {
        let seq = self.next_seq();
        let msg = json!({
            "seq": seq,
            "type": "response",
            "request_seq": request.get("seq").and_then(Value::as_i64).unwrap_or(0),
            "success": false,
            "command": request.get("command").and_then(Value::as_str).unwrap_or(""),
            "message": message,
        });
        let _ = write_message(&mut self.writer, &msg);
    }

    /// Send an event with the given name and body.
    fn event(&mut self, name: &str, body: Value) {
        let seq = self.next_seq();
        let msg = json!({
            "seq": seq,
            "type": "event",
            "event": name,
            "body": body,
        });
        let _ = write_message(&mut self.writer, &msg);
    }

    /// Emit text on the debug console (`output` event, `stdout` category).
    fn output(&mut self, text: &str) {
        self.event("output", json!({ "category": "stdout", "output": text }));
    }

    /// Drain the interpreter's pending program output to the debug console.
    fn flush_output(&mut self, interp: &mut Interpreter) {
        let chunk = interp.take_output();
        if !chunk.is_empty() {
            self.output(&chunk);
        }
    }
}

/// The hook installed into the interpreter for the duration of a run. Holds a
/// clone of the shared session handle.
struct Hook(Rc<RefCell<Session>>);

impl DebugHook for Hook {
    fn on_statement(&self, interp: &mut Interpreter, line: usize, src: &str) -> DebugControl {
        let mut session = self.0.borrow_mut();
        session.at_statement(interp, line, src)
    }
}

impl Session {
    /// Decide whether to stop before the statement at `line`, and if so, run the
    /// stopped request loop until the client resumes (or disconnects).
    fn at_statement(&mut self, interp: &mut Interpreter, line: usize, src: &str) -> DebugControl {
        if self.terminate {
            return DebugControl::Terminate;
        }
        let depth = interp.context.num_scopes();
        let in_program = src == self.program_src;
        let hit_breakpoint = in_program && self.breakpoints.contains(&line);
        let stepped = match self.mode {
            RunMode::Continue => false,
            RunMode::StepIn => true,
            RunMode::StepOver(d) => depth <= d,
            RunMode::StepOut(d) => depth < d,
        };
        if !hit_breakpoint && !stepped {
            return DebugControl::Resume;
        }
        let reason = if hit_breakpoint { "breakpoint" } else { "step" };
        self.stopped_loop(interp, reason)
    }

    /// Sit at a stop point: announce it, then serve requests until a resume.
    fn stopped_loop(&mut self, interp: &mut Interpreter, reason: &str) -> DebugControl {
        // Flush any program output produced up to this stop to the console.
        self.flush_output(interp);
        // A new stop invalidates the previous variablesReference handles.
        self.frames.reset();
        self.event(
            "stopped",
            json!({
                "reason": reason,
                "threadId": MAIN_THREAD,
                "allThreadsStopped": true,
            }),
        );

        loop {
            let request = match read_message(&mut self.reader) {
                Ok(Some(v)) => v,
                // EOF or transport error: treat as a disconnect.
                Ok(None) | Err(_) => return DebugControl::Terminate,
            };
            let command = request
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();

            match command.as_str() {
                "continue" => {
                    self.mode = RunMode::Continue;
                    self.respond(&request, json!({ "allThreadsContinued": true }));
                    return DebugControl::Resume;
                }
                "next" => {
                    self.mode = RunMode::StepOver(interp.context.num_scopes());
                    self.respond(&request, json!({}));
                    return DebugControl::Resume;
                }
                "stepIn" => {
                    self.mode = RunMode::StepIn;
                    self.respond(&request, json!({}));
                    return DebugControl::Resume;
                }
                "stepOut" => {
                    self.mode = RunMode::StepOut(interp.context.num_scopes());
                    self.respond(&request, json!({}));
                    return DebugControl::Resume;
                }
                "disconnect" | "terminate" => {
                    self.terminate = true;
                    self.respond(&request, json!({}));
                    return DebugControl::Terminate;
                }
                // Read-only inspection requests are answered in place; we stay
                // stopped and keep looping.
                _ => self.handle_inspect(interp, &request, &command),
            }
        }
    }

    /// Handle the inspection requests that are valid while stopped (and a few,
    /// like `setBreakpoints`, that may arrive at any time).
    fn handle_inspect(&mut self, interp: &mut Interpreter, request: &Value, command: &str) {
        match command {
            "threads" => self.respond(request, proto::threads_body()),
            "stackTrace" => self.handle_stack_trace(interp, request),
            "scopes" => self.handle_scopes(request),
            "variables" => self.handle_variables(interp, request),
            "evaluate" => self.handle_evaluate(interp, request),
            "setBreakpoints" => self.handle_set_breakpoints(request),
            "pause" => {
                // Already stopped; just acknowledge.
                self.respond(request, json!({}));
            }
            other => {
                self.respond_err(
                    request,
                    &format!("unsupported request while stopped: {other}"),
                );
            }
        }
    }

    /// Build the call stack, innermost frame first (DAP convention).
    fn handle_stack_trace(&mut self, interp: &mut Interpreter, request: &Value) {
        let body = proto::stack_trace_body(interp, &self.program_name(), &self.program_path);
        self.respond(request, body);
    }

    /// One "Locals" scope per frame, via the shared frame allocator.
    fn handle_scopes(&mut self, request: &Value) {
        let frame_id = request
            .get("arguments")
            .and_then(|a| a.get("frameId"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let body = self.frames.scopes_body(frame_id);
        self.respond(request, body);
    }

    /// List the locals of the frame a `variablesReference` points at.
    fn handle_variables(&mut self, interp: &mut Interpreter, request: &Value) {
        let var_ref = request
            .get("arguments")
            .and_then(|a| a.get("variablesReference"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let body = self.frames.variables_body(interp, var_ref);
        self.respond(request, body);
    }

    /// Evaluate an expression in the current (top) frame.
    fn handle_evaluate(&mut self, interp: &mut Interpreter, request: &Value) {
        let expr = request
            .get("arguments")
            .and_then(|a| a.get("expression"))
            .and_then(Value::as_str)
            .unwrap_or("");
        match proto::evaluate_body(interp, expr) {
            Ok(body) => self.respond(request, body),
            Err(e) => self.respond_err(request, &e),
        }
    }

    /// Replace the breakpoint set with the lines from a `setBreakpoints`
    /// request, and confirm each as verified.
    fn handle_set_breakpoints(&mut self, request: &Value) {
        let args = request.get("arguments").cloned().unwrap_or(Value::Null);
        let lines = proto::breakpoint_lines(&args);
        self.breakpoints = lines.iter().copied().collect();
        self.respond(request, proto::verified_breakpoints_body(&lines));
    }

    fn program_name(&self) -> String {
        std::path::Path::new(&self.program_path)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "program".to_string())
    }
}

// ---- top-level driver -------------------------------------------------------

/// Serve a single debug session to completion over the given transport, running
/// a fresh interpreter with the full standard library.
///
/// Returns once the client disconnects after the program terminates.
///
/// # Errors
/// Propagates a transport I/O error from the initial handshake.
pub fn serve(reader: Box<dyn BufRead>, writer: Box<dyn Write>) -> std::io::Result<()> {
    let mut interp = Interpreter::new();
    fm_builtins::register_standard_library(&mut interp);
    let session = Rc::new(RefCell::new(Session::new(reader, writer)));
    run_session(&session, &mut interp)
}

/// Drive one session against `interp`: handshake → run → terminate. Exposed
/// (crate-internal) so tests can supply their own interpreter/transport.
fn run_session(session: &Rc<RefCell<Session>>, interp: &mut Interpreter) -> std::io::Result<()> {
    // Phase 1: configuration handshake. Returns the program source to run, or
    // `None` if the client disconnected before launching anything.
    let program = handshake(session)?;
    let Some(program_src) = program else {
        return Ok(());
    };

    // Phase 2: run the program with the hook installed. A clean
    // `stopped`/resume cycle happens inside the interpreter via the hook.
    interp.set_debugger(Rc::new(Hook(session.clone())));
    let result = interp.run(&program_src);
    interp.clear_debugger();

    // Phase 3: report any uncaught error, then terminate. If the client already
    // asked to disconnect (consumed inside the stopped loop), the run unwound
    // via `terminate` and there is nothing left to drain — draining would block
    // forever waiting for a second `disconnect`.
    let already_disconnected = {
        let mut s = session.borrow_mut();
        s.flush_output(interp); // any output produced after the last stop
        if let Err(e) = result {
            s.output(&format!("Error: {e}\n"));
        }
        s.event("terminated", json!({}));
        s.event("exited", json!({ "exitCode": 0 }));
        s.terminate
    };
    if !already_disconnected {
        drain_until_disconnect(session);
    }
    Ok(())
}

/// Run the configuration handshake. Handles `initialize`, `launch`,
/// `setBreakpoints`, and `configurationDone`, returning the program source once
/// the client has both launched and finished configuration.
fn handshake(session: &Rc<RefCell<Session>>) -> std::io::Result<Option<String>> {
    let mut launched: Option<String> = None;
    let mut configuration_done = false;
    let mut stop_on_entry = false;

    loop {
        let request = {
            let mut s = session.borrow_mut();
            match read_message(&mut s.reader)? {
                Some(v) => v,
                None => return Ok(None), // client closed before launching
            }
        };
        let command = request
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let mut s = session.borrow_mut();
        match command.as_str() {
            "initialize" => {
                s.respond(&request, capabilities());
                // Tell the client it may now send breakpoints/config.
                s.event("initialized", json!({}));
            }
            "launch" | "attach" => {
                let args = request.get("arguments").cloned().unwrap_or(Value::Null);
                stop_on_entry = args
                    .get("stopOnEntry")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                match load_program(&args) {
                    Ok((path, src)) => {
                        s.program_path = path;
                        s.program_src = src.clone();
                        launched = Some(src);
                        s.respond(&request, json!({}));
                    }
                    Err(e) => s.respond_err(&request, &e),
                }
            }
            "setBreakpoints" => s.handle_set_breakpoints(&request),
            "setExceptionBreakpoints" => s.respond(&request, json!({ "filters": [] })),
            "configurationDone" => {
                configuration_done = true;
                s.respond(&request, json!({}));
            }
            "threads" => {
                s.respond(
                    &request,
                    json!({ "threads": [{ "id": MAIN_THREAD, "name": "main" }] }),
                );
            }
            "disconnect" => {
                s.respond(&request, json!({}));
                return Ok(None);
            }
            other => s.respond_err(&request, &format!("unexpected during setup: {other}")),
        }
        drop(s);

        if configuration_done && launched.is_some() {
            // Honour stop-on-entry by arming a step that fires at the first
            // statement.
            if stop_on_entry {
                session.borrow_mut().mode = RunMode::StepIn;
            }
            return Ok(launched);
        }
    }
}

/// After the program ends, answer requests until the client disconnects so the
/// shutdown is clean.
fn drain_until_disconnect(session: &Rc<RefCell<Session>>) {
    loop {
        let request = {
            let mut s = session.borrow_mut();
            match read_message(&mut s.reader) {
                Ok(Some(v)) => v,
                Ok(None) | Err(_) => return,
            }
        };
        let command = request.get("command").and_then(Value::as_str).unwrap_or("");
        let mut s = session.borrow_mut();
        match command {
            "disconnect" | "terminate" => {
                s.respond(&request, json!({}));
                return;
            }
            "threads" => s.respond(
                &request,
                json!({ "threads": [{ "id": MAIN_THREAD, "name": "main" }] }),
            ),
            _ => s.respond(&request, json!({})),
        }
    }
}

/// The capabilities we advertise in the `initialize` response.
fn capabilities() -> Value {
    proto::capabilities()
}

/// Resolve the program source from `launch` arguments. Supports an inline
/// `source` string (handy for tests) or a `program`/`path` file to read.
fn load_program(args: &Value) -> Result<(String, String), String> {
    if let Some(src) = args.get("source").and_then(Value::as_str) {
        let path = args
            .get("program")
            .and_then(Value::as_str)
            .unwrap_or("<inline>")
            .to_string();
        return Ok((path, src.to_string()));
    }
    let path = args
        .get("program")
        .or_else(|| args.get("path"))
        .and_then(Value::as_str)
        .ok_or_else(|| "launch is missing a `program` path or inline `source`".to_string())?;
    let src = std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    Ok((path.to_string(), src))
}
