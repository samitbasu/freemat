//! Operator dispatch: element-wise binary/unary ops with broadcasting and
//! type promotion, plus a naive matrix multiply.
//!
//! Arithmetic is carried out in `double` (or `C64` when complex), then the
//! result is cast to the class [`fm_core::promote`] picks — matching FreeMat's
//! `double`-dominant lattice with MATLAB integer rules. Relational and logical
//! ops always yield `logical`.
//!
//! **Broadcasting** here is MATLAB's classic rule plus scalar expansion: scalar
//! ⊗ array and array ⊗ array of equal shape are supported, as is the common
//! singleton-dimension broadcast (a dimension of length 1 stretches to match).
//! Heavy matrix `*` / `\` / `/` are Stage 5 (faer); `*`/`/`/`\` on non-scalars
//! get a straightforward implementation (`*` naive matmul) or a clear deferral.

use fm_core::{Array, C64, DataClass, ScalarValue, promote};
use fm_parser::ast::{BinaryOp, UnaryOp};

use crate::error::{Flow, InterpError, Signal};
use crate::value::{build_complex, build_real, to_c64_vec, to_f64_vec};

/// Apply a binary operator to two evaluated values.
pub fn binary(op: BinaryOp, lhs: &Array, rhs: &Array) -> Flow<Array> {
    use BinaryOp::{
        Add, And, ElLDiv, ElMul, ElPow, ElRDiv, Eq, Ge, Gt, LDiv, Le, Lt, Mul, Ne, Or, Pow, RDiv,
        ShortAnd, ShortOr, Sub,
    };

    match op {
        Add => elementwise_arith(lhs, rhs, |a, b| a + b, |a, b| a + b),
        Sub => elementwise_arith(lhs, rhs, |a, b| a - b, |a, b| a - b),
        ElMul => elementwise_arith(lhs, rhs, |a, b| a * b, |a, b| a * b),
        ElRDiv => elementwise_arith(lhs, rhs, |a, b| a / b, |a, b| a / b),
        ElLDiv => elementwise_arith(lhs, rhs, |a, b| b / a, |a, b| b / a),
        ElPow => elementwise_pow(lhs, rhs),
        Mul => mul(lhs, rhs),
        RDiv => div(lhs, rhs, false),
        LDiv => div(lhs, rhs, true),
        Pow => pow(lhs, rhs),
        Lt => relational(lhs, rhs, |a, b| a < b),
        Gt => relational(lhs, rhs, |a, b| a > b),
        Le => relational(lhs, rhs, |a, b| a <= b),
        Ge => relational(lhs, rhs, |a, b| a >= b),
        Eq => equality(lhs, rhs, true),
        Ne => equality(lhs, rhs, false),
        And => logical(lhs, rhs, |a, b| a && b),
        Or => logical(lhs, rhs, |a, b| a || b),
        // The short-circuit forms never reach here (the evaluator handles them).
        ShortAnd | ShortOr => Err(Signal::Error(InterpError::msg(
            "short-circuit operator reached operator dispatch",
        ))),
    }
}

/// Apply a unary prefix operator.
pub fn unary(op: UnaryOp, v: &Array) -> Flow<Array> {
    match op {
        UnaryOp::Plus => Ok(v.clone()),
        UnaryOp::Minus => {
            if v.is_complex() {
                let dims = v.dims();
                let data = to_c64_vec(v).into_iter().map(|c| -c).collect();
                Ok(build_complex(&dims, data))
            } else {
                let class = arith_class(v.class());
                let dims = v.dims();
                let data = to_f64_vec(v).into_iter().map(|x| -x).collect();
                Ok(build_real(class, &dims, data))
            }
        }
        UnaryOp::Not => {
            let dims = v.dims();
            let data: Vec<bool> = to_f64_vec(v).into_iter().map(|x| x == 0.0).collect();
            Ok(make_logical(&dims, data))
        }
    }
}

/// The class arithmetic carries out in / stores its result as, for a unary op.
fn arith_class(c: DataClass) -> DataClass {
    match c {
        DataClass::Bool | DataClass::Char => DataClass::Double,
        other => other,
    }
}

/// Compute the broadcast output dimensions of two shapes, MATLAB-style.
///
/// Equal lengths broadcast per-dimension where one side is 1; a scalar (`[1,1]`
/// or numel 1) broadcasts against anything.
fn broadcast_dims(a: &[usize], b: &[usize]) -> Flow<Vec<usize>> {
    let na: usize = a.iter().product();
    let nb: usize = b.iter().product();
    if na == 1 {
        return Ok(b.to_vec());
    }
    if nb == 1 {
        return Ok(a.to_vec());
    }
    let rank = a.len().max(b.len());
    let mut out = Vec::with_capacity(rank);
    for i in 0..rank {
        let da = a.get(i).copied().unwrap_or(1);
        let db = b.get(i).copied().unwrap_or(1);
        if da == db {
            out.push(da);
        } else if da == 1 {
            out.push(db);
        } else if db == 1 {
            out.push(da);
        } else {
            return Err(Signal::Error(InterpError::msg(format!(
                "matrix dimensions must agree ({} vs {})",
                shape_str(a),
                shape_str(b)
            ))));
        }
    }
    Ok(out)
}

fn shape_str(d: &[usize]) -> String {
    d.iter().map(usize::to_string).collect::<Vec<_>>().join("x")
}

/// Column-major strides for a shape (element offset per dimension).
fn strides(dims: &[usize]) -> Vec<usize> {
    let mut s = vec![1usize; dims.len()];
    for i in 1..dims.len() {
        s[i] = s[i - 1] * dims[i - 1];
    }
    s
}

/// Iterate the linear (column-major) indices into operand `src` (shape
/// `src_dims`) needed to fill output position `lin` of shape `out_dims`,
/// applying singleton broadcasting.
fn broadcast_src_index(lin: usize, out_dims: &[usize], src_dims: &[usize]) -> usize {
    if src_dims.iter().product::<usize>() == 1 {
        return 0;
    }
    let out_str = strides(out_dims);
    let src_str = strides(src_dims);
    let mut rem = lin;
    let mut src_lin = 0usize;
    for i in (0..out_dims.len()).rev() {
        let coord = rem / out_str[i];
        rem %= out_str[i];
        let sdim = src_dims.get(i).copied().unwrap_or(1);
        let scoord = if sdim == 1 { 0 } else { coord };
        src_lin += scoord * src_str.get(i).copied().unwrap_or(0);
    }
    src_lin
}

/// Generic element-wise arithmetic (real & complex lanes) with broadcasting.
fn elementwise_arith(
    lhs: &Array,
    rhs: &Array,
    rf: impl Fn(f64, f64) -> f64,
    cf: impl Fn(C64, C64) -> C64,
) -> Flow<Array> {
    check_numeric(lhs, rhs)?;
    let result_class = promote(lhs.class(), rhs.class())
        .map_err(|e| Signal::Error(InterpError::msg(e.to_string())))?;
    let ld = lhs.dims();
    let rd = rhs.dims();
    let out = broadcast_dims(&ld, &rd)?;
    let n: usize = out.iter().product();

    if lhs.is_complex() || rhs.is_complex() {
        let la = to_c64_vec(lhs);
        let ra = to_c64_vec(rhs);
        let mut data = Vec::with_capacity(n);
        for i in 0..n {
            let a = la[broadcast_src_index(i, &out, &ld)];
            let b = ra[broadcast_src_index(i, &out, &rd)];
            data.push(cf(a, b));
        }
        return Ok(build_complex(&out, data));
    }

    let la = to_f64_vec(lhs);
    let ra = to_f64_vec(rhs);
    let mut data = Vec::with_capacity(n);
    for i in 0..n {
        let a = la[broadcast_src_index(i, &out, &ld)];
        let b = ra[broadcast_src_index(i, &out, &rd)];
        data.push(rf(a, b));
    }
    Ok(build_real(result_class, &out, data))
}

/// Element-wise power `.^` (promotes to complex on negative-base/frac-exp).
fn elementwise_pow(lhs: &Array, rhs: &Array) -> Flow<Array> {
    check_numeric(lhs, rhs)?;
    let ld = lhs.dims();
    let rd = rhs.dims();
    let out = broadcast_dims(&ld, &rd)?;
    let n: usize = out.iter().product();
    let la = to_f64_vec(lhs);
    let ra = to_f64_vec(rhs);
    let needs_complex =
        lhs.is_complex() || rhs.is_complex() || powers_need_complex(&la, &ra, &out, &ld, &rd);
    if needs_complex {
        let lc = to_c64_vec(lhs);
        let rc = to_c64_vec(rhs);
        let mut data = Vec::with_capacity(n);
        for i in 0..n {
            let a = lc[broadcast_src_index(i, &out, &ld)];
            let b = rc[broadcast_src_index(i, &out, &rd)];
            data.push(a.powc(b));
        }
        return Ok(build_complex(&out, data));
    }
    let result_class = promote(lhs.class(), rhs.class())
        .map_err(|e| Signal::Error(InterpError::msg(e.to_string())))?;
    let mut data = Vec::with_capacity(n);
    for i in 0..n {
        let a = la[broadcast_src_index(i, &out, &ld)];
        let b = ra[broadcast_src_index(i, &out, &rd)];
        data.push(a.powf(b));
    }
    Ok(build_real(result_class, &out, data))
}

fn powers_need_complex(la: &[f64], ra: &[f64], out: &[usize], ld: &[usize], rd: &[usize]) -> bool {
    let n: usize = out.iter().product();
    (0..n).any(|i| {
        let a = la[broadcast_src_index(i, out, ld)];
        let b = ra[broadcast_src_index(i, out, rd)];
        a < 0.0 && b.fract() != 0.0
    })
}

/// Relational op → logical, with broadcasting (compares real parts / magnitude).
fn relational(lhs: &Array, rhs: &Array, f: impl Fn(f64, f64) -> bool) -> Flow<Array> {
    check_numeric(lhs, rhs)?;
    let ld = lhs.dims();
    let rd = rhs.dims();
    let out = broadcast_dims(&ld, &rd)?;
    let n: usize = out.iter().product();
    let la = to_f64_vec(lhs);
    let ra = to_f64_vec(rhs);
    let mut data = Vec::with_capacity(n);
    for i in 0..n {
        let a = la[broadcast_src_index(i, &out, &ld)];
        let b = ra[broadcast_src_index(i, &out, &rd)];
        data.push(f(a, b));
    }
    Ok(make_logical(&out, data))
}

/// `==` / `~=` → logical, with broadcasting (complex compares both parts).
fn equality(lhs: &Array, rhs: &Array, want_equal: bool) -> Flow<Array> {
    let ld = lhs.dims();
    let rd = rhs.dims();
    let out = broadcast_dims(&ld, &rd)?;
    let n: usize = out.iter().product();
    let mut data = Vec::with_capacity(n);
    if lhs.is_complex() || rhs.is_complex() {
        let la = to_c64_vec(lhs);
        let ra = to_c64_vec(rhs);
        for i in 0..n {
            let a = la[broadcast_src_index(i, &out, &ld)];
            let b = ra[broadcast_src_index(i, &out, &rd)];
            data.push((a == b) == want_equal);
        }
    } else {
        let la = to_f64_vec(lhs);
        let ra = to_f64_vec(rhs);
        for i in 0..n {
            let a = la[broadcast_src_index(i, &out, &ld)];
            let b = ra[broadcast_src_index(i, &out, &rd)];
            data.push((a == b) == want_equal);
        }
    }
    Ok(make_logical(&out, data))
}

/// `&` / `|` → logical (non-zero is true), with broadcasting.
fn logical(lhs: &Array, rhs: &Array, f: impl Fn(bool, bool) -> bool) -> Flow<Array> {
    let ld = lhs.dims();
    let rd = rhs.dims();
    let out = broadcast_dims(&ld, &rd)?;
    let n: usize = out.iter().product();
    let la = to_f64_vec(lhs);
    let ra = to_f64_vec(rhs);
    let mut data = Vec::with_capacity(n);
    for i in 0..n {
        let a = la[broadcast_src_index(i, &out, &ld)] != 0.0;
        let b = ra[broadcast_src_index(i, &out, &rd)] != 0.0;
        data.push(f(a, b));
    }
    Ok(make_logical(&out, data))
}

/// Matrix / scalar `*`. Scalar operand → element-wise; otherwise naive matmul.
fn mul(lhs: &Array, rhs: &Array) -> Flow<Array> {
    if lhs.is_scalar() || rhs.is_scalar() {
        return elementwise_arith(lhs, rhs, |a, b| a * b, |a, b| a * b);
    }
    check_numeric(lhs, rhs)?;
    let ld = lhs.dims();
    let rd = rhs.dims();
    if ld.len() != 2 || rd.len() != 2 {
        return Err(Signal::Error(InterpError::msg(
            "matrix multiply requires 2-D operands",
        )));
    }
    let (m, k) = (ld[0], ld[1]);
    let (k2, p) = (rd[0], rd[1]);
    if k != k2 {
        return Err(Signal::Error(InterpError::msg(format!(
            "inner matrix dimensions must agree ({}x{} * {}x{})",
            m, k, k2, p
        ))));
    }
    // Naive O(m*k*p) column-major matmul (faer-backed fast path is Stage 5).
    if lhs.is_complex() || rhs.is_complex() {
        let a = to_c64_vec(lhs);
        let b = to_c64_vec(rhs);
        let mut out = vec![C64::new(0.0, 0.0); m * p];
        for j in 0..p {
            for l in 0..k {
                let bjl = b[l + j * k2];
                for i in 0..m {
                    out[i + j * m] += a[i + l * m] * bjl;
                }
            }
        }
        return Ok(build_complex(&[m, p], out));
    }
    let a = to_f64_vec(lhs);
    let b = to_f64_vec(rhs);
    let mut out = vec![0.0f64; m * p];
    for j in 0..p {
        for l in 0..k {
            let bjl = b[l + j * k2];
            for i in 0..m {
                out[i + j * m] += a[i + l * m] * bjl;
            }
        }
    }
    Ok(build_real(DataClass::Double, &[m, p], out))
}

/// `/` (right) and `\` (left) division. Scalar cases are element-wise; the
/// general matrix solve is deferred to Stage 5 (faer).
fn div(lhs: &Array, rhs: &Array, left: bool) -> Flow<Array> {
    let (a, b) = if left { (rhs, lhs) } else { (lhs, rhs) };
    // `A / s` or `s \ A`: divide by a scalar element-wise.
    if (left && lhs.is_scalar()) || (!left && rhs.is_scalar()) {
        return elementwise_arith(a, b, |x, y| x / y, |x, y| x / y);
    }
    if lhs.is_scalar() && rhs.is_scalar() {
        return elementwise_arith(a, b, |x, y| x / y, |x, y| x / y);
    }
    Err(Signal::Error(InterpError::msg(
        "matrix solve (\\ and / with matrix divisor) is not yet implemented (Stage 5)",
    )))
}

/// `^` matrix power. Scalar^scalar uses element-wise pow; matrix power deferred.
fn pow(lhs: &Array, rhs: &Array) -> Flow<Array> {
    if lhs.is_scalar() && rhs.is_scalar() {
        return elementwise_pow(lhs, rhs);
    }
    Err(Signal::Error(InterpError::msg(
        "matrix power (^ on a non-scalar) is not yet implemented (Stage 5)",
    )))
}

fn check_numeric(lhs: &Array, rhs: &Array) -> Flow<()> {
    if lhs.class().is_reference() || rhs.class().is_reference() {
        return Err(Signal::Error(InterpError::msg(format!(
            "operator not defined for {} and {}",
            lhs.class_name(),
            rhs.class_name()
        ))));
    }
    Ok(())
}

/// Build a logical array, collapsing a single element to an inline scalar.
fn make_logical(dims: &[usize], data: Vec<bool>) -> Array {
    if data.len() == 1 && dims.iter().product::<usize>() == 1 {
        return Array::Scalar(ScalarValue::Bool(data[0]));
    }
    Array::bool_matrix(dims, data)
}
