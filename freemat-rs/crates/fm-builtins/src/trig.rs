//! Trigonometric and hyperbolic builtins. Real inputs use the `f64` path;
//! `atan2` is a two-argument element-wise builtin.

use fm_core::{Array, DataClass};
use fm_interp::error::Flow;
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};

use crate::util::{map_double, need};

// P3 smoke fixture — P7 will migrate the full doc surface.
// The section the cos entry lives in must exist for docgen's unknown-section
// validation; P7 will seed the full section set from section_descriptors.txt.
fm_doc::register_section! {
    id: "mathfunctions",
    title: "Mathematical Functions",
    summary: "Elementary and special mathematical functions.",
}

fm_doc::register_doc! {
    name: "cos",
    section: "mathfunctions",
    summary: "Trigonometric cosine of the argument.",
    body: r#"
## Usage
Computes `cos(x)` for an n-dimensional numeric array `x`, element-wise, in
radians. Output has the same size as `x`.

```fm-exec
cos(0)
```
"#,
}

pub(crate) fn register(table: &mut FunctionTable) {
    macro_rules! reg {
        ($name:literal, $f:expr) => {
            table.add_builtin($name, |_i, a, _n| simple(a, $name, $f));
        };
    }
    reg!("sin", f64::sin);
    reg!("cos", f64::cos);
    reg!("tan", f64::tan);
    reg!("asin", f64::asin);
    reg!("acos", f64::acos);
    reg!("atan", f64::atan);
    reg!("sinh", f64::sinh);
    reg!("cosh", f64::cosh);
    reg!("tanh", f64::tanh);
    reg!("asinh", f64::asinh);
    reg!("acosh", f64::acosh);
    reg!("atanh", f64::atanh);
    reg!("sec", |x: f64| 1.0 / x.cos());
    reg!("csc", |x: f64| 1.0 / x.sin());
    reg!("cot", |x: f64| 1.0 / x.tan());
    // Hyperbolic reciprocals and their inverses.
    reg!("sech", |x: f64| 1.0 / x.cosh());
    reg!("csch", |x: f64| 1.0 / x.sinh());
    reg!("coth", |x: f64| 1.0 / x.tanh());
    reg!("asech", |x: f64| ((1.0 / x)
        + ((1.0 / (x * x) - 1.0).sqrt()))
    .ln());
    reg!("acsch", |x: f64| ((1.0 / x) + (1.0 / (x * x) + 1.0).sqrt())
        .ln());
    reg!("acoth", |x: f64| 0.5 * ((x + 1.0) / (x - 1.0)).ln());
    // Inverse reciprocal trig.
    reg!("acot", |x: f64| (1.0 / x).atan());
    reg!("asec", |x: f64| (1.0 / x).acos());
    reg!("acsc", |x: f64| (1.0 / x).asin());
    // Degree-valued trig (constants inlined: the builtin signature is a plain
    // `fn` pointer, so the closures must not capture).
    const D2R: f64 = std::f64::consts::PI / 180.0;
    const R2D: f64 = 180.0 / std::f64::consts::PI;
    reg!("sind", |x: f64| (x * D2R).sin());
    reg!("cosd", |x: f64| (x * D2R).cos());
    reg!("tand", |x: f64| (x * D2R).tan());
    reg!("secd", |x: f64| 1.0 / (x * D2R).cos());
    reg!("cscd", |x: f64| 1.0 / (x * D2R).sin());
    reg!("cotd", |x: f64| 1.0 / (x * D2R).tan());
    reg!("asind", |x: f64| x.asin() * R2D);
    reg!("acosd", |x: f64| x.acos() * R2D);
    reg!("atand", |x: f64| x.atan() * R2D);
    reg!("acotd", |x: f64| (1.0 / x).atan() * R2D);
    reg!("asecd", |x: f64| (1.0 / x).acos() * R2D);
    reg!("acscd", |x: f64| (1.0 / x).asin() * R2D);
    table.add_builtin("atan2", b_atan2);
}

fn simple(args: &[Array], name: &str, f: impl Fn(f64) -> f64) -> Flow<Vec<Array>> {
    need(args, 1, name)?;
    Ok(vec![map_double(&args[0], f)])
}

fn b_atan2(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "atan2")?;
    let y = to_f64_vec(&args[0]);
    let x = to_f64_vec(&args[1]);
    let (n, dims) = if y.len() >= x.len() {
        (y.len(), args[0].dims())
    } else {
        (x.len(), args[1].dims())
    };
    let data: Vec<f64> = (0..n)
        .map(|i| {
            let yi = if y.len() == 1 { y[0] } else { y[i] };
            let xi = if x.len() == 1 { x[0] } else { x[i] };
            yi.atan2(xi)
        })
        .collect();
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}
