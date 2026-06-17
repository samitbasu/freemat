//! Shared helpers for the builtins: argument checks and element-wise maps.

use fm_core::{Array, C64, DataClass};
use fm_interp::error::{Flow, InterpError, Signal};
use fm_interp::value::{build_complex, build_real, to_c64_vec, to_f64_vec};

/// Build an error signal from a message.
pub(crate) fn err<T>(msg: impl Into<String>) -> Flow<T> {
    Err(Signal::Error(InterpError::msg(msg)))
}

/// Build a bare error [`Signal`] from a message (for use with `ok_or_else`).
pub(crate) fn err_signal(msg: impl Into<String>) -> Signal {
    Signal::Error(InterpError::msg(msg))
}

/// Require at least `n` arguments.
pub(crate) fn need(args: &[Array], n: usize, name: &str) -> Flow<()> {
    if args.len() < n {
        return err(format!(
            "{name}: expected at least {n} argument(s), got {}",
            args.len()
        ));
    }
    Ok(())
}

/// Map an element-wise real `f64 -> f64` function over an array, keeping the
/// result as `double` (the MATLAB convention for transcendental functions).
pub(crate) fn map_double(a: &Array, f: impl Fn(f64) -> f64) -> Array {
    let dims = a.dims();
    let data = to_f64_vec(a).into_iter().map(f).collect();
    build_real(DataClass::Double, &dims, data)
}

/// Map an element-wise function that produces `f64`, but if any input is
/// complex, route through the complex closure instead. Result is `double` (real
/// lane) or complex `double`.
pub(crate) fn map_real_or_complex(
    a: &Array,
    rf: impl Fn(f64) -> f64,
    cf: impl Fn(C64) -> C64,
) -> Array {
    let dims = a.dims();
    if a.is_complex() {
        let data = to_c64_vec(a).into_iter().map(cf).collect();
        build_complex(&dims, data)
    } else {
        let data = to_f64_vec(a).into_iter().map(rf).collect();
        build_real(DataClass::Double, &dims, data)
    }
}

/// Map an element-wise function whose real branch may itself produce a complex
/// value (e.g. `sqrt` of a negative number, `log` of a negative number). Each
/// real input is lifted to `C64`; the result narrows back to real if every
/// imaginary part vanished.
pub(crate) fn map_complexifying(a: &Array, cf: impl Fn(C64) -> C64) -> Array {
    let dims = a.dims();
    let data = to_c64_vec(a).into_iter().map(cf).collect();
    build_complex(&dims, data)
}

/// Flatten a cell array's elements in **column-major (memory) order**.
///
/// `ndarray`'s `.iter()` walks logical (row-major) order; the rest of the
/// codebase stores column-major, so cell helpers must read memory order to keep
/// linear positions consistent.
pub(crate) fn cell_mem_order(a: &Array) -> Vec<Array> {
    match a.as_cell() {
        Some(d) => {
            if let Some(s) = d.as_slice_memory_order() {
                s.to_vec()
            } else {
                d.t().iter().cloned().collect()
            }
        }
        None => Vec::new(),
    }
}

/// Read a single scalar `f64` from `args[i]`, erroring if absent or non-scalar.
pub(crate) fn scalar_arg(args: &[Array], i: usize, name: &str) -> Flow<f64> {
    args.get(i).and_then(Array::as_f64).ok_or_else(|| {
        Signal::Error(InterpError::msg(format!(
            "{name}: expected a scalar argument"
        )))
    })
}
