//! Array constructors: `zeros`, `ones`, `eye`, `linspace`, `logspace`,
//! `repmat`, `diag`, `colon`.

use fm_core::{Array, DataClass};
use fm_interp::error::Flow;
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};

use crate::util::{err, err_signal, need, scalar_arg};

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("zeros", |_i, a, _n| Ok(vec![filled(a, 0.0)]));
    table.add_builtin("ones", |_i, a, _n| Ok(vec![filled(a, 1.0)]));
    table.add_builtin("eye", b_eye);
    table.add_builtin("linspace", b_linspace);
    table.add_builtin("logspace", b_logspace);
    table.add_builtin("repmat", b_repmat);
    table.add_builtin("diag", b_diag);
}

/// Parse `zeros`/`ones`/`eye`-style dimension arguments.
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

fn filled(args: &[Array], v: f64) -> Array {
    let dims = dims_from_args(args);
    let n: usize = dims.iter().product();
    build_real(DataClass::Double, &dims, vec![v; n])
}

fn b_eye(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let dims = dims_from_args(args);
    let (r, c) = match dims.as_slice() {
        [] => (1, 1),
        [n] => (*n, *n),
        [r, c, ..] => (*r, *c),
    };
    let mut data = vec![0.0; r * c];
    for i in 0..r.min(c) {
        data[i + i * r] = 1.0;
    }
    Ok(vec![build_real(DataClass::Double, &[r, c], data)])
}

fn b_linspace(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "linspace")?;
    let a = scalar_arg(args, 0, "linspace")?;
    let b = scalar_arg(args, 1, "linspace")?;
    let n = if args.len() >= 3 {
        scalar_arg(args, 2, "linspace")?.round().max(1.0) as usize
    } else {
        100
    };
    let data: Vec<f64> = if n == 1 {
        vec![b]
    } else {
        (0..n)
            .map(|i| a + (b - a) * i as f64 / (n - 1) as f64)
            .collect()
    };
    Ok(vec![build_real(DataClass::Double, &[1, n], data)])
}

fn b_logspace(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "logspace")?;
    let a = scalar_arg(args, 0, "logspace")?;
    let b = scalar_arg(args, 1, "logspace")?;
    let n = if args.len() >= 3 {
        scalar_arg(args, 2, "logspace")?.round().max(1.0) as usize
    } else {
        50
    };
    let data: Vec<f64> = if n == 1 {
        vec![10.0_f64.powf(b)]
    } else {
        (0..n)
            .map(|i| 10.0_f64.powf(a + (b - a) * i as f64 / (n - 1) as f64))
            .collect()
    };
    Ok(vec![build_real(DataClass::Double, &[1, n], data)])
}

fn b_repmat(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "repmat")?;
    let a = &args[0];
    let dims = a.dims();
    if dims.len() != 2 {
        return err("repmat: only 2-D inputs are supported");
    }
    let (m, n) = (dims[0], dims[1]);
    // Replication counts: repmat(A, k) or repmat(A, p, q) or repmat(A, [p q]).
    let (p, q) = if args.len() >= 3 {
        (
            scalar_arg(args, 1, "repmat")?.round().max(0.0) as usize,
            scalar_arg(args, 2, "repmat")?.round().max(0.0) as usize,
        )
    } else if args[1].numel() >= 2 {
        let r = to_f64_vec(&args[1]);
        (r[0].max(0.0) as usize, r[1].max(0.0) as usize)
    } else {
        let k = scalar_arg(args, 1, "repmat")?.round().max(0.0) as usize;
        (k, k)
    };
    let (om, on) = (m * p, n * q);
    // Build the column-major source-position permutation for the tiled output,
    // then materialise it for the value's actual element type (numeric, char,
    // complex, or cell).
    let mut perm = vec![0usize; om * on];
    for tj in 0..q {
        for ti in 0..p {
            for j in 0..n {
                for i in 0..m {
                    let oi = ti * m + i;
                    let oj = tj * n + j;
                    perm[oi + oj * om] = i + j * m;
                }
            }
        }
    }
    Ok(vec![tile_by(a, &[om, on], &perm)])
}

/// Materialise a permutation `perm` (column-major source positions) over `a` as
/// a new array of shape `dims`, dispatching on the element type.
fn tile_by(a: &Array, dims: &[usize], perm: &[usize]) -> Array {
    match a {
        Array::Cell(_) => {
            let flat: Vec<Array> = crate::util::cell_mem_order(a);
            let data: Vec<Array> = perm.iter().map(|&p| flat[p].clone()).collect();
            Array::cell(dims, data)
        }
        Array::Char(_) | Array::Scalar(fm_core::ScalarValue::Char(_)) => {
            let flat: Vec<char> = a.as_string().unwrap_or_default().chars().collect();
            let data: Vec<char> = perm.iter().map(|&p| flat[p]).collect();
            fm_interp::value::char_matrix(dims, data)
        }
        _ if a.is_complex() => {
            let flat = fm_interp::value::to_c64_vec(a);
            let data = perm.iter().map(|&p| flat[p]).collect();
            fm_interp::value::build_complex(dims, data)
        }
        _ => {
            let flat = to_f64_vec(a);
            let data: Vec<f64> = perm.iter().map(|&p| flat[p]).collect();
            build_real(a.class(), dims, data)
        }
    }
}

/// `diag(v)` builds a diagonal matrix from a vector; `diag(M)` extracts the
/// main diagonal of a matrix.
fn b_diag(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "diag")?;
    let a = &args[0];
    let dims = a.dims();
    if dims.len() != 2 {
        return err("diag: first argument must be 2-D");
    }
    // The diagonal offset `k` (0 = main, >0 above, <0 below). Mirrors FreeMat's
    // `DiagFunction`: a vector input *builds* a diagonal matrix; a matrix input
    // *extracts* the k-th diagonal as a column vector.
    let k = match args.get(1) {
        None => 0i64,
        Some(arg) => arg
            .as_f64()
            .map(|v| v as i64)
            .ok_or_else(|| err_signal("diag: second argument must be a scalar"))?,
    };
    let (r, c) = (dims[0], dims[1]);
    let is_vector = r == 1 || c == 1;

    if is_vector {
        // Build an M×M matrix placing the vector on the k-th diagonal.
        let n = a.numel();
        let m = n + k.unsigned_abs() as usize;
        // Source linear position for output (row,col); usize::MAX marks a zero.
        let mut perm = vec![usize::MAX; m * m];
        for i in 0..n {
            let (row, col) = if k < 0 {
                (i + k.unsigned_abs() as usize, i)
            } else {
                (i, i + k as usize)
            };
            perm[row + col * m] = i;
        }
        Ok(vec![gather_diag(a, &[m, m], &perm)])
    } else {
        // Extract the k-th diagonal of the matrix as a column vector.
        let out_len = if k < 0 {
            (r as i64 + k).max(0).min(c as i64) as usize
        } else {
            (r as i64).min(c as i64 - k).max(0) as usize
        };
        let mut perm = vec![usize::MAX; out_len];
        for (idx, slot) in perm.iter_mut().enumerate() {
            let (row, col) = if k < 0 {
                (idx + k.unsigned_abs() as usize, idx)
            } else {
                (idx, idx + k as usize)
            };
            *slot = row + col * r;
        }
        Ok(vec![gather_diag(a, &[out_len, 1], &perm)])
    }
}

/// Gather elements of `a` into a new array of shape `dims` using the per-output
/// source linear positions in `perm` (`usize::MAX` ⇒ a zero fill). Preserves
/// the source class (real / complex / char), matching FreeMat's `diag`.
fn gather_diag(a: &Array, dims: &[usize], perm: &[usize]) -> Array {
    match a {
        Array::Char(_) | Array::Scalar(fm_core::ScalarValue::Char(_)) => {
            let flat: Vec<char> = a.as_string().unwrap_or_default().chars().collect();
            let data: Vec<char> = perm
                .iter()
                .map(|&p| if p == usize::MAX { '\u{0}' } else { flat[p] })
                .collect();
            fm_interp::value::char_matrix(dims, data)
        }
        _ if a.is_complex() => {
            let flat = fm_interp::value::to_c64_vec(a);
            let data = perm
                .iter()
                .map(|&p| {
                    if p == usize::MAX {
                        fm_core::C64::new(0.0, 0.0)
                    } else {
                        flat[p]
                    }
                })
                .collect();
            fm_interp::value::build_complex(dims, data)
        }
        _ => {
            let flat = to_f64_vec(a);
            let data: Vec<f64> = perm
                .iter()
                .map(|&p| if p == usize::MAX { 0.0 } else { flat[p] })
                .collect();
            build_real(a.class(), dims, data)
        }
    }
}
