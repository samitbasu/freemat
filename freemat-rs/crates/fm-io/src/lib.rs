//! `fm-io` — FreeMat-rs file I/O, MAT-file support, and FFT/regex builtins.
//!
//! # Contents
//! - [`matfile`] — MATLAB v7 / Level-5 MAT-file read and write (double/single,
//!   complex, all integer classes, logical, char, struct, cell; zlib-compressed
//!   on write). Sparse is deferred to Stage 9.
//! - [`fileio`] — low-level file I/O (`fopen`/`fclose`/`fread`/`fwrite`/
//!   `fgetl`/`fgets`/`frewind`/`feof`) over an open-file table.
//! - [`scanf`] — `sscanf`/`fscanf` formatted parsing.
//! - [`fft`] — `fft`/`ifft`/`fft2`/`ifft2` via [`rustfft`].
//! - [`regexp`] — `regexp`/`regexpi`/`regexprep` via the [`regex`] crate.
//!
//! [`register`] installs all of these (plus `save`/`load`/`fprintf`/`exist`)
//! into an interpreter's [`FunctionTable`].

pub mod builtins;
pub mod fft;
pub mod fileio;
pub mod matfile;
pub mod regexp;
pub mod scanf;

pub use builtins::register;
pub use matfile::{MatError, MatVar, read_mat, write_mat};

use fm_interp::Interpreter;

/// Register the `fm-io` builtins into an [`Interpreter`].
pub fn register_io(interp: &mut Interpreter) {
    builtins::register(&mut interp.functions);
}

#[cfg(test)]
mod tests;
