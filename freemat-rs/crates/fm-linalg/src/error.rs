//! The linalg error type.

use std::fmt;

/// An error from a linear-algebra routine (dimension mismatch, singular matrix,
/// failed decomposition, ...). The interpreter wraps this in its `miette`
/// diagnostic with the operator/call span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinalgError {
    /// The human-readable message.
    pub message: String,
}

impl LinalgError {
    /// Construct an error from any message.
    pub fn new(message: impl Into<String>) -> Self {
        LinalgError {
            message: message.into(),
        }
    }
}

impl fmt::Display for LinalgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for LinalgError {}
