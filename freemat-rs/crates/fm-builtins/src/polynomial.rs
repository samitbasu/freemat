//! Polynomial builtins: `polyval`, `polyfit`, `roots`, `poly`, `polyder`,
//! `polyint`, `conv`, `deconv`. Coefficient vectors are ordered highest power
//! first (MATLAB convention).

use fm_core::{Array, DataClass};
use fm_interp::error::{Flow, InterpError, Signal};
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};

use crate::util::{err, need};

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("polyval", b_polyval);
    table.add_builtin("polyfit", b_polyfit);
    table.add_builtin("roots", b_roots);
    table.add_builtin("poly", b_poly);
    table.add_builtin("polyder", b_polyder);
    table.add_builtin("polyint", b_polyint);
    table.add_builtin("conv", b_conv);
    table.add_builtin("deconv", b_deconv);
}

/// `polyval(p, x)` — evaluate the polynomial `p` (highest power first) at each
/// element of `x` via Horner's rule.
fn b_polyval(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "polyval")?;
    let p = to_f64_vec(&args[0]);
    let x = to_f64_vec(&args[1]);
    let dims = args[1].dims();
    let data: Vec<f64> = x
        .iter()
        .map(|&xi| p.iter().fold(0.0, |acc, &c| acc * xi + c))
        .collect();
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

/// `polyfit(x, y, n)` — least-squares fit of degree `n`, returning the
/// coefficient row vector (highest power first).
fn b_polyfit(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 3, "polyfit")?;
    let x = to_f64_vec(&args[0]);
    let y = to_f64_vec(&args[1]);
    let n = args[2].as_f64().unwrap_or(1.0) as usize;
    if x.len() != y.len() {
        return err("polyfit: x and y must have the same length");
    }
    let m = x.len();
    let cols = n + 1;
    // Vandermonde matrix V (m x cols), column-major, highest power first.
    let mut vdata = vec![0.0f64; m * cols];
    for (i, &xi) in x.iter().enumerate() {
        for j in 0..cols {
            // column j corresponds to power n-j.
            let power = (n - j) as i32;
            vdata[i + j * m] = xi.powi(power);
        }
    }
    let vand = Array::double_matrix(&[m, cols], vdata);
    let yvec = Array::double_matrix(&[m, 1], y);
    // Solve the least-squares system V \ y.
    let coeffs = fm_linalg::mldivide(&vand, &yvec)
        .map_err(|e| Signal::Error(InterpError::msg(e.message)))?;
    let c = to_f64_vec(&coeffs);
    Ok(vec![Array::double_matrix(&[1, cols], c)])
}

/// `roots(p)` — roots of the polynomial via companion-matrix eigenvalues.
fn b_roots(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "roots")?;
    let p = to_f64_vec(&args[0]);
    let r = fm_linalg::roots(&p).map_err(|e| Signal::Error(InterpError::msg(e.message)))?;
    Ok(vec![r])
}

/// `poly(r)` — polynomial (highest power first) whose roots are `r`. If given a
/// square matrix, returns its characteristic polynomial.
fn b_poly(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "poly")?;
    let dims = args[0].dims();
    let roots = if dims.len() == 2 && dims[0] > 1 && dims[1] > 1 {
        // Matrix input: roots are its eigenvalues.
        let ev =
            fm_linalg::eig(&args[0], 1).map_err(|e| Signal::Error(InterpError::msg(e.message)))?;
        to_f64_vec(&ev[0])
    } else {
        to_f64_vec(&args[0])
    };
    // Build coefficients by repeated multiplication: p = prod (x - r_k).
    let mut coeffs = vec![1.0f64];
    for &r in &roots {
        // multiply coeffs by (x - r): new[i] = coeffs[i] - r*coeffs[i-1].
        let mut next = vec![0.0f64; coeffs.len() + 1];
        for (i, &c) in coeffs.iter().enumerate() {
            next[i] += c;
            next[i + 1] -= r * c;
        }
        coeffs = next;
    }
    let len = coeffs.len();
    Ok(vec![Array::double_matrix(&[1, len], coeffs)])
}

/// `polyder(p)` — derivative coefficients of the polynomial `p`.
fn b_polyder(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "polyder")?;
    let p = to_f64_vec(&args[0]);
    let deg = p.len();
    if deg <= 1 {
        return Ok(vec![build_real(DataClass::Double, &[1, 1], vec![0.0])]);
    }
    // d/dx of c_k x^(deg-1-k): coefficient c_k * (deg-1-k).
    let n = deg - 1;
    let data: Vec<f64> = (0..n).map(|k| p[k] * (n - k) as f64).collect();
    let len = data.len();
    Ok(vec![Array::double_matrix(&[1, len], data)])
}

/// `polyint(p)` / `polyint(p, k)` — integral of the polynomial with constant
/// of integration `k` (default 0).
fn b_polyint(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "polyint")?;
    let p = to_f64_vec(&args[0]);
    let k = args.get(1).and_then(Array::as_f64).unwrap_or(0.0);
    let deg = p.len();
    // integral of c_j x^(deg-1-j) is c_j/(deg-j) x^(deg-j).
    let mut data: Vec<f64> = (0..deg).map(|j| p[j] / (deg - j) as f64).collect();
    data.push(k);
    let len = data.len();
    Ok(vec![Array::double_matrix(&[1, len], data)])
}

/// `conv(a, b)` — polynomial multiplication / discrete convolution.
fn b_conv(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "conv")?;
    let a = to_f64_vec(&args[0]);
    let b = to_f64_vec(&args[1]);
    if a.is_empty() || b.is_empty() {
        return Ok(vec![build_real(DataClass::Double, &[1, 0], vec![])]);
    }
    let out = convolve(&a, &b);
    // Orientation: row vector unless first input is a column vector.
    let dims0 = args[0].dims();
    let is_col = dims0.len() == 2 && dims0[1] == 1 && dims0[0] > 1;
    let len = out.len();
    if is_col {
        Ok(vec![Array::double_matrix(&[len, 1], out)])
    } else {
        Ok(vec![Array::double_matrix(&[1, len], out)])
    }
}

fn convolve(a: &[f64], b: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0f64; a.len() + b.len() - 1];
    for (i, &ai) in a.iter().enumerate() {
        for (j, &bj) in b.iter().enumerate() {
            out[i + j] += ai * bj;
        }
    }
    out
}

/// `deconv(a, b)` — polynomial division: returns `[q, r]` with `a = conv(b,q)+r`.
fn b_deconv(_i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 2, "deconv")?;
    let a = to_f64_vec(&args[0]);
    let b = to_f64_vec(&args[1]);
    if b.is_empty() || b[0] == 0.0 {
        return err("deconv: leading divisor coefficient must be non-zero");
    }
    if a.len() < b.len() {
        let q = vec![0.0f64];
        let r = a.clone();
        let rlen = r.len();
        let mut out = vec![Array::double_matrix(&[1, 1], q)];
        if nargout >= 2 {
            out.push(Array::double_matrix(&[1, rlen], r));
        }
        return Ok(out);
    }
    let mut rem = a.clone();
    let qlen = a.len() - b.len() + 1;
    let mut q = vec![0.0f64; qlen];
    for i in 0..qlen {
        let coef = rem[i] / b[0];
        q[i] = coef;
        for (j, &bj) in b.iter().enumerate() {
            rem[i + j] -= coef * bj;
        }
    }
    let mut out = vec![Array::double_matrix(&[1, qlen], q)];
    if nargout >= 2 {
        let rlen = rem.len();
        out.push(Array::double_matrix(&[1, rlen], rem));
    }
    Ok(out)
}
