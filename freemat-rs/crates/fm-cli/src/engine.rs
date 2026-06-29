//! The interpreter **engine**: a single-threaded actor that owns the
//! [`Interpreter`] and serves it to clients over channels.
//!
//! Part of the interpreter-as-service plan (`docs/INTERP_SERVICE_PLAN.md`).
//!
//! - **Phase 0** put the interpreter on its own thread, reachable only by
//!   [`ReplCommand`] message, so the REPL is just one client.
//! - **Phase 2** adds a *second* client: an embedded DAP debugger over TCP. A
//!   socket reader thread forwards DAP requests into the *same* inbox the REPL
//!   feeds ([`EngineMsg`]), so the engine processes REPL evals and debugger
//!   traffic one at a time with **no locking on interpreter state**. When a run
//!   (triggered by a REPL `Eval`) hits a breakpoint, the engine's debug hook
//!   ([`EngineHook`]) stops *in place* and services the debugger until it
//!   resumes — debugging the live REPL session.
//!
//! The interpreter never leaves the engine thread. Only `Send` messages cross
//! the channel: `String`/[`InterpError`] for the REPL, and `serde_json::Value`
//! (plus the socket's write half) for the debugger.

use std::cell::RefCell;
use std::collections::HashSet;
use std::io::{BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;

use fm_dap::proto;
use fm_interp::debug::{DebugControl, DebugHook, ResumeKind};
use fm_interp::{InterpError, Interpreter};
use serde_json::{Value, json};

// ---- client-facing message types -------------------------------------------

/// A command from the REPL (or any non-debugger client).
enum ReplCommand {
    /// Parse and run a source line at the top level; reply with output + error.
    Eval {
        source: String,
        reply: Sender<EvalReply>,
    },
    /// Answer a read-only query about interpreter state (e.g. completion).
    Query {
        kind: QueryKind,
        reply: Sender<QueryResult>,
    },
    /// Stop the engine loop and let the thread exit.
    Shutdown,
}

/// The result of an [`Engine::eval`]: buffered output plus an optional error.
pub struct EvalOutcome {
    /// Everything echoed/`disp`-ed during the run (printed even on error).
    pub output: String,
    /// The runtime error, if the run raised one.
    pub error: Option<InterpError>,
}

/// One reply from the engine for a single eval. A run can produce *several* of
/// these on the same channel: a breakpoint sends a [`EvalReply::Stopped`] and
/// then (after resuming) a final [`EvalReply::Done`].
///
/// The interactive terminal driver (`main.rs`) interprets all three variants to
/// drive the `K>>` prompt; programmatic callers ([`Engine::eval`]) collapse them
/// back into a single [`EvalOutcome`].
pub enum EvalReply {
    /// The run finished (or errored). A terminal reply — nothing follows.
    Done(EvalOutcome),
    /// The run hit a breakpoint / step / pause and is now waiting at the `K>>`
    /// prompt. More replies follow on the same channel once it resumes.
    Stopped(StopInfo),
    /// A resume command (`dbcont`/`dbstep`/`dbquit`) ran in the paused frame:
    /// this `K>>` level is releasing. A terminal reply for *this* nested send.
    Resumed,
}

/// Where (and why) a run paused, for the terminal `K>>` banner.
pub struct StopInfo {
    /// Output produced since the previous reply (print it before the prompt).
    pub output: String,
    /// The function the run stopped in (`""` = the top-level program).
    pub function: String,
    /// The 1-based source line the run stopped on.
    pub line: usize,
    /// Nesting depth of the stop (1 = first `K>>`, 2 = a nested breakpoint, …).
    pub level: usize,
    /// Why the run stopped: `"breakpoint"`, `"step"`, or `"pause"`.
    pub reason: String,
}

/// A read-only query against the engine.
pub enum QueryKind {
    /// Every registered function/builtin name (for completion).
    FunctionNames,
}

/// The answer to a [`QueryKind`].
pub enum QueryResult {
    /// Names from [`QueryKind::FunctionNames`].
    Names(Vec<String>),
}

/// A message into the engine's single inbox. The REPL and (when a debugger is
/// attached) the DAP socket reader both feed this one channel.
enum EngineMsg {
    /// Traffic from the REPL / a [`EngineClient`].
    Repl(ReplCommand),
    /// A DAP request forwarded verbatim by the socket reader thread.
    Dap(Value),
    /// A debugger connected; carries the socket's write half.
    DapConnect(Box<dyn Write + Send>),
    /// The debugger socket closed.
    DapDisconnect,
}

// ---- the engine handle ------------------------------------------------------

/// A handle to the running engine. Owns the thread join handle and shuts the
/// engine down on drop.
pub struct Engine {
    tx: Sender<EngineMsg>,
    handle: Option<JoinHandle<()>>,
    /// Cooperative interrupt flag (Ctrl-C). Raising it aborts the in-flight run
    /// at the next statement. Shared with the interpreter on the engine thread.
    interrupt: Arc<AtomicBool>,
}

impl Engine {
    /// Spawn a plain engine (no debugger). Zero per-statement overhead — the
    /// debug hook is not even installed.
    pub fn spawn<F>(setup: F) -> Self
    where
        F: FnOnce(&mut Interpreter) + Send + 'static,
    {
        Self::spawn_inner(None, setup).expect("spawn interpreter engine thread")
    }

    /// Spawn an engine with an embedded DAP "attach" server listening on
    /// `127.0.0.1:<port>` (use `0` for an ephemeral port). Returns the engine
    /// and the actual bound port. A debugger that connects can set breakpoints
    /// and step the live REPL session.
    ///
    /// # Errors
    /// Fails if the TCP port cannot be bound or a thread cannot be spawned.
    pub fn spawn_with_dap<F>(port: u16, setup: F) -> std::io::Result<(Self, u16)>
    where
        F: FnOnce(&mut Interpreter) + Send + 'static,
    {
        let listener = TcpListener::bind(("127.0.0.1", port))?;
        let actual = listener.local_addr()?.port();
        let engine = Self::spawn_inner(Some(listener), setup)?;
        Ok((engine, actual))
    }

    fn spawn_inner<F>(listener: Option<TcpListener>, setup: F) -> std::io::Result<Self>
    where
        F: FnOnce(&mut Interpreter) + Send + 'static,
    {
        let (tx, rx) = channel::<EngineMsg>();
        let interrupt = Arc::new(AtomicBool::new(false));
        // Async pause request (DAP `pause`): the socket-reader thread raises it,
        // the debug hook honours it at the next statement.
        let pause = Arc::new(AtomicBool::new(false));
        let thread_interrupt = interrupt.clone();
        let thread_pause = pause.clone();
        let handle = std::thread::Builder::new()
            .name("fm-interp".to_string())
            .spawn(move || engine_thread(rx, thread_interrupt, thread_pause, Box::new(setup)))?;
        if let Some(listener) = listener {
            let tx = tx.clone();
            std::thread::Builder::new()
                .name("fm-dap-accept".to_string())
                .spawn(move || run_acceptor(&listener, &tx, &pause))?;
        }
        Ok(Engine {
            tx,
            handle: Some(handle),
            interrupt,
        })
    }

    /// The cooperative interrupt flag. Set it (e.g. from a Ctrl-C handler) to
    /// abort the in-flight run at the next statement.
    #[must_use]
    pub fn interrupt_flag(&self) -> Arc<AtomicBool> {
        self.interrupt.clone()
    }

    /// Run one source line, blocking until the run fully finishes. If the run
    /// pauses at a breakpoint with no one driving the `K>>` prompt it will block
    /// until another client resumes it; for the interactive, debugger-aware path
    /// use [`Self::send_eval_raw`] instead.
    pub fn eval(&self, source: impl Into<String>) -> EvalOutcome {
        send_eval(&self.tx, source.into())
    }

    /// Send a line to run and return the raw reply channel, for the interactive
    /// terminal driver: it interprets [`EvalReply::Stopped`] to open a `K>>`
    /// prompt and [`EvalReply::Resumed`] to close it. Most callers want
    /// [`Self::eval`].
    pub fn send_eval_raw(&self, source: impl Into<String>) -> Receiver<EvalReply> {
        send_eval_raw(&self.tx, source.into())
    }

    /// Every registered function/builtin name (for tab-completion).
    pub fn function_names(&self) -> Vec<String> {
        let (reply, rx) = channel();
        let cmd = EngineMsg::Repl(ReplCommand::Query {
            kind: QueryKind::FunctionNames,
            reply,
        });
        if self.tx.send(cmd).is_err() {
            return Vec::new();
        }
        match rx.recv() {
            Ok(QueryResult::Names(names)) => names,
            Err(_) => Vec::new(),
        }
    }

    /// A cheap, `Send` client handle that can drive the engine from another
    /// thread (e.g. a test that runs an `Eval` while also driving a DAP client).
    pub fn client(&self) -> EngineClient {
        EngineClient {
            tx: self.tx.clone(),
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.tx.send(EngineMsg::Repl(ReplCommand::Shutdown));
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// A cloneable handle to send `Eval`s to the engine from another thread.
pub struct EngineClient {
    tx: Sender<EngineMsg>,
}

impl EngineClient {
    /// Run one source line, blocking until the engine replies.
    pub fn eval(&self, source: impl Into<String>) -> EvalOutcome {
        send_eval(&self.tx, source.into())
    }
}

/// Send a line to run and return the raw reply channel. On a dead engine,
/// returns a channel pre-loaded with a `Done` error so callers never hang.
fn send_eval_raw(tx: &Sender<EngineMsg>, source: String) -> Receiver<EvalReply> {
    let (reply, rx) = channel();
    let cmd = EngineMsg::Repl(ReplCommand::Eval { source, reply });
    if tx.send(cmd).is_err() {
        let (dead, dead_rx) = channel();
        let _ = dead.send(EvalReply::Done(EvalOutcome {
            output: String::new(),
            error: Some(InterpError::msg("interpreter engine is not running")),
        }));
        return dead_rx;
    }
    rx
}

/// Drive an eval to completion, collapsing the `Stopped`/`Resumed`/`Done`
/// stream into one [`EvalOutcome`] (accumulating output across stops). Used by
/// the blocking [`Engine::eval`] / [`EngineClient::eval`] — a programmatic
/// caller does not open a `K>>` prompt, so it just waits for the run to finish.
fn send_eval(tx: &Sender<EngineMsg>, source: String) -> EvalOutcome {
    let rx = send_eval_raw(tx, source);
    let mut output = String::new();
    loop {
        match rx.recv() {
            Ok(EvalReply::Done(o)) => {
                output.push_str(&o.output);
                return EvalOutcome {
                    output,
                    error: o.error,
                };
            }
            Ok(EvalReply::Stopped(info)) => output.push_str(&info.output),
            Ok(EvalReply::Resumed) => {
                return EvalOutcome {
                    output,
                    error: None,
                };
            }
            Err(_) => {
                return EvalOutcome {
                    output,
                    error: Some(InterpError::msg(
                        "interpreter engine stopped before replying",
                    )),
                };
            }
        }
    }
}

// ---- the engine thread ------------------------------------------------------

fn engine_thread(
    rx: Receiver<EngineMsg>,
    interrupt: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
    setup: Box<dyn FnOnce(&mut Interpreter) + Send>,
) {
    let mut interp = Interpreter::new();
    fm_builtins::register_standard_library(&mut interp);
    interp.set_interrupt(interrupt);
    setup(&mut interp);

    // The inbox is shared (single-thread `Rc`) between this main loop and the
    // debug hook, which reads it while stopped at a breakpoint.
    let inbox = Rc::new(rx);
    let state = Rc::new(RefCell::new(DebugState::new(pause)));
    // The hook is installed unconditionally: terminal `dbstop` breakpoints work
    // even without a DAP client. When nothing is armed (no breakpoints, no step,
    // no DAP writer) `on_statement` takes a cheap early-out and resumes.
    interp.set_debugger(Rc::new(EngineHook {
        inbox: inbox.clone(),
        state: state.clone(),
    }));
    serve(&inbox, &mut interp, &state);
}

/// The engine's main (idle) loop: process one inbox message at a time, then
/// drain any REPL evals deferred while stopped.
fn serve(
    inbox: &Rc<Receiver<EngineMsg>>,
    interp: &mut Interpreter,
    state: &Rc<RefCell<DebugState>>,
) {
    loop {
        let Ok(msg) = inbox.recv() else { break };
        if !process(msg, interp, state) {
            break;
        }
    }
}

/// Handle one inbox message. Returns `false` on shutdown.
fn process(msg: EngineMsg, interp: &mut Interpreter, state: &Rc<RefCell<DebugState>>) -> bool {
    match msg {
        EngineMsg::Repl(ReplCommand::Eval { source, reply }) => {
            run_eval(interp, state, source, reply)
        }
        EngineMsg::Repl(ReplCommand::Query { kind, reply }) => answer_query(interp, kind, reply),
        EngineMsg::Repl(ReplCommand::Shutdown) => return false,
        EngineMsg::Dap(req) => handle_dap_idle(&req, interp, state),
        EngineMsg::DapConnect(writer) => state.borrow_mut().attach(writer),
        EngineMsg::DapDisconnect => state.borrow_mut().detach(),
    }
    true
}

fn run_eval(
    interp: &mut Interpreter,
    state: &Rc<RefCell<DebugState>>,
    source: String,
    reply: Sender<EvalReply>,
) {
    {
        // Record the top-level source so the hook can tell program statements
        // (breakpoint-eligible) from statements inside called functions. Reset
        // the run-output accumulator and any stale pause request. Push the reply
        // channel so the hook can send a `Stopped` notification on it if the run
        // pauses at a breakpoint.
        let mut s = state.borrow_mut();
        s.program_src = source.clone();
        s.run_output.clear();
        let _ = s.pause_requested();
        s.reply_stack.push(reply.clone());
    }
    // Isolate the run: a panic in a builtin must not take down the engine thread
    // (which would hang every client). Catch it, recover the interpreter, and
    // report it as an error.
    let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| interp.run(&source)));
    let error = match run {
        Ok(result) => result.err(),
        Err(panic) => {
            interp.reset_run_state();
            let message = panic_message(panic.as_ref());
            let mut s = state.borrow_mut();
            if s.writer.is_some() {
                // Tell the debugger the session blew up so it can disconnect.
                s.emit_output(&format!("Internal error: {message}\n"));
                s.event("terminated", json!({}));
            }
            Some(InterpError::with_id(
                "FreeMat:internal",
                format!("internal error: {message}"),
            ))
        }
    };
    // Flush any output produced since the last stop to the console, then return
    // the output accumulated since the last reply to the REPL.
    drain_output(interp, state);
    let output = {
        let mut s = state.borrow_mut();
        s.reply_stack.pop();
        std::mem::take(&mut s.run_output)
    };
    let _ = reply.send(EvalReply::Done(EvalOutcome { output, error }));
}

/// Extract a human-readable message from a caught panic payload.
fn panic_message(panic: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = panic.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else {
        "panicked".to_string()
    }
}

/// Take the interpreter's pending output, append it to the current run's
/// accumulator, and (if a debugger is attached) mirror it to the debug console
/// as an `output` event.
fn drain_output(interp: &mut Interpreter, state: &Rc<RefCell<DebugState>>) {
    let chunk = interp.take_output();
    if chunk.is_empty() {
        return;
    }
    let mut s = state.borrow_mut();
    s.run_output.push_str(&chunk);
    if s.writer.is_some() {
        s.emit_output(&chunk);
    }
}

/// Run `source` in the **current (paused) frame** as a nested debugger-REPL
/// line. The line executes against the live suspended scope — it reads and
/// mutates the same variables — and breakpoints inside it fire (the re-entrant
/// seam pushes a deeper stop). Run mode is forced to `Continue` for the nested
/// line so an outer step doesn't immediately re-trip on its first statement, and
/// the outer program's buffered output is preserved across the nested run.
fn run_nested(
    interp: &mut Interpreter,
    state: &Rc<RefCell<DebugState>>,
    source: &str,
) -> (String, Option<InterpError>) {
    let outer_output = interp.take_output();
    // Terminal breakpoints/steps are suppressed at this frame's depth so the
    // literal command line doesn't re-trip a breakpoint on its own coinciding
    // line number; they still fire in functions the command calls (deeper).
    let floor = interp.context.num_scopes();
    let (saved_src, saved_mode, saved_floor) = {
        let mut s = state.borrow_mut();
        let saved = (
            std::mem::replace(&mut s.program_src, source.to_string()),
            s.mode,
            s.nested_floor.replace(floor),
        );
        s.mode = RunMode::Continue;
        saved
    };
    let error = interp.run(source).err();
    let output = interp.take_output();
    {
        let mut s = state.borrow_mut();
        s.program_src = saved_src;
        s.mode = saved_mode;
        s.nested_floor = saved_floor;
    }
    interp.emit(&outer_output); // restore the outer run's pending output
    (output, error)
}

/// Run `source` in the current frame with the debugger seam suppressed (no
/// nested breakpoints). Used by `evaluate` with `context: "repl"` — the IDE
/// debug console — so typing a statement there mutates the paused frame without
/// emitting a nested `stopped` event the IDE wouldn't expect.
fn run_in_frame_suppressed(
    interp: &mut Interpreter,
    source: &str,
) -> (String, Option<InterpError>) {
    let outer_output = interp.take_output();
    let error = interp.eval_suppressed(|i| i.run(source)).err();
    let output = interp.take_output();
    interp.emit(&outer_output);
    (output, error)
}

fn answer_query(interp: &Interpreter, kind: QueryKind, reply: Sender<QueryResult>) {
    let result = match kind {
        QueryKind::FunctionNames => QueryResult::Names(interp.functions.names()),
    };
    let _ = reply.send(result);
}

// ---- the debug hook (runs at the statement seam) ---------------------------

/// The debugger attached to the engine. Holds shared handles to the inbox (read
/// while stopped) and the debug state (breakpoints, run mode, socket writer).
struct EngineHook {
    inbox: Rc<Receiver<EngineMsg>>,
    state: Rc<RefCell<DebugState>>,
}

impl DebugHook for EngineHook {
    fn on_statement(&self, interp: &mut Interpreter, line: usize, src: &str) -> DebugControl {
        let depth = interp.context.num_scopes();

        // ---- terminal debugging (`dbstop`/`dbstep`): works with or without a
        // DAP client attached. Keyed by the executing function's name (`""` for
        // the top-level program), so a breakpoint fires inside a called `.m`.
        // Suppressed at/above a nested `K>>` command's own frame depth, so the
        // command line never re-trips a breakpoint on its own line number.
        let term_reason = if self.state.borrow().nested_floor.is_some_and(|f| depth <= f) {
            None
        } else {
            let func = interp.context.top().name.to_string();
            let session = interp.debug_session();
            let mut s = session.borrow_mut();
            let reason = if s.step.is_some_and(|d| depth <= d) {
                Some("step")
            } else if s.breakpoint_hit(&func, line) {
                Some("breakpoint")
            } else {
                None
            };
            if reason.is_some() {
                s.step = None; // a single-step is consumed by the stop it causes
            }
            reason
        };

        // ---- DAP (IDE) debugging: only active while a client is attached. ----
        let dap_reason = {
            let s = self.state.borrow();
            if s.writer.is_none() {
                None
            } else if s.terminate {
                return DebugControl::Terminate;
            } else {
                let paused = s.pause_requested(); // async DAP `pause`
                let hit = src == s.program_src && s.breakpoints.contains(&line);
                let stepped = match s.mode {
                    RunMode::Continue => false,
                    RunMode::StepIn => true,
                    RunMode::StepOver(d) => depth <= d,
                    RunMode::StepOut(d) => depth < d,
                };
                if hit {
                    Some("breakpoint")
                } else if paused {
                    Some("pause")
                } else if stepped {
                    Some("step")
                } else {
                    None
                }
            }
        };

        match term_reason.or(dap_reason) {
            Some(reason) => self.stopped_loop(interp, reason),
            None => DebugControl::Resume,
        }
    }
}

impl EngineHook {
    /// Sit at a stop point: announce it (with the nesting level), service the
    /// debugger until it resumes, then unwind one pause level. Because
    /// [`Self::serve_stopped`] runs nested REPL evals through the live seam, this
    /// can be entered recursively — a breakpoint hit inside a nested `K>>`
    /// command pushes another level, and resuming it pops back here.
    fn stopped_loop(&self, interp: &mut Interpreter, reason: &str) -> DebugControl {
        // Flush output produced up to this stop to the debug console first.
        drain_output(interp, &self.state);
        // The terminal location of the stop, for the `K>>` banner.
        let (function, line) = {
            let top = interp.context.top();
            (top.name.clone(), top.current_line.unwrap_or(0))
        };
        let (output, term_reply, level) = {
            let mut s = self.state.borrow_mut();
            s.pause_level += 1;
            s.frames.reset();
            let level = s.pause_level;
            // `fmPauseLevel` is a non-standard hint (clients ignore unknown
            // fields) that surfaces the nested `K>>` depth.
            let mut body = proto::stopped_body(reason);
            body["fmPauseLevel"] = json!(level);
            s.event("stopped", body);
            // Hand the terminal driver the output up to this stop + the location.
            let output = std::mem::take(&mut s.run_output);
            (output, s.reply_stack.last().cloned(), level)
        };
        // Mark the session paused so the `db*` resume builtins are valid.
        interp.debug_session().borrow_mut().paused = true;
        if let Some(reply) = term_reply {
            let _ = reply.send(EvalReply::Stopped(StopInfo {
                output,
                function,
                line,
                level,
                reason: reason.to_string(),
            }));
        }
        let control = self.serve_stopped(interp);
        let level_now = {
            let mut s = self.state.borrow_mut();
            s.pause_level -= 1;
            s.pause_level
        };
        // Still paused only if an outer `K>>` level remains on the stack.
        interp.debug_session().borrow_mut().paused = level_now > 0;
        control
    }

    /// The stopped request loop: service the debugger and run nested REPL evals
    /// in the paused frame until the client resumes (or disconnects).
    fn serve_stopped(&self, interp: &mut Interpreter) -> DebugControl {
        loop {
            let Ok(msg) = self.inbox.recv() else {
                return DebugControl::Terminate;
            };
            match msg {
                EngineMsg::Dap(req) => {
                    if let Some(control) = self.handle_stopped_dap(interp, &req) {
                        return control;
                    }
                }
                EngineMsg::Repl(ReplCommand::Eval { source, reply }) => {
                    // The nested debugger REPL: run the line in the *paused
                    // frame's* live context. It sees and mutates the suspended
                    // scope, and (via the re-entrant seam) a breakpoint inside it
                    // pushes a deeper stop.
                    self.state.borrow_mut().reply_stack.push(reply.clone());
                    let (output, error) = run_nested(interp, &self.state, &source);
                    self.state.borrow_mut().reply_stack.pop();
                    // A `dbcont`/`dbstep`/`dbquit` run in the frame asks the run
                    // to resume: pop this `K>>` level and act on it.
                    let resume = interp.debug_session().borrow_mut().resume.take();
                    if let Some(kind) = resume {
                        let _ = reply.send(EvalReply::Resumed);
                        return self.apply_resume(interp, kind);
                    }
                    let _ = reply.send(EvalReply::Done(EvalOutcome { output, error }));
                }
                EngineMsg::Repl(ReplCommand::Query { kind, reply }) => {
                    answer_query(interp, kind, reply);
                }
                EngineMsg::Repl(ReplCommand::Shutdown) => return DebugControl::Terminate,
                EngineMsg::DapConnect(writer) => self.state.borrow_mut().attach(writer),
                EngineMsg::DapDisconnect => {
                    self.state.borrow_mut().detach();
                    return DebugControl::Terminate;
                }
            }
        }
    }

    /// Apply a terminal resume request (`dbcont`/`dbstep`/`dbquit`) issued at the
    /// `K>>` prompt: set the run mode (and, for `dbstep`, arm a single-step at
    /// the current call depth) and return the control the seam should obey.
    fn apply_resume(&self, interp: &mut Interpreter, kind: ResumeKind) -> DebugControl {
        match kind {
            ResumeKind::Continue => {
                self.state.borrow_mut().mode = RunMode::Continue;
                DebugControl::Resume
            }
            ResumeKind::Quit => {
                self.state.borrow_mut().terminate = true;
                DebugControl::Terminate
            }
            ResumeKind::Step => {
                self.state.borrow_mut().mode = RunMode::Continue;
                // Stop again at the next statement at this depth or shallower.
                let depth = interp.context.num_scopes();
                interp.debug_session().borrow_mut().step = Some(depth);
                DebugControl::Resume
            }
        }
    }

    /// Handle a DAP request received while stopped. Returns `Some(control)` for
    /// the requests that resume/abort the run, `None` for inspection requests
    /// (which keep us stopped).
    fn handle_stopped_dap(&self, interp: &mut Interpreter, req: &Value) -> Option<DebugControl> {
        match proto::command_of(req) {
            "continue" => {
                self.state.borrow_mut().mode = RunMode::Continue;
                self.state
                    .borrow_mut()
                    .respond(req, json!({ "allThreadsContinued": true }));
                Some(DebugControl::Resume)
            }
            "next" => {
                let depth = interp.context.num_scopes();
                self.state.borrow_mut().mode = RunMode::StepOver(depth);
                self.state.borrow_mut().respond(req, json!({}));
                Some(DebugControl::Resume)
            }
            "stepIn" => {
                self.state.borrow_mut().mode = RunMode::StepIn;
                self.state.borrow_mut().respond(req, json!({}));
                Some(DebugControl::Resume)
            }
            "stepOut" => {
                let depth = interp.context.num_scopes();
                self.state.borrow_mut().mode = RunMode::StepOut(depth);
                self.state.borrow_mut().respond(req, json!({}));
                Some(DebugControl::Resume)
            }
            "disconnect" | "terminate" => {
                let mut s = self.state.borrow_mut();
                s.terminate = true;
                s.respond(req, json!({}));
                Some(DebugControl::Terminate)
            }
            _ => {
                handle_inspect(req, interp, &self.state);
                None
            }
        }
    }
}

// ---- DAP request handling (shared by idle + stopped paths) ------------------

/// Handle a DAP request while the engine is **idle** (not stopped): the
/// configuration handshake plus inspection requests.
fn handle_dap_idle(req: &Value, interp: &mut Interpreter, state: &Rc<RefCell<DebugState>>) {
    match proto::command_of(req) {
        "initialize" => {
            let mut s = state.borrow_mut();
            s.respond(req, proto::capabilities());
            s.event("initialized", json!({}));
        }
        "attach" | "launch" => state.borrow_mut().respond(req, json!({})),
        "setBreakpoints" => set_breakpoints(req, state),
        "setExceptionBreakpoints" => state.borrow_mut().respond(req, json!({ "filters": [] })),
        "configurationDone" => state.borrow_mut().respond(req, json!({})),
        "disconnect" | "terminate" => {
            state.borrow_mut().respond(req, json!({}));
            state.borrow_mut().detach();
        }
        _ => handle_inspect(req, interp, state),
    }
}

/// Inspection requests valid in any state (answered against the live
/// interpreter). `setBreakpoints` is allowed here too — clients may edit
/// breakpoints while stopped.
fn handle_inspect(req: &Value, interp: &mut Interpreter, state: &Rc<RefCell<DebugState>>) {
    match proto::command_of(req) {
        "threads" => state.borrow_mut().respond(req, proto::threads_body()),
        "stackTrace" => {
            let (name, path) = {
                let s = state.borrow();
                (s.source_name(), s.source_path.clone())
            };
            let body = proto::stack_trace_body(interp, &name, &path);
            state.borrow_mut().respond(req, body);
        }
        "scopes" => {
            let frame_id = req
                .get("arguments")
                .and_then(|a| a.get("frameId"))
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let body = state.borrow_mut().frames.scopes_body(frame_id);
            state.borrow_mut().respond(req, body);
        }
        "variables" => {
            let var_ref = req
                .get("arguments")
                .and_then(|a| a.get("variablesReference"))
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let body = state.borrow().frames.variables_body(interp, var_ref);
            state.borrow_mut().respond(req, body);
        }
        "evaluate" => {
            let args = req.get("arguments");
            let expr = args
                .and_then(|a| a.get("expression"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let context = args
                .and_then(|a| a.get("context"))
                .and_then(Value::as_str)
                .unwrap_or("");
            if context == "repl" {
                // Debug-console line: run it as a statement in the paused frame
                // (mutations stick), with breakpoints suppressed so it doesn't
                // emit a nested stop the IDE wouldn't expect.
                let (output, error) = run_in_frame_suppressed(interp, &expr);
                match error {
                    None => state.borrow_mut().respond(
                        req,
                        json!({ "result": output.trim_end_matches('\n'), "variablesReference": 0 }),
                    ),
                    Some(e) => state.borrow_mut().respond_err(req, &e.message),
                }
            } else {
                // Hover / watch: evaluate an expression (suppressed, no side effects beyond eval).
                match proto::evaluate_body(interp, &expr) {
                    Ok(body) => state.borrow_mut().respond(req, body),
                    Err(e) => state.borrow_mut().respond_err(req, &e),
                }
            }
        }
        "setBreakpoints" => set_breakpoints(req, state),
        "pause" => {
            // The reader already raised the pause flag out-of-band; by the time
            // we handle the request here we're either already stopped or idle,
            // so just consume any lingering flag and acknowledge.
            let mut s = state.borrow_mut();
            let _ = s.pause_requested();
            s.respond(req, json!({}));
        }
        other => state
            .borrow_mut()
            .respond_err(req, &format!("unsupported DAP request: {other}")),
    }
}

fn set_breakpoints(req: &Value, state: &Rc<RefCell<DebugState>>) {
    let args = req.get("arguments").cloned().unwrap_or(Value::Null);
    let lines = proto::breakpoint_lines(&args);
    let mut s = state.borrow_mut();
    if let Some(path) = args
        .get("source")
        .and_then(|src| src.get("path"))
        .and_then(Value::as_str)
    {
        s.source_path = path.to_string();
    }
    s.breakpoints = lines.iter().copied().collect();
    s.respond(req, proto::verified_breakpoints_body(&lines));
}

// ---- shared debug state -----------------------------------------------------

/// How execution should proceed (mirrors `fm_dap`'s private `RunMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunMode {
    Continue,
    StepIn,
    StepOver(usize),
    StepOut(usize),
}

/// All debug state, plus the socket write half. Lives behind `Rc<RefCell<…>>`
/// on the engine thread (single-threaded — no mutex), shared by the engine main
/// loop and the [`EngineHook`].
struct DebugState {
    /// The socket's write half; `None` until a debugger connects. The hook is a
    /// no-op while this is `None`.
    writer: Option<Box<dyn Write + Send>>,
    /// Outgoing message sequence counter.
    seq: i64,
    /// Breakpoint lines (1-based), set by the client.
    breakpoints: HashSet<usize>,
    /// How the current run should proceed.
    mode: RunMode,
    /// Per-stop `variablesReference` allocator.
    frames: proto::Frames,
    /// Nesting depth of active stops (the `K>>` level). 0 when running freely.
    pause_level: usize,
    /// Async pause flag, raised out-of-band by the socket reader on a `pause`
    /// request (so it can interrupt a program that's mid-run).
    pause: Arc<AtomicBool>,
    /// Accumulates the current top-level run's output (echo / `disp`) so it can
    /// be both streamed to the debug console *and* returned to the REPL.
    run_output: String,
    /// `true` once the client asked to disconnect/terminate.
    terminate: bool,
    /// The source of the currently-running top-level eval (breakpoint matching).
    program_src: String,
    /// The source path the client set breakpoints against (for stack frames).
    source_path: String,
    /// The reply channels of the eval(s) currently in flight, innermost last.
    /// The debug hook clones the top one to send a `Stopped` notification to the
    /// interactive terminal driver when a run pauses (the blocking
    /// [`Engine::eval`] wrapper just ignores it).
    reply_stack: Vec<Sender<EvalReply>>,
    /// While running a nested `K>>` command, the call depth of that command's
    /// own frame: terminal breakpoints/steps are suppressed at this depth or
    /// shallower (so the literal command line doesn't re-trip a breakpoint on
    /// its own coinciding line number) but still fire in functions it *calls*
    /// (deeper frames). `None` outside a nested command.
    nested_floor: Option<usize>,
}

impl DebugState {
    fn new(pause: Arc<AtomicBool>) -> Self {
        DebugState {
            writer: None,
            seq: 0,
            breakpoints: HashSet::new(),
            mode: RunMode::Continue,
            frames: proto::Frames::new(),
            pause_level: 0,
            pause,
            run_output: String::new(),
            terminate: false,
            program_src: String::new(),
            source_path: "fm-repl".to_string(),
            reply_stack: Vec::new(),
            nested_floor: None,
        }
    }

    /// Whether an async pause was requested; consumes the flag.
    fn pause_requested(&self) -> bool {
        use std::sync::atomic::Ordering;
        self.pause.swap(false, Ordering::Relaxed)
    }

    /// Emit a chunk of program output on the debug console (`output` event).
    fn emit_output(&mut self, text: &str) {
        self.event("output", json!({ "category": "stdout", "output": text }));
    }

    /// A debugger connected: install its writer and clear any stale stop state.
    fn attach(&mut self, writer: Box<dyn Write + Send>) {
        self.writer = Some(writer);
        self.terminate = false;
        self.mode = RunMode::Continue;
    }

    /// The debugger went away: drop the writer and disarm debugging so REPL runs
    /// stop pausing.
    fn detach(&mut self) {
        self.writer = None;
        self.breakpoints.clear();
        self.mode = RunMode::Continue;
        self.terminate = false;
        self.frames.reset();
        let _ = self.pause_requested();
    }

    fn next_seq(&mut self) -> i64 {
        self.seq += 1;
        self.seq
    }

    fn write_msg(&mut self, msg: Value) {
        if let Some(writer) = self.writer.as_mut() {
            let _ = fm_dap::write_message(writer, &msg);
        }
    }

    fn respond(&mut self, req: &Value, body: Value) {
        let seq = self.next_seq();
        self.write_msg(proto::response(req, body, seq));
    }

    fn respond_err(&mut self, req: &Value, message: &str) {
        let seq = self.next_seq();
        self.write_msg(proto::error_response(req, message, seq));
    }

    fn event(&mut self, name: &str, body: Value) {
        let seq = self.next_seq();
        self.write_msg(proto::event(name, body, seq));
    }

    /// The file name for stack-frame `source.name`.
    fn source_name(&self) -> String {
        std::path::Path::new(&self.source_path)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.source_path.clone())
    }
}

// ---- DAP socket plumbing (acceptor + reader threads) ------------------------

/// Accept debugger connections; for each, hand the engine the write half and
/// spawn a reader thread that forwards requests into the inbox.
fn run_acceptor(listener: &TcpListener, tx: &Sender<EngineMsg>, pause: &Arc<AtomicBool>) {
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let Ok(read_half) = stream.try_clone() else {
            continue;
        };
        let write_half: Box<dyn Write + Send> = Box::new(stream);
        if tx.send(EngineMsg::DapConnect(write_half)).is_err() {
            return; // engine gone
        }
        let tx = tx.clone();
        let pause = pause.clone();
        std::thread::Builder::new()
            .name("fm-dap-read".to_string())
            .spawn(move || run_reader(read_half, &tx, &pause))
            .ok();
    }
}

/// Read framed DAP requests off the socket and forward them into the inbox. A
/// `pause` request *also* raises the shared pause flag directly (out of band),
/// so it can stop a program that's mid-run — the engine thread is busy in
/// `interp.run` then and isn't reading the inbox.
fn run_reader(stream: TcpStream, tx: &Sender<EngineMsg>, pause: &Arc<AtomicBool>) {
    use std::sync::atomic::Ordering;
    let mut reader = BufReader::new(stream);
    loop {
        match fm_dap::read_message(&mut reader) {
            Ok(Some(req)) => {
                if proto::command_of(&req) == "pause" {
                    pause.store(true, Ordering::Relaxed);
                }
                if tx.send(EngineMsg::Dap(req)).is_err() {
                    return;
                }
            }
            Ok(None) | Err(_) => {
                let _ = tx.send(EngineMsg::DapDisconnect);
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_round_trips_output_and_state() {
        let engine = Engine::spawn(|_| {});
        let out = engine.eval("x = 6 * 7");
        assert!(
            out.error.is_none(),
            "unexpected error: {:?}",
            out.error.map(|e| e.message)
        );
        assert!(
            out.output.contains("42"),
            "echo missing 42: {:?}",
            out.output
        );

        let out2 = engine.eval("y = x + 1;");
        assert!(out2.error.is_none());
        let out3 = engine.eval("y");
        assert!(
            out3.output.contains("43"),
            "state not retained: {:?}",
            out3.output
        );
    }

    #[test]
    fn eval_reports_errors_without_killing_the_engine() {
        let engine = Engine::spawn(|_| {});
        let bad = engine.eval("undefined_thing_xyz");
        assert!(bad.error.is_some(), "expected an error");
        let ok = engine.eval("1 + 1");
        assert!(ok.error.is_none());
        assert!(ok.output.contains("2"));
    }

    #[test]
    fn query_answers_while_engine_is_idle() {
        let engine = Engine::spawn(|_| {});
        let names = engine.function_names();
        assert!(
            names.iter().any(|n| n == "sin"),
            "expected builtins in name list"
        );
    }

    #[test]
    fn a_panicking_builtin_does_not_kill_the_engine() {
        // Register a builtin that panics, then call it. The engine must survive
        // and keep serving.
        let engine = Engine::spawn(|interp| {
            interp
                .functions
                .add_builtin("boom", |_, _, _| panic!("kaboom"));
        });
        let out = engine.eval("boom()");
        let err = out.error.expect("a panicking run should report an error");
        assert_eq!(err.identifier.as_deref(), Some("FreeMat:internal"));
        assert!(
            err.message.contains("kaboom"),
            "lost panic message: {}",
            err.message
        );

        // The session is intact and usable.
        let ok = engine.eval("5 + 5");
        assert!(ok.error.is_none());
        assert!(ok.output.contains("10"));
    }

    #[test]
    fn interrupt_flag_aborts_a_runaway_eval() {
        use std::sync::atomic::Ordering;
        use std::time::Duration;

        let engine = Engine::spawn(|_| {});
        // Raise the interrupt shortly after the (infinite) eval starts.
        let flag = engine.interrupt_flag();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            flag.store(true, Ordering::Relaxed);
        });

        let out = engine.eval("while true\n  x = 1;\nend\n");
        let err = out.error.expect("runaway eval should be interrupted");
        assert_eq!(err.identifier.as_deref(), Some("FreeMat:interrupt"));

        // The engine survives and keeps serving.
        let ok = engine.eval("3 + 4");
        assert!(ok.error.is_none());
        assert!(ok.output.contains("7"));
    }
}
