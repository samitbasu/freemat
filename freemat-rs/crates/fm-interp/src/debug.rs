//! Debugger seam (Stage 10).
//!
//! The interpreter exposes a single statement chokepoint
//! ([`crate::Interpreter::exec_statement`]). When a [`DebugHook`] is installed,
//! it is consulted *before* each statement runs. The hook owns the actual
//! debugger (e.g. a Debug Adapter Protocol session in the `fm-dap` crate); the
//! interpreter knows nothing about wire formats — it just hands the hook the
//! current location and a borrow of itself for inspection.
//!
//! ## Re-entrancy
//! Consultation is **re-entrant**: the hook stays installed while it runs, so
//! statements the hook executes itself (e.g. a nested debugger REPL command, or
//! code it calls) *do* re-enter the seam and can hit further breakpoints. This
//! is what makes recursive debugging (MATLAB's nested `K>>`) possible. Because
//! the hook may be on the call stack at several levels at once,
//! [`DebugHook::on_statement`] takes `&self` — keep mutable hook state behind
//! interior mutability (`Cell`/`RefCell`) and never hold a borrow across a
//! nested call.
//!
//! When the hook explicitly does *not* want re-entry — evaluating a watch /
//! hover expression, which must never trigger a breakpoint — it wraps the work
//! in [`Interpreter::eval_suppressed`], which disables the seam for the duration.

use crate::Interpreter;

/// What the interpreter should do after the hook has inspected a statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugControl {
    /// Continue executing the statement normally.
    Resume,
    /// Abort the run (the client disconnected or asked to terminate). The
    /// interpreter unwinds the current run as if a top-level `return` ran.
    Terminate,
}

/// A debugger attached to the interpreter's statement chokepoint.
pub trait DebugHook {
    /// Consulted before every statement executes.
    ///
    /// - `interp` is the live interpreter, which the hook may freely read
    ///   ([`Interpreter::context`]: locals, call stack, current line) and run
    ///   statements against (a nested debugger REPL, or a suppressed watch
    ///   evaluation via [`Interpreter::eval_suppressed`]).
    /// - `line` is the 1-based source line of the statement about to run.
    /// - `src` is the source text of the unit currently executing. It lets the
    ///   hook tell the top-level program apart from function bodies (which have
    ///   their own source), so a line breakpoint set in the main file does not
    ///   spuriously fire on the same line number inside a called function.
    ///
    /// Takes `&self` because the hook may be re-entered while already on the
    /// stack (see the module docs on re-entrancy).
    fn on_statement(&self, interp: &mut Interpreter, line: usize, src: &str) -> DebugControl;
}
