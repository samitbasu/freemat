//! The interpreter **engine**: a single-threaded actor that owns the
//! [`Interpreter`] and serves it to clients over channels.
//!
//! This is Phase 0 of the interpreter-as-service plan (`docs/INTERP_SERVICE_PLAN.md`).
//! Today it has exactly one client — the terminal REPL — so it does nothing the
//! old in-line `interp.run(line)` loop didn't. The point is *structural*: the
//! interpreter now lives on its own thread, reachable only by message, which is
//! the precondition for attaching a second client (the DAP debugger) that can
//! talk to the *same live session* without fighting the REPL's blocking
//! `readline` or the interpreter's `!Send`-ness.
//!
//! The interpreter never leaves the engine thread; only [`ReplCommand`]s and
//! their replies cross the channel, and those carry owned, `Send` data
//! (`String`, [`InterpError`]) — never `Array` handles or interpreter internals.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;

use fm_interp::{InterpError, Interpreter};

/// A command sent to the engine thread. Each variant that produces a result
/// carries a one-shot `reply` channel the caller blocks on.
enum ReplCommand {
    /// Parse and run a source line at the top level; reply with its output and
    /// any error.
    Eval {
        source: String,
        reply: Sender<EvalOutcome>,
    },
    /// Answer a read-only question about interpreter state (e.g. for REPL
    /// tab-completion). Kept separate from `Eval` so a future debugger's control
    /// traffic and the REPL's eval traffic don't share a queue.
    Query {
        kind: QueryKind,
        reply: Sender<QueryResult>,
    },
    /// Stop the engine loop and let the thread exit.
    Shutdown,
}

/// The result of an [`Engine::eval`]: buffered output plus an optional error.
/// Mirrors what the old in-line loop did with `interp.run` + `take_output`.
pub struct EvalOutcome {
    /// Everything the interpreter echoed/`disp`-ed during the run (printed even
    /// when the run ended in an error, matching the old behavior).
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

/// A handle to the running engine. Cloneable senders aside, this owns the
/// thread join handle and shuts the engine down on drop.
pub struct Engine {
    tx: Sender<ReplCommand>,
    handle: Option<JoinHandle<()>>,
}

impl Engine {
    /// Spawn the engine thread. It builds a fresh interpreter with the full
    /// standard library, runs `setup` against it (e.g. to install a graphics
    /// sink), then serves commands until shut down.
    ///
    /// `setup` runs **on the engine thread**, so anything it captures must be
    /// `Send` (e.g. the graphics [`ServerHandle`](crate::ServerHandle), which is
    /// `Send` via `GraphicsSink: Send + Sync`).
    pub fn spawn<F>(setup: F) -> Self
    where
        F: FnOnce(&mut Interpreter) + Send + 'static,
    {
        let (tx, rx) = channel();
        let handle = std::thread::Builder::new()
            .name("fm-interp".to_string())
            .spawn(move || {
                let mut interp = Interpreter::new();
                fm_builtins::register_standard_library(&mut interp);
                setup(&mut interp);
                serve(&rx, &mut interp);
            })
            .expect("spawn interpreter engine thread");
        Engine {
            tx,
            handle: Some(handle),
        }
    }

    /// Run one source line, blocking until the engine replies. If the engine
    /// thread has died, returns an error outcome rather than panicking.
    pub fn eval(&self, source: impl Into<String>) -> EvalOutcome {
        let (reply, rx) = channel();
        let cmd = ReplCommand::Eval {
            source: source.into(),
            reply,
        };
        if self.tx.send(cmd).is_err() {
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

    /// Every registered function/builtin name (for tab-completion). Empty if the
    /// engine has stopped.
    pub fn function_names(&self) -> Vec<String> {
        let (reply, rx) = channel();
        let cmd = ReplCommand::Query {
            kind: QueryKind::FunctionNames,
            reply,
        };
        if self.tx.send(cmd).is_err() {
            return Vec::new();
        }
        match rx.recv() {
            Ok(QueryResult::Names(names)) => names,
            Err(_) => Vec::new(),
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        // Best-effort clean shutdown so the engine thread returns and any
        // graphics sink it holds is dropped on its own thread.
        let _ = self.tx.send(ReplCommand::Shutdown);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// The engine loop: process one command at a time against the owned interpreter.
fn serve(rx: &Receiver<ReplCommand>, interp: &mut Interpreter) {
    while let Ok(cmd) = rx.recv() {
        match cmd {
            ReplCommand::Eval { source, reply } => {
                let error = interp.run(&source).err();
                let output = interp.take_output();
                let _ = reply.send(EvalOutcome { output, error });
            }
            ReplCommand::Query { kind, reply } => {
                let result = match kind {
                    QueryKind::FunctionNames => QueryResult::Names(interp.functions.names()),
                };
                let _ = reply.send(result);
            }
            ReplCommand::Shutdown => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_round_trips_output_and_state() {
        let engine = Engine::spawn(|_| {});
        // Output is echoed (no trailing semicolon).
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

        // State persists across evals on the same engine (same live session).
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
        // The engine survives the error and keeps serving.
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
