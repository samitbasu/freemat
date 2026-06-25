//! Phase 1 (interpreter-as-service): the statement seam is **re-entrant** and
//! can be **suppressed**.
//!
//! These exercise the `DebugHook` contract directly with tiny stub hooks, so
//! they pin the seam's behavior independently of the DAP wire layer:
//! - a hook that runs a statement of its own re-enters the seam (the basis for a
//!   nested debugger REPL with recursive breakpoints), and
//! - `Interpreter::eval_suppressed` disables the seam for the duration (so a
//!   watch/hover evaluation never trips a breakpoint).

use std::cell::Cell;
use std::rc::Rc;

use fm_interp::Interpreter;
use fm_interp::debug::{DebugControl, DebugHook};

/// Counts seam consultations, and on its first call runs a nested statement to
/// prove the seam re-enters while the hook is already on the stack.
#[derive(Default)]
struct ReentryProbe {
    calls: Cell<usize>,
    nested_done: Cell<bool>,
}

impl DebugHook for ReentryProbe {
    fn on_statement(&self, interp: &mut Interpreter, _line: usize, _src: &str) -> DebugControl {
        self.calls.set(self.calls.get() + 1);
        if !self.nested_done.get() {
            // Guard against unbounded recursion: only nest once.
            self.nested_done.set(true);
            // Running a statement here must itself flow through the seam.
            let _ = interp.run("1 + 1;");
        }
        DebugControl::Resume
    }
}

#[test]
fn seam_is_re_entrant() {
    let probe = Rc::new(ReentryProbe::default());
    let mut interp = Interpreter::new();
    interp.set_debugger(probe.clone());

    interp.run("2 + 2;").unwrap();

    // Outer statement (`2 + 2;`) → call 1, which runs `1 + 1;` → call 2.
    // Without re-entrancy the nested run would never have been seen.
    assert!(
        probe.calls.get() >= 2,
        "seam did not re-enter: {} call(s)",
        probe.calls.get()
    );
    assert!(probe.nested_done.get(), "nested statement never ran");
}

/// Counts seam consultations; on its first call it evaluates a nested statement
/// *under suppression*, which must NOT be seen by the seam.
#[derive(Default)]
struct SuppressProbe {
    calls: Cell<usize>,
    started: Cell<bool>,
}

impl DebugHook for SuppressProbe {
    fn on_statement(&self, interp: &mut Interpreter, _line: usize, _src: &str) -> DebugControl {
        self.calls.set(self.calls.get() + 1);
        if !self.started.get() {
            self.started.set(true);
            // A watch-style evaluation: the seam is disabled for its duration,
            // so this nested run contributes no further consultations.
            interp.eval_suppressed(|i| {
                let _ = i.run("1 + 1;");
            });
        }
        DebugControl::Resume
    }
}

#[test]
fn eval_suppressed_disables_the_seam() {
    let probe = Rc::new(SuppressProbe::default());
    let mut interp = Interpreter::new();
    interp.set_debugger(probe.clone());

    interp.run("2 + 2;").unwrap();

    // Only the single outer statement should have been seen; the suppressed
    // nested `1 + 1;` must contribute nothing.
    assert_eq!(
        probe.calls.get(),
        1,
        "suppressed evaluation re-entered the seam"
    );
}

/// A hook that asks the interpreter to terminate the run.
struct TerminateProbe;

impl DebugHook for TerminateProbe {
    fn on_statement(&self, _interp: &mut Interpreter, _line: usize, _src: &str) -> DebugControl {
        DebugControl::Terminate
    }
}

#[test]
fn terminate_unwinds_the_run_without_running_statements() {
    let mut interp = Interpreter::new();
    interp.set_debugger(Rc::new(TerminateProbe));

    // `x` is assigned by the first statement; Terminate fires *before* it runs,
    // so the assignment must not take effect.
    interp.run("x = 99;").unwrap();
    assert!(
        interp.context.lookup("x").is_none(),
        "statement ran despite Terminate"
    );
}
