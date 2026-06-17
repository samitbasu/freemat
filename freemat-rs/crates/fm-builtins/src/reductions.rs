//! Reductions: `sum`, `prod`, `mean`, `min`, `max`, `cumsum`, `cumprod`,
//! `median`, `var`, `std`, `norm` (vector). `min`/`max` return `[val, idx]`.
//!
//! MATLAB reduces along the **first non-singleton dimension** by default (for a
//! 2-D matrix: down each column), or along an explicit `dim` argument. The
//! result keeps the reduced dimension at length 1.

use fm_core::{Array, DataClass};
use fm_interp::error::Flow;
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};

use crate::util::{err, need};

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("sum", b_sum);
    table.add_builtin("prod", b_prod);
    table.add_builtin("mean", b_mean);
    table.add_builtin("min", b_min);
    table.add_builtin("max", b_max);
    table.add_builtin("cumsum", b_cumsum);
    table.add_builtin("cumprod", b_cumprod);
    table.add_builtin("median", b_median);
    table.add_builtin("var", b_var);
    table.add_builtin("std", b_std);
}

/// The first non-singleton dimension of `dims` (1-based), or 1 if all singleton.
fn first_nonsingleton(dims: &[usize]) -> usize {
    for (i, &d) in dims.iter().enumerate() {
        if d != 1 {
            return i + 1;
        }
    }
    1
}

/// Resolve the reduction dimension: explicit `args[1]` if present, else the
/// first non-singleton dimension.
fn resolve_dim(args: &[Array], dims: &[usize]) -> usize {
    if let Some(d) = args.get(1).and_then(Array::as_f64) {
        (d.round() as usize).max(1)
    } else {
        first_nonsingleton(dims)
    }
}

/// Reduce `a` along dimension `dim` (1-based) with `f`, seeded by `init`. Returns
/// the reduced data (column-major) plus the output dimensions.
fn reduce(a: &Array, dim: usize, init: f64, f: impl Fn(f64, f64) -> f64) -> (Vec<f64>, Vec<usize>) {
    let dims = a.dims();
    let data = to_f64_vec(a);
    let d = dim - 1;
    if d >= dims.len() || dims[d] <= 1 {
        // Reducing over a singleton (or absent) dimension is the identity.
        return (data, dims);
    }
    let mut out_dims = dims.clone();
    out_dims[d] = 1;
    let stride: usize = dims[..d].iter().product();
    let dlen = dims[d];
    let outer: usize = dims[d + 1..].iter().product();
    let mut out = Vec::with_capacity(out_dims.iter().product());
    for o in 0..outer {
        for s in 0..stride {
            let base = o * stride * dlen + s;
            let mut acc = init;
            for k in 0..dlen {
                acc = f(acc, data[base + k * stride]);
            }
            out.push(acc);
        }
    }
    (out, out_dims)
}

fn b_sum(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "sum")?;
    let dim = resolve_dim(args, &args[0].dims());
    let (data, dims) = reduce(&args[0], dim, 0.0, |a, b| a + b);
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

fn b_prod(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "prod")?;
    let dim = resolve_dim(args, &args[0].dims());
    let (data, dims) = reduce(&args[0], dim, 1.0, |a, b| a * b);
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

fn b_mean(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "mean")?;
    let dims = args[0].dims();
    let dim = resolve_dim(args, &dims);
    let n = dims.get(dim - 1).copied().unwrap_or(1).max(1) as f64;
    let (data, out_dims) = reduce(&args[0], dim, 0.0, |a, b| a + b);
    let data = data.into_iter().map(|s| s / n).collect();
    Ok(vec![build_real(DataClass::Double, &out_dims, data)])
}

/// `min`/`max` with `[val, idx]` outputs. With two args (`min(a,b)`), this is an
/// element-wise binary minimum/maximum instead.
fn minmax(args: &[Array], nargout: usize, want_max: bool, name: &str) -> Flow<Vec<Array>> {
    need(args, 1, name)?;
    // `min(a, b)` element-wise form: second arg present and non-empty, no dim.
    if args.len() >= 2 && args[1].numel() > 0 {
        return elementwise_minmax(&args[0], &args[1], want_max);
    }
    let dims = args[0].dims();
    let dim = if args.len() >= 3 {
        args[2]
            .as_f64()
            .map(|d| d.round() as usize)
            .unwrap_or(1)
            .max(1)
    } else {
        first_nonsingleton(&dims)
    };
    let data = to_f64_vec(&args[0]);
    let d = dim - 1;
    if d >= dims.len() || dims[d] <= 1 {
        let idx = vec![1.0; data.len()];
        return Ok(vec![
            build_real(DataClass::Double, &dims, data),
            build_real(DataClass::Double, &dims, idx),
        ]);
    }
    let mut out_dims = dims.clone();
    out_dims[d] = 1;
    let stride: usize = dims[..d].iter().product();
    let dlen = dims[d];
    let outer: usize = dims[d + 1..].iter().product();
    let mut vals = Vec::new();
    let mut idxs = Vec::new();
    for o in 0..outer {
        for s in 0..stride {
            let base = o * stride * dlen + s;
            let mut best = data[base];
            let mut best_i = 0usize;
            for k in 1..dlen {
                let v = data[base + k * stride];
                let take = if want_max { v > best } else { v < best };
                if take || best.is_nan() {
                    best = v;
                    best_i = k;
                }
            }
            vals.push(best);
            idxs.push((best_i + 1) as f64);
        }
    }
    let mut out = vec![build_real(DataClass::Double, &out_dims, vals)];
    if nargout >= 2 {
        out.push(build_real(DataClass::Double, &out_dims, idxs));
    }
    Ok(out)
}

fn elementwise_minmax(a: &Array, b: &Array, want_max: bool) -> Flow<Vec<Array>> {
    let x = to_f64_vec(a);
    let y = to_f64_vec(b);
    if x.len() != y.len() && x.len() != 1 && y.len() != 1 {
        return err("min/max: array sizes must match");
    }
    let (n, dims) = if x.len() >= y.len() {
        (x.len(), a.dims())
    } else {
        (y.len(), b.dims())
    };
    let data: Vec<f64> = (0..n)
        .map(|i| {
            let xi = if x.len() == 1 { x[0] } else { x[i] };
            let yi = if y.len() == 1 { y[0] } else { y[i] };
            if want_max { xi.max(yi) } else { xi.min(yi) }
        })
        .collect();
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

fn b_min(_i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    minmax(args, nargout, false, "min")
}

fn b_max(_i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    minmax(args, nargout, true, "max")
}

/// Cumulative reduction along `dim`.
fn cumulative(a: &Array, dim: usize, init: f64, f: impl Fn(f64, f64) -> f64) -> Vec<f64> {
    let dims = a.dims();
    let mut data = to_f64_vec(a);
    let d = dim - 1;
    if d >= dims.len() {
        return data;
    }
    let stride: usize = dims[..d].iter().product();
    let dlen = dims[d];
    let outer: usize = dims[d + 1..].iter().product();
    for o in 0..outer {
        for s in 0..stride {
            let base = o * stride * dlen + s;
            let mut acc = init;
            for k in 0..dlen {
                acc = f(acc, data[base + k * stride]);
                data[base + k * stride] = acc;
            }
        }
    }
    data
}

fn b_cumsum(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "cumsum")?;
    let dims = args[0].dims();
    let dim = resolve_dim(args, &dims);
    let data = cumulative(&args[0], dim, 0.0, |a, b| a + b);
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

fn b_cumprod(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "cumprod")?;
    let dims = args[0].dims();
    let dim = resolve_dim(args, &dims);
    let data = cumulative(&args[0], dim, 1.0, |a, b| a * b);
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

/// Apply a per-slice statistic along `dim`, returning the reduced array.
fn per_slice(a: &Array, dim: usize, f: impl Fn(&[f64]) -> f64) -> (Vec<f64>, Vec<usize>) {
    let dims = a.dims();
    let data = to_f64_vec(a);
    let d = dim - 1;
    if d >= dims.len() || dims[d] <= 1 {
        return (data, dims);
    }
    let mut out_dims = dims.clone();
    out_dims[d] = 1;
    let stride: usize = dims[..d].iter().product();
    let dlen = dims[d];
    let outer: usize = dims[d + 1..].iter().product();
    let mut out = Vec::new();
    let mut slice = vec![0.0; dlen];
    for o in 0..outer {
        for s in 0..stride {
            let base = o * stride * dlen + s;
            for k in 0..dlen {
                slice[k] = data[base + k * stride];
            }
            out.push(f(&slice));
        }
    }
    (out, out_dims)
}

fn b_median(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "median")?;
    let dims = args[0].dims();
    let dim = resolve_dim(args, &dims);
    let (data, out_dims) = per_slice(&args[0], dim, median_of);
    Ok(vec![build_real(DataClass::Double, &out_dims, data)])
}

fn median_of(s: &[f64]) -> f64 {
    let mut v = s.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = v.len();
    if n == 0 {
        f64::NAN
    } else if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

fn b_var(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "var")?;
    let dims = args[0].dims();
    let dim = resolve_dim(args, &dims);
    let (data, out_dims) = per_slice(&args[0], dim, variance_of);
    Ok(vec![build_real(DataClass::Double, &out_dims, data)])
}

fn b_std(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "std")?;
    let dims = args[0].dims();
    let dim = resolve_dim(args, &dims);
    let (data, out_dims) = per_slice(&args[0], dim, |s| variance_of(s).sqrt());
    Ok(vec![build_real(DataClass::Double, &out_dims, data)])
}

/// Sample variance (N-1 normalization, MATLAB default).
fn variance_of(s: &[f64]) -> f64 {
    let n = s.len();
    if n < 2 {
        return 0.0;
    }
    let mean = s.iter().sum::<f64>() / n as f64;
    let ss: f64 = s.iter().map(|&x| (x - mean) * (x - mean)).sum();
    ss / (n as f64 - 1.0)
}
