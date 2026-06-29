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

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

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

/// A shared handle to the terminal-debug [`DebugSession`].
pub type DebugSessionHandle = Rc<RefCell<DebugSession>>;

/// How a paused run should proceed when the user resumes from the terminal
/// `K>>` prompt (set by the `dbcont` / `dbstep` / `dbquit` builtins).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeKind {
    /// `dbcont`: run until the next breakpoint (or completion).
    Continue,
    /// `dbstep`: stop again at the next statement in the current frame.
    Step,
    /// `dbquit`: abort the whole run back to the top-level prompt.
    Quit,
}

/// Terminal-side ("`K>>`") debug state, shared between the `db*` builtins (which
/// **write** it — set breakpoints, request a resume) and the engine's debug hook
/// (which **reads** it to decide whether to stop). It is single-threaded: it
/// lives behind `Rc<RefCell<…>>` on the interpreter/engine thread, never crossing
/// a channel. It is independent of the DAP wire state (sockets / sequence
/// numbers) so a `dbstop` breakpoint works with or without an IDE attached.
#[derive(Debug, Default)]
pub struct DebugSession {
    /// Breakpoints keyed by function name (`""` = the top-level REPL/script),
    /// each holding the set of 1-based lines to stop on.
    breakpoints: BTreeMap<String, BTreeSet<usize>>,
    /// `true` while stopped at a breakpoint (the `K>>` prompt is active). The
    /// `dbcont`/`dbstep`/`dbquit` builtins refuse to run unless this is set.
    pub paused: bool,
    /// A resume request raised by a `db*` control builtin while paused; consumed
    /// by the engine when it resumes the run.
    pub resume: Option<ResumeKind>,
    /// A pending single-step: stop again at the next statement whose call-stack
    /// depth is `≤` this value (armed when `dbstep` resumes).
    pub step: Option<usize>,
}

impl DebugSession {
    /// Set a breakpoint at `line` in `func` (`""` for the top-level program).
    pub fn add_breakpoint(&mut self, func: &str, line: usize) {
        self.breakpoints
            .entry(func.to_string())
            .or_default()
            .insert(line);
    }

    /// Clear one breakpoint (`Some(line)`) or every breakpoint in `func`
    /// (`None`). Returns whether anything was removed.
    pub fn clear_breakpoint(&mut self, func: &str, line: Option<usize>) -> bool {
        match line {
            None => self.breakpoints.remove(func).is_some(),
            Some(line) => {
                let Some(lines) = self.breakpoints.get_mut(func) else {
                    return false;
                };
                let removed = lines.remove(&line);
                if lines.is_empty() {
                    self.breakpoints.remove(func);
                }
                removed
            }
        }
    }

    /// Remove every breakpoint.
    pub fn clear_all(&mut self) {
        self.breakpoints.clear();
    }

    /// Whether a statement at `line` in `func` is a breakpoint.
    #[must_use]
    pub fn breakpoint_hit(&self, func: &str, line: usize) -> bool {
        self.breakpoints
            .get(func)
            .is_some_and(|lines| lines.contains(&line))
    }

    /// Whether no breakpoints are set (the hook's fast-path gate).
    #[must_use]
    pub fn has_breakpoints(&self) -> bool {
        !self.breakpoints.is_empty()
    }

    /// All breakpoints as `(func, line)` pairs, sorted by function then line.
    #[must_use]
    pub fn list(&self) -> Vec<(String, usize)> {
        self.breakpoints
            .iter()
            .flat_map(|(func, lines)| lines.iter().map(move |&line| (func.clone(), line)))
            .collect()
    }
}
