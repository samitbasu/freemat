//! Inspection / type / string helpers: `typeof`, `float`, `complex`,
//! `dcomplex`, `feps`, `strcmp`, `strcmpi`, `iscellstr`, `issame`, plus the
//! `is*` predicates `isvector`/`isscalar`/`ismatrix`/`isrow`/`iscolumn`/
//! `iscomplex`/`isfloat`/`isinteger`/`islogical`.

use fm_core::{Array, C64, DataClass};
use fm_interp::error::Flow;
use fm_interp::value::{to_c64_vec, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};

use crate::util::need;

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("typeof", b_typeof);
    table.add_builtin("version", |_i, _a, _n| Ok(vec![Array::char_string("4.2")]));
    table.add_builtin("verstring", |_i, _a, _n| {
        Ok(vec![Array::char_string("FreeMat v4.2")])
    });
    table.add_builtin("float", |_i, a, _n| cast_single(a));
    table.add_builtin("complex", b_complex);
    table.add_builtin("dcomplex", b_dcomplex);
    table.add_builtin("feps", |_i, _a, _n| Ok(vec![Array::single(f32::EPSILON)]));
    table.add_builtin("eps", b_eps);
    table.add_builtin("realmax", |_i, _a, _n| Ok(vec![Array::double(f64::MAX)]));
    table.add_builtin("realmin", |_i, _a, _n| {
        Ok(vec![Array::double(f64::MIN_POSITIVE)])
    });
    table.add_builtin("strcmp", |_i, a, _n| strcmp(a, false));
    table.add_builtin("strcmpi", |_i, a, _n| strcmp(a, true));
    table.add_builtin("iscellstr", b_iscellstr);
    table.add_builtin("issame", b_issame);
    table.add_builtin("iscomplex", |_i, a, _n| {
        pred(a, "iscomplex", Array::is_complex)
    });
    table.add_builtin("isfloat", |_i, a, _n| {
        pred(a, "isfloat", |x| x.class().is_float())
    });
    table.add_builtin("isinteger", |_i, a, _n| {
        pred(a, "isinteger", |x| x.class().is_integer())
    });
    table.add_builtin("islogical", |_i, a, _n| {
        pred(a, "islogical", |x| x.class() == DataClass::Bool)
    });
    table.add_builtin("isvector", |_i, a, _n| pred(a, "isvector", is_vector));
    table.add_builtin("isscalar", |_i, a, _n| {
        pred(a, "isscalar", |x| x.numel() == 1)
    });
    table.add_builtin("ismatrix", |_i, a, _n| {
        pred(a, "ismatrix", |x| x.dims().len() <= 2)
    });
    table.add_builtin("isrow", |_i, a, _n| {
        pred(a, "isrow", |x| {
            let d = x.dims();
            d.len() == 2 && d[0] == 1
        })
    });
    table.add_builtin("iscolumn", |_i, a, _n| {
        pred(a, "iscolumn", |x| {
            let d = x.dims();
            d.len() == 2 && d[1] == 1
        })
    });
    table.add_builtin("isstruct", |_i, a, _n| {
        pred(a, "isstruct", |x| x.class() == DataClass::Struct)
    });
}

/// A one-argument logical predicate over an [`Array`].
fn pred(args: &[Array], name: &str, f: impl Fn(&Array) -> bool) -> Flow<Vec<Array>> {
    need(args, 1, name)?;
    Ok(vec![Array::bool(f(&args[0]))])
}

/// FreeMat's `typeof` == `Array::className()`: the storage class name. FreeMat
/// tracks complexity as a *flag*, not a class, so `typeof` of a complex array
/// is just `single`/`double` (e.g. `typeof(2.0+i)` is `'double'`, and
/// `typeof(2.0f+i)` is `'single'`), never `complex`/`dcomplex`.
fn b_typeof(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "typeof")?;
    Ok(vec![Array::char_string(args[0].class_name())])
}

/// Reference types (cell / struct / function handle) have no numeric or char
/// conversion — FreeMat raises here. Without this guard `to_f64_vec` returns an
/// empty vec while `dims` stays non-empty, so the array builders panic on the
/// shape mismatch (e.g. `single({4})` / `complex({4})` crashed the interpreter).
fn reject_reference_cast(a: &Array, name: &str) -> Flow<()> {
    if matches!(
        a.class(),
        DataClass::Cell | DataClass::Struct | DataClass::FunctionHandle
    ) {
        return crate::util::err(format!(
            "{name}: cannot convert a {} array to {name}",
            a.class_name()
        ));
    }
    Ok(())
}

fn cast_single(args: &[Array]) -> Flow<Vec<Array>> {
    need(args, 1, "float")?;
    let a = &args[0];
    reject_reference_cast(a, "single")?;
    let dims = a.dims();
    // `float`/`single` preserves complexity (yielding a complex single array).
    if a.is_complex() {
        let data = fm_interp::value::to_c64_vec(a);
        return Ok(vec![fm_interp::value::build_complex_class(
            DataClass::Float,
            &dims,
            data,
        )]);
    }
    let data = to_f64_vec(a);
    Ok(vec![fm_interp::value::build_real(
        DataClass::Float,
        &dims,
        data,
    )])
}

/// `complex(re, im)` — build a complex `single` array (FreeMat: `complex`).
fn b_complex(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    make_complex(args, true)
}

/// `dcomplex(re, im)` — build a complex `double` array.
fn b_dcomplex(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    make_complex(args, false)
}

fn make_complex(args: &[Array], single: bool) -> Flow<Vec<Array>> {
    need(args, 1, "complex")?;
    for a in args {
        reject_reference_cast(a, if single { "complex" } else { "dcomplex" })?;
    }
    let class = if single {
        DataClass::Float
    } else {
        DataClass::Double
    };
    // Single-argument form: cast to the target complex width, *preserving* the
    // input's imaginary part (FreeMat: `complex(x)` returns `x`; `dcomplex(x)`
    // is `x.toClass(Double)`). The general real+imag build path below would drop
    // the imaginary component of an already-complex input.
    if args.len() == 1 {
        let data = fm_interp::value::to_c64_vec(&args[0]);
        let dims = args[0].dims();
        let n: usize = dims.iter().product();
        if data.iter().all(|c| c.im == 0.0) && n == 1 {
            // Keep a complex scalar (FreeMat's `complex`/`dcomplex` always yield
            // a complex array even when the imaginary part is zero).
            return Ok(vec![if single {
                Array::Scalar(fm_core::ScalarValue::Complex32(fm_core::C32::new(
                    data[0].re as f32,
                    0.0,
                )))
            } else {
                Array::complex64(data[0])
            }]);
        }
        return Ok(vec![fm_interp::value::build_complex_class(
            class, &dims, data,
        )]);
    }
    let re = to_f64_vec(&args[0]);
    let im = if args.len() >= 2 {
        to_f64_vec(&args[1])
    } else {
        vec![0.0; re.len()]
    };
    let n = re.len().max(im.len());
    let dims = if re.len() >= im.len() {
        args[0].dims()
    } else {
        args[1].dims()
    };
    let data: Vec<C64> = (0..n)
        .map(|i| {
            let r = if re.len() == 1 { re[0] } else { re[i] };
            let m = if im.len() == 1 { im[0] } else { im[i] };
            C64::new(r, m)
        })
        .collect();
    // Force a complex result even when the imaginary part is zero (FreeMat's
    // `complex` always yields a complex array), so re-narrow only if it matters.
    if data.iter().all(|c| c.im == 0.0) && n == 1 {
        return Ok(vec![if single {
            Array::Scalar(fm_core::ScalarValue::Complex32(fm_core::C32::new(
                data[0].re as f32,
                0.0,
            )))
        } else {
            Array::complex64(data[0])
        }]);
    }
    Ok(vec![fm_interp::value::build_complex_class(
        class, &dims, data,
    )])
}

/// `strcmp(a, b)` — string equality (case-insensitive for `strcmpi`).
fn strcmp(args: &[Array], ci: bool) -> Flow<Vec<Array>> {
    need(args, 2, "strcmp")?;
    let a = args[0].as_string();
    let b = args[1].as_string();
    let eq = match (a, b) {
        (Some(x), Some(y)) => {
            if ci {
                x.to_lowercase() == y.to_lowercase()
            } else {
                x == y
            }
        }
        _ => false,
    };
    Ok(vec![Array::bool(eq)])
}

fn b_iscellstr(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "iscellstr")?;
    let ok = match args[0].as_cell() {
        Some(cells) => cells.iter().all(Array::is_char),
        None => false,
    };
    Ok(vec![Array::bool(ok)])
}

/// `issame(a, b)` — true iff same class, size, and (numeric) values, or equal
/// strings. Used widely by the conformance corpus.
fn b_issame(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "issame")?;
    let a = &args[0];
    let b = &args[1];
    if a.class() != b.class() || a.dims() != b.dims() {
        return Ok(vec![Array::bool(false)]);
    }
    if a.is_char() {
        return Ok(vec![Array::bool(a.as_string() == b.as_string())]);
    }
    if a.is_complex() || b.is_complex() {
        let x = to_c64_vec(a);
        let y = to_c64_vec(b);
        return Ok(vec![Array::bool(x == y)]);
    }
    let x = to_f64_vec(a);
    let y = to_f64_vec(b);
    Ok(vec![Array::bool(x == y)])
}

fn is_vector(a: &Array) -> bool {
    let d = a.dims();
    d.len() == 2 && (d[0] == 1 || d[1] == 1)
}

/// `eps` / `eps(x)` / `eps('single'|'double')` — floating-point spacing.
///
/// With no arguments returns `eps(1.0)` for double. With a string class name
/// returns the machine epsilon for that class. With a numeric argument returns
/// the spacing of floating-point numbers near `abs(x)` (`nextafter`-style),
/// honoring single vs double precision.
fn b_eps(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return Ok(vec![Array::double(f64::EPSILON)]);
    }
    if let Some(s) = args[0].as_string() {
        return match s.as_str() {
            "single" => Ok(vec![Array::single(f32::EPSILON)]),
            _ => Ok(vec![Array::double(f64::EPSILON)]),
        };
    }
    let single = matches!(args[0].class(), DataClass::Float);
    let x = args[0].as_f64().unwrap_or(1.0).abs();
    if single {
        let xf = x as f32;
        let spacing = if xf == 0.0 {
            f32::from_bits(1)
        } else {
            let next = f32::from_bits(xf.to_bits() + 1);
            next - xf
        };
        Ok(vec![Array::single(spacing)])
    } else {
        let spacing = if x == 0.0 {
            f64::from_bits(1)
        } else {
            let next = f64::from_bits(x.to_bits() + 1);
            next - x
        };
        Ok(vec![Array::double(spacing)])
    }
}
