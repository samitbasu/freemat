//! The minimal builtin set needed to run Stage 3 end-to-end tests.
//!
//! Registration mirrors FreeMat's `addFunction` (plain builtins) and
//! `addSpecialFunction` (builtins that need the interpreter): every builtin here
//! receives `&mut Interpreter`, so the same signature covers both. The bulk of
//! the libCore builtins land in Stages 5/6 — this is deliberately small:
//! `disp`, `display`, `error`, `size`, `numel`, `length`, `ndims`, `isempty`,
//! `prod`, `sum`, `class`, `zeros`, `ones`, `true`, `false`, `isa`, `mod`,
//! `rem`, `abs`, `floor`, `num2str`.

use fm_core::{Array, DataClass};

use crate::error::{Flow, InterpError, Signal};
use crate::function::FunctionTable;
use crate::interp::Interpreter;

/// Register every default builtin into `table`.
pub fn register_defaults(table: &mut FunctionTable) {
    table.add_builtin("disp", b_disp);
    table.add_builtin("display", b_disp);
    table.add_builtin("error", b_error);
    table.add_builtin("size", b_size);
    table.add_builtin("numel", b_numel);
    table.add_builtin("length", b_length);
    table.add_builtin("ndims", b_ndims);
    table.add_builtin("isempty", b_isempty);
    table.add_builtin("prod", b_prod);
    table.add_builtin("sum", b_sum);
    table.add_builtin("class", b_class);
    table.add_builtin("zeros", b_zeros);
    table.add_builtin("ones", b_ones);
    table.add_builtin("isa", b_isa);
    table.add_builtin("ischar", b_ischar);
    table.add_builtin("isnumeric", b_isnumeric);
    table.add_builtin("iscell", b_iscell);
    table.add_builtin("isreal", b_isreal);
    table.add_builtin("mod", b_mod);
    table.add_builtin("rem", b_rem);
    table.add_builtin("abs", b_abs);
    table.add_builtin("floor", b_floor);
    table.add_builtin("ceil", b_ceil);
    table.add_builtin("round", b_round);
    table.add_builtin("num2str", b_num2str);
    // Minimal type-cast builtins (enough for promotion tests; full set Stage 5).
    table.add_builtin("double", |_i, a, _n| cast(a, DataClass::Double, "double"));
    table.add_builtin("single", |_i, a, _n| cast(a, DataClass::Float, "single"));
    table.add_builtin("logical", |_i, a, _n| cast(a, DataClass::Bool, "logical"));
    table.add_builtin("char", |_i, a, _n| b_char(a));
    table.add_builtin("string", |_i, a, _n| cast(a, DataClass::Char, "string"));
    table.add_builtin("cast", b_cast);
    table.add_builtin("int8", |_i, a, _n| cast(a, DataClass::Int8, "int8"));
    table.add_builtin("uint8", |_i, a, _n| cast(a, DataClass::UInt8, "uint8"));
    table.add_builtin("int16", |_i, a, _n| cast(a, DataClass::Int16, "int16"));
    table.add_builtin("uint16", |_i, a, _n| cast(a, DataClass::UInt16, "uint16"));
    table.add_builtin("int32", |_i, a, _n| cast(a, DataClass::Int32, "int32"));
    table.add_builtin("uint32", |_i, a, _n| cast(a, DataClass::UInt32, "uint32"));
    table.add_builtin("int64", |_i, a, _n| cast(a, DataClass::Int64, "int64"));
    table.add_builtin("uint64", |_i, a, _n| cast(a, DataClass::UInt64, "uint64"));
}

/// `char(...)` — convert to a char array. Three forms beyond a plain cast:
///   `char(X)`                       — numeric/char cast (delegates to `cast`)
///   `char({'a','bb',...})`          — cellstr → padded char matrix
///   `char('a','bb', ...)`           — multiple strings → padded char matrix
fn b_char(args: &[Array]) -> Flow<Vec<Array>> {
    need(args, 1, "char")?;
    // Collect the row strings if this is a cellstr or a multi-string call.
    let rows: Option<Vec<String>> = if args.len() == 1 {
        if let Some(cells) = args[0].as_cell() {
            let strs: Vec<String> = cells.iter().filter_map(Array::as_string).collect();
            if strs.len() == cells.len() {
                Some(strs)
            } else {
                return Err(Signal::Error(InterpError::msg(
                    "char: cell array must contain only strings",
                )));
            }
        } else {
            None
        }
    } else {
        // Multiple arguments: each must be a string (row).
        let strs: Vec<String> = args.iter().filter_map(Array::as_string).collect();
        if strs.len() == args.len() {
            Some(strs)
        } else {
            None
        }
    };

    if let Some(strs) = rows {
        if strs.is_empty() {
            return Ok(vec![Array::char_string("")]);
        }
        let nrows = strs.len();
        let chars: Vec<Vec<char>> = strs.iter().map(|s| s.chars().collect()).collect();
        let ncols = chars.iter().map(Vec::len).max().unwrap_or(0);
        // Column-major buffer: element (i,j) at index i + j*nrows. Pad with spaces.
        let mut data = vec![' '; nrows * ncols];
        for (i, row) in chars.iter().enumerate() {
            for (j, &c) in row.iter().enumerate() {
                data[i + j * nrows] = c;
            }
        }
        return Ok(vec![crate::value::char_matrix(&[nrows, ncols], data)]);
    }

    cast(args, DataClass::Char, "char")
}

/// `cast(X, 'classname')` — convert `X` to the class named by the string arg.
fn b_cast(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "cast")?;
    let name = args[1].as_string().ok_or_else(|| {
        Signal::Error(InterpError::msg(
            "cast: second argument must be a class name",
        ))
    })?;
    let class = class_from_name(&name)
        .ok_or_else(|| Signal::Error(InterpError::msg(format!("cast: unknown class '{name}'"))))?;
    cast(&args[..1], class, "cast")
}

/// Map a FreeMat class name to its [`DataClass`].
fn class_from_name(name: &str) -> Option<DataClass> {
    Some(match name {
        "double" => DataClass::Double,
        "single" | "float" => DataClass::Float,
        "logical" => DataClass::Bool,
        "char" | "string" => DataClass::Char,
        "int8" => DataClass::Int8,
        "uint8" => DataClass::UInt8,
        "int16" => DataClass::Int16,
        "uint16" => DataClass::UInt16,
        "int32" => DataClass::Int32,
        "uint32" => DataClass::UInt32,
        "int64" => DataClass::Int64,
        "uint64" => DataClass::UInt64,
        _ => return None,
    })
}

/// Cast every element of `args[0]` to `class`.
fn cast(args: &[Array], class: DataClass, name: &str) -> Flow<Vec<Array>> {
    need(args, 1, name)?;
    let a = &args[0];
    // Reference types (cell / struct / function handle) have no numeric or char
    // conversion — FreeMat raises here. Without this guard `to_f64_vec` returns
    // an empty vec while `dims` stays non-empty, so `build_real`/`build_integer`
    // panic on the shape mismatch (e.g. `int8({4})` crashed the interpreter).
    if matches!(
        a.class(),
        DataClass::Cell | DataClass::Struct | DataClass::FunctionHandle
    ) {
        return Err(Signal::Error(InterpError::msg(format!(
            "{name}: cannot convert a {} array to {name}",
            a.class_name()
        ))));
    }
    let dims = a.dims();
    // `double`/`single` of a complex array preserve complexity (FreeMat's
    // `toClass(Double/Float)` keeps the complex flag); integer/logical/char
    // casts take the real part only.
    if a.is_complex() && matches!(class, DataClass::Double | DataClass::Float) {
        let data = crate::value::to_c64_vec(a);
        return Ok(vec![crate::value::build_complex_class(class, &dims, data)]);
    }
    let data = crate::value::to_f64_vec(a);
    Ok(vec![crate::value::build_real(class, &dims, data)])
}

fn need(args: &[Array], n: usize, name: &str) -> Flow<()> {
    if args.len() < n {
        return Err(Signal::Error(InterpError::msg(format!(
            "{name}: expected at least {n} argument(s), got {}",
            args.len()
        ))));
    }
    Ok(())
}

fn b_disp(interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "disp")?;
    let body = args[0].format(interp.format);
    interp.emit(&body);
    interp.emit("\n");
    Ok(vec![])
}

fn b_error(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "error")?;
    let first = args[0].as_string().unwrap_or_default();
    // `error('id:comp', msg)` form: first arg is an identifier if it has a colon
    // and there is a second arg.
    if args.len() >= 2 && first.contains(':') && !first.contains(' ') {
        let msg = args[1].as_string().unwrap_or_default();
        return Err(Signal::Error(InterpError::with_id(first, msg)));
    }
    Err(Signal::Error(InterpError::msg(first)))
}

fn b_size(_interp: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "size")?;
    let dims = args[0].dims();
    if args.len() >= 2 {
        // size(x, dim)
        let d = args[1].as_f64().unwrap_or(1.0) as usize;
        let v = dims.get(d.saturating_sub(1)).copied().unwrap_or(1);
        return Ok(vec![Array::double(v as f64)]);
    }
    if nargout > 1 {
        // [r, c, ...] = size(x): spread dims across outputs, last collects rest.
        let mut out = Vec::with_capacity(nargout);
        for i in 0..nargout {
            if i + 1 == nargout {
                let rest: usize = dims.iter().skip(i).product::<usize>().max(1);
                out.push(Array::double(rest as f64));
            } else {
                out.push(Array::double(dims.get(i).copied().unwrap_or(1) as f64));
            }
        }
        return Ok(out);
    }
    let data: Vec<f64> = dims.iter().map(|&d| d as f64).collect();
    Ok(vec![Array::double_matrix(&[1, data.len()], data)])
}

fn b_numel(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "numel")?;
    Ok(vec![Array::double(args[0].numel() as f64)])
}

fn b_length(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "length")?;
    let dims = args[0].dims();
    let len = if args[0].numel() == 0 {
        0
    } else {
        dims.iter().copied().max().unwrap_or(0)
    };
    Ok(vec![Array::double(len as f64)])
}

fn b_ndims(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "ndims")?;
    Ok(vec![Array::double(args[0].dims().len().max(2) as f64)])
}

fn b_isempty(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "isempty")?;
    Ok(vec![Array::bool(args[0].numel() == 0)])
}

fn b_prod(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "prod")?;
    let v = crate::value::to_f64_vec(&args[0]);
    // For a vector, prod over all elements; for a matrix, MATLAB prods columns,
    // but the Stage 3 set only needs the vector case (used by isvector/numel).
    let dims = args[0].dims();
    if dims.len() == 2 && (dims[0] == 1 || dims[1] == 1) {
        Ok(vec![Array::double(v.iter().product())])
    } else if dims.len() == 2 {
        // Column products.
        let (r, c) = (dims[0], dims[1]);
        let mut out = Vec::with_capacity(c);
        for j in 0..c {
            let mut p = 1.0;
            for i in 0..r {
                p *= v[i + j * r];
            }
            out.push(p);
        }
        Ok(vec![Array::double_matrix(&[1, c], out)])
    } else {
        Ok(vec![Array::double(v.iter().product())])
    }
}

fn b_sum(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "sum")?;
    let v = crate::value::to_f64_vec(&args[0]);
    let dims = args[0].dims();
    if dims.len() == 2 && (dims[0] == 1 || dims[1] == 1) {
        Ok(vec![Array::double(v.iter().sum())])
    } else if dims.len() == 2 {
        let (r, c) = (dims[0], dims[1]);
        let mut out = Vec::with_capacity(c);
        for j in 0..c {
            let mut s = 0.0;
            for i in 0..r {
                s += v[i + j * r];
            }
            out.push(s);
        }
        Ok(vec![Array::double_matrix(&[1, c], out)])
    } else {
        Ok(vec![Array::double(v.iter().sum())])
    }
}

fn b_class(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "class")?;
    Ok(vec![Array::char_string(args[0].class_name())])
}

fn dims_from_args(args: &[Array]) -> Vec<usize> {
    if args.is_empty() {
        return vec![1, 1];
    }
    if args.len() == 1 {
        if args[0].numel() == 1 {
            let n = args[0].as_f64().unwrap_or(0.0).max(0.0) as usize;
            return vec![n, n];
        }
        // A size vector argument.
        return crate::value::to_f64_vec(&args[0])
            .into_iter()
            .map(|x| x.max(0.0) as usize)
            .collect();
    }
    args.iter()
        .map(|a| a.as_f64().unwrap_or(0.0).max(0.0) as usize)
        .collect()
}

fn b_zeros(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    let dims = dims_from_args(args);
    let n: usize = dims.iter().product();
    Ok(vec![Array::double_matrix(&dims, vec![0.0; n])])
}

fn b_ones(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    let dims = dims_from_args(args);
    let n: usize = dims.iter().product();
    Ok(vec![Array::double_matrix(&dims, vec![1.0; n])])
}

fn b_isa(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 2, "isa")?;
    let want = args[1].as_string().unwrap_or_default();
    let cls = args[0].class();
    let yes = match want.as_str() {
        "numeric" => cls.is_numeric(),
        "float" => cls.is_float(),
        "integer" => cls.is_integer(),
        other => cls.name() == other,
    };
    Ok(vec![Array::bool(yes)])
}

fn b_ischar(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "ischar")?;
    Ok(vec![Array::bool(args[0].is_char())])
}

fn b_isnumeric(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "isnumeric")?;
    let c = args[0].class();
    Ok(vec![Array::bool(
        c.is_numeric() && !matches!(c, DataClass::Bool),
    )])
}

fn b_iscell(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "iscell")?;
    Ok(vec![Array::bool(args[0].class() == DataClass::Cell)])
}

fn b_isreal(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "isreal")?;
    Ok(vec![Array::bool(!args[0].is_complex())])
}

/// Element-wise unary `f64` map preserving the (real) class of the input.
fn map_real(a: &Array, f: impl Fn(f64) -> f64) -> Array {
    let dims = a.dims();
    let data = crate::value::to_f64_vec(a).into_iter().map(f).collect();
    crate::value::build_real(a.class(), &dims, data)
}

fn b_abs(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "abs")?;
    if args[0].is_complex() {
        let dims = args[0].dims();
        let data = crate::value::to_c64_vec(&args[0])
            .into_iter()
            .map(|z| z.norm())
            .collect();
        return Ok(vec![crate::value::build_real(
            DataClass::Double,
            &dims,
            data,
        )]);
    }
    Ok(vec![map_real(&args[0], f64::abs)])
}

fn b_floor(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "floor")?;
    Ok(vec![map_real(&args[0], f64::floor)])
}

fn b_ceil(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "ceil")?;
    Ok(vec![map_real(&args[0], f64::ceil)])
}

fn b_round(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "round")?;
    Ok(vec![map_real(&args[0], f64::round)])
}

fn b_mod(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 2, "mod")?;
    let a = args[0].as_f64().unwrap_or(0.0);
    let n = args[1].as_f64().unwrap_or(0.0);
    let r = if n == 0.0 { a } else { a - n * (a / n).floor() };
    Ok(vec![Array::double(r)])
}

fn b_rem(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 2, "rem")?;
    let a = args[0].as_f64().unwrap_or(0.0);
    let n = args[1].as_f64().unwrap_or(0.0);
    let r = if n == 0.0 {
        f64::NAN
    } else {
        a - n * (a / n).trunc()
    };
    Ok(vec![Array::double(r)])
}

fn b_num2str(_interp: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "num2str")?;
    let s = if let Some(v) = args[0].as_f64() {
        if v.fract() == 0.0 && v.is_finite() {
            format!("{v}")
        } else {
            format!("{v:.4}")
        }
    } else {
        args[0].format(fm_core::FormatMode::Short)
    };
    Ok(vec![Array::char_string(&s)])
}
