//! Miscellaneous numeric builtins: `vec`, `diff`, `dot`, `cross`, `meshgrid`,
//! `ndgrid`, `deal`, and special functions `erf`/`erfc`/`gamma`/`gammaln`.

use fm_core::{Array, DataClass};
use fm_interp::error::Flow;
use fm_interp::value::{build_real, to_f64_vec};
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
