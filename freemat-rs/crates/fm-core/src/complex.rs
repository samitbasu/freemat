//! Complex number aliases.
//!
//! FreeMat stores complex data interleaved (real/imag adjacent). We use
//! `num_complex::Complex`, which is `#[repr(C)]` with that exact interleaved
//! layout, so a contiguous buffer of `Complex<f64>` matches FreeMat's storage
//! and the Stage 5 `faer` boundary stays (near) zero-copy.

/// Single-precision complex.
pub type C32 = num_complex::Complex<f32>;
/// Double-precision complex.
pub type C64 = num_complex::Complex<f64>;
