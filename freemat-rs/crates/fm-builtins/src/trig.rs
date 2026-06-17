//! Trigonometric and hyperbolic builtins. Real inputs use the `f64` path;
//! `atan2` is a two-argument element-wise builtin.

use fm_core::{Array, DataClass};
use fm_interp::error::Flow;
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};

use crate::util::{map_double, need};

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
