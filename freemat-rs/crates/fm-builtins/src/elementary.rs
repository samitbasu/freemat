//! Elementary math builtins: `sqrt`, `exp`, `log`, `log2`, `log10`, `power`,
//! `sign`, `abs`, `floor`, `ceil`, `round`, `fix`, `mod`, `rem`, `conj`,
//! `real`, `imag`, `angle`, `hypot`, `gcd`, `lcm`, `factorial`,
//! plus `isnan`/`isinf`/`isfinite`.

use fm_core::{Array, C64, DataClass};
use fm_interp::error::Flow;
use fm_interp::value::{build_complex, build_real, to_c64_vec, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};

use crate::util::{err, map_complexifying, map_double, map_real_or_complex, need};

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("sqrt", b_sqrt);
    table.add_builtin("exp", b_exp);
    table.add_builtin("log", b_log);
    table.add_builtin("log2", b_log2);
    table.add_builtin("log10", b_log10);
    table.add_builtin("log1p", |_i, a, _n| simple(a, "log1p", f64::ln_1p));
    table.add_builtin("expm1", |_i, a, _n| simple(a, "expm1", f64::exp_m1));
    table.add_builtin("power", b_power);
    table.add_builtin("sign", b_sign);
    table.add_builtin("fix", |_i, a, _n| simple(a, "fix", f64::trunc));
    table.add_builtin("conj", b_conj);
    table.add_builtin("real", b_real);
    table.add_builtin("imag", b_imag);
    table.add_builtin("angle", b_angle);
    table.add_builtin("arg", b_angle);
    table.add_builtin("hypot", b_hypot);
    table.add_builtin("gcd", b_gcd);
    table.add_builtin("lcm", b_lcm);
    table.add_builtin("factorial", b_factorial);
    table.add_builtin("isnan", |_i, a, _n| predicate(a, "isnan", f64::is_nan));
    table.add_builtin("IsNaN", |_i, a, _n| predicate(a, "IsNaN", f64::is_nan));
    table.add_builtin("isinf", |_i, a, _n| predicate(a, "isinf", f64::is_infinite));
    table.add_builtin("IsInf", |_i, a, _n| predicate(a, "IsInf", f64::is_infinite));
    table.add_builtin("isfinite", |_i, a, _n| {
        predicate(a, "isfinite", f64::is_finite)
    });
    table.add_builtin("deg2rad", |_i, a, _n| simple(a, "deg2rad", f64::to_radians));
    table.add_builtin("rad2deg", |_i, a, _n| simple(a, "rad2deg", f64::to_degrees));
}

/// A trivial real-only element-wise builtin.
fn simple(args: &[Array], name: &str, f: impl Fn(f64) -> f64) -> Flow<Vec<Array>> {
    need(args, 1, name)?;
    Ok(vec![map_double(&args[0], f)])
}

/// An element-wise predicate yielding a `logical` array.
fn predicate(args: &[Array], name: &str, f: impl Fn(f64) -> bool) -> Flow<Vec<Array>> {
    need(args, 1, name)?;
    let dims = args[0].dims();
    let data: Vec<f64> = to_f64_vec(&args[0])
        .into_iter()
        .map(|v| if f(v) { 1.0 } else { 0.0 })
        .collect();
    Ok(vec![build_real(DataClass::Bool, &dims, data)])
}

fn b_sqrt(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "sqrt")?;
    let a = &args[0];
    // sqrt of a negative real returns complex.
    if !a.is_complex() && to_f64_vec(a).iter().all(|&x| x >= 0.0 || x.is_nan()) {
        return Ok(vec![map_double(a, f64::sqrt)]);
    }
    Ok(vec![map_complexifying(a, C64::sqrt)])
}

fn b_exp(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "exp")?;
    Ok(vec![map_real_or_complex(&args[0], f64::exp, C64::exp)])
}

fn b_log(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "log")?;
    let a = &args[0];
    if !a.is_complex() && to_f64_vec(a).iter().all(|&x| x >= 0.0 || x.is_nan()) {
        return Ok(vec![map_double(a, f64::ln)]);
    }
    Ok(vec![map_complexifying(a, C64::ln)])
}

fn b_log2(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "log2")?;
    let a = &args[0];
    if !a.is_complex() && to_f64_vec(a).iter().all(|&x| x >= 0.0 || x.is_nan()) {
        return Ok(vec![map_double(a, f64::log2)]);
    }
    Ok(vec![map_complexifying(a, |z| {
        z.ln() / C64::new(2.0_f64.ln(), 0.0)
    })])
}

fn b_log10(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "log10")?;
    let a = &args[0];
    if !a.is_complex() && to_f64_vec(a).iter().all(|&x| x >= 0.0 || x.is_nan()) {
        return Ok(vec![map_double(a, f64::log10)]);
    }
    Ok(vec![map_complexifying(a, |z| {
        z.ln() / C64::new(10.0_f64.ln(), 0.0)
    })])
}

/// `power(x, y)` — element-wise `x .^ y` (scalar-or-equal-shape broadcast).
fn b_power(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "power")?;
    let xc = args[0].is_complex();
    let yc = args[1].is_complex();
    let x = to_f64_vec(&args[0]);
    let y = to_f64_vec(&args[1]);
    let (n, dims) = if x.len() >= y.len() {
        (x.len(), args[0].dims())
    } else {
        (y.len(), args[1].dims())
    };
    let getx = |i: usize| if x.len() == 1 { x[0] } else { x[i] };
    let gety = |i: usize| if y.len() == 1 { y[0] } else { y[i] };
    // Promote to complex if either input is complex or any negative base meets a
    // fractional exponent.
    let needs_complex = xc || yc || (0..n).any(|i| getx(i) < 0.0 && gety(i).fract() != 0.0);
    if needs_complex {
        let xz = to_c64_vec(&args[0]);
        let yz = to_c64_vec(&args[1]);
        let getxz = |i: usize| if xz.len() == 1 { xz[0] } else { xz[i] };
        let getyz = |i: usize| if yz.len() == 1 { yz[0] } else { yz[i] };
        let data = (0..n).map(|i| getxz(i).powc(getyz(i))).collect();
        return Ok(vec![build_complex(&dims, data)]);
    }
    let data = (0..n).map(|i| getx(i).powf(gety(i))).collect();
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

fn b_sign(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "sign")?;
    let a = &args[0];
    if a.is_complex() {
        // sign(z) = z / |z| for nonzero z, else 0.
        let dims = a.dims();
        let data = to_c64_vec(a)
            .into_iter()
            .map(|z| {
                let m = z.norm();
                if m == 0.0 {
                    C64::new(0.0, 0.0)
                } else {
                    z / C64::new(m, 0.0)
                }
            })
            .collect();
        return Ok(vec![build_complex(&dims, data)]);
    }
    Ok(vec![map_double(a, f64::signum_zero)])
}

/// Extension trait for a MATLAB-style `sign` (signum that maps 0 -> 0, NaN -> NaN).
trait SignumZero {
    fn signum_zero(self) -> f64;
}
impl SignumZero for f64 {
    fn signum_zero(self) -> f64 {
        if self.is_nan() {
            f64::NAN
        } else if self > 0.0 {
            1.0
        } else if self < 0.0 {
            -1.0
        } else {
            0.0
        }
    }
}

fn b_conj(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "conj")?;
    let a = &args[0];
    // Sparse stays sparse (negate the imaginary part).
    if let Some(s) = a.as_sparse() {
        return Ok(vec![Array::sparse(s.conj())]);
    }
    if !a.is_complex() {
        return Ok(vec![a.clone()]);
    }
    let dims = a.dims();
    let data = to_c64_vec(a).into_iter().map(|z| z.conj()).collect();
    Ok(vec![build_complex(&dims, data)])
}

fn b_real(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "real")?;
    let a = &args[0];
    if let Some(s) = a.as_sparse() {
        return Ok(vec![Array::sparse(s.real_part())]);
    }
    let dims = a.dims();
    let data = to_c64_vec(a).into_iter().map(|z| z.re).collect();
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

fn b_imag(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "imag")?;
    let a = &args[0];
    if let Some(s) = a.as_sparse() {
        return Ok(vec![Array::sparse(s.imag_part())]);
    }
    let dims = a.dims();
    let data = to_c64_vec(a).into_iter().map(|z| z.im).collect();
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

fn b_angle(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "angle")?;
    let a = &args[0];
    let dims = a.dims();
    let data = to_c64_vec(a).into_iter().map(|z| z.arg()).collect();
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

fn b_hypot(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "hypot")?;
    let x = to_f64_vec(&args[0]);
    let y = to_f64_vec(&args[1]);
    if x.len() != y.len() && x.len() != 1 && y.len() != 1 {
        return err("hypot: argument sizes must match");
    }
    let n = x.len().max(y.len());
    let dims = if x.len() >= y.len() {
        args[0].dims()
    } else {
        args[1].dims()
    };
    let data: Vec<f64> = (0..n)
        .map(|i| {
            let a = if x.len() == 1 { x[0] } else { x[i] };
            let b = if y.len() == 1 { y[0] } else { y[i] };
            a.hypot(b)
        })
        .collect();
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

fn b_gcd(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "gcd")?;
    let a = scalar_i64(&args[0], "gcd")?;
    let b = scalar_i64(&args[1], "gcd")?;
    Ok(vec![Array::double(gcd(a.abs(), b.abs()) as f64)])
}

fn b_lcm(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "lcm")?;
    let a = scalar_i64(&args[0], "lcm")?.abs();
    let b = scalar_i64(&args[1], "lcm")?.abs();
    let g = gcd(a, b);
    let v = if g == 0 { 0 } else { a / g * b };
    Ok(vec![Array::double(v as f64)])
}

fn b_factorial(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "factorial")?;
    let dims = args[0].dims();
    let data: Vec<f64> = to_f64_vec(&args[0])
        .into_iter()
        .map(|n| {
            let n = n.round().max(0.0) as u64;
            (1..=n).fold(1.0_f64, |acc, k| acc * k as f64)
        })
        .collect();
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

fn scalar_i64(a: &Array, name: &str) -> Flow<i64> {
    a.as_f64().map(|v| v.round() as i64).ok_or_else(|| {
        fm_interp::Signal::Error(fm_interp::InterpError::msg(format!(
            "{name}: expected a scalar integer"
        )))
    })
}

fn gcd(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}
