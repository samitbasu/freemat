//! Array-manipulation builtins: `reshape`, `sort`, `unique`, `permute`,
//! `ipermute`, `squeeze`, `fliplr`/`flipud`/`flip`/`rot90`, `circshift`,
//! `sub2ind`/`ind2sub`, `cumprod`, `cat`/`horzcat`/`vertcat`.

use fm_core::{Array, DataClass};
use fm_interp::error::Flow;
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};

use crate::util::{err, need};

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("reshape", b_reshape);
    table.add_builtin("sort", b_sort);
    table.add_builtin("unique", b_unique);
    table.add_builtin("permute", |_i, a, _n| permute(a, false));
    table.add_builtin("ipermute", |_i, a, _n| permute(a, true));
    table.add_builtin("squeeze", b_squeeze);
    table.add_builtin("fliplr", |_i, a, _n| flip(a, 1));
    table.add_builtin("flipud", |_i, a, _n| flip(a, 0));
    table.add_builtin("flip", b_flip);
    table.add_builtin("rot90", b_rot90);
    table.add_builtin("circshift", b_circshift);
    table.add_builtin("sub2ind", b_sub2ind);
    table.add_builtin("ind2sub", b_ind2sub);
    table.add_builtin("horzcat", b_horzcat);
    table.add_builtin("vertcat", b_vertcat);
    table.add_builtin("cat", b_cat);
}

/// Reorder a value's column-major elements according to `perm` (a list of
/// 0-based source positions) producing a new array of shape `dims`.
fn permute_by(a: &Array, dims: &[usize], perm: &[usize]) -> Array {
    match a {
        Array::Char(d) => {
            let flat = pick_chars(d);
            let data: Vec<char> = perm.iter().map(|&p| flat[p]).collect();
            fm_interp::value::char_matrix(dims, data)
        }
        Array::Cell(d) => {
            let flat: Vec<Array> = mem_cell(d);
            let data: Vec<Array> = perm.iter().map(|&p| flat[p].clone()).collect();
            Array::cell(dims, data)
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

fn pick_chars(d: &ndarray::ArrayD<char>) -> Vec<char> {
    if let Some(s) = d.as_slice_memory_order() {
        s.to_vec()
    } else {
        d.t().iter().copied().collect()
    }
}

fn mem_cell(d: &ndarray::ArrayD<Array>) -> Vec<Array> {
    if let Some(s) = d.as_slice_memory_order() {
        s.to_vec()
    } else {
        d.t().iter().cloned().collect()
    }
}

fn b_reshape(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "reshape")?;
    let a = &args[0];
    // Target dims: reshape(A, [m n ...]) or reshape(A, m, n, ...). A single `[]`
    // placeholder dimension is inferred from the element count.
    let mut infer: Option<usize> = None;
    let mut dims: Vec<usize> = if args.len() == 2 && args[1].numel() > 1 {
        to_f64_vec(&args[1])
            .into_iter()
            .map(|x| x.max(0.0) as usize)
            .collect()
    } else {
        args[1..]
            .iter()
            .enumerate()
            .map(|(k, x)| {
                if x.numel() == 0 {
                    infer = Some(k);
                    1 // placeholder, filled in below
                } else {
                    x.as_f64().unwrap_or(0.0).max(0.0) as usize
                }
            })
            .collect()
    };
    while dims.len() < 2 {
        dims.push(1);
    }
    if let Some(k) = infer {
        let known: usize = dims
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != k)
            .map(|(_, &d)| d)
            .product();
        dims[k] = a.numel().checked_div(known).unwrap_or(0);
    }
    let want: usize = dims.iter().product();
    if want != a.numel() {
        return err(format!(
            "reshape: cannot reshape {} elements into a [{}] array",
            a.numel(),
            dims.iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join("x")
        ));
    }
    // Reshape preserves column-major element order: the permutation is identity.
    let perm: Vec<usize> = (0..want).collect();
    Ok(vec![permute_by(a, &dims, &perm)])
}

/// `sort` — ascending stable sort. Supports `[s, idx] = sort(x)` and the
/// `'descend'`/`'ascend'` mode string. Operates along the first non-singleton
/// dimension (columns for a matrix, the single axis for a vector).
fn b_sort(_i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "sort")?;
    let a = &args[0];
    let descend = args
        .iter()
        .skip(1)
        .filter_map(Array::as_string)
        .any(|s| s.eq_ignore_ascii_case("descend"));
    let dims = a.dims();
    let (rows, cols, by_col) = if dims.len() == 2 && dims[0] == 1 {
        // Row vector: sort the single row.
        (1usize, dims[1], false)
    } else if dims.len() == 2 {
        (dims[0], dims[1], true)
    } else {
        (a.numel(), 1, false)
    };

    let data = to_f64_vec(a);
    let n = data.len();
    let mut sorted = vec![0.0f64; n];
    let mut indices = vec![0.0f64; n];

    if by_col {
        for j in 0..cols {
            let mut col: Vec<(usize, f64)> = (0..rows).map(|i| (i, data[i + j * rows])).collect();
            sort_pairs(&mut col, descend);
            for (k, (orig, val)) in col.into_iter().enumerate() {
                sorted[k + j * rows] = val;
                indices[k + j * rows] = (orig + 1) as f64;
            }
        }
    } else {
        let mut v: Vec<(usize, f64)> = data.iter().copied().enumerate().collect();
        sort_pairs(&mut v, descend);
        for (k, (orig, val)) in v.into_iter().enumerate() {
            sorted[k] = val;
            indices[k] = (orig + 1) as f64;
        }
    }

    let s = build_real(a.class(), &dims, sorted);
    let mut out = vec![s];
    if nargout >= 2 {
        out.push(build_real(DataClass::Double, &dims, indices));
    }
    Ok(out)
}

/// Stable sort of `(index, value)` pairs, NaNs last (MATLAB convention).
fn sort_pairs(v: &mut [(usize, f64)], descend: bool) {
    v.sort_by(|a, b| {
        let (x, y) = (a.1, b.1);
        let ord = match (x.is_nan(), y.is_nan()) {
            (true, true) => std::cmp::Ordering::Equal,
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            (false, false) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
        };
        if descend { ord.reverse() } else { ord }
    });
}

/// `unique` — the sorted set of distinct values, as a column vector (or a row
/// vector for a row-vector input). Cell-of-strings input returns a sorted cell.
fn b_unique(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "unique")?;
    let a = &args[0];
    if let Some(cells) = a.as_cell() {
        // Unique strings.
        let mut strs: Vec<String> = cells
            .iter()
            .filter_map(Array::as_string)
            .collect::<Vec<_>>();
        strs.sort();
        strs.dedup();
        let n = strs.len();
        let data: Vec<Array> = strs.into_iter().map(|s| Array::char_string(&s)).collect();
        return Ok(vec![Array::cell(&[n, 1], data)]);
    }
    let mut v = to_f64_vec(a);
    v.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
    v.dedup_by(|x, y| x == y);
    let dims = a.dims();
    let n = v.len();
    let out_dims = if dims.len() == 2 && dims[0] == 1 {
        vec![1, n]
    } else {
        vec![n, 1]
    };
    Ok(vec![build_real(a.class(), &out_dims, v)])
}

/// `permute(A, order)` — rearrange dimensions; `ipermute` inverts `order`.
fn permute(args: &[Array], inverse: bool) -> Flow<Vec<Array>> {
    need(args, 2, "permute")?;
    let a = &args[0];
    let order: Vec<usize> = to_f64_vec(&args[1])
        .into_iter()
        .map(|x| (x as usize).saturating_sub(1))
        .collect();
    let mut dims = a.dims();
    while dims.len() < order.len() {
        dims.push(1);
    }
    let order = if inverse {
        let mut inv = vec![0usize; order.len()];
        for (i, &o) in order.iter().enumerate() {
            inv[o] = i;
        }
        inv
    } else {
        order
    };
    if order.len() < dims.len() {
        return err("permute: ORDER must have at least NDIMS(A) elements");
    }
    let new_dims: Vec<usize> = order
        .iter()
        .map(|&o| dims.get(o).copied().unwrap_or(1))
        .collect();

    // Strides of the source (column-major).
    let src_strides = strides(&dims);
    let total: usize = new_dims.iter().product();
    let out_strides = strides(&new_dims);
    let mut perm = vec![0usize; total];
    for (lin, slot) in perm.iter_mut().enumerate() {
        // Decompose `lin` in the result coordinate space.
        let mut rem = lin;
        let mut src_pos = 0usize;
        for d in (0..new_dims.len()).rev() {
            let coord = rem / out_strides[d];
            rem %= out_strides[d];
            // result axis d came from source axis order[d].
            src_pos += coord * src_strides[order[d]];
        }
        *slot = src_pos;
    }
    Ok(vec![permute_by(a, &squeeze_trailing(new_dims), &perm)])
}

fn strides(dims: &[usize]) -> Vec<usize> {
    let mut s = vec![1usize; dims.len().max(1)];
    for i in 1..dims.len() {
        s[i] = s[i - 1] * dims[i - 1];
    }
    s
}

fn squeeze_trailing(mut d: Vec<usize>) -> Vec<usize> {
    while d.len() > 2 && *d.last().unwrap() == 1 {
        d.pop();
    }
    while d.len() < 2 {
        d.push(1);
    }
    d
}

/// `squeeze` — drop singleton dimensions (keeping at least 2-D).
fn b_squeeze(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "squeeze")?;
    let a = &args[0];
    let dims = a.dims();
    let mut kept: Vec<usize> = dims.iter().copied().filter(|&d| d != 1).collect();
    while kept.len() < 2 {
        kept.push(1);
    }
    let perm: Vec<usize> = (0..a.numel()).collect();
    Ok(vec![permute_by(a, &kept, &perm)])
}

/// Flip along `axis` (0 = rows/up-down, 1 = cols/left-right) for a 2-D array.
fn flip(args: &[Array], axis: usize) -> Flow<Vec<Array>> {
    need(args, 1, "flip")?;
    let a = &args[0];
    let mut dims = a.dims();
    while dims.len() < 2 {
        dims.push(1);
    }
    let (r, c) = (dims[0], dims[1]);
    let mut perm = Vec::with_capacity(r * c);
    for j in 0..c {
        for i in 0..r {
            let (si, sj) = if axis == 0 {
                (r - 1 - i, j)
            } else {
                (i, c - 1 - j)
            };
            perm.push(si + sj * r);
        }
    }
    Ok(vec![permute_by(a, &dims, &perm)])
}

fn b_flip(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "flip")?;
    let dims = args[0].dims();
    // Default dim = first non-singleton.
    let axis = if args.len() >= 2 {
        (args[1].as_f64().unwrap_or(1.0) as usize).saturating_sub(1)
    } else {
        dims.iter().position(|&d| d != 1).unwrap_or(0)
    };
    flip(args, axis.min(1))
}

/// `rot90(A)` / `rot90(A, k)` — rotate a 2-D array 90° counter-clockwise `k`
/// times.
fn b_rot90(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "rot90")?;
    let k = if args.len() >= 2 {
        args[1].as_f64().unwrap_or(1.0) as i64
    } else {
        1
    };
    let k = k.rem_euclid(4);
    let mut a = args[0].clone();
    for _ in 0..k {
        a = rot90_once(&a);
    }
    Ok(vec![a])
}

/// One counter-clockwise 90° rotation of a 2-D array.
fn rot90_once(a: &Array) -> Array {
    let mut dims = a.dims();
    while dims.len() < 2 {
        dims.push(1);
    }
    let (r, c) = (dims[0], dims[1]);
    // result[i', j'] where result is [c, r]; rotated: result(i,j) = A(j, c-1-i).
    let nr = c;
    let nc = r;
    let mut perm = Vec::with_capacity(nr * nc);
    for j in 0..nc {
        for i in 0..nr {
            // source coords: row = j, col = c-1-i
            let sr = j;
            let sc = c - 1 - i;
            perm.push(sr + sc * r);
        }
    }
    permute_by(a, &[nr, nc], &perm)
}

/// `circshift(A, k)` — circularly shift elements. Scalar `k` shifts the first
/// non-singleton dimension; a 2-vector `[p q]` shifts rows then columns.
fn b_circshift(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "circshift")?;
    let a = &args[0];
    let mut dims = a.dims();
    while dims.len() < 2 {
        dims.push(1);
    }
    let (r, c) = (dims[0], dims[1]);
    let shifts = to_f64_vec(&args[1]);
    let (sr, sc) = if shifts.len() >= 2 {
        (shifts[0] as i64, shifts[1] as i64)
    } else {
        // Shift the first non-singleton dimension.
        let s = shifts.first().copied().unwrap_or(0.0) as i64;
        if dims[0] == 1 { (0, s) } else { (s, 0) }
    };
    let wrap = |idx: i64, len: usize| -> usize {
        if len == 0 {
            0
        } else {
            ((idx).rem_euclid(len as i64)) as usize
        }
    };
    let mut perm = Vec::with_capacity(r * c);
    for j in 0..c {
        for i in 0..r {
            let si = wrap(i as i64 - sr, r);
            let sj = wrap(j as i64 - sc, c);
            perm.push(si + sj * r);
        }
    }
    Ok(vec![permute_by(a, &dims, &perm)])
}

/// `sub2ind(sz, i, j, ...)` — convert subscripts to linear indices (1-based).
fn b_sub2ind(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "sub2ind")?;
    let sz: Vec<usize> = to_f64_vec(&args[0])
        .into_iter()
        .map(|x| x as usize)
        .collect();
    let str = strides(&sz);
    let subs: Vec<Vec<f64>> = args[1..].iter().map(to_f64_vec).collect();
    let n = subs.iter().map(Vec::len).max().unwrap_or(0);
    let out: Vec<f64> = (0..n)
        .map(|k| {
            let mut lin = 0usize;
            for (d, sub) in subs.iter().enumerate() {
                let v = if sub.len() == 1 { sub[0] } else { sub[k] } as usize;
                lin += (v - 1) * str.get(d).copied().unwrap_or(1);
            }
            (lin + 1) as f64
        })
        .collect();
    let dims = if args.len() > 2 {
        args[1].dims()
    } else {
        vec![n, 1]
    };
    Ok(vec![build_real(DataClass::Double, &dims, out)])
}

/// `[i, j, ...] = ind2sub(sz, ind)` — convert linear indices to subscripts.
fn b_ind2sub(_i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 2, "ind2sub")?;
    let sz: Vec<usize> = to_f64_vec(&args[0])
        .into_iter()
        .map(|x| x as usize)
        .collect();
    let nd = nargout.max(1).max(sz.len());
    let str = strides(&sz);
    let inds = to_f64_vec(&args[1]);
    let dims = args[1].dims();
    let mut outs: Vec<Vec<f64>> = vec![Vec::with_capacity(inds.len()); nd];
    for &ind in &inds {
        let mut rem = (ind as usize).saturating_sub(1);
        for d in (0..nd).rev() {
            let s = str.get(d).copied().unwrap_or(1);
            let coord = rem / s;
            rem %= s;
            outs[d].push((coord + 1) as f64);
        }
    }
    Ok(outs
        .into_iter()
        .map(|o| build_real(DataClass::Double, &dims, o))
        .collect())
}

// ---- cat / horzcat / vertcat ------------------------------------------------

fn b_horzcat(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    concat_dim(i, 2, args)
}

fn b_vertcat(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    concat_dim(i, 1, args)
}

fn b_cat(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "cat")?;
    let dim = args[0].as_f64().unwrap_or(1.0) as usize;
    concat_dim(i, dim, &args[1..])
}

/// Concatenate `args` along dimension `dim` (1 = vertical, 2 = horizontal) by
/// re-using the interpreter's matrix-literal concatenation.
fn concat_dim(i: &mut Interpreter, dim: usize, args: &[Array]) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return Ok(vec![Array::empty()]);
    }
    let result = i.concat_values(dim, args)?;
    Ok(vec![result])
}
