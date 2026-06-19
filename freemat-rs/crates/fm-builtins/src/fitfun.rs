//! Nonlinear least-squares curve fitting: the FreeMat `fitfun` builtin.
//!
//! Ported from `libs/libFN/OptFun.cpp` (`FitFunFunction`), which wraps
//! `levmar`'s `dlevmar_dif` (Levenberg–Marquardt with a forward-difference
//! Jacobian). `fitfun(fcn, xinit, y, weights, tol, varargin...)` finds the
//! parameter vector `x` minimising `sum_i (w_i * (fcn(x)_i - y_i))^2`, where
//! `fcn(x, varargin...)` returns a vector the same length as `y`.

use fm_core::{Array, DataClass};
use fm_interp::error::{Flow, InterpError, Signal};
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};
use fm_parser::Span;

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("fitfun", b_fitfun);
}

// levmar defaults (from `lm.h`): initial damping factor, finite-difference
// step, and the default stop threshold when no tolerance is supplied.
const LM_INIT_MU: f64 = 1e-3;
const LM_DIFF_DELTA: f64 = 1e-6;
const LM_STOP_THRESH: f64 = 1e-17;

fn err(msg: &str) -> Signal {
    Signal::Error(InterpError::msg(msg.to_string()))
}

/// Evaluate the user function `fcn(p, varargin...)` and return its (flattened)
/// output as a length-`n` vector. `n == 0` skips the size check (used for the
/// final call, where the raw output array is returned instead).
fn eval_fcn(
    i: &mut Interpreter,
    fcn: &Array,
    p_arr: &Array,
    varargin: &[Array],
) -> Flow<Vec<Array>> {
    let mut call_args = Vec::with_capacity(1 + varargin.len());
    call_args.push(p_arr.clone());
    call_args.extend_from_slice(varargin);
    if fcn.is_function_handle() {
        i.invoke_handle(fcn, &call_args, 1, "", Span::empty(0))
    } else {
        let name = fcn
            .as_string()
            .ok_or_else(|| err("fitfun: first argument must be a function name or handle"))?;
        i.call_function(&name, &call_args, 1, "", Span::empty(0))
    }
}

/// Run `fcn(p)` and return the model vector `w .* fcn(p)` (length `n`).
fn model(
    i: &mut Interpreter,
    fcn: &Array,
    dims: &[usize],
    p: &[f64],
    w: &[f64],
    varargin: &[Array],
) -> Flow<Vec<f64>> {
    let p_arr = build_real(DataClass::Double, dims, p.to_vec());
    let out = eval_fcn(i, fcn, &p_arr, varargin)?;
    let first = out
        .into_iter()
        .next()
        .ok_or_else(|| err("fitfun: function to be optimized does not return any outputs"))?;
    let f = to_f64_vec(&first);
    if f.len() != w.len() {
        return Err(err(
            "fitfun: function output does not match size of vector 'y'",
        ));
    }
    Ok(f.iter().zip(w).map(|(fi, wi)| fi * wi).collect())
}

/// Forward-difference Jacobian of `model` at `p`: `jac[j]` is the length-`n`
/// column `d model / d p_j` (matches `levmar`'s `lm_fdif_forw_jac_x`).
fn jacobian(
    i: &mut Interpreter,
    fcn: &Array,
    dims: &[usize],
    p: &[f64],
    w: &[f64],
    varargin: &[Array],
    m0: &[f64],
) -> Flow<Vec<Vec<f64>>> {
    let m = p.len();
    let n = m0.len();
    let mut jac = vec![vec![0.0f64; n]; m];
    for j in 0..m {
        let mut d = 1e-4 * p[j];
        if d < LM_DIFF_DELTA {
            d = LM_DIFF_DELTA;
        }
        let mut pj = p.to_vec();
        pj[j] += d;
        let mj = model(i, fcn, dims, &pj, w, varargin)?;
        let inv = 1.0 / d;
        for k in 0..n {
            jac[j][k] = (mj[k] - m0[k]) * inv;
        }
    }
    Ok(jac)
}

/// Solve the SPD system `(A + mu*I) x = b` by Gaussian elimination with partial
/// pivoting. `a` is `m x m` (row-major as `Vec<Vec<f64>>`). Returns `None` if
/// the augmented matrix is singular.
fn solve_damped(a: &[Vec<f64>], b: &[f64], mu: f64) -> Option<Vec<f64>> {
    let m = b.len();
    let mut aug: Vec<Vec<f64>> = (0..m)
        .map(|r| {
            let mut row = a[r].clone();
            row[r] += mu;
            row.push(b[r]);
            row
        })
        .collect();
    for col in 0..m {
        // Partial pivot.
        let mut piv = col;
        for r in (col + 1)..m {
            if aug[r][col].abs() > aug[piv][col].abs() {
                piv = r;
            }
        }
        if aug[piv][col].abs() < 1e-300 {
            return None;
        }
        aug.swap(col, piv);
        let pivot = aug[col][col];
        // Snapshot the pivot row's tail so the inner update borrows only `row`.
        let pivot_row: Vec<f64> = aug[col][col..=m].to_vec();
        for row in aug.iter_mut().skip(col + 1) {
            let factor = row[col] / pivot;
            if factor != 0.0 {
                for (off, &pv) in pivot_row.iter().enumerate() {
                    row[col + off] -= factor * pv;
                }
            }
        }
    }
    let mut x = vec![0.0f64; m];
    for r in (0..m).rev() {
        let mut s = aug[r][m];
        for c in (r + 1)..m {
            s -= aug[r][c] * x[c];
        }
        x[r] = s / aug[r][r];
    }
    Some(x)
}

fn inf_norm(v: &[f64]) -> f64 {
    v.iter().fold(0.0f64, |acc, &x| acc.max(x.abs()))
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn b_fitfun(i: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    if args.len() < 4 {
        return Err(err("fitfun requires at least four arguments"));
    }
    let fcn = args[0].clone();
    let dims = args[1].dims().to_vec();
    let mut p = to_f64_vec(&args[1]);
    let m = p.len();
    let y = to_f64_vec(&args[2]);
    let n = y.len();
    let w = to_f64_vec(&args[3]);
    if w.len() != n {
        return Err(err(
            "fitfun: weight vector must be the same size as the output vector 'y'",
        ));
    }
    let tol = args
        .get(4)
        .and_then(Array::as_f64)
        .unwrap_or(LM_STOP_THRESH);
    let varargin: Vec<Array> = args.get(5..).map(<[Array]>::to_vec).unwrap_or_default();

    // Weighted target and the levmar stop thresholds.
    let target: Vec<f64> = y.iter().zip(&w).map(|(yi, wi)| yi * wi).collect();
    let eps1 = tol; // ||g||_inf
    let eps2 = tol; // ||dp||
    let eps3 = tol * 1e-5; // ||e||^2
    let itmax = 200 * (m + 1);

    // Initial model, residual, cost.
    let mut md = model(i, &fcn, &dims, &p, &w, &varargin)?;
    let mut e: Vec<f64> = target.iter().zip(&md).map(|(t, m)| t - m).collect();
    let mut cost = dot(&e, &e);

    // Initial normal equations: A = J^T J, g = J^T e.
    let build_normal = |jac: &[Vec<f64>], e: &[f64]| -> (Vec<Vec<f64>>, Vec<f64>) {
        let m = jac.len();
        let mut a = vec![vec![0.0f64; m]; m];
        let mut g = vec![0.0f64; m];
        for jj in 0..m {
            for kk in jj..m {
                let s = dot(&jac[jj], &jac[kk]);
                a[jj][kk] = s;
                a[kk][jj] = s;
            }
            g[jj] = dot(&jac[jj], e);
        }
        (a, g)
    };

    let mut jac = jacobian(i, &fcn, &dims, &p, &w, &varargin, &md)?;
    let (mut a, mut g) = build_normal(&jac, &e);
    let mut mu = LM_INIT_MU * (0..m).fold(0.0f64, |acc, d| acc.max(a[d][d]));
    let mut nu = 2.0f64;

    for _ in 0..itmax {
        if inf_norm(&g) <= eps1 || cost <= eps3 {
            break;
        }
        let Some(dp) = solve_damped(&a, &g, mu) else {
            // Singular system: increase damping and retry.
            mu *= nu;
            nu *= 2.0;
            continue;
        };
        let dp_norm = dot(&dp, &dp).sqrt();
        let p_norm = dot(&p, &p).sqrt();
        if dp_norm <= eps2 * (p_norm + eps2) {
            break;
        }
        let p_new: Vec<f64> = p.iter().zip(&dp).map(|(pi, d)| pi + d).collect();
        let md_new = model(i, &fcn, &dims, &p_new, &w, &varargin)?;
        let e_new: Vec<f64> = target.iter().zip(&md_new).map(|(t, m)| t - m).collect();
        let cost_new = dot(&e_new, &e_new);

        // Gain ratio (predicted reduction = dp · (mu*dp + g)).
        let denom: f64 = dp.iter().zip(&g).map(|(d, gi)| d * (mu * d + gi)).sum();
        let rho = if denom > 0.0 {
            (cost - cost_new) / denom
        } else {
            -1.0
        };

        if rho > 0.0 {
            // Accept the step.
            p = p_new;
            md = md_new;
            e = e_new;
            cost = cost_new;
            jac = jacobian(i, &fcn, &dims, &p, &w, &varargin, &md)?;
            let (na, ng) = build_normal(&jac, &e);
            a = na;
            g = ng;
            let factor = 1.0 - (2.0 * rho - 1.0).powi(3);
            mu *= factor.max(1.0 / 3.0);
            nu = 2.0;
        } else {
            mu *= nu;
            nu *= 2.0;
        }
    }

    // Final params and the (unweighted) function output at the optimum.
    let xopt = build_real(DataClass::Double, &dims, p.clone());
    let yopt = eval_fcn(i, &fcn, &xopt, &varargin)?
        .into_iter()
        .next()
        .unwrap_or_else(|| Array::double_matrix(&[0, 0], Vec::new()));
    Ok(vec![xopt, yopt])
}
