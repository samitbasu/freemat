//! Logical reductions and helpers: `all`, `any`, `xor`, `not`, `isequal`,
//! `true`, `false`, `find`.
//!
//! `all`/`any` reduce along the first non-singleton dimension (or an explicit
//! `dim`), yielding a `logical` array. They are the high-priority unblockers:
//! the conformance `test()` helper is `y = all(x(:))`.

use fm_core::{Array, DataClass};
use fm_interp::error::Flow;
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};

use crate::util::need;

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("all", b_all);
    table.add_builtin("any", b_any);
    table.add_builtin("xor", b_xor);
    table.add_builtin("not", b_not);
    table.add_builtin("isequal", b_isequal);
    table.add_builtin("true", b_true);
    table.add_builtin("false", b_false);
    table.add_builtin("find", b_find);
}

fn first_nonsingleton(dims: &[usize]) -> usize {
    for (i, &d) in dims.iter().enumerate() {
        if d != 1 {
            return i + 1;
        }
    }
    1
}

/// Reduce booleans along `dim` with `f`, seeded by `init`.
fn bool_reduce(
    a: &Array,
    args: &[Array],
    init: bool,
    f: impl Fn(bool, bool) -> bool,
) -> (Vec<f64>, Vec<usize>) {
    let dims = a.dims();
    let data = to_f64_vec(a);
    if a.numel() == 0 {
        // all([]) is true, any([]) is false (MATLAB).
        return (vec![if init { 1.0 } else { 0.0 }], vec![1, 1]);
    }
    let dim = if let Some(d) = args.get(1).and_then(Array::as_f64) {
        (d.round() as usize).max(1)
    } else {
        first_nonsingleton(&dims)
    };
    let d = dim - 1;
    if d >= dims.len() || dims[d] <= 1 {
        let out = data
            .iter()
            .map(|&v| if v != 0.0 { 1.0 } else { 0.0 })
            .collect();
        return (out, dims);
    }
    let mut out_dims = dims.clone();
    out_dims[d] = 1;
    let stride: usize = dims[..d].iter().product();
    let dlen = dims[d];
    let outer: usize = dims[d + 1..].iter().product();
    let mut out = Vec::new();
    for o in 0..outer {
        for s in 0..stride {
            let base = o * stride * dlen + s;
            let mut acc = init;
            for k in 0..dlen {
                acc = f(acc, data[base + k * stride] != 0.0);
            }
            out.push(if acc { 1.0 } else { 0.0 });
        }
    }
    (out, out_dims)
}

fn b_all(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "all")?;
    let (data, dims) = bool_reduce(&args[0], args, true, |a, b| a && b);
    Ok(vec![build_real(DataClass::Bool, &dims, data)])
}

fn b_any(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "any")?;
    let (data, dims) = bool_reduce(&args[0], args, false, |a, b| a || b);
    Ok(vec![build_real(DataClass::Bool, &dims, data)])
}

fn b_xor(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "xor")?;
    let x = to_f64_vec(&args[0]);
    let y = to_f64_vec(&args[1]);
    let (n, dims) = if x.len() >= y.len() {
        (x.len(), args[0].dims())
    } else {
        (y.len(), args[1].dims())
    };
    let data: Vec<f64> = (0..n)
        .map(|i| {
            let a = (if x.len() == 1 { x[0] } else { x[i] }) != 0.0;
            let b = (if y.len() == 1 { y[0] } else { y[i] }) != 0.0;
            if a ^ b { 1.0 } else { 0.0 }
        })
        .collect();
    Ok(vec![build_real(DataClass::Bool, &dims, data)])
}

fn b_not(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "not")?;
    let dims = args[0].dims();
    let data = to_f64_vec(&args[0])
        .into_iter()
        .map(|v| if v == 0.0 { 1.0 } else { 0.0 })
        .collect();
    Ok(vec![build_real(DataClass::Bool, &dims, data)])
}

fn b_isequal(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "isequal")?;
    let first = &args[0];
    let eq = args[1..].iter().all(|other| arrays_equal(first, other));
    Ok(vec![Array::bool(eq)])
}

/// Structural + numeric equality (used by `isequal`).
fn arrays_equal(a: &Array, b: &Array) -> bool {
    if a.is_char() && b.is_char() {
        return a.as_string() == b.as_string();
    }
    if a.dims() != b.dims() {
        return false;
    }
    let x = to_f64_vec(a);
    let y = to_f64_vec(b);
    x.len() == y.len() && x.iter().zip(&y).all(|(p, q)| p == q)
}

fn b_true(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    Ok(vec![fill_logical(args, true)])
}

fn b_false(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    Ok(vec![fill_logical(args, false)])
}

fn fill_logical(args: &[Array], val: bool) -> Array {
    if args.is_empty() {
        return Array::bool(val);
    }
    let dims = dims_from_args(args);
    let n: usize = dims.iter().product();
    let v = if val { 1.0 } else { 0.0 };
    build_real(DataClass::Bool, &dims, vec![v; n])
}

/// `find(x)` — linear indices of nonzero elements, as a column (or row for a
/// row-vector input), 1-based.
fn b_find(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "find")?;
    let data = to_f64_vec(&args[0]);
    let idx: Vec<f64> = data
        .iter()
        .enumerate()
        .filter(|(_, v)| **v != 0.0)
        .map(|(i, _)| (i + 1) as f64)
        .collect();
    let dims = args[0].dims();
    let n = idx.len();
    // A row-vector input yields a row vector; otherwise a column vector.
    let out_dims = if dims.len() == 2 && dims[0] == 1 {
        vec![1, n]
    } else {
        vec![n, 1]
    };
    Ok(vec![build_real(DataClass::Double, &out_dims, idx)])
}

/// Shared dims-from-arguments parsing (mirrors `zeros`/`ones`).
fn dims_from_args(args: &[Array]) -> Vec<usize> {
    if args.is_empty() {
        return vec![1, 1];
    }
    if args.len() == 1 {
        if args[0].numel() == 1 {
            let n = args[0].as_f64().unwrap_or(0.0).max(0.0) as usize;
            return vec![n, n];
        }
        return to_f64_vec(&args[0])
            .into_iter()
            .map(|x| x.max(0.0) as usize)
            .collect();
    }
    args.iter()
        .map(|a| a.as_f64().unwrap_or(0.0).max(0.0) as usize)
        .collect()
}
