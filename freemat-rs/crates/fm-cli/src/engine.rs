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
use std::collections::{HashSet, VecDeque};
use std::io::{BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;

use fm_dap::proto;
use fm_interp::debug::{DebugControl, DebugHook};
use fm_interp::{InterpError, Interpreter};
use serde_json::{Value, json};

// ---- client-facing message types -------------------------------------------

/// A command from the REPL (or any non-debugger client).
enum ReplCommand {
    /// Parse and run a source line at the top level; reply with output + error.
    Eval {
        source: String,
        reply: Sender<EvalOutcome>,
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
        let dap_enabled = listener.is_some();
        let handle = std::thread::Builder::new()
            .name("fm-interp".to_string())
            .spawn(move || engine_thread(rx, dap_enabled, Box::new(setup)))?;
        if let Some(listener) = listener {
            let tx = tx.clone();
            std::thread::Builder::new()
                .name("fm-dap-accept".to_string())
                .spawn(move || run_acceptor(&listener, &tx))?;
        }
        Ok(Engine {
            tx,
            handle: Some(handle),
        })
    }

    /// Run one source line, blocking until the engine replies.
    pub fn eval(&self, source: impl Into<String>) -> EvalOutcome {
        send_eval(&self.tx, source.into())
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

fn send_eval(tx: &Sender<EngineMsg>, source: String) -> EvalOutcome {
    let (reply, rx) = channel();
    let cmd = EngineMsg::Repl(ReplCommand::Eval { source, reply });
    if tx.send(cmd).is_err() {
        return EvalOutcome {
            output: String::new(),
            error: Some(InterpError::msg("interpreter engine is not running")),
        };
    }
    rx.recv().unwrap_or_else(|_| EvalOutcome {
        output: String::new(),
        error: Some(InterpError::msg(
            "interpreter engine stopped before replying",
        )),
    })
}

// ---- the engine thread ------------------------------------------------------

fn engine_thread(
    rx: Receiver<EngineMsg>,
    dap_enabled: bool,
    setup: Box<dyn FnOnce(&mut Interpreter) + Send>,
) {
    let mut interp = Interpreter::new();
    fm_builtins::register_standard_library(&mut interp);
    setup(&mut interp);

    // The inbox is shared (single-thread `Rc`) between this main loop and the
    // debug hook, which reads it while stopped at a breakpoint.
    let inbox = Rc::new(rx);
    let state = Rc::new(RefCell::new(DebugState::new()));
    if dap_enabled {
        interp.set_debugger(Rc::new(EngineHook {
            inbox: inbox.clone(),
            state: state.clone(),
        }));
    }
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
        // Run any evals that arrived (and were stashed) while we were stopped.
        loop {
            let next = state.borrow_mut().deferred.pop_front();
            match next {
                Some((source, reply)) => run_eval(interp, state, source, reply),
                None => break,
            }
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
    reply: Sender<EvalOutcome>,
) {
    // Record the top-level source so the hook can tell program statements
    // (breakpoint-eligible) from statements inside called functions.
    state.borrow_mut().program_src = source.clone();
    let error = interp.run(&source).err();
    let output = interp.take_output();
    let _ = reply.send(EvalOutcome { output, error });
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
        // Decide whether to stop, holding the state borrow only for the check.
        let reason: &str = {
            let s = self.state.borrow();
            if s.writer.is_none() {
                return DebugControl::Resume; // no debugger attached
            }
            if s.terminate {
                return DebugControl::Terminate;
            }
            let hit_breakpoint = src == s.program_src && s.breakpoints.contains(&line);
            let depth = interp.context.num_scopes();
            let stepped = match s.mode {
                RunMode::Continue => false,
                RunMode::StepIn => true,
                RunMode::StepOver(d) => depth <= d,
                RunMode::StepOut(d) => depth < d,
            };
            if !hit_breakpoint && !stepped {
                return DebugControl::Resume;
            }
            if hit_breakpoint { "breakpoint" } else { "step" }
        };
        self.stopped_loop(interp, reason)
    }
}

impl EngineHook {
    /// Sit at a stop point: announce it, then service the debugger (and stash
    /// any REPL eval that races in) until the client resumes or disconnects.
    fn stopped_loop(&self, interp: &mut Interpreter, reason: &str) -> DebugControl {
        {
            let mut s = self.state.borrow_mut();
            s.frames.reset();
            let body = proto::stopped_body(reason);
            s.event("stopped", body);
        }
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
                    // Phase 2: can't run a new top-level eval mid-stop; defer it
                    // until the current run resumes and unwinds. (Phase 3 will
                    // run it here, in the paused frame's context.)
                    self.state.borrow_mut().deferred.push_back((source, reply));
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
        // The engine is idle, so there's nothing running to pause.
        "pause" => state.borrow_mut().respond(req, json!({})),
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
            let expr = req
                .get("arguments")
                .and_then(|a| a.get("expression"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            match proto::evaluate_body(interp, &expr) {
                Ok(body) => state.borrow_mut().respond(req, body),
                Err(e) => state.borrow_mut().respond_err(req, &e),
            }
        }
        "setBreakpoints" => set_breakpoints(req, state),
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
    /// REPL evals stashed while stopped, run once the current run resumes.
    deferred: VecDeque<(String, Sender<EvalOutcome>)>,
    /// `true` once the client asked to disconnect/terminate.
    terminate: bool,
    /// The source of the currently-running top-level eval (breakpoint matching).
    program_src: String,
    /// The source path the client set breakpoints against (for stack frames).
    source_path: String,
}

impl DebugState {
    fn new() -> Self {
        DebugState {
            writer: None,
            seq: 0,
            breakpoints: HashSet::new(),
            mode: RunMode::Continue,
            frames: proto::Frames::new(),
            deferred: VecDeque::new(),
            terminate: false,
            program_src: String::new(),
            source_path: "fm-repl".to_string(),
        }
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
fn run_acceptor(listener: &TcpListener, tx: &Sender<EngineMsg>) {
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
        std::thread::Builder::new()
            .name("fm-dap-read".to_string())
            .spawn(move || run_reader(read_half, &tx))
            .ok();
    }
}

/// Read framed DAP requests off the socket and forward them into the inbox.
fn run_reader(stream: TcpStream, tx: &Sender<EngineMsg>) {
    let mut reader = BufReader::new(stream);
    loop {
        match fm_dap::read_message(&mut reader) {
            Ok(Some(req)) => {
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
}
