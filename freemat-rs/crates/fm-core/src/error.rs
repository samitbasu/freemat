//! Error type for `fm-core` operations.

use std::fmt;

use crate::class::DataClass;

/// Errors produced by core array operations.
///
/// `fm-core` sits below the interpreter, so these are plain Rust errors. The
/// interpreter (Stage 3) wraps user-facing failures in `miette` diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// The requested element type does not match the array's storage class.
    TypeMismatch {
        /// The class the caller expected.
        expected: DataClass,
        /// The class actually stored.
        found: DataClass,
    },
    /// An operation required a scalar but the array was not scalar.
    NotScalar {
        /// The class of the offending array.
        class: DataClass,
    },
    /// A reshape/resize requested a different number of elements.
    ShapeMismatch {
        /// Element count requested.
        requested: usize,
        /// Element count available.
        available: usize,
    },
    /// Two arrays could not be promoted to a common type.
    Incompatible {
        /// Left-hand class.
        lhs: DataClass,
        /// Right-hand class.
        rhs: DataClass,
    },
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::TypeMismatch { expected, found } => {
                write!(
                    f,
                    "type mismatch: expected {}, found {}",
                    expected.name(),
                    found.name()
                )
            }
            CoreError::NotScalar { class } => {
                write!(f, "expected a scalar {} value", class.name())
            }
            CoreError::ShapeMismatch {
                requested,
                available,
            } => {
                write!(
                    f,
                    "shape mismatch: cannot fit {available} elements into {requested}"
                )
            }
            CoreError::Incompatible { lhs, rhs } => {
                write!(f, "incompatible classes: {} and {}", lhs.name(), rhs.name())
            }
        }
    }
}

impl std::error::Error for CoreError {}

/// Convenient result alias for core operations.
pub type Result<T> = std::result::Result<T, CoreError>;
