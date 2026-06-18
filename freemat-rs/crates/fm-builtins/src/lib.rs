//! `fm-builtins` — FreeMat-rs builtin functions ported from libCore.
//!
//! This is the first tranche of the libCore builtin surface (Stage 5):
//! elementary math, trig, reductions, logical reductions (`all`/`any`),
//! constructors (`zeros`/`ones`/`eye`/`linspace`), relational/logical helpers,
//! random number generators, and the linear-algebra builtins wrapping
//! [`fm_linalg`].
//!
//! Registration mirrors FreeMat's `addFunction` (plain) / `addSpecialFunction`
//! (interpreter-aware): every builtin receives `&mut Interpreter`, so the one
//! [`fm_interp::BuiltinFn`] signature covers both. [`register_standard_library`]
//! installs the whole set on top of the interpreter's minimal defaults.

use fm_interp::{FunctionTable, Interpreter};

mod array_manip;
mod cellstruct;
mod constructors;
mod elementary;
mod graphics;
mod inspection;
mod interp_ops;
mod linalg;
mod logical;
mod random;
mod reductions;
mod setops;
mod strings;
mod trig;
mod util;

/// Register the full Stage-5 standard library into an [`Interpreter`].
///
/// Call this after constructing the interpreter (which already registers the
/// minimal Stage-3 defaults); these registrations layer on top.
pub fn register_standard_library(interp: &mut Interpreter) {
    register_into(&mut interp.functions);
}

/// Register every builtin into a [`FunctionTable`] directly.
pub fn register_into(table: &mut FunctionTable) {
    elementary::register(table);
    trig::register(table);
    reductions::register(table);
    logical::register(table);
    constructors::register(table);
    random::register(table);
    linalg::register(table);
    inspection::register(table);
    array_manip::register(table);
    strings::register(table);
    setops::register(table);
    cellstruct::register(table);
    interp_ops::register(table);
    graphics::register(table);
    graphics::register_log_plots(table);
    // Stage 8: MAT save/load, file I/O, FFT, regex. Registered last so its
    // `exist` (which also checks for files on disk) shadows the interp_ops one.
    fm_io::register(table);
}

#[cfg(test)]
mod tests;
