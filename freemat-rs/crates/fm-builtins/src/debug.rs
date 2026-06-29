//! Terminal debugging builtins (`db*`): set/clear breakpoints, inspect the call
//! stack, walk frames, and resume a paused run from the `K>>` prompt.
//!
//! Two kinds of command:
//!
//! - **Stack inspection** (`dbstack`, `dbup`, `dbdown`) operates directly on the
//!   interpreter's [`Context`](fm_interp::Context): the switchable *active*
//!   scope index is the basis for `dbup`/`dbdown`, and the per-scope current
//!   line drives `dbstack`. These work in any interpreter.
//! - **Breakpoints + resume** (`dbstop`, `dbclear`, `dbstatus`/`dblist`,
//!   `dbstep`, `dbcont`, `dbquit`) read/write the shared
//!   [`DebugSession`](fm_interp::debug::DebugSession). Setting a breakpoint takes
//!   effect immediately; the resume commands only take effect when a run is
//!   actually paused at the `K>>` prompt (the engine's debug hook honours them).
//!
//! See `docs/INTERP_SERVICE_PLAN.md` and `PROGRESS.md` (Stage 10) for how the
//! engine drives the terminal `K>>` prompt around these.

use fm_core::Array;
use fm_interp::debug::ResumeKind;
use fm_interp::error::Flow;
use fm_interp::{FunctionTable, Interpreter};

use crate::util::{err, err_signal};

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("dbstop", b_dbstop);
    table.add_builtin("dbclear", b_dbclear);
    table.add_builtin("dbdelete", b_dbclear);
    table.add_builtin("dbstatus", b_dbstatus);
    table.add_builtin("dblist", b_dbstatus);
    table.add_builtin("dbstack", b_dbstack);
    table.add_builtin("dbup", b_dbup);
    table.add_builtin("dbdown", b_dbdown);
    table.add_builtin("dbstep", b_dbstep);
    table.add_builtin("dbcont", b_dbcont);
    table.add_builtin("dbquit", b_dbquit);
}

// ---- breakpoints -----------------------------------------------------------

/// `dbstop(func, line)` / `dbstop line` / `dbstop(line)` — set a breakpoint.
/// A bare line number targets the top-level program (`""`); a leading name
/// targets that function's body.
fn b_dbstop(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let (func, line) = parse_bp_args(args, "dbstop")?;
    i.debug_session().borrow_mut().add_breakpoint(&func, line);
    Ok(vec![])
}

/// `dbclear all` / `dbclear(func)` / `dbclear(func, line)` — remove breakpoints.
fn b_dbclear(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let session = i.debug_session();
    let mut s = session.borrow_mut();
    match args.first().and_then(Array::as_string) {
        Some(word) if word.eq_ignore_ascii_case("all") => s.clear_all(),
        _ => {
            let (func, line) = parse_bp_args(args, "dbclear")?;
            // `dbclear func` (no line) clears every breakpoint in that function.
            let line = if args.len() >= 2 { Some(line) } else { None };
            s.clear_breakpoint(&func, line);
        }
    }
    Ok(vec![])
}

/// `dbstatus` / `dblist` — list the active breakpoints.
fn b_dbstatus(i: &mut Interpreter, _args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let session = i.debug_session();
    let list = session.borrow().list();
    let mut out = String::new();
    if list.is_empty() {
        out.push_str("No breakpoints set.\n");
    } else {
        for (func, line) in list {
            if func.is_empty() {
                out.push_str(&format!("Breakpoint at line {line}.\n"));
            } else {
                out.push_str(&format!("Breakpoint in {func} at line {line}.\n"));
            }
        }
    }
    i.emit(&out);
    Ok(vec![])
}

// ---- stack inspection ------------------------------------------------------

/// `dbstack` — print the call stack (most-recent frame first), marking the
/// active (inspected) frame.
fn b_dbstack(i: &mut Interpreter, _args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let trace = i.context.stack_trace(); // base → top
    let active = i.context.active_index();
    let mut out = String::new();
    for idx in (0..trace.len()).rev() {
        let (name, line) = &trace[idx];
        let label = if name.is_empty() {
            "base".to_string()
        } else {
            name.clone()
        };
        let marker = if idx == active { '>' } else { ' ' };
        match line {
            Some(l) => out.push_str(&format!("{marker} In {label} (line {l})\n")),
            None => out.push_str(&format!("{marker} In {label}\n")),
        }
    }
    i.emit(&out);
    Ok(vec![])
}

/// `dbup` — move the inspected frame one level toward the caller (base).
fn b_dbup(i: &mut Interpreter, _args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let active = i.context.active_index();
    if active == 0 {
        i.emit("Already at the base workspace.\n");
    } else {
        i.context.set_active(active - 1);
        emit_active_frame(i);
    }
    Ok(vec![])
}

/// `dbdown` — move the inspected frame one level toward the executing frame.
fn b_dbdown(i: &mut Interpreter, _args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let active = i.context.active_index();
    if active + 1 >= i.context.num_scopes() {
        i.emit("Already at the executing workspace.\n");
    } else {
        i.context.set_active(active + 1);
        emit_active_frame(i);
    }
    Ok(vec![])
}

/// Announce which frame is now active (after `dbup`/`dbdown`).
fn emit_active_frame(i: &mut Interpreter) {
    let active = i.context.active_index();
    let (name, line) = i
        .context
        .scope_at(active)
        .map(|s| (s.name.clone(), s.current_line))
        .unwrap_or_default();
    let label = if name.is_empty() { "base" } else { &name };
    match line {
        Some(l) => i.emit(&format!("In {label} (line {l})\n")),
        None => i.emit(&format!("In {label}\n")),
    }
}

// ---- resume control (only valid while paused at `K>>`) ---------------------

/// `dbstep` — resume and stop again at the next statement in this frame.
fn b_dbstep(i: &mut Interpreter, _args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    request_resume(i, ResumeKind::Step, "dbstep")
}

/// `dbcont` — resume the paused run until the next breakpoint (or completion).
fn b_dbcont(i: &mut Interpreter, _args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    request_resume(i, ResumeKind::Continue, "dbcont")
}

/// `dbquit` — abort the paused run back to the top-level prompt.
fn b_dbquit(i: &mut Interpreter, _args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    request_resume(i, ResumeKind::Quit, "dbquit")
}

/// Record a resume request for the engine, refusing if no run is paused.
fn request_resume(i: &mut Interpreter, kind: ResumeKind, name: &str) -> Flow<Vec<Array>> {
    let session = i.debug_session();
    let mut s = session.borrow_mut();
    if !s.paused {
        return err(format!("{name}: not in debug mode (no run is paused)"));
    }
    s.resume = Some(kind);
    Ok(vec![])
}

// ---- argument parsing ------------------------------------------------------

/// Interpret `dbstop`/`dbclear` arguments into `(function, line)`. Accepts
/// `dbstop(line)`, `dbstop('line')`, `dbstop(func, line)`, and the command-syntax
/// forms `dbstop 5` / `dbstop foo 5` (every argument arrives as a string).
fn parse_bp_args(args: &[Array], name: &str) -> Flow<(String, usize)> {
    match args.len() {
        0 => err(format!("{name}: requires a line number")),
        1 => Ok((String::new(), arg_to_line(&args[0], name)?)),
        _ => {
            let func = args[0].as_string().ok_or_else(|| {
                err_signal(format!("{name}: first argument must be a function name"))
            })?;
            Ok((func, arg_to_line(&args[1], name)?))
        }
    }
}

/// Coerce a string or numeric argument into a 1-based line number.
fn arg_to_line(a: &Array, name: &str) -> Flow<usize> {
    if let Some(s) = a.as_string() {
        s.trim()
            .parse::<usize>()
            .map_err(|_| err_signal(format!("{name}: invalid line number '{s}'")))
    } else if let Some(f) = a.as_f64() {
        Ok(f as usize)
    } else {
        Err(err_signal(format!("{name}: invalid line number")))
    }
}
