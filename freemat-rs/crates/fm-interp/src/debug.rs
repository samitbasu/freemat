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
//! The interpreter *takes* the hook out of itself before calling
//! [`DebugHook::on_statement`] and restores it afterwards (see
//! `Interpreter::debug_check`). While the hook runs, `interp.debugger` is
//! `None`, so any statements the hook executes itself (e.g. evaluating a watch
//! expression that calls a function) do **not** recursively re-enter the hook.

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
    /// - `interp` is the live interpreter with the hook temporarily removed, so
    ///   the hook may freely read [`Interpreter::context`] (locals, call stack,
    ///   current line) and even evaluate expressions in the current frame.
    /// - `line` is the 1-based source line of the statement about to run.
    /// - `src` is the source text of the unit currently executing. It lets the
    ///   hook tell the top-level program apart from function bodies (which have
    ///   their own source), so a line breakpoint set in the main file does not
    ///   spuriously fire on the same line number inside a called function.
    fn on_statement(&mut self, interp: &mut Interpreter, line: usize, src: &str) -> DebugControl;
}
