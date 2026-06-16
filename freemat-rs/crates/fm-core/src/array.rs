//! The [`Array`] value — the type the whole interpreter operates on.
//!
//! # Layout
//! - **Inline scalar fast path:** [`Array::Scalar`] stores one [`ScalarValue`]
//!   directly (no heap, no 1x1 ndarray). This mirrors FreeMat's union + scalar
//!   flag and keeps scalar temporaries allocation-free.
//! - **Dense variants** hold `Arc<ArrayD<T>>`, stored **column-major (F-order)**
//!   to match MATLAB and to keep the Stage 5 `faer` boundary near zero-copy.
//! - **COW:** sharing clones the `Arc` (cheap); mutation goes through
//!   [`Arc::make_mut`], which deep-copies only if the buffer is shared.
//!
//! `Cell` and `Struct` are reference containers and have no scalar fast path.

use std::sync::Arc;

use ndarray::{ArrayD, IxDyn, ShapeBuilder};

use crate::class::DataClass;
use crate::complex::{C32, C64};
use crate::scalar::ScalarValue;
use crate::struct_array::{StructArray, StructHandle};

/// A shared, copy-on-write dense buffer of element type `T`, column-major.
pub type Dense<T> = Arc<ArrayD<T>>;

/// A FreeMat value: a typed, N-dimensional, column-major array (or an inline
/// scalar) with copy-on-write sharing.
#[derive(Debug, Clone, PartialEq)]
pub enum Array {
    /// Inline scalar — no heap allocation. The performance fast path.
    Scalar(ScalarValue),

    /// Dense logical array.
    Bool(Dense<bool>),
    /// Dense `int8` array.
    Int8(Dense<i8>),
    /// Dense `uint8` array.
    UInt8(Dense<u8>),
    /// Dense `int16` array.
    Int16(Dense<i16>),
    /// Dense `uint16` array.
    UInt16(Dense<u16>),
    /// Dense `int32` array.
    Int32(Dense<i32>),
    /// Dense `uint32` array.
    UInt32(Dense<u32>),
    /// Dense `int64` array.
    Int64(Dense<i64>),
    /// Dense `uint64` array.
    UInt64(Dense<u64>),
    /// Dense real `single` array.
    Float(Dense<f32>),
    /// Dense real `double` array.
    Double(Dense<f64>),
    /// Dense complex `single` array (interleaved).
    Complex32(Dense<C32>),
    /// Dense complex `double` array (interleaved).
    Complex64(Dense<C64>),
    /// Char array (UTF-32 code points), column-major.
    Char(Dense<char>),

    /// Cell array — each element is an [`Array`].
    Cell(Dense<Array>),
    /// Struct array — named fields.
    Struct(StructHandle),
}

/// Build a new column-major (F-order) `ArrayD` from `dims` + a column-major
/// data vector. Returns the dynamic array.
fn from_col_major<T: Clone>(dims: &[usize], data: Vec<T>) -> ArrayD<T> {
    // ndarray's `from_shape_vec` interprets the vec in the shape's memory order.
    // `IxDyn(dims).f()` yields an F-order (column-major) layout, so a
    // column-major `data` slots in directly.
    ArrayD::from_shape_vec(IxDyn(dims).f(), data)
        .expect("element count must match dims (checked by callers)")
}

impl Array {
    // ---- Scalar constructors (inline, heap-free) --------------------------

    /// Inline scalar from any [`ScalarValue`].
    #[must_use]
    pub const fn scalar(v: ScalarValue) -> Self {
        Array::Scalar(v)
    }

    /// Inline `double` scalar — the most common literal.
    #[must_use]
    pub const fn double(v: f64) -> Self {
        Array::Scalar(ScalarValue::Double(v))
    }

    /// Inline `single` scalar.
    #[must_use]
    pub const fn single(v: f32) -> Self {
        Array::Scalar(ScalarValue::Float(v))
    }

    /// Inline logical scalar.
    #[must_use]
    pub const fn bool(v: bool) -> Self {
        Array::Scalar(ScalarValue::Bool(v))
    }

    /// Inline `int32` scalar.
    #[must_use]
    pub const fn int32(v: i32) -> Self {
        Array::Scalar(ScalarValue::Int32(v))
    }

    /// Inline complex `double` scalar.
    #[must_use]
    pub const fn complex64(v: C64) -> Self {
        Array::Scalar(ScalarValue::Complex64(v))
    }

    /// Inline `char` scalar.
    #[must_use]
    pub const fn char_scalar(v: char) -> Self {
        Array::Scalar(ScalarValue::Char(v))
    }

    // ---- Dense constructors ----------------------------------------------

    /// A dense `double` array from column-major `data` shaped as `dims`.
    ///
    /// # Panics
    /// Panics if `data.len()` differs from the product of `dims`.
    #[must_use]
    pub fn double_matrix(dims: &[usize], data: Vec<f64>) -> Self {
        Array::Double(Arc::new(from_col_major(dims, data)))
    }

    /// A dense `single` array from column-major data.
    ///
    /// # Panics
    /// Panics if `data.len()` differs from the product of `dims`.
    #[must_use]
    pub fn single_matrix(dims: &[usize], data: Vec<f32>) -> Self {
        Array::Float(Arc::new(from_col_major(dims, data)))
    }

    /// A dense logical array from column-major data.
    ///
    /// # Panics
    /// Panics if `data.len()` differs from the product of `dims`.
    #[must_use]
    pub fn bool_matrix(dims: &[usize], data: Vec<bool>) -> Self {
        Array::Bool(Arc::new(from_col_major(dims, data)))
    }

    /// A dense `int32` array from column-major data.
    ///
    /// # Panics
    /// Panics if `data.len()` differs from the product of `dims`.
    #[must_use]
    pub fn int32_matrix(dims: &[usize], data: Vec<i32>) -> Self {
        Array::Int32(Arc::new(from_col_major(dims, data)))
    }

    /// A dense complex `double` array from column-major data.
    ///
    /// # Panics
    /// Panics if `data.len()` differs from the product of `dims`.
    #[must_use]
    pub fn complex64_matrix(dims: &[usize], data: Vec<C64>) -> Self {
        Array::Complex64(Arc::new(from_col_major(dims, data)))
    }

    /// A char array (row vector) from a string's code points.
    #[must_use]
    pub fn char_string(s: &str) -> Self {
        let data: Vec<char> = s.chars().collect();
        if data.len() == 1 {
            return Array::char_scalar(data[0]);
        }
        let n = data.len();
        Array::Char(Arc::new(from_col_major(&[1, n], data)))
    }

    /// A cell array from column-major elements shaped as `dims`.
    ///
    /// # Panics
    /// Panics if `data.len()` differs from the product of `dims`.
    #[must_use]
    pub fn cell(dims: &[usize], data: Vec<Array>) -> Self {
        Array::Cell(Arc::new(from_col_major(dims, data)))
    }

    /// A struct array.
    #[must_use]
    pub fn struct_array(s: StructArray) -> Self {
        Array::Struct(Arc::new(s))
    }

    /// The canonical empty value (`[]`): a 0x0 double matrix.
    #[must_use]
    pub fn empty() -> Self {
        Array::Double(Arc::new(from_col_major::<f64>(&[0, 0], Vec::new())))
    }

    // ---- Class / shape queries -------------------------------------------

    /// The data class of this value.
    #[must_use]
    pub fn class(&self) -> DataClass {
        match self {
            Array::Scalar(s) => s.class(),
            Array::Bool(_) => DataClass::Bool,
            Array::Int8(_) => DataClass::Int8,
            Array::UInt8(_) => DataClass::UInt8,
            Array::Int16(_) => DataClass::Int16,
            Array::UInt16(_) => DataClass::UInt16,
            Array::Int32(_) => DataClass::Int32,
            Array::UInt32(_) => DataClass::UInt32,
            Array::Int64(_) => DataClass::Int64,
            Array::UInt64(_) => DataClass::UInt64,
            Array::Float(_) | Array::Complex32(_) => DataClass::Float,
            Array::Double(_) | Array::Complex64(_) => DataClass::Double,
            Array::Char(_) => DataClass::Char,
            Array::Cell(_) => DataClass::Cell,
            Array::Struct(_) => DataClass::Struct,
        }
    }

    /// The class name (`class(x)`).
    #[must_use]
    pub fn class_name(&self) -> &'static str {
        self.class().name()
    }

    /// Dimensions of this value (column-major). Scalars report `[1, 1]`.
    #[must_use]
    pub fn dims(&self) -> Vec<usize> {
        match self {
            Array::Scalar(_) => vec![1, 1],
            Array::Bool(a) => a.shape().to_vec(),
            Array::Int8(a) => a.shape().to_vec(),
            Array::UInt8(a) => a.shape().to_vec(),
            Array::Int16(a) => a.shape().to_vec(),
            Array::UInt16(a) => a.shape().to_vec(),
            Array::Int32(a) => a.shape().to_vec(),
            Array::UInt32(a) => a.shape().to_vec(),
            Array::Int64(a) => a.shape().to_vec(),
            Array::UInt64(a) => a.shape().to_vec(),
            Array::Float(a) => a.shape().to_vec(),
            Array::Double(a) => a.shape().to_vec(),
            Array::Complex32(a) => a.shape().to_vec(),
            Array::Complex64(a) => a.shape().to_vec(),
            Array::Char(a) => a.shape().to_vec(),
            Array::Cell(a) => a.shape().to_vec(),
            Array::Struct(s) => s.dims().to_vec(),
        }
    }

    /// Number of elements.
    #[must_use]
    pub fn numel(&self) -> usize {
        match self {
            Array::Scalar(_) => 1,
            Array::Struct(s) => s.numel(),
            _ => self.dims().iter().product(),
        }
    }

    /// Whether this is a scalar (1 element).
    #[must_use]
    pub fn is_scalar(&self) -> bool {
        matches!(self, Array::Scalar(_)) || self.numel() == 1
    }

    /// Whether this is empty (0 elements).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !matches!(self, Array::Scalar(_)) && self.numel() == 0
    }

    /// Whether this value is complex (has a non-trivial imaginary part).
    #[must_use]
    pub fn is_complex(&self) -> bool {
        matches!(self, Array::Complex32(_) | Array::Complex64(_))
            || matches!(self, Array::Scalar(s) if s.is_complex())
    }

    /// Whether this is a string (char) value.
    #[must_use]
    pub fn is_char(&self) -> bool {
        self.class() == DataClass::Char
    }

    // ---- Scalar extraction ------------------------------------------------

    /// The inline scalar value, if this is a scalar of any kind.
    ///
    /// Dense 1-element arrays are read out into a [`ScalarValue`] too.
    #[must_use]
    pub fn as_scalar(&self) -> Option<ScalarValue> {
        match self {
            Array::Scalar(s) => Some(*s),
            _ if self.numel() == 1 => self.first_scalar(),
            _ => None,
        }
    }

    /// Read this value's first element as a [`ScalarValue`] (helper).
    fn first_scalar(&self) -> Option<ScalarValue> {
        macro_rules! first {
            ($a:expr, $ctor:path) => {
                $a.iter().next().copied().map($ctor)
            };
        }
        match self {
            Array::Scalar(s) => Some(*s),
            Array::Bool(a) => first!(a, ScalarValue::Bool),
            Array::Int8(a) => first!(a, ScalarValue::Int8),
            Array::UInt8(a) => first!(a, ScalarValue::UInt8),
            Array::Int16(a) => first!(a, ScalarValue::Int16),
            Array::UInt16(a) => first!(a, ScalarValue::UInt16),
            Array::Int32(a) => first!(a, ScalarValue::Int32),
            Array::UInt32(a) => first!(a, ScalarValue::UInt32),
            Array::Int64(a) => first!(a, ScalarValue::Int64),
            Array::UInt64(a) => first!(a, ScalarValue::UInt64),
            Array::Float(a) => first!(a, ScalarValue::Float),
            Array::Double(a) => first!(a, ScalarValue::Double),
            Array::Complex32(a) => first!(a, ScalarValue::Complex32),
            Array::Complex64(a) => first!(a, ScalarValue::Complex64),
            Array::Char(a) => first!(a, ScalarValue::Char),
            Array::Cell(_) | Array::Struct(_) => None,
        }
    }

    /// Coerce a scalar to `f64` (lossy for complex/integer), like FreeMat's
    /// `asDouble`. Returns `None` for non-scalar or reference values.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        self.as_scalar().map(ScalarValue::as_f64)
    }

    /// If this is a char value, decode it back to a `String` (column-major
    /// order, which for a row vector is left-to-right).
    #[must_use]
    pub fn as_string(&self) -> Option<String> {
        match self {
            Array::Scalar(ScalarValue::Char(c)) => Some(c.to_string()),
            Array::Char(a) => Some(a.iter().collect()),
            _ => None,
        }
    }

    /// Borrow struct data if this is a struct array.
    #[must_use]
    pub fn as_struct(&self) -> Option<&StructArray> {
        match self {
            Array::Struct(s) => Some(s),
            _ => None,
        }
    }

    /// Borrow cell elements if this is a cell array.
    #[must_use]
    pub fn as_cell(&self) -> Option<&ArrayD<Array>> {
        match self {
            Array::Cell(a) => Some(a),
            _ => None,
        }
    }

    // ---- Copy-on-write ----------------------------------------------------

    /// Whether this value's backing buffer is uniquely owned (not shared).
    ///
    /// Inline scalars are always "unique" (they own their value by copy).
    #[must_use]
    pub fn is_unique(&self) -> bool {
        macro_rules! one {
            ($a:expr) => {
                Arc::strong_count($a) == 1 && Arc::weak_count($a) == 0
            };
        }
        match self {
            Array::Scalar(_) => true,
            Array::Bool(a) => one!(a),
            Array::Int8(a) => one!(a),
            Array::UInt8(a) => one!(a),
            Array::Int16(a) => one!(a),
            Array::UInt16(a) => one!(a),
            Array::Int32(a) => one!(a),
            Array::UInt32(a) => one!(a),
            Array::Int64(a) => one!(a),
            Array::UInt64(a) => one!(a),
            Array::Float(a) => one!(a),
            Array::Double(a) => one!(a),
            Array::Complex32(a) => one!(a),
            Array::Complex64(a) => one!(a),
            Array::Char(a) => one!(a),
            Array::Cell(a) => one!(a),
            Array::Struct(a) => one!(a),
        }
    }
}

/// Mutable, copy-on-write access to a dense buffer of a chosen element type.
///
/// `make_mut` deep-copies only when the `Arc` is shared, exactly mirroring
/// `Arc::make_mut`. Returns `None` if `self` is not that variant.
macro_rules! cow_accessor {
    ($name:ident, $variant:ident, $t:ty) => {
        impl Array {
            #[doc = concat!("COW-mutable access to the backing `", stringify!($t), "` buffer.")]
            #[must_use]
            pub fn $name(&mut self) -> Option<&mut ArrayD<$t>> {
                match self {
                    Array::$variant(a) => Some(Arc::make_mut(a)),
                    _ => None,
                }
            }
        }
    };
}

cow_accessor!(make_mut_bool, Bool, bool);
cow_accessor!(make_mut_int8, Int8, i8);
cow_accessor!(make_mut_uint8, UInt8, u8);
cow_accessor!(make_mut_int16, Int16, i16);
cow_accessor!(make_mut_uint16, UInt16, u16);
cow_accessor!(make_mut_int32, Int32, i32);
cow_accessor!(make_mut_uint32, UInt32, u32);
cow_accessor!(make_mut_int64, Int64, i64);
cow_accessor!(make_mut_uint64, UInt64, u64);
cow_accessor!(make_mut_float, Float, f32);
cow_accessor!(make_mut_double, Double, f64);
cow_accessor!(make_mut_complex32, Complex32, C32);
cow_accessor!(make_mut_complex64, Complex64, C64);
cow_accessor!(make_mut_char, Char, char);
cow_accessor!(make_mut_cell, Cell, Array);
