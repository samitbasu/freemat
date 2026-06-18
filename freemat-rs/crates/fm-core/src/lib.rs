//! `fm-core` — FreeMat-rs core value types and the [`Array`] container.
//!
//! This crate defines the single value type the whole interpreter operates on.
//! Design (see `docs/PLAN.md`, Stage 1):
//!
//! - [`Array`] is an `enum` over element type, backed by [`ndarray`] with
//!   `IxDyn` and stored **column-major (F-order)** to match MATLAB.
//! - Sharing is **copy-on-write** via `Arc` + `Arc::make_mut`.
//! - There is a mandatory **inline [`Array::Scalar`] variant** ([`ScalarValue`])
//!   so scalar temporaries never heap-allocate — mirroring FreeMat's union +
//!   scalar-flag fast path.
//! - Complex data uses [`num_complex`], stored interleaved.
//! - Char/string, [`cell`](Array::cell), and [`struct`](Array::struct_array)
//!   arrays are supported.
//! - MATLAB-style [`Array::format`] display with `format short`/`long`.

pub mod array;
pub mod class;
pub mod complex;
pub mod error;
pub mod format;
pub mod promote;
pub mod scalar;
pub mod sparse;
pub mod struct_array;

pub use array::{Array, Dims};
pub use class::DataClass;
pub use complex::{C32, C64};
pub use error::{CoreError, Result};
pub use format::FormatMode;
pub use promote::promote;
pub use scalar::ScalarValue;
pub use sparse::SparseMatrix;
pub use struct_array::StructArray;
