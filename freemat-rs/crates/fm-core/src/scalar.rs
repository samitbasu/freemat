//! The inline scalar value — the heap-free fast path.
//!
//! FreeMat keeps a `union` of every primitive type inside `Array` so a 1x1
//! value never touches the heap (`Array.hpp`, the `Data` union + `Scalar`
//! flag). We replicate that with an enum: [`crate::Array::Scalar`] stores a
//! [`ScalarValue`] directly, so scalar temporaries in tight interpreter loops
//! allocate nothing.

use crate::class::DataClass;
use crate::complex::{C32, C64};

/// A single value of any numeric (or char) class, stored inline.
///
/// Cell/struct have no scalar fast path (they are always reference-backed), so
/// they are absent here by design.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScalarValue {
    /// Logical scalar.
    Bool(bool),
    /// `int8` scalar.
    Int8(i8),
    /// `uint8` scalar.
    UInt8(u8),
    /// `int16` scalar.
    Int16(i16),
    /// `uint16` scalar.
    UInt16(u16),
    /// `int32` scalar.
    Int32(i32),
    /// `uint32` scalar.
    UInt32(u32),
    /// `int64` scalar.
    Int64(i64),
    /// `uint64` scalar.
    UInt64(u64),
    /// `single` (real) scalar.
    Float(f32),
    /// `double` (real) scalar.
    Double(f64),
    /// `single` complex scalar.
    Complex32(C32),
    /// `double` complex scalar.
    Complex64(C64),
    /// `char` scalar (a single code point).
    Char(char),
}

impl ScalarValue {
    /// The data class of this scalar.
    #[must_use]
    pub const fn class(self) -> DataClass {
        match self {
            ScalarValue::Bool(_) => DataClass::Bool,
            ScalarValue::Int8(_) => DataClass::Int8,
            ScalarValue::UInt8(_) => DataClass::UInt8,
            ScalarValue::Int16(_) => DataClass::Int16,
            ScalarValue::UInt16(_) => DataClass::UInt16,
            ScalarValue::Int32(_) => DataClass::Int32,
            ScalarValue::UInt32(_) => DataClass::UInt32,
            ScalarValue::Int64(_) => DataClass::Int64,
            ScalarValue::UInt64(_) => DataClass::UInt64,
            ScalarValue::Float(_) | ScalarValue::Complex32(_) => DataClass::Float,
            ScalarValue::Double(_) | ScalarValue::Complex64(_) => DataClass::Double,
            ScalarValue::Char(_) => DataClass::Char,
        }
    }

    /// `true` if this is a complex scalar.
    #[must_use]
    pub const fn is_complex(self) -> bool {
        matches!(self, ScalarValue::Complex32(_) | ScalarValue::Complex64(_))
    }

    /// Add two scalars in `double` (the common interpreter fast path).
    ///
    /// This is deliberately heap-free: it stays inside [`ScalarValue`] and never
    /// touches an `ArrayD`. Promotion/typed arithmetic for the full class
    /// lattice arrives with the interpreter; this covers the scalar hot loop the
    /// Stage 1 performance requirement targets. Complex operands keep their
    /// imaginary parts.
    #[must_use]
    pub fn add_scalar(self, rhs: ScalarValue) -> ScalarValue {
        match (self, rhs) {
            (ScalarValue::Complex64(a), _) => {
                ScalarValue::Complex64(a + C64::new(rhs.as_f64(), 0.0))
            }
            (_, ScalarValue::Complex64(b)) => {
                ScalarValue::Complex64(C64::new(self.as_f64(), 0.0) + b)
            }
            _ => ScalarValue::Double(self.as_f64() + rhs.as_f64()),
        }
    }

    /// Coerce to `f64`. Complex values yield their real part; this matches the
    /// lossy `asDouble` behaviour used for indexing/printing decisions.
    #[must_use]
    pub fn as_f64(self) -> f64 {
        match self {
            ScalarValue::Bool(v) => f64::from(u8::from(v)),
            ScalarValue::Int8(v) => f64::from(v),
            ScalarValue::UInt8(v) => f64::from(v),
            ScalarValue::Int16(v) => f64::from(v),
            ScalarValue::UInt16(v) => f64::from(v),
            ScalarValue::Int32(v) => f64::from(v),
            ScalarValue::UInt32(v) => f64::from(v),
            ScalarValue::Int64(v) => v as f64,
            ScalarValue::UInt64(v) => v as f64,
            ScalarValue::Float(v) => f64::from(v),
            ScalarValue::Double(v) => v,
            ScalarValue::Complex32(v) => f64::from(v.re),
            ScalarValue::Complex64(v) => v.re,
            ScalarValue::Char(v) => f64::from(u32::from(v)),
        }
    }
}
