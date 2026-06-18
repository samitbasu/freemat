//! Array constructors: `zeros`, `ones`, `eye`, `linspace`, `logspace`,
//! `repmat`, `diag`, `colon`.

use fm_core::{Array, DataClass};
use fm_interp::error::Flow;
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};

use crate::util::{err, err_signal, need, scalar_arg};

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("zeros", |_i, a, _n| Ok(vec![filled(a, 0.0)]));
    table.add_builtin("ones", |_i, a, _n| Ok(vec![filled(a, 1.0)]));
    table.add_builtin("eye", b_eye);
    table.add_builtin("linspace", b_linspace);
    table.add_builtin("logspace", b_logspace);
    table.add_builtin("repmat", b_repmat);
    table.add_builtin("diag", b_diag);
}

/// Parse `zeros`/`ones`/`eye`-style dimension arguments.
fn dims_from_args(args: &[Array]) -> Vec<usize> {
    let mut dims = if args.is_empty() {
        vec![1, 1]
    } else if args.len() == 1 {
        if args[0].numel() == 1 {
            let n = args[0].as_f64().unwrap_or(0.0).max(0.0) as usize;
            vec![n, n]
        } else {
            to_f64_vec(&args[0])
                .into_iter()
                .map(|x| x.max(0.0) as usize)
                .collect()
        }
    } else {
        args.iter()
            .map(|a| a.as_f64().unwrap_or(0.0).max(0.0) as usize)
            .collect()
    };
    // Drop trailing singleton dimensions beyond rank 2 (MATLAB keeps ≥2 dims):
    // `ones(2,2,1)` is a 2×2 matrix, not a 2×2×1 array.
    while dims.len() > 2 && *dims.last().unwrap() == 1 {
        dims.pop();
    }
    while dims.len() < 2 {
        dims.push(1);
    }
    dims
}

fn filled(args: &[Array], v: f64) -> Array {
    // A trailing string argument names the result class: `zeros(2,3,'single')`.
    let (dim_args, class) = split_class_arg(args);
    let dims = dims_from_args(dim_args);
    let n: usize = dims.iter().product();
    build_real(class, &dims, vec![v; n])
}

/// Split a trailing class-name string (`'single'`, `'int8'`, …) off the end of
/// a constructor's arguments, returning the remaining dimension args and the
/// requested class (defaulting to `double`).
fn split_class_arg(args: &[Array]) -> (&[Array], DataClass) {
    if let Some(last) = args.last()
        && let Some(s) = last.as_string()
        && let Some(class) = class_from_name(&s)
    {
        return (&args[..args.len() - 1], class);
    }
    (args, DataClass::Double)
}

/// Map a FreeMat class name to its [`DataClass`].
fn class_from_name(name: &str) -> Option<DataClass> {
    Some(match name {
        "double" => DataClass::Double,
        "single" | "float" => DataClass::Float,
        "logical" => DataClass::Bool,
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

fn b_eye(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let dims = dims_from_args(args);
    let (r, c) = match dims.as_slice() {
        [] => (1, 1),
        [n] => (*n, *n),
        [r, c, ..] => (*r, *c),
    };
    let mut data = vec![0.0; r * c];
    for i in 0..r.min(c) {
        data[i + i * r] = 1.0;
    }
    Ok(vec![build_real(DataClass::Double, &[r, c], data)])
}

fn b_linspace(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "linspace")?;
    let a = scalar_arg(args, 0, "linspace")?;
    let b = scalar_arg(args, 1, "linspace")?;
    let n = if args.len() >= 3 {
        scalar_arg(args, 2, "linspace")?.round().max(1.0) as usize
    } else {
        100
    };
    let data: Vec<f64> = if n == 1 {
        vec![b]
    } else {
        (0..n)
            .map(|i| a + (b - a) * i as f64 / (n - 1) as f64)
            .collect()
    };
    Ok(vec![build_real(DataClass::Double, &[1, n], data)])
}

fn b_logspace(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "logspace")?;
    let a = scalar_arg(args, 0, "logspace")?;
    let b = scalar_arg(args, 1, "logspace")?;
    let n = if args.len() >= 3 {
        scalar_arg(args, 2, "logspace")?.round().max(1.0) as usize
    } else {
        50
    };
    let data: Vec<f64> = if n == 1 {
        vec![10.0_f64.powf(b)]
    } else {
        (0..n)
            .map(|i| 10.0_f64.powf(a + (b - a) * i as f64 / (n - 1) as f64))
            .collect()
    };
    Ok(vec![build_real(DataClass::Double, &[1, n], data)])
}

fn b_repmat(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "repmat")?;
    let a = &args[0];
    let src_dims = a.dims();

    // Replication counts: repmat(A, k) → k×k…; repmat(A, p, q, …) per-dim;
    // repmat(A, [p q …]) per-dim from a vector.
    let reps: Vec<usize> = if args.len() >= 3 {
        (1..args.len())
            .map(|k| scalar_arg(args, k, "repmat").map(|v| v.round().max(0.0) as usize))
            .collect::<Flow<Vec<usize>>>()?
    } else if args[1].numel() >= 2 {
        to_f64_vec(&args[1])
            .iter()
            .map(|&v| v.max(0.0) as usize)
            .collect()
    } else {
        let k = scalar_arg(args, 1, "repmat")?.round().max(0.0) as usize;
        vec![k, k]
    };

    // Pad source dims and rep counts to a common rank.
    let rank = src_dims.len().max(reps.len());
    let sd: Vec<usize> = (0..rank)
        .map(|i| src_dims.get(i).copied().unwrap_or(1))
        .collect();
    let rp: Vec<usize> = (0..rank)
        .map(|i| reps.get(i).copied().unwrap_or(1))
        .collect();
    let out_dims: Vec<usize> = sd.iter().zip(&rp).map(|(&s, &r)| s * r).collect();
    let out_total: usize = out_dims.iter().product();

    let sstr = col_strides(&sd);
    let ostr = col_strides(&out_dims);
    // For each output position, map to the corresponding source position by
    // taking the per-axis output coordinate modulo the source extent.
    let mut perm = vec![0usize; out_total];
    for (out_lin, slot) in perm.iter_mut().enumerate() {
        let mut src = 0usize;
        for d in 0..rank {
            let oc = (out_lin / ostr[d]) % out_dims[d];
            let sc = if sd[d] == 0 { 0 } else { oc % sd[d] };
            src += sc * sstr[d];
        }
        *slot = src;
    }
    Ok(vec![tile_by(a, &out_dims, &perm)])
}

/// Column-major strides for `dims`.
fn col_strides(dims: &[usize]) -> Vec<usize> {
    let mut s = vec![1usize; dims.len()];
    for i in 1..dims.len() {
        s[i] = s[i - 1] * dims[i - 1];
    }
    s
}

/// Materialise a permutation `perm` (column-major source positions) over `a` as
/// a new array of shape `dims`, dispatching on the element type.
fn tile_by(a: &Array, dims: &[usize], perm: &[usize]) -> Array {
    match a {
        Array::Cell(_) => {
            let flat: Vec<Array> = crate::util::cell_mem_order(a);
            let data: Vec<Array> = perm.iter().map(|&p| flat[p].clone()).collect();
            Array::cell(dims, data)
        }
        Array::Char(_) | Array::Scalar(fm_core::ScalarValue::Char(_)) => {
            let flat: Vec<char> = a.as_string().unwrap_or_default().chars().collect();
            let data: Vec<char> = perm.iter().map(|&p| flat[p]).collect();
            fm_interp::value::char_matrix(dims, data)
        }
        _ if a.is_complex() => {
            let flat = fm_interp::value::to_c64_vec(a);
            let data = perm.iter().map(|&p| flat[p]).collect();
            fm_interp::value::build_complex(dims, data)
        }
        _ => {
            let flat = to_f64_vec(a);
            let data: Vec<f64> = perm.iter().map(|&p| flat[p]).collect();
            build_real(a.class(), dims, data)
        }
    }
}

/// `diag(v)` builds a diagonal matrix from a vector; `diag(M)` extracts the
/// main diagonal of a matrix.
fn b_diag(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "diag")?;
    let a = &args[0];
    let dims = a.dims();
    if dims.len() != 2 {
        return err("diag: first argument must be 2-D");
    }
    // The diagonal offset `k` (0 = main, >0 above, <0 below). Mirrors FreeMat's
    // `DiagFunction`: a vector input *builds* a diagonal matrix; a matrix input
    // *extracts* the k-th diagonal as a column vector.
    let k = match args.get(1) {
        None => 0i64,
        Some(arg) => arg
            .as_f64()
            .map(|v| v as i64)
            .ok_or_else(|| err_signal("diag: second argument must be a scalar"))?,
    };
    let (r, c) = (dims[0], dims[1]);
    let is_vector = r == 1 || c == 1;

    if is_vector {
        // Build an M×M matrix placing the vector on the k-th diagonal.
        let n = a.numel();
        let m = n + k.unsigned_abs() as usize;
        // Source linear position for output (row,col); usize::MAX marks a zero.
        let mut perm = vec![usize::MAX; m * m];
        for i in 0..n {
            let (row, col) = if k < 0 {
                (i + k.unsigned_abs() as usize, i)
            } else {
                (i, i + k as usize)
            };
            perm[row + col * m] = i;
        }
        Ok(vec![gather_diag(a, &[m, m], &perm)])
    } else {
        // Extract the k-th diagonal of the matrix as a column vector.
        let out_len = if k < 0 {
            (r as i64 + k).max(0).min(c as i64) as usize
        } else {
            (r as i64).min(c as i64 - k).max(0) as usize
        };
        let mut perm = vec![usize::MAX; out_len];
        for (idx, slot) in perm.iter_mut().enumerate() {
            let (row, col) = if k < 0 {
                (idx + k.unsigned_abs() as usize, idx)
            } else {
                (idx, idx + k as usize)
            };
            *slot = row + col * r;
        }
        Ok(vec![gather_diag(a, &[out_len, 1], &perm)])
    }
}

/// Gather elements of `a` into a new array of shape `dims` using the per-output
/// source linear positions in `perm` (`usize::MAX` ⇒ a zero fill). Preserves
/// the source class (real / complex / char), matching FreeMat's `diag`.
fn gather_diag(a: &Array, dims: &[usize], perm: &[usize]) -> Array {
    match a {
        Array::Char(_) | Array::Scalar(fm_core::ScalarValue::Char(_)) => {
            let flat: Vec<char> = a.as_string().unwrap_or_default().chars().collect();
            let data: Vec<char> = perm
                .iter()
                .map(|&p| if p == usize::MAX { '\u{0}' } else { flat[p] })
                .collect();
            fm_interp::value::char_matrix(dims, data)
        }
        _ if a.is_complex() => {
            let flat = fm_interp::value::to_c64_vec(a);
            let data = perm
                .iter()
                .map(|&p| {
                    if p == usize::MAX {
                        fm_core::C64::new(0.0, 0.0)
                    } else {
                        flat[p]
                    }
                })
                .collect();
            fm_interp::value::build_complex(dims, data)
        }
        _ => {
            let flat = to_f64_vec(a);
            let data: Vec<f64> = perm
                .iter()
                .map(|&p| if p == usize::MAX { 0.0 } else { flat[p] })
                .collect();
            build_real(a.class(), dims, data)
        }
    }
}
