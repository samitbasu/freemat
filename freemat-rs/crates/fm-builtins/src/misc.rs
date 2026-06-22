//! Miscellaneous numeric builtins: `vec`, `diff`, `dot`, `cross`, `meshgrid`,
//! `ndgrid`, `deal`, and special functions `erf`/`erfc`/`gamma`/`gammaln`.

use fm_core::{Array, C64, DataClass};
use fm_interp::error::Flow;
use fm_interp::value::{build_real, to_c64_vec, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};

use crate::util::{err, map_double, need};

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("vec", b_vec);
    table.add_builtin("diff", b_diff);
    table.add_builtin("dot", b_dot);
    table.add_builtin("cross", b_cross);
    table.add_builtin("meshgrid", b_meshgrid);
    table.add_builtin("ndgrid", b_ndgrid);
    table.add_builtin("deal", b_deal);
    table.add_builtin("erf", |_i, a, _n| {
        need(a, 1, "erf")?;
        Ok(vec![map_double(&a[0], erf)])
    });
    table.add_builtin("erfc", |_i, a, _n| {
        need(a, 1, "erfc")?;
        Ok(vec![map_double(&a[0], |x| 1.0 - erf(x))])
    });
    table.add_builtin("gamma", |_i, a, _n| {
        need(a, 1, "gamma")?;
        Ok(vec![map_double(&a[0], gamma)])
    });
    table.add_builtin("gammaln", |_i, a, _n| {
        need(a, 1, "gammaln")?;
        Ok(vec![map_double(&a[0], |x| gamma(x).abs().ln())])
    });
    table.add_builtin("erfinv", |_i, a, _n| {
        need(a, 1, "erfinv")?;
        Ok(vec![map_double(&a[0], erfinv)])
    });
    table.add_builtin("idiv", b_idiv);
    table.add_builtin("betainc", b_betainc);
    table.add_builtin("legendre", b_legendre);
    table.add_builtin("interplin1", b_interplin1);
    table.add_builtin("cov", b_cov);
    table.add_builtin("teps", b_teps);
}

/// `teps(x)` — type-based epsilon: `feps` for single-precision inputs, `eps`
/// for double / integer inputs. Mirrors FreeMat's `toolbox/numerical/teps.m`.
fn b_teps(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "teps")?;
    let cls = args[0].class();
    match cls {
        DataClass::Float => Ok(vec![Array::single(f32::EPSILON)]),
        DataClass::Double
        | DataClass::Int8
        | DataClass::UInt8
        | DataClass::Int16
        | DataClass::UInt16
        | DataClass::Int32
        | DataClass::UInt32
        | DataClass::Int64
        | DataClass::UInt64 => Ok(vec![Array::double(f64::EPSILON)]),
        _ => err("teps only applies to numerical arrays"),
    }
}

/// `cov(x)` / `cov(x,z)` / `cov(x,d)` / `cov(x,z,d)` — covariance.
///
/// Mirrors FreeMat's `toolbox/stat/cov.m`:
/// - `cov(x)` with a vector returns the (scalar) variance of `x`.
/// - `cov(x)` with a matrix returns the covariance matrix of its columns,
///   normalized by `L-1` (L = number of rows).
/// - `cov(x,z)` with same-size args is `cov([x(:),z(:)])`.
/// - A scalar `1` (biased) normalizes by `L`; `0` (default) by `max(1,L-1)`.
fn b_cov(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "cov")?;
    let is_vector = |a: &Array| {
        let d = a.dims();
        d.len() == 2 && (d[0] == 1 || d[1] == 1)
    };
    let is_scalar = |a: &Array| a.numel() == 1;

    match args.len() {
        1 => {
            if is_vector(&args[0]) {
                let v = variance_vec(&to_f64_vec(&args[0]), 1.0);
                Ok(vec![Array::double(v)])
            } else {
                let rows = args[0].dims()[0];
                cov_matrix(&args[0], (rows.max(1) - 1).max(1) as f64)
            }
        }
        2 => {
            if is_scalar(&args[1]) && !is_vector(&args[0]) {
                // cov(x, d): normalization flag.
                let d = args[1].as_f64().unwrap_or(0.0);
                let rows = args[0].dims()[0];
                let nf = if d == 1.0 {
                    rows as f64
                } else {
                    (rows.max(1) - 1).max(1) as f64
                };
                cov_matrix(&args[0], nf)
            } else if args[0].dims() == args[1].dims() {
                // cov(x, z): build [x(:), z(:)] and recurse.
                let stacked = stack_cols(&args[0], &args[1]);
                let rows = stacked.dims()[0];
                cov_matrix(&stacked, (rows.max(1) - 1).max(1) as f64)
            } else if is_scalar(&args[1]) && is_vector(&args[0]) {
                // cov(vector, d): variance with the given normalization.
                let d = args[1].as_f64().unwrap_or(0.0);
                let data = to_f64_vec(&args[0]);
                let nf = if d == 1.0 { 0.0 } else { 1.0 };
                Ok(vec![Array::double(variance_vec(&data, nf))])
            } else {
                err("cov: unrecognized form for call to cov")
            }
        }
        _ => {
            // cov(x, z, d)
            if args[0].dims() != args[1].dims() {
                return err("cov: x and z must be the same size");
            }
            let stacked = stack_cols(&args[0], &args[1]);
            let d = args[2].as_f64().unwrap_or(0.0);
            let rows = stacked.dims()[0];
            let nf = if d == 1.0 {
                rows as f64
            } else {
                (rows.max(1) - 1).max(1) as f64
            };
            cov_matrix(&stacked, nf)
        }
    }
}

/// Variance of a flat sample. `ddof` is subtracted from N for the divisor
/// (1.0 → N-1, 0.0 → N).
fn variance_vec(data: &[f64], ddof: f64) -> f64 {
    let n = data.len() as f64;
    if n == 0.0 {
        return f64::NAN;
    }
    let mean = data.iter().sum::<f64>() / n;
    let ss: f64 = data.iter().map(|&v| (v - mean) * (v - mean)).sum();
    let div = (n - ddof).max(1.0);
    ss / div
}

/// Build the `[x(:), z(:)]` column-stacked matrix from two same-shape arrays.
fn stack_cols(x: &Array, z: &Array) -> Array {
    let xv = to_f64_vec(x);
    let zv = to_f64_vec(z);
    let m = xv.len();
    let mut data = Vec::with_capacity(2 * m);
    data.extend_from_slice(&xv);
    data.extend_from_slice(&zv);
    build_real(DataClass::Double, &[m, 2], data)
}

/// Covariance matrix of the columns of `x` with explicit normalization factor.
/// `y = (xc' * xc) / norm_factor` where `xc` is column-mean-centered `x`.
fn cov_matrix(x: &Array, norm_factor: f64) -> Flow<Vec<Array>> {
    let dims = x.dims();
    if dims.len() != 2 {
        return err("cov: only 2-D matrices are supported");
    }
    let (m, c) = (dims[0], dims[1]);
    let data = to_f64_vec(x);
    // Column means.
    let mut means = vec![0.0f64; c];
    for j in 0..c {
        let mut s = 0.0;
        for i in 0..m {
            s += data[i + j * m];
        }
        means[j] = if m > 0 { s / m as f64 } else { 0.0 };
    }
    // Centered data.
    let mut xc = vec![0.0f64; m * c];
    for j in 0..c {
        for i in 0..m {
            xc[i + j * m] = data[i + j * m] - means[j];
        }
    }
    // y(a,b) = sum_i xc(i,a)*xc(i,b) / norm_factor.
    let mut y = vec![0.0f64; c * c];
    for a in 0..c {
        for b in 0..c {
            let mut s = 0.0;
            for i in 0..m {
                s += xc[i + a * m] * xc[i + b * m];
            }
            y[a + b * c] = s / norm_factor;
        }
    }
    Ok(vec![build_real(DataClass::Double, &[c, c], y)])
}

/// `betainc(x, a, b)` — regularized incomplete beta function `I_x(a,b)`.
/// FreeMat maps `betainc(X,Y,Z)` to `ibeta(Y, Z, X)`, i.e. `a=Y`, `b=Z`.
/// Supports scalar broadcasting of any argument. `tail='upper'` returns the
/// complementary integral `1 - I_x(a,b)`.
fn b_betainc(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 3, "betainc")?;
    let upper = if args.len() >= 4 {
        match args[3].as_string() {
            Some(s) if s == "upper" => true,
            Some(s) if s == "lower" => false,
            Some(_) => return err("betainc: tail must be 'lower' or 'upper'"),
            None => return err("betainc: tail must be a string"),
        }
    } else {
        false
    };
    let x = to_f64_vec(&args[0]);
    let a = to_f64_vec(&args[1]);
    let b = to_f64_vec(&args[2]);
    // Determine output dims/length: the largest non-scalar argument.
    let lens = [x.len(), a.len(), b.len()];
    let n = *lens.iter().max().unwrap();
    for (arg, &l) in args[..3].iter().zip(lens.iter()) {
        if l != 1 && l != n {
            let _ = arg;
            return err("betainc: all arrays must be the same size or scalar");
        }
    }
    let dims = if x.len() == n {
        args[0].dims()
    } else if a.len() == n {
        args[1].dims()
    } else {
        args[2].dims()
    };
    let at = |v: &[f64], i: usize| if v.len() == 1 { v[0] } else { v[i] };
    let data: Vec<f64> = (0..n)
        .map(|i| {
            let v = betai(at(&a, i), at(&b, i), at(&x, i));
            if upper { 1.0 - v } else { v }
        })
        .collect();
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

/// Regularized incomplete beta function `I_x(a,b)` via the Numerical Recipes
/// continued-fraction expansion (`betai`/`betacf`).
fn betai(a: f64, b: f64, x: f64) -> f64 {
    if !(0.0..=1.0).contains(&x) {
        return f64::NAN;
    }
    if x == 0.0 {
        return 0.0;
    }
    if x == 1.0 {
        return 1.0;
    }
    // ln B(a,b) factor.
    let bt = (lgamma(a + b) - lgamma(a) - lgamma(b) + a * x.ln() + b * (1.0 - x).ln()).exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        bt * betacf(a, b, x) / a
    } else {
        1.0 - bt * betacf(b, a, 1.0 - x) / b
    }
}

/// Continued-fraction evaluation for the incomplete beta function (NR `betacf`).
fn betacf(a: f64, b: f64, x: f64) -> f64 {
    const MAXIT: usize = 200;
    const EPS: f64 = 3.0e-12;
    const FPMIN: f64 = 1.0e-300;
    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < FPMIN {
        d = FPMIN;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..=MAXIT {
        let m = m as f64;
        let m2 = 2.0 * m;
        // Even step.
        let aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = 1.0 + aa / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        h *= d * c;
        // Odd step.
        let aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = 1.0 + aa / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < EPS {
            break;
        }
    }
    h
}

/// Natural log of the gamma function (Lanczos), used by `betai`.
fn lgamma(x: f64) -> f64 {
    gamma(x).abs().ln()
}

/// `legendre(n, x)` — the (unassociated) Legendre polynomial `P_n(x)`,
/// matching FreeMat's `boost::math::legendre_p<double>(n, x)`. The result has
/// the same size as `x`.
fn b_legendre(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "legendre")?;
    let n = args[0]
        .as_f64()
        .ok_or_else(|| crate::util::err_signal("legendre: first argument must be a scalar"))?;
    if n < 0.0 || n.fract() != 0.0 {
        return err("legendre: first argument must be a non-negative integer");
    }
    let n = n as u32;
    let dims = args[1].dims();
    let data: Vec<f64> = to_f64_vec(&args[1])
        .into_iter()
        .map(|x| legendre_p(n, x))
        .collect();
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

/// Legendre polynomial `P_n(x)` via the standard three-term recurrence.
fn legendre_p(n: u32, x: f64) -> f64 {
    if n == 0 {
        return 1.0;
    }
    if n == 1 {
        return x;
    }
    let mut p0 = 1.0;
    let mut p1 = x;
    for k in 2..=n {
        let k = k as f64;
        let p2 = ((2.0 * k - 1.0) * x * p1 - (k - 1.0) * p0) / k;
        p0 = p1;
        p1 = p2;
    }
    p1
}

/// `interplin1(x1, y1, xi[, extrapflag])` — piecewise-linear 1-D interpolation.
/// Mirrors FreeMat's `Interplin1Function`. `x1` must be monotonically
/// increasing; `extrapflag` is one of `'nan'` (default), `'zero'`,
/// `'endpoint'`, `'extrap'`. Complex `y1` is interpolated component-wise.
fn b_interplin1(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 3, "interplin1")?;
    if args[0].is_complex() || args[2].is_complex() {
        return err("interplin1: x-coordinates cannot be complex");
    }
    let x1 = to_f64_vec(&args[0]);
    let xi = to_f64_vec(&args[2]);
    let y_complex = args[1].is_complex();
    let xtrap = if args.len() >= 4 {
        match args[3].as_string().as_deref() {
            Some("nan") => 0,
            Some("zero") => 1,
            Some("endpoint") => 2,
            Some("extrap") => 3,
            Some(_) => return err("interplin1: unrecognized extrapolation type flag"),
            None => return err("interplin1: extrapolation flag must be a string"),
        }
    } else {
        0
    };
    let y_len = args[1].numel();
    if x1.len() != y_len {
        return err("interplin1: vectors x1 and y1 must be the same length");
    }
    for w in x1.windows(2) {
        if w[0] > w[1] {
            return err("interplin1: vector x1 must be monotonically increasing");
        }
    }
    let dims = args[2].dims();
    if y_complex {
        let yc = to_c64_vec(&args[1]);
        let yr: Vec<f64> = yc.iter().map(|c| c.re).collect();
        let yi: Vec<f64> = yc.iter().map(|c| c.im).collect();
        let re = lin_interp(&x1, &yr, &xi, xtrap);
        let im = lin_interp(&x1, &yi, &xi, xtrap);
        let data: Vec<C64> = re
            .into_iter()
            .zip(im)
            .map(|(r, i)| C64::new(r, i))
            .collect();
        Ok(vec![fm_interp::value::build_complex(&dims, data)])
    } else {
        let y1 = to_f64_vec(&args[1]);
        let out = lin_interp(&x1, &y1, &xi, xtrap);
        Ok(vec![build_real(DataClass::Double, &dims, out)])
    }
}

/// Core linear-interpolation loop shared by the real and complex branches.
fn lin_interp(x1: &[f64], y1: &[f64], xi: &[f64], xtrap: i32) -> Vec<f64> {
    let n = x1.len();
    xi.iter()
        .map(|&xq| {
            if n == 0 {
                return f64::NAN;
            }
            if n == 1 {
                return if xq == x1[0] {
                    y1[0]
                } else {
                    extrap_scalar(y1, xtrap, xq, x1)
                };
            }
            if xq < x1[0] {
                // Below range.
                return match xtrap {
                    3 => {
                        let frac = (xq - x1[0]) / (x1[1] - x1[0]);
                        y1[0] + frac * (y1[1] - y1[0])
                    }
                    1 => 0.0,
                    2 => y1[0],
                    _ => f64::NAN,
                };
            }
            if xq > x1[n - 1] {
                // Above range.
                return match xtrap {
                    3 => {
                        let frac = (xq - x1[n - 2]) / (x1[n - 1] - x1[n - 2]);
                        y1[n - 2] + frac * (y1[n - 1] - y1[n - 2])
                    }
                    1 => 0.0,
                    2 => y1[n - 1],
                    _ => f64::NAN,
                };
            }
            // In range: binary-search for the last index with x1[i] <= xq.
            let mut lo = 0usize;
            let mut hi = n - 1;
            while lo < hi {
                let mid = (lo + hi).div_ceil(2);
                if x1[mid] <= xq {
                    lo = mid;
                } else {
                    hi = mid - 1;
                }
            }
            let left = lo.min(n - 2);
            let denom = x1[left + 1] - x1[left];
            if denom == 0.0 {
                return y1[left];
            }
            let frac = (xq - x1[left]) / denom;
            y1[left] + frac * (y1[left + 1] - y1[left])
        })
        .collect()
}

/// Extrapolation for the degenerate single-knot case.
fn extrap_scalar(y1: &[f64], xtrap: i32, _xq: f64, _x1: &[f64]) -> f64 {
    match xtrap {
        1 => 0.0,
        2 => y1[0],
        3 => y1[0],
        _ => f64::NAN,
    }
}

/// `idiv(a, b)` — integer (truncating-toward-zero) division, element-wise with
/// scalar broadcast. Mirrors FreeMat's `IdivFunction`.
fn b_idiv(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "idiv")?;
    let a = to_f64_vec(&args[0]);
    let b = to_f64_vec(&args[1]);
    let (dims, n) = if args[0].numel() == 1 {
        (args[1].dims(), b.len())
    } else {
        (args[0].dims(), a.len())
    };
    if a.len() != 1 && b.len() != 1 && a.len() != b.len() {
        return err("idiv: arguments must be the same size or scalar");
    }
    let at = |v: &[f64], i: usize| if v.len() == 1 { v[0] } else { v[i] };
    let data: Vec<f64> = (0..n)
        .map(|i| {
            let d = at(&b, i);
            if d == 0.0 {
                0.0
            } else {
                (at(&a, i) / d).trunc()
            }
        })
        .collect();
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

/// Inverse error function via the Giles (2010) rational approximation, refined
/// with one Newton step using `erf`.
fn erfinv(x: f64) -> f64 {
    if x <= -1.0 {
        return f64::NEG_INFINITY;
    }
    if x >= 1.0 {
        return f64::INFINITY;
    }
    if x == 0.0 {
        return 0.0;
    }
    let w = -((1.0 - x) * (1.0 + x)).ln();
    let mut p;
    if w < 5.0 {
        let w = w - 2.5;
        p = 2.810_226_36e-08;
        p = 3.432_739_39e-07 + p * w;
        p = -3.523_387_7e-06 + p * w;
        p = -4.391_506_54e-06 + p * w;
        p = 0.000_218_580_87 + p * w;
        p = -0.001_253_725_03 + p * w;
        p = -0.004_177_681_64 + p * w;
        p = 0.246_640_727 + p * w;
        p = 1.501_409_41 + p * w;
    } else {
        let w = w.sqrt() - 3.0;
        p = -0.000_200_214_257;
        p = 0.000_100_950_24 + p * w;
        p = 0.001_349_343_78 + p * w;
        p = -0.003_673_428_44 + p * w;
        p = 0.005_739_507_73 + p * w;
        p = -0.007_622_461_3 + p * w;
        p = 0.009_438_870_47 + p * w;
        p = 1.001_674_06 + p * w;
        p = 2.832_976_82 + p * w;
    }
    let mut r = p * x;
    // One Newton refinement: f(r) = erf(r) - x.
    let e = erf(r) - x;
    r -= e / (2.0 / std::f64::consts::PI.sqrt() * (-r * r).exp());
    r
}

/// `vec(A)` — flatten to a column vector (FreeMat's `A(:)`).
fn b_vec(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "vec")?;
    let data = to_f64_vec(&args[0]);
    let n = data.len();
    Ok(vec![build_real(DataClass::Double, &[n, 1], data)])
}

/// `diff(X)` — first difference along the first non-singleton dimension. We
/// implement the vector case and the column-wise matrix case.
fn b_diff(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "diff")?;
    let dims = args[0].dims();
    let data = to_f64_vec(&args[0]);
    // Vector case (row or column).
    if dims.len() == 2 && (dims[0] == 1 || dims[1] == 1) {
        let n = data.len();
        if n < 2 {
            let d = if dims[0] == 1 { vec![1, 0] } else { vec![0, 1] };
            return Ok(vec![build_real(DataClass::Double, &d, vec![])]);
        }
        let out: Vec<f64> = (1..n).map(|i| data[i] - data[i - 1]).collect();
        let outlen = out.len();
        let d = if dims[0] == 1 {
            vec![1, outlen]
        } else {
            vec![outlen, 1]
        };
        return Ok(vec![build_real(DataClass::Double, &d, out)]);
    }
    // Matrix case: difference down columns (column-major layout).
    if dims.len() == 2 {
        let (m, c) = (dims[0], dims[1]);
        if m < 2 {
            return Ok(vec![build_real(DataClass::Double, &[0, c], vec![])]);
        }
        let mut out = vec![0.0f64; (m - 1) * c];
        for j in 0..c {
            for i in 1..m {
                out[(i - 1) + j * (m - 1)] = data[i + j * m] - data[(i - 1) + j * m];
            }
        }
        return Ok(vec![build_real(DataClass::Double, &[m - 1, c], out)]);
    }
    err("diff: only vectors and 2-D matrices are supported")
}

/// `dot(a, b)` — scalar dot product of two vectors.
fn b_dot(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "dot")?;
    let a = to_f64_vec(&args[0]);
    let b = to_f64_vec(&args[1]);
    if a.len() != b.len() {
        return err("dot: inputs must have the same length");
    }
    let s: f64 = a.iter().zip(&b).map(|(x, y)| x * y).sum();
    Ok(vec![build_real(DataClass::Double, &[1, 1], vec![s])])
}

/// `cross(a, b)` — cross product of two 3-element vectors.
fn b_cross(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "cross")?;
    let a = to_f64_vec(&args[0]);
    let b = to_f64_vec(&args[1]);
    if a.len() != 3 || b.len() != 3 {
        return err("cross: inputs must be 3-element vectors");
    }
    let c = vec![
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    let dims = args[0].dims();
    let is_col = dims.len() == 2 && dims[1] == 1;
    if is_col {
        Ok(vec![build_real(DataClass::Double, &[3, 1], c)])
    } else {
        Ok(vec![build_real(DataClass::Double, &[1, 3], c)])
    }
}

/// `meshgrid(x)` / `meshgrid(x, y)` — `[X, Y]` grids. `X` rows are `x`, `Y`
/// columns are `y`.
fn b_meshgrid(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "meshgrid")?;
    let x = to_f64_vec(&args[0]);
    let y = if args.len() >= 2 {
        to_f64_vec(&args[1])
    } else {
        x.clone()
    };
    let (nx, ny) = (x.len(), y.len());
    // Result is ny x nx. X(i,j) = x[j], Y(i,j) = y[i]. Column-major.
    let mut xdata = vec![0.0f64; ny * nx];
    let mut ydata = vec![0.0f64; ny * nx];
    for j in 0..nx {
        for i in 0..ny {
            xdata[i + j * ny] = x[j];
            ydata[i + j * ny] = y[i];
        }
    }
    Ok(vec![
        build_real(DataClass::Double, &[ny, nx], xdata),
        build_real(DataClass::Double, &[ny, nx], ydata),
    ])
}

/// `ndgrid(x, y)` — like meshgrid but with matrix (not Cartesian) indexing:
/// `X(i,j) = x[i]`, `Y(i,j) = y[j]`.
fn b_ndgrid(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "ndgrid")?;
    let x = to_f64_vec(&args[0]);
    let y = if args.len() >= 2 {
        to_f64_vec(&args[1])
    } else {
        x.clone()
    };
    let (nx, ny) = (x.len(), y.len());
    let mut xdata = vec![0.0f64; nx * ny];
    let mut ydata = vec![0.0f64; nx * ny];
    for j in 0..ny {
        for i in 0..nx {
            xdata[i + j * nx] = x[i];
            ydata[i + j * nx] = y[j];
        }
    }
    Ok(vec![
        build_real(DataClass::Double, &[nx, ny], xdata),
        build_real(DataClass::Double, &[nx, ny], ydata),
    ])
}

/// `deal(a, b, ...)` — distribute inputs to outputs. With one input, copies it
/// to every requested output.
fn b_deal(_i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "deal")?;
    let n = nargout.max(1);
    if args.len() == 1 {
        return Ok(vec![args[0].clone(); n]);
    }
    if args.len() != n {
        return err("deal: number of inputs must match number of outputs");
    }
    Ok(args.to_vec())
}

// ---- Special functions (rational/series approximations) ----------------------

/// Error function (Abramowitz & Stegun 7.1.26, |error| < 1.5e-7).
fn erf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.3275911 * x.abs());
    let y = 1.0
        - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t
            + 0.254829592)
            * t
            * (-x * x).exp();
    if x < 0.0 { -y } else { y }
}

/// Gamma function via the Lanczos approximation.
fn gamma(x: f64) -> f64 {
    const G: f64 = 7.0;
    const C: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        // Reflection formula.
        std::f64::consts::PI / ((std::f64::consts::PI * x).sin() * gamma(1.0 - x))
    } else {
        let x = x - 1.0;
        let mut a = C[0];
        let t = x + G + 0.5;
        for (i, &c) in C.iter().enumerate().skip(1) {
            a += c / (x + i as f64);
        }
        (2.0 * std::f64::consts::PI).sqrt() * t.powf(x + 0.5) * (-t).exp() * a
    }
}
