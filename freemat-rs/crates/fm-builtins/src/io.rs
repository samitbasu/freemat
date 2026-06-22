//! Display-control builtins: `format` (display precision mode),
//! `getprintlimit` / `setprintlimit` (array element print limit). These mutate
//! interpreter display state rather than producing data.

use fm_core::{Array, DataClass, FormatMode};
use fm_interp::error::Flow;
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};

use crate::util::{err, need};

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("format", b_format);
    table.add_builtin("getprintlimit", b_getprintlimit);
    table.add_builtin("setprintlimit", b_setprintlimit);
}

/// `format [mode...]` — set the display precision mode, or, with no arguments
/// and an output requested, return the current mode as a string.
///
/// Modes: `short`, `long`, `short e`, `long e`, `short g`, `long g`. Command
/// syntax (`format short e`) arrives as separate string arguments which we join
/// with a single space before parsing.
fn b_format(interp: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    // `t = format` (no args) returns the current mode name.
    if args.is_empty() {
        if nargout >= 1 {
            return Ok(vec![Array::char_string(interp.format.name())]);
        }
        // Bare `format` resets to the default short mode (FreeMat behavior).
        interp.format = FormatMode::Short;
        return Ok(vec![]);
    }
    // Join all string arguments into one lower-cased mode descriptor.
    let mut parts = Vec::with_capacity(args.len());
    for a in args {
        let Some(s) = a.as_string() else {
            return err("format: arguments must be strings");
        };
        parts.push(s.to_ascii_lowercase());
    }
    let desc = parts.join(" ");
    match FormatMode::from_arg(&desc) {
        Some(mode) => {
            interp.format = mode;
            Ok(vec![])
        }
        None => err(format!("format: unknown format mode '{desc}'")),
    }
}

/// `n = getprintlimit` — return the current array element print limit.
fn b_getprintlimit(interp: &mut Interpreter, _args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    Ok(vec![build_real(
        DataClass::Double,
        &[1, 1],
        vec![interp.print_limit as f64],
    )])
}

/// `setprintlimit(n)` — set the array element print limit.
fn b_setprintlimit(interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "setprintlimit")?;
    if args[0].numel() != 1 {
        return err("setprintlimit: argument must be a scalar");
    }
    let v = to_f64_vec(&args[0]).first().copied().unwrap_or(0.0);
    if !v.is_finite() || v < 0.0 {
        return err("setprintlimit: limit must be a nonnegative number");
    }
    interp.print_limit = v as usize;
    Ok(vec![])
}
