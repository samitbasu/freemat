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
    table.add_builtin("flipdim", b_flipdim);
    table.add_builtin("shiftdim", b_shiftdim);
    table.add_builtin("transpose", b_transpose);
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

    // Cell-of-strings: lexicographic sort, preserving the cell's shape.
    if let Some(cells) = a.as_cell() {
        let flat: Vec<Array> = mem_cell(cells);
        let strs: Vec<String> = flat.iter().filter_map(Array::as_string).collect();
        if strs.len() != flat.len() {
            return err("sort: cell array input must be a cell array of strings");
        }
        let mut order: Vec<usize> = (0..strs.len()).collect();
        order.sort_by(|&x, &y| {
            let ord = strs[x].cmp(&strs[y]);
            if descend { ord.reverse() } else { ord }
        });
        let data: Vec<Array> = order.iter().map(|&k| flat[k].clone()).collect();
        let sorted = Array::cell(&dims, data);
        let idx: Vec<f64> = order.iter().map(|&k| (k + 1) as f64).collect();
        let mut out = vec![sorted];
        if nargout >= 2 {
            out.push(build_real(DataClass::Double, &dims, idx));
        }
        return Ok(out);
    }
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
/// Given `nitems` keys (addressed only through `cmp`/`eq`), return
/// `(m, n)` where `m[g]` is the first-occurrence original index of the `g`-th
/// distinct value (sorted ascending) and `n[i]` is the distinct-group index of
/// item `i`. By construction `value[m]` is sorted-unique and `value[m[n[i]]] ==
/// value[i]`, which is what gives `y = x(m)` / `x = y(n)`.
fn unique_with_indices(
    nitems: usize,
    cmp: impl Fn(usize, usize) -> std::cmp::Ordering,
    eq: impl Fn(usize, usize) -> bool,
) -> (Vec<usize>, Vec<usize>) {
    // Sort indices by key, breaking ties by original index so each group's first
    // element is its lowest original index (first occurrence).
    let mut order: Vec<usize> = (0..nitems).collect();
    order.sort_by(|&i, &j| cmp(i, j).then(i.cmp(&j)));
    let mut m: Vec<usize> = Vec::new();
    let mut n = vec![0usize; nitems];
    for &orig in &order {
        match m.last() {
            Some(&rep) if eq(orig, rep) => n[orig] = m.len() - 1,
            _ => {
                m.push(orig);
                n[orig] = m.len() - 1;
            }
        }
    }
    (m, n)
}

/// Build the `[y, m, n]` output triple, trimmed to `nargout` (at least 1). `m`
/// and `n` are returned as 1-based double index vectors shaped like `idx_dims`.
fn unique_outputs(
    y: Array,
    m: &[usize],
    n: &[usize],
    nargout: usize,
    m_dims: [usize; 2],
    n_dims: [usize; 2],
) -> Vec<Array> {
    let mut out = vec![y];
    if nargout >= 2 {
        let md: Vec<f64> = m.iter().map(|&i| (i + 1) as f64).collect();
        out.push(build_real(DataClass::Double, &m_dims, md));
    }
    if nargout >= 3 {
        let nd: Vec<f64> = n.iter().map(|&i| (i + 1) as f64).collect();
        out.push(build_real(DataClass::Double, &n_dims, nd));
    }
    out
}

fn b_unique(_i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "unique")?;
    let a = &args[0];
    let nout = nargout.max(1);
    // Optional 'rows' flag.
    let rows = match args.get(1) {
        None => false,
        Some(opt) => {
            if !opt
                .as_string()
                .is_some_and(|s| s.eq_ignore_ascii_case("rows"))
            {
                return err("unique: second argument must be 'rows'");
            }
            true
        }
    };
    let dims = a.dims();
    let is_row = dims.len() == 2 && dims[0] == 1;

    // Cell array of strings: unique strings (FreeMat ignores 'rows' for these).
    if let Some(cells) = a.as_cell() {
        let strs: Vec<String> = cells.iter().filter_map(Array::as_string).collect();
        let (m, n) = unique_with_indices(
            strs.len(),
            |i, j| strs[i].cmp(&strs[j]),
            |i, j| strs[i] == strs[j],
        );
        let k = m.len();
        let data: Vec<Array> = m.iter().map(|&i| Array::char_string(&strs[i])).collect();
        let y_dims = if is_row { [1, k] } else { [k, 1] };
        let y = Array::cell(&y_dims, data);
        let m_dims = if is_row { [1, k] } else { [k, 1] };
        let n_dims = if is_row { [1, n.len()] } else { [n.len(), 1] };
        return Ok(unique_outputs(y, &m, &n, nout, m_dims, n_dims));
    }

    if rows && dims.len() == 2 && dims[0] > 1 {
        // Unique rows: compare whole rows lexicographically (column-major store).
        let (nrows, ncols) = (dims[0], dims[1]);
        let v = to_f64_vec(a);
        let row_val = |r: usize, c: usize| v[c * nrows + r];
        let cmp = |i: usize, j: usize| {
            for c in 0..ncols {
                let o = row_val(i, c).total_cmp(&row_val(j, c));
                if o != std::cmp::Ordering::Equal {
                    return o;
                }
            }
            std::cmp::Ordering::Equal
        };
        let eq = |i: usize, j: usize| cmp(i, j) == std::cmp::Ordering::Equal;
        let (m, n) = unique_with_indices(nrows, cmp, eq);
        let k = m.len();
        // Assemble the k-by-ncols result, column-major.
        let mut data = vec![0.0; k * ncols];
        for (g, &orig) in m.iter().enumerate() {
            for c in 0..ncols {
                data[c * k + g] = row_val(orig, c);
            }
        }
        let y = build_real(a.class(), &[k, ncols], data);
        return Ok(unique_outputs(y, &m, &n, nout, [k, 1], [nrows, 1]));
    }

    // Vector mode: flatten, unique sorted values.
    let v = to_f64_vec(a);
    let (m, n) = unique_with_indices(
        v.len(),
        |i, j| v[i].total_cmp(&v[j]),
        |i, j| v[i].total_cmp(&v[j]) == std::cmp::Ordering::Equal,
    );
    let k = m.len();
    let yv: Vec<f64> = m.iter().map(|&i| v[i]).collect();
    let y_dims = if is_row { [1, k] } else { [k, 1] };
    let y = build_real(a.class(), &y_dims, yv);
    let m_dims = if is_row { [1, k] } else { [k, 1] };
    let n_dims = if is_row { [1, n.len()] } else { [n.len(), 1] };
    Ok(unique_outputs(y, &m, &n, nout, m_dims, n_dims))
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

/// Flip along `axis` (0-based). Handles N-D arrays by reversing the
/// coordinate along `axis` while leaving every other coordinate fixed.
fn flip(args: &[Array], axis: usize) -> Flow<Vec<Array>> {
    need(args, 1, "flip")?;
    let a = &args[0];
    let mut dims = a.dims();
    while dims.len() < 2 {
        dims.push(1);
    }
    let total: usize = dims.iter().product();
    let str = strides(&dims);
    let len = dims.get(axis).copied().unwrap_or(1);
    let mut perm = Vec::with_capacity(total);
    for lin in 0..total {
        // Source position = lin with the `axis` coordinate reversed.
        let src = if axis < dims.len() && len > 1 {
            let coord = (lin / str[axis]) % len;
            let delta = (len - 1 - coord) as i64 - coord as i64;
            (lin as i64 + delta * str[axis] as i64) as usize
        } else {
            lin
        };
        perm.push(src);
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
    flip(args, axis)
}

/// `flipdim(A, dim)` — flip along the given (1-based) dimension. N-D aware.
fn b_flipdim(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "flipdim")?;
    let axis = (args[1].as_f64().unwrap_or(1.0) as usize).saturating_sub(1);
    flip(&args[..1], axis)
}

/// `transpose(A)` — non-conjugate transpose of a 2-D array (the `.'` operator).
fn b_transpose(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "transpose")?;
    let a = &args[0];
    let mut dims = a.dims();
    while dims.len() < 2 {
        dims.push(1);
    }
    if dims.len() != 2 {
        return err("transpose: argument must be 2-D");
    }
    let (r, c) = (dims[0], dims[1]);
    // result[i, j] (shape [c, r]) = source[j, i].
    let mut perm = Vec::with_capacity(r * c);
    for j in 0..r {
        for i in 0..c {
            perm.push(j + i * r);
        }
    }
    Ok(vec![permute_by(a, &[c, r], &perm)])
}

/// `[B, nshifts] = shiftdim(A)` collapses leading singleton dimensions;
/// `shiftdim(A, n)` shifts dimensions left by `n` (a circular permute) or, for
/// negative `n`, prepends `|n|` singleton dimensions. Mirrors FreeMat.
fn b_shiftdim(_i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "shiftdim")?;
    let a = &args[0];
    let dims = a.dims();
    if args.len() < 2 {
        // Remove leading singleton dimensions.
        let nshift = dims.iter().take_while(|&&d| d == 1).count();
        // But never collapse a true scalar away entirely.
        let nshift = nshift.min(dims.len().saturating_sub(1));
        let mut new_dims: Vec<usize> = dims[nshift..].to_vec();
        while new_dims.len() < 2 {
            new_dims.push(1);
        }
        let perm: Vec<usize> = (0..a.numel()).collect();
        let mut out = vec![permute_by(a, &squeeze_trailing(new_dims), &perm)];
        if nargout >= 2 {
            out.push(build_real(DataClass::Double, &[1, 1], vec![nshift as f64]));
        }
        return Ok(out);
    }
    let n = args[1].as_f64().unwrap_or(0.0) as i64;
    if n == 0 {
        return Ok(vec![a.clone()]);
    }
    if n < 0 {
        // Prepend |n| singleton dimensions: reshape.
        let mut new_dims = vec![1usize; (-n) as usize];
        new_dims.extend_from_slice(&dims);
        let perm: Vec<usize> = (0..a.numel()).collect();
        return Ok(vec![permute_by(a, &squeeze_trailing(new_dims), &perm)]);
    }
    // n > 0: circular left-shift of the dimension order by n.
    let nd = dims.len().max(2);
    let mut full = dims.clone();
    while full.len() < nd {
        full.push(1);
    }
    let n = (n as usize) % nd;
    let order: Vec<usize> = (0..nd).map(|d| (d + n) % nd).collect();
    let order_arr = build_real(
        DataClass::Double,
        &[1, nd],
        order.iter().map(|&o| (o + 1) as f64).collect(),
    );
    permute(&[a.clone(), order_arr], false)
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
    let shifts_in = to_f64_vec(&args[1]);
    // Per-dimension shift amounts. A scalar `k` shifts the first non-singleton
    // dimension; a vector shifts dimension `d` by `shifts[d]`.
    let mut shifts = vec![0i64; dims.len()];
    if shifts_in.len() == 1 {
        let s = shifts_in[0] as i64;
        let d = dims.iter().position(|&d| d != 1).unwrap_or(0);
        shifts[d] = s;
    } else {
        for (d, s) in shifts_in.iter().enumerate() {
            if d < shifts.len() {
                shifts[d] = *s as i64;
            }
        }
    }
    let total: usize = dims.iter().product();
    let str = strides(&dims);
    let mut perm = Vec::with_capacity(total);
    for lin in 0..total {
        let mut src = lin;
        for d in 0..dims.len() {
            let len = dims[d];
            if len <= 1 || shifts[d] == 0 {
                continue;
            }
            let coord = (lin / str[d]) % len;
            let new_coord = (coord as i64 - shifts[d]).rem_euclid(len as i64) as usize;
            // Replace the d-th coordinate with new_coord.
            src = src - coord * str[d] + new_coord * str[d];
        }
        perm.push(src);
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
