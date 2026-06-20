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
mod baseconv;
mod bitops;
mod cellstruct;
mod constructors;
mod docs;
mod elementary;
mod fitfun;
mod graphics;
mod help;
mod inspection;
mod interp_ops;
mod linalg;
mod logical;
mod misc;
mod polynomial;
mod random;
mod reductions;
mod setops;
mod sparse;
mod strings;
mod time;
mod trig;
mod util;

/// Register the full Stage-5 standard library into an [`Interpreter`].
///
/// Call this after constructing the interpreter (which already registers the
/// minimal Stage-3 defaults); these registrations layer on top.
pub fn register_standard_library(interp: &mut Interpreter) {
    register_into(&mut interp.functions);
    register_toolbox_m(interp);
}

/// Define the handful of FreeMat `toolbox/*.m` functions we ship as embedded
/// source (rather than native builtins) because they are most faithfully
/// expressed in M-code on top of native primitives. Currently the curve-fitting
/// wrappers `gausfit`/`gfitfun`, which build on the native `fitfun`.
fn register_toolbox_m(interp: &mut Interpreter) {
    // Verbatim from `toolbox/fitting/gfitfun.m` and `gausfit.m`.
    const GFITFUN_M: &str = "\
function y = gfitfun(x,times)
y = x(4)*exp(-((times-x(1)).^2)/(2*x(2)^2)) + x(3);
";
    const GAUSFIT_M: &str = "\
function [mu,sigma,dc,gain,yhat] = gausfit(t,y,w,mug,sigmag,dcg,gaing)
if (~isset('w'))
  w = y*0+1;
end
if (~isset('dcg'))
  dcg = min(y(:));
end
ycor = y - dcg;
if (~isset('gaing'))
  gaing = max(ycor);
end
ycor = ycor/gaing;
if (~isset('mug'))
  mug = sum(ycor.*t)/sum(ycor);
end
if (~isset('sigmag'))
  sigmag = sqrt(abs(sum((ycor).*(t-mug).^2)/sum(ycor)));
end
[xopt,err] = fitfun('gfitfun',[mug,sigmag,dcg,gaing],y,w,eps,t);
mu = xopt(1);
sigma = xopt(2);
dc = xopt(3);
gain = xopt(4);
yhat = gfitfun(xopt,t);
";
    for src in [GFITFUN_M, GAUSFIT_M] {
        // Embedded source is known-good; a parse failure is a build-time bug.
        interp
            .define_source(src)
            .expect("embedded toolbox M-source must parse");
    }
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
    time::register(table);
    interp_ops::register(table);
    bitops::register(table);
    baseconv::register(table);
    polynomial::register(table);
    fitfun::register(table);
    misc::register(table);
    // Sparse builtins. Registered after the dense modules so `find`/`full`
    // become sparse-aware (they shadow the dense versions).
    sparse::register(table);
    graphics::register(table);
    graphics::register_log_plots(table);
    // Stage 8: MAT save/load, file I/O, FFT, regex. Registered last so its
    // `exist` (which also checks for files on disk) shadows the interp_ops one.
    fm_io::register(table);
    // Help system (P5): `help`/`helpwin`.
    help::register(table);
}

#[cfg(test)]
mod tests;
