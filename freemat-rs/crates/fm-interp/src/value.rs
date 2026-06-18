//! Value helpers: extracting flat numeric data from [`Array`] and rebuilding
//! typed arrays from a `double` result.
//!
//! The operator and indexing layers work in two regimes:
//! - **`double` fast lane** — read every numeric/char/logical value as a
//!   column-major `Vec<f64>`, compute, and rebuild. This covers the bulk of
//!   FreeMat arithmetic (which is carried out in `double`).
//! - **complex lane** — when either operand is complex, read `Vec<C64>`.
//!
//! Integer-class results are re-cast (with saturation) to the promoted integer
//! class by [`cast_f64_to_class`].

use fm_core::{Array, C64, DataClass, ScalarValue};

use crate::error::{Flow, InterpError, Signal};

/// Iterate a dense `ArrayD` in **column-major (memory) order**.
///
/// Arrays are always built F-order, but ndarray's `.iter()` walks *logical*
/// (row-major) order; for our column-major model we must read memory order so
/// linear positions line up. `as_slice_memory_order` returns the contiguous
/// F-order buffer; the (rare) non-contiguous fallback rebuilds it.
pub(crate) fn mem_order<T: Clone>(d: &ndarray::ArrayD<T>) -> Vec<T> {
    if let Some(s) = d.as_slice_memory_order() {
        s.to_vec()
    } else {
        // Non-contiguous: gather by column-major linear position.
        d.t().iter().cloned().collect()
    }
}

/// Read an array's elements, column-major, as `f64`.
///
/// Logical → 0/1, char → code point, integers → value, complex → real part
/// (callers that care about complex use [`to_c64_vec`] instead).
pub fn to_f64_vec(a: &Array) -> Vec<f64> {
    match a {
        Array::Scalar(s) => vec![s.as_f64()],
        Array::Bool(d) => mem_order(d)
            .iter()
            .map(|&v| f64::from(u8::from(v)))
            .collect(),
        Array::Int8(d) => mem_order(d).iter().map(|&v| f64::from(v)).collect(),
        Array::UInt8(d) => mem_order(d).iter().map(|&v| f64::from(v)).collect(),
        Array::Int16(d) => mem_order(d).iter().map(|&v| f64::from(v)).collect(),
        Array::UInt16(d) => mem_order(d).iter().map(|&v| f64::from(v)).collect(),
        Array::Int32(d) => mem_order(d).iter().map(|&v| f64::from(v)).collect(),
        Array::UInt32(d) => mem_order(d).iter().map(|&v| f64::from(v)).collect(),
        Array::Int64(d) => mem_order(d).iter().map(|&v| v as f64).collect(),
        Array::UInt64(d) => mem_order(d).iter().map(|&v| v as f64).collect(),
        Array::Float(d) => mem_order(d).iter().map(|&v| f64::from(v)).collect(),
        Array::Double(d) => mem_order(d),
        Array::Complex32(d) => mem_order(d).iter().map(|v| f64::from(v.re)).collect(),
        Array::Complex64(d) => mem_order(d).iter().map(|v| v.re).collect(),
        Array::Char(d) => mem_order(d)
            .iter()
            .map(|&c| f64::from(u32::from(c)))
            .collect(),
        Array::Cell(_) | Array::Struct(_) => Vec::new(),
        Array::Sparse(s) => s.to_dense_cols().0,
    }
}

/// Read an array's elements, column-major, as complex `C64`.
pub fn to_c64_vec(a: &Array) -> Vec<C64> {
    match a {
        Array::Scalar(ScalarValue::Complex64(c)) => vec![*c],
        Array::Scalar(ScalarValue::Complex32(c)) => {
            vec![C64::new(f64::from(c.re), f64::from(c.im))]
        }
        Array::Complex64(d) => mem_order(d),
        Array::Complex32(d) => mem_order(d)
            .iter()
            .map(|v| C64::new(f64::from(v.re), f64::from(v.im)))
            .collect(),
        Array::Sparse(s) => {
            let (re, im) = s.to_dense_cols();
            match im {
                Some(iv) => re.iter().zip(iv).map(|(&r, m)| C64::new(r, m)).collect(),
                None => re.into_iter().map(|r| C64::new(r, 0.0)).collect(),
            }
        }
        _ => to_f64_vec(a)
            .into_iter()
            .map(|r| C64::new(r, 0.0))
            .collect(),
    }
}

/// Build a real `double`-or-narrower array of `class` from a `double` result
/// vector shaped as `dims` (column-major).
pub fn build_real(class: DataClass, dims: &[usize], data: Vec<f64>) -> Array {
    if data.len() == 1 && dims.iter().product::<usize>() == 1 {
        return Array::Scalar(cast_scalar(class, data[0]));
    }
    match class {
        DataClass::Bool => Array::bool_matrix(dims, data.iter().map(|&v| v != 0.0).collect()),
        DataClass::Double => Array::double_matrix(dims, data),
        DataClass::Float => Array::single_matrix(dims, data.iter().map(|&v| v as f32).collect()),
        DataClass::Char => {
            let chars: Vec<char> = data
                .iter()
                .map(|&v| char::from_u32(v as u32).unwrap_or('\u{fffd}'))
                .collect();
            // Use the generic char constructor path.
            char_matrix(dims, chars)
        }
        c if c.is_integer() => build_integer(c, dims, &data),
        // Cell/Struct never reach here.
        _ => Array::double_matrix(dims, data),
    }
}

/// Build a complex `double` array from a `C64` result vector.
pub fn build_complex(dims: &[usize], data: Vec<C64>) -> Array {
    // Narrow to real if the imaginary part vanished everywhere (MATLAB does).
    if data.iter().all(|c| c.im == 0.0) {
        return build_real(DataClass::Double, dims, data.iter().map(|c| c.re).collect());
    }
    if data.len() == 1 && dims.iter().product::<usize>() == 1 {
        return Array::Scalar(ScalarValue::Complex64(data[0]));
    }
    Array::complex64_matrix(dims, data)
}

/// Convert a sparse [`Array::Sparse`] to its equivalent **dense** [`Array`],
/// restoring the stored element class. Non-sparse inputs are returned unchanged
/// (cloned). This is the densify-on-demand path: any operation that does not
/// have a genuinely sparse implementation densifies first.
pub fn densify(a: &Array) -> Array {
    let Some(s) = a.as_sparse() else {
        return a.clone();
    };
    let dims = [s.rows(), s.cols()];
    let (re, im) = s.to_dense_cols();
    match im {
        Some(iv) => {
            let data: Vec<C64> = re.iter().zip(iv).map(|(&r, m)| C64::new(r, m)).collect();
            // Preserve single-vs-double width via the class.
            if s.class() == DataClass::Float {
                let c32: Vec<fm_core::C32> = data
                    .iter()
                    .map(|c| fm_core::C32::new(c.re as f32, c.im as f32))
                    .collect();
                use ndarray::{ArrayD, IxDyn, ShapeBuilder};
                use std::sync::Arc;
                Array::Complex32(Arc::new(
                    ArrayD::from_shape_vec(IxDyn(&dims).f(), c32).expect("dims match"),
                ))
            } else {
                build_complex(&dims, data)
            }
        }
        None => build_real(s.class(), &dims, re),
    }
}

/// Build a sparse [`Array`] from a dense (or scalar) value, reading its
/// column-major data and class. Char/cell/struct fall back to densify-noop
/// (sparse is numeric only).
pub fn sparsify(a: &Array) -> Array {
    if a.is_sparse() {
        return a.clone();
    }
    let dims = a.shape();
    let (rows, cols) = if dims.len() >= 2 {
        (dims[0], dims[1..].iter().product())
    } else {
        (dims.first().copied().unwrap_or(0), 1)
    };
    let class = a.class();
    if a.is_complex() {
        let cv = to_c64_vec(a);
        let re: Vec<f64> = cv.iter().map(|c| c.re).collect();
        let im: Vec<f64> = cv.iter().map(|c| c.im).collect();
        Array::sparse(fm_core::SparseMatrix::from_dense_cols(
            rows,
            cols,
            class,
            &re,
            Some(&im),
        ))
    } else {
        let re = to_f64_vec(a);
        Array::sparse(fm_core::SparseMatrix::from_dense_cols(
            rows, cols, class, &re, None,
        ))
    }
}

/// A char array from a column-major char vector (handles the scalar case).
pub fn char_matrix(dims: &[usize], data: Vec<char>) -> Array {
    use ndarray::{ArrayD, IxDyn, ShapeBuilder};
    use std::sync::Arc;
    if data.len() == 1 && dims.iter().product::<usize>() == 1 {
        return Array::char_scalar(data[0]);
    }
    let arr = ArrayD::from_shape_vec(IxDyn(dims).f(), data).expect("dims match data");
    Array::Char(Arc::new(arr))
}

/// Cast a single `double` to a [`ScalarValue`] of `class` (saturating for ints).
pub fn cast_scalar(class: DataClass, v: f64) -> ScalarValue {
    match class {
        DataClass::Bool => ScalarValue::Bool(v != 0.0),
        DataClass::Int8 => ScalarValue::Int8(sat_i(v) as i8),
        DataClass::UInt8 => ScalarValue::UInt8(sat_u(v) as u8),
        DataClass::Int16 => ScalarValue::Int16(sat_i(v) as i16),
        DataClass::UInt16 => ScalarValue::UInt16(sat_u(v) as u16),
        DataClass::Int32 => ScalarValue::Int32(sat_i(v) as i32),
        DataClass::UInt32 => ScalarValue::UInt32(sat_u(v) as u32),
        DataClass::Int64 => ScalarValue::Int64(sat_i(v)),
        DataClass::UInt64 => ScalarValue::UInt64(sat_u(v)),
        DataClass::Float => ScalarValue::Float(v as f32),
        DataClass::Double => ScalarValue::Double(v),
        DataClass::Char => ScalarValue::Char(char::from_u32(v as u32).unwrap_or('\u{fffd}')),
        DataClass::Cell | DataClass::Struct => ScalarValue::Double(v),
    }
}

fn build_integer(class: DataClass, dims: &[usize], data: &[f64]) -> Array {
    use ndarray::{ArrayD, IxDyn, ShapeBuilder};
    use std::sync::Arc;
    macro_rules! mk {
        ($variant:ident, $t:ty, $conv:expr) => {{
            let v: Vec<$t> = data.iter().map(|&x| $conv(x)).collect();
            let arr = ArrayD::from_shape_vec(IxDyn(dims).f(), v).expect("dims match");
            Array::$variant(Arc::new(arr))
        }};
    }
    match class {
        DataClass::Int8 => mk!(Int8, i8, |x| sat_i(x) as i8),
        DataClass::UInt8 => mk!(UInt8, u8, |x| sat_u(x) as u8),
        DataClass::Int16 => mk!(Int16, i16, |x| sat_i(x) as i16),
        DataClass::UInt16 => mk!(UInt16, u16, |x| sat_u(x) as u16),
        DataClass::Int32 => mk!(Int32, i32, |x| sat_i(x) as i32),
        DataClass::UInt32 => mk!(UInt32, u32, |x| sat_u(x) as u32),
        DataClass::Int64 => mk!(Int64, i64, sat_i),
        DataClass::UInt64 => mk!(UInt64, u64, sat_u),
        _ => unreachable!("build_integer called with non-integer class"),
    }
}

/// Saturating cast of `f64` → `i64` (MATLAB rounds to nearest, saturates).
pub(crate) fn sat_i(v: f64) -> i64 {
    if v.is_nan() {
        0
    } else {
        let r = v.round();
        if r >= i64::MAX as f64 {
            i64::MAX
        } else if r <= i64::MIN as f64 {
            i64::MIN
        } else {
            r as i64
        }
    }
}

/// Saturating cast of `f64` → `u64`.
pub(crate) fn sat_u(v: f64) -> u64 {
    if v.is_nan() || v < 0.0 {
        0
    } else {
        let r = v.round();
        if r >= u64::MAX as f64 {
            u64::MAX
        } else {
            r as u64
        }
    }
}

/// Interpret a value as a boolean condition (`if`/`while`/short-circuit):
/// **all** elements must be non-zero (and the array non-empty), as in MATLAB.
pub fn truth(a: &Array) -> Flow<bool> {
    match a {
        Array::Scalar(s) => Ok(s.as_f64() != 0.0),
        Array::Cell(_) | Array::Struct(_) => Err(Signal::Error(InterpError::msg(
            "conditional on a cell/struct is not allowed",
        ))),
        _ => {
            if a.numel() == 0 {
                return Ok(false);
            }
            Ok(to_f64_vec(a).into_iter().all(|v| v != 0.0))
        }
    }
}

/// Convert a value to a usize index (1-based MATLAB index → 0-based), erroring
/// on non-positive / non-integer / non-scalar input.
pub fn to_index(v: f64) -> Flow<usize> {
    if v < 1.0 || v.fract() != 0.0 {
        return Err(Signal::Error(InterpError::msg(format!(
            "index must be a positive integer, got {v}"
        ))));
    }
    Ok((v as usize) - 1)
}
