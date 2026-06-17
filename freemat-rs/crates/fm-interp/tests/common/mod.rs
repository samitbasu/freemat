//! Shared test helpers.
//!
//! Each integration-test binary uses a different subset, so some helpers look
//! unused per-binary; allow that rather than gate every helper behind a binary.
#![allow(dead_code, reason = "shared helpers used across several test binaries")]

use fm_core::Array;
use fm_interp::Interpreter;

/// Run a snippet, returning the interpreter so the caller can inspect variables.
pub fn run(src: &str) -> Interpreter {
    let mut interp = Interpreter::new();
    interp.run(src).expect("snippet should run without error");
    interp
}

/// Run a snippet and return the named variable's value.
pub fn run_var(src: &str, name: &str) -> Array {
    let interp = run(src);
    interp
        .context
        .lookup(name)
        .unwrap_or_else(|| panic!("variable '{name}' not defined"))
        .clone()
}

/// Run a snippet and return the named variable as `f64`.
pub fn run_f64(src: &str, name: &str) -> f64 {
    run_var(src, name)
        .as_f64()
        .expect("variable should be a scalar")
}

/// Run a snippet whose final statement is a bare expression (which sets `ans`),
/// and return `ans` as `f64`.
pub fn eval_f64(expr: &str) -> f64 {
    run_f64(expr, "ans")
}

/// The flat column-major `f64` data of a variable.
pub fn run_vec(src: &str, name: &str) -> Vec<f64> {
    fm_interp::value::to_f64_vec(&run_var(src, name))
}
