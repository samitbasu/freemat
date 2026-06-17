//! Array constructors: `zeros`, `ones`, `eye`, `linspace`, `logspace`,
//! `repmat`, `diag`, `colon`.

use fm_core::{Array, DataClass};
use fm_interp::error::Flow;
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};

use crate::util::{err, need, scalar_arg};

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
    let src = to_f64_vec(a);
    let (om, on) = (m * p, n * q);
    let mut data = vec![0.0; om * on];
    for tj in 0..q {
        for ti in 0..p {
            for j in 0..n {
                for i in 0..m {
                    let oi = ti * m + i;
                    let oj = tj * n + j;
                    data[oi + oj * om] = src[i + j * m];
                }
            }
        }
    }
    Ok(vec![build_real(DataClass::Double, &[om, on], data)])
}

/// `diag(v)` builds a diagonal matrix from a vector; `diag(M)` extracts the
/// main diagonal of a matrix.
fn b_diag(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "diag")?;
    let a = &args[0];
    let dims = a.dims();
    let data = to_f64_vec(a);
    let is_vector = dims.len() == 2 && (dims[0] == 1 || dims[1] == 1);
    if is_vector {
        let k = data.len();
        let mut out = vec![0.0; k * k];
        for (i, &v) in data.iter().enumerate() {
            out[i + i * k] = v;
        }
        Ok(vec![build_real(DataClass::Double, &[k, k], out)])
    } else if dims.len() == 2 {
        let (r, c) = (dims[0], dims[1]);
        let k = r.min(c);
        let out: Vec<f64> = (0..k).map(|i| data[i + i * r]).collect();
        Ok(vec![build_real(DataClass::Double, &[k, 1], out)])
    } else {
        err("diag: input must be 2-D")
    }
}
