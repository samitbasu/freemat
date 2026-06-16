//! Data classes — the set of element/value types an [`crate::Array`] can hold.
//!
//! Mirrors FreeMat's `DataClass` enum (`libs/libFreeMat/Array.hpp`), minus the
//! sparse flag (deferred). `Index` in FreeMat aliases `Double`; we do not model
//! it separately.

/// The class of values stored in an array.
///
/// Numeric ordering of the integer/float variants follows FreeMat's promotion
/// lattice loosely, but promotion logic lives in [`crate::promote`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DataClass {
    /// Logical (`bool`).
    Bool,
    /// Signed 8-bit integer.
    Int8,
    /// Unsigned 8-bit integer.
    UInt8,
    /// Signed 16-bit integer.
    Int16,
    /// Unsigned 16-bit integer.
    UInt16,
    /// Signed 32-bit integer.
    Int32,
    /// Unsigned 32-bit integer.
    UInt32,
    /// Signed 64-bit integer.
    Int64,
    /// Unsigned 64-bit integer.
    UInt64,
    /// Single-precision float (real or complex).
    Float,
    /// Double-precision float (real or complex).
    Double,
    /// Character data (UTF-32 code points), printed as text.
    Char,
    /// Cell array (heterogeneous container).
    Cell,
    /// Struct array (named fields).
    Struct,
}

impl DataClass {
    /// The MATLAB/FreeMat class name (`class(x)` result).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            DataClass::Bool => "logical",
            DataClass::Int8 => "int8",
            DataClass::UInt8 => "uint8",
            DataClass::Int16 => "int16",
            DataClass::UInt16 => "uint16",
            DataClass::Int32 => "int32",
            DataClass::UInt32 => "uint32",
            DataClass::Int64 => "int64",
            DataClass::UInt64 => "uint64",
            DataClass::Float => "single",
            DataClass::Double => "double",
            DataClass::Char => "char",
            DataClass::Cell => "cell",
            DataClass::Struct => "struct",
        }
    }

    /// `true` for the integer classes (signed or unsigned, not `Bool`).
    #[must_use]
    pub const fn is_integer(self) -> bool {
        matches!(
            self,
            DataClass::Int8
                | DataClass::UInt8
                | DataClass::Int16
                | DataClass::UInt16
                | DataClass::Int32
                | DataClass::UInt32
                | DataClass::Int64
                | DataClass::UInt64
        )
    }

    /// `true` for `Float`/`Double`.
    #[must_use]
    pub const fn is_float(self) -> bool {
        matches!(self, DataClass::Float | DataClass::Double)
    }

    /// `true` for any numeric class (bool counts as numeric in MATLAB math).
    #[must_use]
    pub const fn is_numeric(self) -> bool {
        self.is_integer() || self.is_float() || matches!(self, DataClass::Bool)
    }

    /// `true` for reference-style containers (cell/struct).
    #[must_use]
    pub const fn is_reference(self) -> bool {
        matches!(self, DataClass::Cell | DataClass::Struct)
    }
}
