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
//! Heavy matrix `*` / `\` / `/` / `^` route through [`fm_linalg`] (faer): matrix
//! multiply, LU / least-squares solves, and integer matrix powers. Scalar and
//! element-wise forms stay in this module.

use fm_core::{Array, C64, DataClass, ScalarValue, promote};
use fm_parser::ast::{BinaryOp, UnaryOp};

use crate::error::{Flow, InterpError, Signal};
use crate::value::{build_complex, build_real, cast_scalar, to_c64_vec, to_f64_vec};

/// If both operands are inline scalars, read their `(f64, c64, is_complex)`
/// forms without touching the heap. Returns `None` if either is not an inline
/// scalar (so the caller takes the general array path).
fn scalar_pair(lhs: &Array, rhs: &Array) -> Option<(ScalarValue, ScalarValue)> {
    match (lhs, rhs) {
        (Array::Scalar(a), Array::Scalar(b)) => Some((*a, *b)),
        _ => None,
    }
}

/// Complex value of a scalar (real scalars get a zero imaginary part).
fn scalar_c64(s: ScalarValue) -> C64 {
    match s {
        ScalarValue::Complex64(c) => c,
        ScalarValue::Complex32(c) => C64::new(f64::from(c.re), f64::from(c.im)),
        other => C64::new(other.as_f64(), 0.0),
    }
}

/// Apply a binary operator to two evaluated values.
pub fn binary(op: BinaryOp, lhs: &Array, rhs: &Array) -> Flow<Array> {
    use BinaryOp::{
        Add, And, ElLDiv, ElMul, ElPow, ElRDiv, Eq, Ge, Gt, LDiv, Le, Lt, Mul, Ne, Or, Pow, RDiv,
        ShortAnd, ShortOr, Sub,
    };

    // Sparse operands: keep genuinely sparse paths for matrix `*`, scalar·A,
    // and A±B so large sparse matrices never densify (which would blow up
    // memory). All other ops densify on demand (correct, and what `full()`-based
    // comparisons expect).
    if lhs.is_sparse() || rhs.is_sparse() {
        if let Some(out) = sparse_binary(op, lhs, rhs)? {
            return Ok(out);
        }
        let ld = crate::value::densify(lhs);
        let rd = crate::value::densify(rhs);
        return binary(op, &ld, &rd);
    }

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

/// Handle the binary ops that have a genuinely sparse implementation (so the
/// result stays sparse and large matrices never densify). Returns `Ok(None)` if
/// the op should fall through to the densify path.
fn sparse_binary(op: BinaryOp, lhs: &Array, rhs: &Array) -> Flow<Option<Array>> {
    use BinaryOp::{Add, ElMul, Mul, Sub};
    let l_sp = lhs.as_sparse();
    let r_sp = rhs.as_sparse();
    let l_scalar = lhs.as_scalar();
    let r_scalar = rhs.as_scalar();

    let scale = |s: &fm_core::SparseMatrix, sv: ScalarValue| -> Array {
        if sv.is_complex() {
            let c = scalar_c64(sv);
            Array::sparse(s.scale_complex(c.re, c.im))
        } else {
            Array::sparse(s.scale_real(sv.as_f64()))
        }
    };

    match op {
        // sparse · sparse (matrix multiply), keeping the result sparse — but a
        // 1×1 sparse operand acts as a scalar scale.
        Mul => match (l_sp, r_sp) {
            (Some(a), Some(b)) => {
                if a.rows() == 1 && a.cols() == 1 {
                    let v = a.get(0, 0);
                    Ok(Some(scale(b, scalar_from_pair(v))))
                } else if b.rows() == 1 && b.cols() == 1 {
                    let v = b.get(0, 0);
                    Ok(Some(scale(a, scalar_from_pair(v))))
                } else {
                    a.matmul(b)
                        .map(|m| Some(Array::sparse(m)))
                        .map_err(|e| Signal::Error(InterpError::msg(e)))
                }
            }
            // scalar · sparse → scaled sparse.
            (None, Some(b)) if l_scalar.is_some() => Ok(Some(scale(b, l_scalar.unwrap()))),
            (Some(a), None) if r_scalar.is_some() => Ok(Some(scale(a, r_scalar.unwrap()))),
            _ => Ok(None),
        },
        // scalar .* sparse → scaled sparse.
        ElMul => match (l_sp, r_sp) {
            (None, Some(b)) if l_scalar.is_some() => Ok(Some(scale(b, l_scalar.unwrap()))),
            (Some(a), None) if r_scalar.is_some() => Ok(Some(scale(a, r_scalar.unwrap()))),
            _ => Ok(None),
        },
        // sparse ± sparse → sparse.
        Add | Sub => {
            if let (Some(a), Some(b)) = (l_sp, r_sp) {
                a.add_sub(b, matches!(op, Sub))
                    .map(|m| Some(Array::sparse(m)))
                    .map_err(|e| Signal::Error(InterpError::msg(e)))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

/// A [`ScalarValue`] from a sparse `(re, im)` entry.
fn scalar_from_pair((re, im): (f64, f64)) -> ScalarValue {
    if im != 0.0 {
        ScalarValue::Complex64(C64::new(re, im))
    } else {
        ScalarValue::Double(re)
    }
}

/// Apply a unary prefix operator.
pub fn unary(op: UnaryOp, v: &Array) -> Flow<Array> {
    match op {
        UnaryOp::Plus => Ok(v.clone()),
        UnaryOp::Minus => {
            if let Array::Scalar(s) = v {
                // Scalar negate inline (no `dims()`/`to_*_vec` allocations).
                if s.is_complex() {
                    return Ok(build_complex(&[1, 1], vec![-scalar_c64(*s)]));
                }
                return Ok(Array::Scalar(cast_scalar(
                    arith_class(v.class()),
                    -s.as_f64(),
                )));
            }
            if v.is_complex() {
                let data = to_c64_vec(v).into_iter().map(|c| -c).collect();
                Ok(build_complex(v.shape(), data))
            } else {
                let class = arith_class(v.class());
                let data = to_f64_vec(v).into_iter().map(|x| -x).collect();
                Ok(build_real(class, v.shape(), data))
            }
        }
        UnaryOp::Not => {
            if let Array::Scalar(s) = v {
                return Ok(Array::bool(s.as_f64() == 0.0));
            }
            let data: Vec<bool> = to_f64_vec(v).into_iter().map(|x| x == 0.0).collect();
            Ok(make_logical(v.shape(), data))
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

    // Scalar ⊕ scalar fast path: compute inline (no `dims()`/`to_f64_vec`/
    // result-`Vec` allocations) and return an inline `Array::Scalar`. This
    // mirrors the general path exactly: a complex operand routes through `cf`
    // and `build_complex` (which narrows back to real if the imaginary part
    // vanishes), otherwise `rf` + `cast_scalar` casts to `result_class`.
    if let Some((a, b)) = scalar_pair(lhs, rhs) {
        if a.is_complex() || b.is_complex() {
            let r = cf(scalar_c64(a), scalar_c64(b));
            return Ok(build_complex(&[1, 1], vec![r]));
        }
        return Ok(Array::Scalar(cast_scalar(
            result_class,
            rf(a.as_f64(), b.as_f64()),
        )));
    }

    let ld = lhs.shape();
    let rd = rhs.shape();
    let out = broadcast_dims(ld, rd)?;
    let n: usize = out.iter().product();

    if lhs.is_complex() || rhs.is_complex() {
        let la = to_c64_vec(lhs);
        let ra = to_c64_vec(rhs);
        let mut data = Vec::with_capacity(n);
        for i in 0..n {
            let a = la[broadcast_src_index(i, &out, ld)];
            let b = ra[broadcast_src_index(i, &out, rd)];
            data.push(cf(a, b));
        }
        return Ok(build_complex(&out, data));
    }

    let la = to_f64_vec(lhs);
    let ra = to_f64_vec(rhs);
    let mut data = Vec::with_capacity(n);
    for i in 0..n {
        let a = la[broadcast_src_index(i, &out, ld)];
        let b = ra[broadcast_src_index(i, &out, rd)];
        data.push(rf(a, b));
    }
    Ok(build_real(result_class, &out, data))
}

/// Element-wise power `.^` (promotes to complex on negative-base/frac-exp).
fn elementwise_pow(lhs: &Array, rhs: &Array) -> Flow<Array> {
    check_numeric(lhs, rhs)?;
    let ld = lhs.shape();
    let rd = rhs.shape();
    let out = broadcast_dims(ld, rd)?;
    let n: usize = out.iter().product();
    let la = to_f64_vec(lhs);
    let ra = to_f64_vec(rhs);
    let needs_complex =
        lhs.is_complex() || rhs.is_complex() || powers_need_complex(&la, &ra, &out, ld, rd);
    if needs_complex {
        let lc = to_c64_vec(lhs);
        let rc = to_c64_vec(rhs);
        let mut data = Vec::with_capacity(n);
        for i in 0..n {
            let a = lc[broadcast_src_index(i, &out, ld)];
            let b = rc[broadcast_src_index(i, &out, rd)];
            data.push(a.powc(b));
        }
        return Ok(build_complex(&out, data));
    }
    let result_class = promote(lhs.class(), rhs.class())
        .map_err(|e| Signal::Error(InterpError::msg(e.to_string())))?;
    let mut data = Vec::with_capacity(n);
    for i in 0..n {
        let a = la[broadcast_src_index(i, &out, ld)];
        let b = ra[broadcast_src_index(i, &out, rd)];
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
    // Scalar ⊕ scalar: compare real parts inline, return a logical scalar.
    if let Some((a, b)) = scalar_pair(lhs, rhs) {
        return Ok(Array::bool(f(a.as_f64(), b.as_f64())));
    }
    let ld = lhs.shape();
    let rd = rhs.shape();
    let out = broadcast_dims(ld, rd)?;
    let n: usize = out.iter().product();
    let la = to_f64_vec(lhs);
    let ra = to_f64_vec(rhs);
    let mut data = Vec::with_capacity(n);
    for i in 0..n {
        let a = la[broadcast_src_index(i, &out, ld)];
        let b = ra[broadcast_src_index(i, &out, rd)];
        data.push(f(a, b));
    }
    Ok(make_logical(&out, data))
}

/// `==` / `~=` → logical, with broadcasting (complex compares both parts).
fn equality(lhs: &Array, rhs: &Array, want_equal: bool) -> Flow<Array> {
    // Scalar ⊕ scalar: compare inline (complex compares both parts), return a
    // logical scalar.
    if let Some((a, b)) = scalar_pair(lhs, rhs) {
        let eq = if a.is_complex() || b.is_complex() {
            scalar_c64(a) == scalar_c64(b)
        } else {
            a.as_f64() == b.as_f64()
        };
        return Ok(Array::bool(eq == want_equal));
    }
    let ld = lhs.shape();
    let rd = rhs.shape();
    let out = broadcast_dims(ld, rd)?;
    let n: usize = out.iter().product();
    let mut data = Vec::with_capacity(n);
    if lhs.is_complex() || rhs.is_complex() {
        let la = to_c64_vec(lhs);
        let ra = to_c64_vec(rhs);
        for i in 0..n {
            let a = la[broadcast_src_index(i, &out, ld)];
            let b = ra[broadcast_src_index(i, &out, rd)];
            data.push((a == b) == want_equal);
        }
    } else {
        let la = to_f64_vec(lhs);
        let ra = to_f64_vec(rhs);
        for i in 0..n {
            let a = la[broadcast_src_index(i, &out, ld)];
            let b = ra[broadcast_src_index(i, &out, rd)];
            data.push((a == b) == want_equal);
        }
    }
    Ok(make_logical(&out, data))
}

/// `&` / `|` → logical (non-zero is true), with broadcasting.
fn logical(lhs: &Array, rhs: &Array, f: impl Fn(bool, bool) -> bool) -> Flow<Array> {
    // Scalar ⊕ scalar: combine inline (non-zero is true), return a logical scalar.
    if let Some((a, b)) = scalar_pair(lhs, rhs) {
        return Ok(Array::bool(f(a.as_f64() != 0.0, b.as_f64() != 0.0)));
    }
    let ld = lhs.shape();
    let rd = rhs.shape();
    let out = broadcast_dims(ld, rd)?;
    let n: usize = out.iter().product();
    let la = to_f64_vec(lhs);
    let ra = to_f64_vec(rhs);
    let mut data = Vec::with_capacity(n);
    for i in 0..n {
        let a = la[broadcast_src_index(i, &out, ld)] != 0.0;
        let b = ra[broadcast_src_index(i, &out, rd)] != 0.0;
        data.push(f(a, b));
    }
    Ok(make_logical(&out, data))
}

/// Wrap a [`fm_linalg::LinalgError`] as an interpreter error.
fn linalg_err(e: fm_linalg::LinalgError) -> Signal {
    Signal::Error(InterpError::msg(e.message))
}

/// Matrix / scalar `*`. A scalar operand → element-wise; otherwise faer matmul.
fn mul(lhs: &Array, rhs: &Array) -> Flow<Array> {
    if lhs.is_scalar() || rhs.is_scalar() {
        return elementwise_arith(lhs, rhs, |a, b| a * b, |a, b| a * b);
    }
    check_numeric(lhs, rhs)?;
    if lhs.shape().len() != 2 || rhs.shape().len() != 2 {
        return Err(Signal::Error(InterpError::msg(
            "matrix multiply requires 2-D operands",
        )));
    }
    fm_linalg::mtimes(lhs, rhs).map_err(linalg_err)
}

/// `/` (right) and `\` (left) division. Scalar cases are element-wise; the
/// general matrix solve goes through faer (LU / least-squares).
fn div(lhs: &Array, rhs: &Array, left: bool) -> Flow<Array> {
    let (a, b) = if left { (rhs, lhs) } else { (lhs, rhs) };
    // `A / s` or `s \ A`: divide by a scalar element-wise.
    if (left && lhs.is_scalar()) || (!left && rhs.is_scalar()) {
        return elementwise_arith(a, b, |x, y| x / y, |x, y| x / y);
    }
    if lhs.is_scalar() && rhs.is_scalar() {
        return elementwise_arith(a, b, |x, y| x / y, |x, y| x / y);
    }
    check_numeric(lhs, rhs)?;
    // `A \ B` solves `A x = B`; `B / A` solves `x A = B`.
    if left {
        fm_linalg::mldivide(lhs, rhs).map_err(linalg_err)
    } else {
        fm_linalg::mrdivide(rhs, lhs).map_err(linalg_err)
    }
}

/// `^` matrix power. Scalar^scalar uses element-wise pow; matrix power via faer.
fn pow(lhs: &Array, rhs: &Array) -> Flow<Array> {
    if lhs.is_scalar() && rhs.is_scalar() {
        return elementwise_pow(lhs, rhs);
    }
    // `A ^ p` with `A` a matrix and `p` a scalar.
    if rhs.is_scalar() {
        let p = rhs.as_f64().ok_or_else(|| {
            Signal::Error(InterpError::msg("matrix power exponent must be a scalar"))
        })?;
        return fm_linalg::mpower(lhs, p).map_err(linalg_err);
    }
    Err(Signal::Error(InterpError::msg(
        "matrix power with a non-scalar exponent is not supported",
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
