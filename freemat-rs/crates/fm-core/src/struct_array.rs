//! Struct arrays — an ordered set of named fields, each an array of cells.
//!
//! MATLAB struct arrays are rectangular: every field has one [`Array`] per
//! element, and all fields share the same dimensions. We store this as an
//! insertion-ordered field map; each field maps to a flat (column-major)
//! `Vec<Array>` of length `numel`.

use std::sync::Arc;

use crate::array::Array;

/// A struct array: field names plus, per field, one [`Array`] per element.
///
/// `dims` gives the array shape (column-major); `numel` is their product.
/// Field order is preserved (MATLAB shows fields in definition order).
#[derive(Debug, Clone, PartialEq)]
pub struct StructArray {
    pub(crate) dims: Vec<usize>,
    /// Field name + its column-major element vector (length == numel).
    pub(crate) fields: Vec<(String, Vec<Array>)>,
}

impl StructArray {
    /// An empty (0x0) struct with no fields.
    #[must_use]
    pub fn empty() -> Self {
        StructArray {
            dims: vec![0, 0],
            fields: Vec::new(),
        }
    }

    /// A 1x1 scalar struct from `(name, value)` pairs, in the given order.
    #[must_use]
    pub fn scalar(fields: impl IntoIterator<Item = (String, Array)>) -> Self {
        let fields = fields.into_iter().map(|(n, v)| (n, vec![v])).collect();
        StructArray {
            dims: vec![1, 1],
            fields,
        }
    }

    /// The struct array dimensions.
    #[must_use]
    pub fn dims(&self) -> &[usize] {
        &self.dims
    }

    /// Number of elements (product of dims).
    #[must_use]
    pub fn numel(&self) -> usize {
        self.dims.iter().product()
    }

    /// Field names in order.
    #[must_use]
    pub fn field_names(&self) -> Vec<&str> {
        self.fields.iter().map(|(n, _)| n.as_str()).collect()
    }

    /// Whether a field exists.
    #[must_use]
    pub fn has_field(&self, name: &str) -> bool {
        self.fields.iter().any(|(n, _)| n == name)
    }

    /// Borrow a field's element vector.
    #[must_use]
    pub fn field(&self, name: &str) -> Option<&[Array]> {
        self.fields
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_slice())
    }

    /// The single value of `field` for a scalar struct.
    #[must_use]
    pub fn scalar_field(&self, name: &str) -> Option<&Array> {
        self.field(name).and_then(|v| v.first())
    }
}

/// Convenience wrapper used by [`Array`] to share struct data copy-on-write.
pub(crate) type StructHandle = Arc<StructArray>;
