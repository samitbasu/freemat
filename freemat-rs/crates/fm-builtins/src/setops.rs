//! Set operations: `union`, `intersect`, `setdiff`, `ismember`.
//! (`unique` lives in `array_manip`.)

use fm_core::{Array, DataClass};
use fm_interp::error::Flow;
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};

use crate::util::need;

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("union", b_union);
    table.add_builtin("intersect", b_intersect);
    table.add_builtin("setdiff", b_setdiff);
    table.add_builtin("ismember", b_ismember);
}

/// Whether both args are cell-arrays of strings (the string set-op path).
fn both_cellstr(a: &Array, b: &Array) -> bool {
    matches!((a.as_cell(), b.as_cell()), (Some(_), Some(_)))
}

fn cell_strings(a: &Array) -> Vec<String> {
    a.as_cell()
        .map(|c| c.iter().filter_map(Array::as_string).collect())
        .unwrap_or_default()
}

fn sorted_unique(mut v: Vec<f64>) -> Vec<f64> {
    v.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
    v.dedup_by(|x, y| x == y);
    v
}

fn row_like(a: &Array, b: &Array) -> bool {
    let da = a.dims();
    let db = b.dims();
    (da.len() == 2 && da[0] == 1) && (db.len() == 2 && db[0] == 1)
}

fn make_vec(v: Vec<f64>, row: bool) -> Array {
    let n = v.len();
    let dims = if row { vec![1, n] } else { vec![n, 1] };
    build_real(DataClass::Double, &dims, v)
}

fn make_cellvec(v: Vec<String>) -> Array {
    let n = v.len();
    let data: Vec<Array> = v.into_iter().map(|s| Array::char_string(&s)).collect();
    Array::cell(&[n, 1], data)
}

fn b_union(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "union")?;
    if both_cellstr(&args[0], &args[1]) {
        let mut s = cell_strings(&args[0]);
        s.extend(cell_strings(&args[1]));
        s.sort();
        s.dedup();
        return Ok(vec![make_cellvec(s)]);
    }
    let mut v = to_f64_vec(&args[0]);
    v.extend(to_f64_vec(&args[1]));
    Ok(vec![make_vec(
        sorted_unique(v),
        row_like(&args[0], &args[1]),
    )])
}

fn b_intersect(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "intersect")?;
    if both_cellstr(&args[0], &args[1]) {
        let b: std::collections::BTreeSet<String> = cell_strings(&args[1]).into_iter().collect();
        let mut s: Vec<String> = cell_strings(&args[0])
            .into_iter()
            .filter(|x| b.contains(x))
            .collect();
        s.sort();
        s.dedup();
        return Ok(vec![make_cellvec(s)]);
    }
    let b = sorted_unique(to_f64_vec(&args[1]));
    let v: Vec<f64> = sorted_unique(to_f64_vec(&args[0]))
        .into_iter()
        .filter(|x| b.binary_search_by(|p| p.partial_cmp(x).unwrap()).is_ok())
        .collect();
    Ok(vec![make_vec(v, row_like(&args[0], &args[1]))])
}

fn b_setdiff(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "setdiff")?;
    if both_cellstr(&args[0], &args[1]) {
        let b: std::collections::BTreeSet<String> = cell_strings(&args[1]).into_iter().collect();
        let mut s: Vec<String> = cell_strings(&args[0])
            .into_iter()
            .filter(|x| !b.contains(x))
            .collect();
        s.sort();
        s.dedup();
        return Ok(vec![make_cellvec(s)]);
    }
    let b = sorted_unique(to_f64_vec(&args[1]));
    let v: Vec<f64> = sorted_unique(to_f64_vec(&args[0]))
        .into_iter()
        .filter(|x| b.binary_search_by(|p| p.partial_cmp(x).unwrap()).is_err())
        .collect();
    Ok(vec![make_vec(v, row_like(&args[0], &args[1]))])
}

/// `ismember(a, s)` — logical mask of which elements of `a` are in `s`, with an
/// optional second output giving the (1-based) location in `s` (or 0).
fn b_ismember(_i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 2, "ismember")?;
    // Cell-of-strings membership, including a scalar string in a cellstr set.
    if args[1].as_cell().is_some() {
        let set: Vec<String> = cell_strings(&args[1]);
        let probe: Vec<String> = if let Some(c) = args[0].as_cell() {
            c.iter().filter_map(Array::as_string).collect()
        } else {
            vec![args[0].as_string().unwrap_or_default()]
        };
        let mask: Vec<bool> = probe.iter().map(|p| set.iter().any(|s| s == p)).collect();
        let loc: Vec<f64> = probe
            .iter()
            .map(|p| {
                set.iter()
                    .position(|s| s == p)
                    .map_or(0.0, |i| (i + 1) as f64)
            })
            .collect();
        let dims = args[0].dims();
        let out = if mask.len() == 1 {
            Array::bool(mask[0])
        } else {
            Array::bool_matrix(&dims, mask)
        };
        let mut res = vec![out];
        if nargout >= 2 {
            res.push(build_real(DataClass::Double, &dims, loc));
        }
        return Ok(res);
    }

    let set = to_f64_vec(&args[1]);
    let probe = to_f64_vec(&args[0]);
    let mask: Vec<bool> = probe.iter().map(|x| set.contains(x)).collect();
    let loc: Vec<f64> = probe
        .iter()
        .map(|x| {
            set.iter()
                .position(|s| s == x)
                .map_or(0.0, |i| (i + 1) as f64)
        })
        .collect();
    let dims = args[0].dims();
    let out = if mask.len() == 1 {
        Array::bool(mask[0])
    } else {
        Array::bool_matrix(&dims, mask)
    };
    let mut res = vec![out];
    if nargout >= 2 {
        res.push(build_real(DataClass::Double, &dims, loc));
    }
    Ok(res)
}
