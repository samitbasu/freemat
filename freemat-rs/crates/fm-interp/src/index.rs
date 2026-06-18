//! Indexing: paren `()`, brace `{}`, and field `.f` reads and writes, including
//! linear & subscript indices, logical masks, the `:` magic colon, `end`, and
//! **grow-on-assign** semantics.
//!
//! Everything reduces to column-major linear positions. A resolved subscript is
//! a list of 0-based linear element positions ([`IndexPlan`]); reads gather,
//! writes scatter (growing the target if a position is out of bounds).

use fm_core::{Array, C64, DataClass, Dims, ScalarValue, StructArray};
use smallvec::SmallVec;

use crate::error::{Flow, InterpError, Signal};
use crate::value::{to_f64_vec, to_index};

/// A short list of 0-based linear positions, kept on the stack for the common
/// case (a handful of indexed elements, e.g. `A(i,j)`).
type Linear = SmallVec<[usize; 4]>;

/// Whether `rhs` is the empty `[]` value that triggers element deletion in a
/// paren-assignment `x(idx) = []`.
fn is_deletion(rhs: &Array) -> bool {
    rhs.numel() == 0 && !matches!(rhs, Array::Cell(_) | Array::Struct(_))
}

/// A resolved index along one or more dimensions, as a flat list of 0-based
/// column-major linear positions into the (possibly grown) target, plus the
/// shape the *result* should take.
pub struct IndexPlan {
    /// Linear (column-major) positions to gather/scatter, in result order.
    pub linear: Linear,
    /// The shape of the result of a *read* with this plan.
    pub result_dims: Dims,
    /// The dims the *target* must have to hold every position (grow target to
    /// at least this on assignment).
    pub needed_dims: Dims,
    /// For a 2-D subscript with exactly one non-colon axis (`x(i, :)` /
    /// `x(:, j)`), the axis (0 = rows, 1 = cols) the index selects. Used by
    /// `x(i,:) = []` row/column deletion to know which dimension to collapse.
    pub deleted_axis: Option<usize>,
}

/// A single index argument, already evaluated to a value (or the magic colon).
pub enum IndexArg {
    /// `:` — every element along this dimension.
    Colon,
    /// A value used as subscripts (numeric → 1-based; logical → mask).
    Value(Array),
}

/// Build column-major strides for `dims`.
fn strides(dims: &[usize]) -> Dims {
    let mut s: Dims = SmallVec::from_elem(1usize, dims.len().max(1));
    for i in 1..dims.len() {
        s[i] = s[i - 1] * dims[i - 1];
    }
    s
}

/// Convert a per-dimension subscript value into 0-based positions along that
/// dimension; `dim_len` is the current length of the dimension (for `:` /
/// logical). Returns the chosen positions and the max position + 1 (for growth).
fn resolve_dim(arg: &IndexArg, dim_len: usize) -> Flow<Linear> {
    match arg {
        IndexArg::Colon => Ok((0..dim_len).collect()),
        IndexArg::Value(v) => {
            if v.class() == DataClass::Bool {
                // Logical mask along this dimension.
                let mask = to_f64_vec(v);
                let mut out = Linear::new();
                for (i, &m) in mask.iter().enumerate() {
                    if m != 0.0 {
                        out.push(i);
                    }
                }
                Ok(out)
            } else {
                to_f64_vec(v).into_iter().map(to_index).collect()
            }
        }
    }
}

/// Try the scalar subscript fast path: every argument resolves to exactly one
/// in-bounds integer (the common `A(i)` / `A(i,j)` write/read). Computes the
/// single linear offset with **no `Vec`/`SmallVec` growth beyond one element**.
/// Returns `None` if any argument is `:`, logical, non-scalar, out of bounds, or
/// otherwise needs the general path (so the caller falls through).
fn plan_scalar(dims: &[usize], args: &[IndexArg]) -> Option<IndexPlan> {
    // Resolve each subscript to a single 0-based, in-bounds coordinate, and
    // accumulate the column-major linear offset on the fly.
    let n = args.len();
    let mut lin = 0usize;
    let mut stride = 1usize;
    for (axis, arg) in args.iter().enumerate() {
        let IndexArg::Value(v) = arg else {
            return None; // `:` — not the scalar path.
        };
        // Inline scalar, numeric (not logical), positive integer only.
        let s = match v {
            Array::Scalar(s) if !matches!(s, ScalarValue::Bool(_)) => *s,
            _ => return None,
        };
        let f = s.as_f64();
        if f < 1.0 || f.fract() != 0.0 {
            return None;
        }
        let coord = (f as usize) - 1;
        // Current extent of this axis (last subscript folds the trailing dims;
        // an empty product is 1, matching `plan_subscript`'s `eff`).
        let extent = if axis + 1 == n {
            dims.iter().skip(axis).product::<usize>()
        } else {
            dims.get(axis).copied().unwrap_or(1)
        };
        if coord >= extent {
            return None; // out of bounds → would grow; let the general path handle it.
        }
        lin += coord * stride;
        stride *= extent;
    }
    let mut needed = Dims::from_slice(dims);
    while needed.len() < 2 {
        needed.push(1);
    }
    Some(IndexPlan {
        linear: SmallVec::from_elem(lin, 1),
        result_dims: SmallVec::from_slice(&[1, 1]),
        needed_dims: needed,
        deleted_axis: None,
    })
}

/// Resolve a list of index arguments against a target of shape `dims` into an
/// [`IndexPlan`]. Handles linear (1 arg) and subscript (N args) indexing.
pub fn plan_index(dims: &[usize], args: &[IndexArg]) -> Flow<IndexPlan> {
    plan_index_rhs(dims, args, None)
}

/// Like [`plan_index`], but for **assignment**: `rhs_dims` (the shape of the
/// value being assigned) lets a colon index grow into a zero-length axis. For
/// `x = []; x(:,1) = 3`, the colon on the empty row axis adopts the RHS's
/// extent (1) instead of resolving to an empty range. Mirrors FreeMat, where a
/// `:` subscript against an empty/under-sized dimension takes the assigned
/// value's size.
pub fn plan_index_rhs(
    dims: &[usize],
    args: &[IndexArg],
    rhs_dims: Option<&[usize]>,
) -> Flow<IndexPlan> {
    if args.is_empty() {
        return Err(Signal::Error(InterpError::msg("empty index")));
    }
    let total: usize = dims.iter().product();

    // Scalar fast path: `A(i)` / `A(i,j)` with single in-bounds integer
    // subscripts — one linear offset, no per-dimension vectors.
    if let Some(plan) = plan_scalar(dims, args) {
        return Ok(plan);
    }

    if args.len() == 1 {
        return plan_linear(dims, total, &args[0], rhs_dims);
    }
    plan_subscript(dims, args, rhs_dims)
}

/// Linear (single-argument) indexing.
fn plan_linear(
    dims: &[usize],
    total: usize,
    arg: &IndexArg,
    rhs_dims: Option<&[usize]>,
) -> Flow<IndexPlan> {
    match arg {
        IndexArg::Colon => {
            // `A(:)` → column vector of all elements. On assignment into an
            // empty target (`a = []; a(:) = b`), the colon adopts the RHS's
            // element count and grows `a` to a column vector of that length
            // (FreeMat's grow-on-assign colon).
            if total == 0
                && let Some(rd) = rhs_dims
            {
                let k: usize = rd.iter().product();
                let linear: Linear = (0..k).collect();
                return Ok(IndexPlan {
                    result_dims: SmallVec::from_slice(&[k, 1]),
                    needed_dims: SmallVec::from_slice(&[k, 1]),
                    linear,
                    deleted_axis: None,
                });
            }
            let linear: Linear = (0..total).collect();
            Ok(IndexPlan {
                result_dims: SmallVec::from_slice(&[total, 1]),
                needed_dims: Dims::from_slice(dims),
                linear,
                deleted_axis: None,
            })
        }
        IndexArg::Value(v) => {
            if v.class() == DataClass::Bool {
                let mask = to_f64_vec(v);
                let linear: Linear = mask
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &m)| (m != 0.0).then_some(i))
                    .collect();
                let n = linear.len();
                // Logical linear indexing returns a column vector (row stays row
                // if the source is a row vector — MATLAB orients to the source).
                let result_dims = if dims.len() == 2 && dims[0] == 1 {
                    SmallVec::from_slice(&[1, n])
                } else {
                    SmallVec::from_slice(&[n, 1])
                };
                Ok(IndexPlan {
                    needed_dims: Dims::from_slice(dims),
                    linear,
                    result_dims,
                    deleted_axis: None,
                })
            } else {
                let idx = to_f64_vec(v);
                let linear: Linear = idx.iter().map(|&x| to_index(x)).collect::<Flow<Linear>>()?;
                let max = linear.iter().copied().max().map_or(0, |m| m + 1);
                // Result shape follows the index's shape, except a vector source
                // indexed by a vector keeps the source orientation.
                let result_dims = linear_result_dims(dims, v);
                let needed_dims = if max > total {
                    grow_linear_dims(dims, max)
                } else {
                    Dims::from_slice(dims)
                };
                Ok(IndexPlan {
                    linear,
                    result_dims,
                    needed_dims,
                    deleted_axis: None,
                })
            }
        }
    }
}

/// Result shape for numeric linear indexing of `dims` by index value `v`.
fn linear_result_dims(dims: &[usize], v: &Array) -> Dims {
    let vd = v.shape();
    let is_row_src = dims.len() == 2 && dims[0] == 1;
    let is_col_src = dims.len() == 2 && dims[1] == 1;
    let idx_is_vec = vd.len() == 2 && (vd[0] == 1 || vd[1] == 1);
    if (is_row_src || is_col_src) && idx_is_vec {
        // Vector source indexed by a vector → orient like the source.
        let n: usize = vd.iter().product();
        if is_row_src {
            SmallVec::from_slice(&[1, n])
        } else {
            SmallVec::from_slice(&[n, 1])
        }
    } else {
        // Otherwise the result takes the index's shape.
        Dims::from_slice(vd)
    }
}

/// Grow a shape so a linear position `max-1` fits.
///
/// Mirrors FreeMat's `BasicArray::resize(index_t)`: an empty/scalar target or a
/// row vector grows to a `1×needed` row; a column vector grows to `needed×1`;
/// and a **non-vector** matrix is *flattened to a row vector* `1×needed` (the
/// existing elements keep their column-major order). This is why
/// `a = [1 2;3 4]; a(7) = 9` yields a `1×7` row in FreeMat (not an error).
fn grow_linear_dims(dims: &[usize], needed: usize) -> Dims {
    if dims.len() == 2 && dims[1] == 1 && dims[0] > 1 {
        // Column vector grows downward.
        SmallVec::from_slice(&[needed, 1])
    } else {
        // Row vector, empty/scalar, or any matrix flattened to a row.
        SmallVec::from_slice(&[1, needed])
    }
}

/// Subscript (N-argument) indexing.
fn plan_subscript(
    dims: &[usize],
    args: &[IndexArg],
    rhs_dims: Option<&[usize]>,
) -> Flow<IndexPlan> {
    let n = args.len();
    // Effective target dims padded/merged to `n` axes.
    let mut eff: Dims = SmallVec::from_elem(1usize, n);
    for (i, slot) in eff.iter_mut().enumerate() {
        if i + 1 == n {
            // Last subscript collapses the remaining dimensions.
            *slot = dims.iter().skip(i).product::<usize>();
            if dims.len() <= i {
                *slot = 1;
            }
        } else {
            *slot = dims.get(i).copied().unwrap_or(1);
        }
    }

    // Assignment into an empty/under-sized axis: a colon over a zero-length
    // dimension adopts the assigned value's extent for that axis (FreeMat's
    // grow-on-assign colon). Only applies when assigning (`rhs_dims` set).
    if let Some(rd) = rhs_dims {
        for (i, arg) in args.iter().enumerate() {
            if matches!(arg, IndexArg::Colon) && eff[i] == 0 {
                eff[i] = rd.get(i).copied().unwrap_or(1);
            }
        }
    }

    // Resolve each dimension's positions.
    let mut per_dim: SmallVec<[Linear; 4]> = SmallVec::with_capacity(n);
    for (i, arg) in args.iter().enumerate() {
        per_dim.push(resolve_dim(arg, eff[i])?);
    }

    // Needed extent per dim (for growth) = max(existing, max-position+1).
    let mut needed = eff.clone();
    for (i, positions) in per_dim.iter().enumerate() {
        let m = positions.iter().copied().max().map_or(0, |p| p + 1);
        needed[i] = needed[i].max(m);
    }

    // Generate linear positions in column-major (first index fastest) order.
    let result_dims: Dims = per_dim.iter().map(SmallVec::len).collect();
    let nstr = strides(&needed);
    let total_out: usize = result_dims.iter().product();
    let mut linear: Linear = SmallVec::with_capacity(total_out);
    let out_str = strides(&result_dims);
    for out_lin in 0..total_out {
        let mut pos = 0usize;
        let mut rem = out_lin;
        for d in (0..n).rev() {
            let coord = rem / out_str[d];
            rem %= out_str[d];
            pos += per_dim[d][coord] * nstr[d];
        }
        linear.push(pos);
    }

    // The full needed shape, accounting for the original trailing dims.
    let needed_dims = merge_needed(dims, &needed, n);
    // Detect the single selected axis for `x(i,:)` / `x(:,j)` deletion.
    let deleted_axis = if n == 2 {
        let c0 = matches!(args[0], IndexArg::Colon);
        let c1 = matches!(args[1], IndexArg::Colon);
        match (c0, c1) {
            (false, true) => Some(0),
            (true, false) => Some(1),
            _ => None,
        }
    } else {
        None
    };
    Ok(IndexPlan {
        linear,
        result_dims: squeeze_result(result_dims),
        needed_dims,
        deleted_axis,
    })
}

/// Build the target's needed dims from a subscript's per-axis extents.
fn merge_needed(orig: &[usize], needed: &[usize], n: usize) -> Dims {
    if n == orig.len() {
        return Dims::from_slice(needed);
    }
    // When fewer subscripts than dims, the last subscript indexes the flattened
    // tail; growth there is uncommon — keep the original tail dims if unchanged.
    let mut out = Dims::from_slice(needed);
    while out.len() < 2 {
        out.push(1);
    }
    out
}

/// Drop trailing singleton dims beyond rank 2 (MATLAB keeps ≥2 dims).
fn squeeze_result(mut d: Dims) -> Dims {
    while d.len() > 2 && *d.last().unwrap() == 1 {
        d.pop();
    }
    while d.len() < 2 {
        d.push(1);
    }
    d
}

/// Read elements of `base` at the positions in `plan` (paren-index read).
pub fn gather(base: &Array, plan: &IndexPlan) -> Flow<Array> {
    let total = base.numel();
    for &p in &plan.linear {
        if p >= total {
            return Err(Signal::Error(InterpError::msg(format!(
                "index {} out of bounds (numel = {total})",
                p + 1
            ))));
        }
    }
    Ok(gather_unchecked(base, &plan.linear, &plan.result_dims))
}

/// Read a single element at column-major linear position `i` as a
/// [`ScalarValue`], without allocating. Returns `None` for cell/struct (which
/// have no scalar form) so the caller takes the general gather path.
fn scalar_at(base: &Array, i: usize) -> Option<ScalarValue> {
    macro_rules! at {
        ($d:expr, $ctor:path) => {
            $d.as_slice_memory_order().map(|s| $ctor(s[i]))
        };
    }
    match base {
        Array::Scalar(s) => Some(*s),
        Array::Bool(d) => at!(d, ScalarValue::Bool),
        Array::Int8(d) => at!(d, ScalarValue::Int8),
        Array::UInt8(d) => at!(d, ScalarValue::UInt8),
        Array::Int16(d) => at!(d, ScalarValue::Int16),
        Array::UInt16(d) => at!(d, ScalarValue::UInt16),
        Array::Int32(d) => at!(d, ScalarValue::Int32),
        Array::UInt32(d) => at!(d, ScalarValue::UInt32),
        Array::Int64(d) => at!(d, ScalarValue::Int64),
        Array::UInt64(d) => at!(d, ScalarValue::UInt64),
        Array::Float(d) => at!(d, ScalarValue::Float),
        Array::Double(d) => at!(d, ScalarValue::Double),
        Array::Complex32(d) => at!(d, ScalarValue::Complex32),
        Array::Complex64(d) => at!(d, ScalarValue::Complex64),
        Array::Char(d) => at!(d, ScalarValue::Char),
        Array::Cell(_) | Array::Struct(_) => None,
        Array::Sparse(s) => {
            let (re, im) = s.get_linear(i);
            if im != 0.0 {
                Some(ScalarValue::Complex64(fm_core::C64::new(re, im)))
            } else {
                Some(ScalarValue::Double(re))
            }
        }
    }
}

/// Gather without bounds-checking (callers pre-validate). All reads are in
/// column-major (memory) order so linear positions line up.
fn gather_unchecked(base: &Array, linear: &[usize], result_dims: &[usize]) -> Array {
    // Single-element gather → an inline `Array::Scalar` (no heap, no `ArrayD`).
    // This is the common `A(i,j)` read and, crucially, how a `for` loop variable
    // and any 1×1 subexpression stay on the scalar fast path (so subsequent
    // arithmetic / indexing never re-materialises a 1-element array). Mirrors
    // MATLAB: a single-subscript result is a scalar.
    if linear.len() == 1
        && result_dims.iter().product::<usize>() == 1
        && let Some(s) = scalar_at(base, linear[0])
    {
        return Array::Scalar(s);
    }
    // Pick elements at `linear` straight from the column-major memory buffer:
    // O(count), no whole-array clone. `mem_order` (which copies the entire
    // buffer) is only used as a fallback for the rare non-contiguous view.
    macro_rules! gather_dense {
        ($d:expr, $build:path) => {{
            let data: Vec<_> = if let Some(flat) = $d.as_slice_memory_order() {
                linear.iter().map(|&i| flat[i].clone()).collect()
            } else {
                let flat = crate::value::mem_order($d);
                linear.iter().map(|&i| flat[i].clone()).collect()
            };
            $build(result_dims, data)
        }};
    }
    match base {
        Array::Scalar(_) => base.clone(),
        Array::Double(d) => gather_dense!(d, Array::double_matrix),
        Array::Float(d) => gather_dense!(d, Array::single_matrix),
        Array::Bool(d) => gather_dense!(d, Array::bool_matrix),
        Array::Int32(d) => gather_dense!(d, Array::int32_matrix),
        Array::Complex64(d) => gather_dense!(d, Array::complex64_matrix),
        Array::Char(d) => {
            let data: Vec<char> = if let Some(flat) = d.as_slice_memory_order() {
                linear.iter().map(|&i| flat[i]).collect()
            } else {
                let flat = crate::value::mem_order(d);
                linear.iter().map(|&i| flat[i]).collect()
            };
            crate::value::char_matrix(result_dims, data)
        }
        Array::Cell(d) => {
            let data: Vec<Array> = if let Some(flat) = d.as_slice_memory_order() {
                linear.iter().map(|&i| flat[i].clone()).collect()
            } else {
                let flat = crate::value::mem_order(d);
                linear.iter().map(|&i| flat[i].clone()).collect()
            };
            Array::cell(result_dims, data)
        }
        Array::Struct(s) => {
            // Gather a sub-struct-array: same fields, elements at `linear`.
            let fields: Vec<(String, Vec<Array>)> = s
                .field_pairs()
                .iter()
                .map(|(name, vals)| {
                    let picked: Vec<Array> = linear.iter().map(|&i| vals[i].clone()).collect();
                    (name.clone(), picked)
                })
                .collect();
            Array::struct_array(StructArray::from_fields(result_dims.to_vec(), fields))
        }
        Array::Sparse(s) => {
            // Gather from a sparse matrix by column-major linear position. The
            // result is dense (correct; comparisons go through `full`).
            if s.is_complex() {
                let data: Vec<fm_core::C64> = linear
                    .iter()
                    .map(|&i| {
                        let (re, im) = s.get_linear(i);
                        fm_core::C64::new(re, im)
                    })
                    .collect();
                crate::value::build_complex(result_dims, data)
            } else {
                let data: Vec<f64> = linear.iter().map(|&i| s.get_linear(i).0).collect();
                crate::value::build_real(s.class(), result_dims, data)
            }
        }
        // Other integer/complex32 classes: route through f64 (loses nothing for
        // integers; complex32 handled via the dedicated arm above for c64).
        _ => {
            let flat = to_f64_vec(base);
            let data: Vec<f64> = linear.iter().map(|&i| flat[i]).collect();
            crate::value::build_real(base.class(), result_dims, data)
        }
    }
}

/// Cell-content (brace) read: returns the gathered cells' *contents* as a list.
pub fn gather_cell_contents(base: &Array, plan: &IndexPlan) -> Flow<Vec<Array>> {
    let cells = base.as_cell().ok_or_else(|| {
        Signal::Error(InterpError::msg(format!(
            "'{{}}' indexing requires a cell array, got {}",
            base.class_name()
        )))
    })?;
    let flat: Vec<Array> = crate::value::mem_order(cells);
    let total = flat.len();
    let mut out = Vec::with_capacity(plan.linear.len());
    for &p in &plan.linear {
        if p >= total {
            return Err(Signal::Error(InterpError::msg(format!(
                "index {} out of bounds (numel = {total})",
                p + 1
            ))));
        }
        out.push(flat[p].clone());
    }
    Ok(out)
}

/// Scatter `rhs` into `target` **in place when possible**, otherwise rebuild.
///
/// This is the hot-path entry point for `A(idx) = rhs`. When the assignment
/// needs no growth, no type promotion, and stays within the dense
/// real-numeric / logical / char classes (no complex, cell, struct, or
/// deletion), it writes `rhs` straight into `target`'s column-major buffer via
/// the copy-on-write `make_mut_*` accessor — O(count) and zero whole-array
/// rebuild, deep-copying only if the backing `Arc` is shared (COW correctness).
///
/// Every other case (growth, type-promote, complex, cell, struct, deletion)
/// falls back to the materialise-and-rebuild [`scatter`] and stores the result
/// into `*target`, so behaviour is identical to the old path.
pub fn scatter_into(target: &mut Array, plan: &IndexPlan, rhs: &Array) -> Flow<()> {
    if let Some(()) = try_scatter_in_place(target, plan, rhs)? {
        return Ok(());
    }
    *target = scatter(target, plan, rhs)?;
    Ok(())
}

/// Attempt the in-place fast path. Returns `Ok(Some(()))` if it handled the
/// write, `Ok(None)` if the caller must fall back to the rebuild path, or an
/// error for an in-place size mismatch.
fn try_scatter_in_place(target: &mut Array, plan: &IndexPlan, rhs: &Array) -> Flow<Option<()>> {
    // Bail out of the fast path for any case the rebuild path must own:
    // deletion, struct/cell, growth, complex, or a class change (promotion).
    if is_deletion(rhs) && target.numel() > 0 {
        return Ok(None);
    }
    if matches!(target, Array::Cell(_) | Array::Struct(_))
        || matches!(rhs, Array::Cell(_) | Array::Struct(_))
    {
        return Ok(None);
    }
    // Sparse target: no in-place buffer accessor — rebuild path densifies it.
    if target.is_sparse() {
        return Ok(None);
    }
    if target.is_complex() || rhs.is_complex() {
        return Ok(None);
    }
    // No growth: every needed extent must already fit, and an inline scalar
    // target (numel 1, no buffer) can't be written in place — let it rebuild.
    if matches!(target, Array::Scalar(_)) {
        return Ok(None);
    }
    if plan.needed_dims.iter().product::<usize>() > target.numel() {
        return Ok(None);
    }
    // Same class only (no promotion): assigning e.g. int8 into a double array
    // keeps the double class but changes values — that's fine and stays in the
    // fast path because we read the rhs as f64 and the target buffer is f64.
    // But a class *change* of the target (empty base adopting rhs, or char) is
    // routed to the rebuild path for simplicity / fidelity.
    let class = target.class();
    if class == DataClass::Char {
        // Char writes go through the rebuild path (code-point conversion).
        return Ok(None);
    }

    let count = plan.linear.len();
    // Read the rhs without allocating a `Vec` when it is an inline scalar (the
    // overwhelmingly common `A(i,j) = scalar` case): `as_f64()` reads the single
    // value straight from the inline `ScalarValue`. A multi-element rhs still
    // materialises its values once, as before.
    let scalar_val = rhs.as_f64();
    let rhs_vals = if scalar_val.is_some() {
        Vec::new()
    } else {
        to_f64_vec(rhs)
    };
    let rhs_len = if scalar_val.is_some() {
        1
    } else {
        rhs_vals.len()
    };
    if rhs_len != 1 && rhs_len != count {
        return Err(Signal::Error(InterpError::msg(format!(
            "assignment size mismatch: {rhs_len} elements into {count} positions"
        ))));
    }
    let scalar = rhs_len == 1;

    // Write only the indexed positions into the target's column-major buffer.
    macro_rules! write_in_place {
        ($accessor:ident, $conv:expr) => {{
            let buf = target
                .$accessor()
                .expect("class checked above")
                .as_slice_memory_order_mut()
                .expect("dense F-order buffer is contiguous");
            for (i, &p) in plan.linear.iter().enumerate() {
                // `scalar` ⇒ `scalar_val` is `Some` (a numel-1 rhs); otherwise
                // `rhs_vals` holds one value per indexed position.
                let v = if scalar {
                    scalar_val.expect("scalar rhs has a single value")
                } else {
                    rhs_vals[i]
                };
                buf[p] = $conv(v);
            }
        }};
    }

    // Saturating integer casts reuse the exact `value` helpers so an in-place
    // write produces identical bytes to the rebuild path (`build_integer`).
    use crate::value::{sat_i, sat_u};
    match class {
        DataClass::Double => write_in_place!(make_mut_double, |v: f64| v),
        DataClass::Float => write_in_place!(make_mut_float, |v: f64| v as f32),
        DataClass::Bool => write_in_place!(make_mut_bool, |v: f64| v != 0.0),
        DataClass::Int8 => write_in_place!(make_mut_int8, |v: f64| sat_i(v) as i8),
        DataClass::UInt8 => write_in_place!(make_mut_uint8, |v: f64| sat_u(v) as u8),
        DataClass::Int16 => write_in_place!(make_mut_int16, |v: f64| sat_i(v) as i16),
        DataClass::UInt16 => write_in_place!(make_mut_uint16, |v: f64| sat_u(v) as u16),
        DataClass::Int32 => write_in_place!(make_mut_int32, |v: f64| sat_i(v) as i32),
        DataClass::UInt32 => write_in_place!(make_mut_uint32, |v: f64| sat_u(v) as u32),
        DataClass::Int64 => write_in_place!(make_mut_int64, sat_i),
        DataClass::UInt64 => write_in_place!(make_mut_uint64, sat_u),
        // Char handled above; complex/cell/struct bailed out earlier.
        _ => return Ok(None),
    }
    Ok(Some(()))
}

/// Scatter `rhs` into `base` at `plan`'s positions, growing `base` to
/// `plan.needed_dims` if necessary. Returns the updated array.
pub fn scatter(base: &Array, plan: &IndexPlan, rhs: &Array) -> Flow<Array> {
    // Element deletion: `x(idx) = []` removes the indexed elements.
    if is_deletion(rhs) && base.numel() > 0 {
        return scatter_delete(base, plan);
    }

    // Struct-array paren-assignment: `s(i) = structValue` (grow / overwrite).
    if matches!(base, Array::Struct(_)) || matches!(rhs, Array::Struct(_)) {
        return scatter_struct(base, plan, rhs);
    }

    // Decide the result class: keep base's class unless base is empty (then take
    // rhs's class — assigning into `[]` adopts the rhs type).
    let class = if base.numel() == 0 && !matches!(base, Array::Scalar(_)) {
        rhs.class()
    } else {
        promote_assign(base.class(), rhs.class())?
    };

    // Cell-array paren-assignment stays a cell.
    if class == DataClass::Cell || base.class() == DataClass::Cell {
        return scatter_cell(base, plan, rhs);
    }

    let needed: usize = plan.needed_dims.iter().product();
    let needed_dims: Dims = if plan.needed_dims.iter().product::<usize>() >= base.numel() {
        plan.needed_dims.clone()
    } else {
        base.dims_smallvec()
    };

    // RHS values, broadcast: scalar fills, else must match index count.
    let rhs_vals = to_f64_vec(rhs);
    let count = plan.linear.len();
    let take = |i: usize| -> Flow<f64> {
        if rhs_vals.len() == 1 {
            Ok(rhs_vals[0])
        } else if rhs_vals.len() == count {
            Ok(rhs_vals[i])
        } else {
            Err(Signal::Error(InterpError::msg(format!(
                "assignment size mismatch: {} elements into {count} positions",
                rhs_vals.len()
            ))))
        }
    };

    if class == DataClass::Char {
        let mut flat: Vec<char> = match base {
            Array::Char(d) => crate::value::mem_order(d),
            Array::Scalar(ScalarValue::Char(c)) => vec![*c],
            _ => vec!['\u{0}'; base.numel()],
        };
        flat.resize(needed.max(flat.len()), '\u{0}');
        for (i, &p) in plan.linear.iter().enumerate() {
            flat[p] = char::from_u32(take(i)? as u32).unwrap_or('\u{fffd}');
        }
        return Ok(crate::value::char_matrix(&needed_dims, flat));
    }

    if class == DataClass::Double && (base.is_complex() || rhs.is_complex()) {
        let mut flat = crate::value::to_c64_vec(base);
        flat.resize(needed.max(flat.len()), C64::new(0.0, 0.0));
        let rc = crate::value::to_c64_vec(rhs);
        for (i, &p) in plan.linear.iter().enumerate() {
            let val = if rc.len() == 1 { rc[0] } else { rc[i] };
            flat[p] = val;
        }
        return Ok(crate::value::build_complex(&needed_dims, flat));
    }

    let mut flat = to_f64_vec(base);
    flat.resize(needed.max(flat.len()), 0.0);
    for (i, &p) in plan.linear.iter().enumerate() {
        flat[p] = take(i)?;
    }
    Ok(crate::value::build_real(class, &needed_dims, flat))
}

/// Scatter into / grow a cell array via paren-assignment (`c(i) = {..}`), where
/// `rhs` is itself a cell whose contents are placed at the positions.
fn scatter_cell(base: &Array, plan: &IndexPlan, rhs: &Array) -> Flow<Array> {
    let needed: usize = plan.needed_dims.iter().product();
    let mut flat: Vec<Array> = match base {
        Array::Cell(d) => crate::value::mem_order(d),
        _ if base.numel() == 0 => Vec::new(),
        _ => {
            return Err(Signal::Error(InterpError::msg(
                "cannot paren-assign a cell into a non-cell",
            )));
        }
    };
    flat.resize(needed.max(flat.len()), Array::empty());
    let rhs_cells: Vec<Array> = match rhs.as_cell() {
        Some(c) => crate::value::mem_order(c),
        None => {
            return Err(Signal::Error(InterpError::msg(
                "right-hand side of cell paren-assignment must be a cell array",
            )));
        }
    };
    for (i, &p) in plan.linear.iter().enumerate() {
        let v = if rhs_cells.len() == 1 {
            rhs_cells[0].clone()
        } else {
            rhs_cells[i].clone()
        };
        flat[p] = v;
    }
    Ok(Array::cell(&plan.needed_dims, flat))
}

/// Scatter cell *contents* via brace-assignment (`c{i} = val`): grows the cell
/// and places each `rhs` value directly into the cell slot.
pub fn scatter_cell_contents(base: &Array, plan: &IndexPlan, rhs: Array) -> Flow<Array> {
    let needed: usize = plan.needed_dims.iter().product();
    let mut flat: Vec<Array> = match base {
        Array::Cell(d) => crate::value::mem_order(d),
        _ if base.numel() == 0 => Vec::new(),
        _ => {
            return Err(Signal::Error(InterpError::msg(format!(
                "'{{}}' assignment requires a cell array, got {}",
                base.class_name()
            ))));
        }
    };
    flat.resize(needed.max(flat.len()), Array::empty());
    // Single rhs into possibly many positions (brace-assign one value).
    for &p in &plan.linear {
        flat[p] = rhs.clone();
    }
    let dims: Dims = if plan.needed_dims.iter().product::<usize>() >= flat.len() {
        plan.needed_dims.clone()
    } else {
        SmallVec::from_slice(&[flat.len(), 1])
    };
    Ok(Array::cell(&dims, flat))
}

/// Delete the positions in `plan` from `base` (`x(idx) = []`).
///
/// Linear deletion from a vector keeps the source orientation; row/column
/// deletion via `x(i, :)` / `x(:, j)` removes whole rows/columns. Deleting a
/// proper sub-block of a matrix (neither a full row nor column set) is an error
/// in MATLAB.
fn scatter_delete(base: &Array, plan: &IndexPlan) -> Flow<Array> {
    use std::collections::BTreeSet;
    let dims = base.dims();
    let total = base.numel();
    let remove: BTreeSet<usize> = plan.linear.iter().copied().collect();
    for &p in &remove {
        if p >= total {
            return Err(Signal::Error(InterpError::msg(format!(
                "index {} out of bounds (numel = {total})",
                p + 1
            ))));
        }
    }

    // Determine the kept linear positions and the resulting shape.
    let (keep, new_dims) = if plan.deleted_axis == Some(0) && dims.len() == 2 {
        // Row deletion: remove the chosen rows, keep all columns.
        let (r, c) = (dims[0], dims[1]);
        let drop_rows: BTreeSet<usize> = remove.iter().map(|&p| p % r).collect();
        let kept_rows: Vec<usize> = (0..r).filter(|i| !drop_rows.contains(i)).collect();
        let mut keep = Vec::with_capacity(kept_rows.len() * c);
        for j in 0..c {
            for &i in &kept_rows {
                keep.push(i + j * r);
            }
        }
        (keep, vec![kept_rows.len(), c])
    } else if plan.deleted_axis == Some(1) && dims.len() == 2 {
        // Column deletion: remove the chosen columns, keep all rows.
        let (r, c) = (dims[0], dims[1]);
        let drop_cols: BTreeSet<usize> = remove.iter().map(|&p| p / r).collect();
        let kept_cols: Vec<usize> = (0..c).filter(|j| !drop_cols.contains(j)).collect();
        let mut keep = Vec::with_capacity(r * kept_cols.len());
        for &j in &kept_cols {
            for i in 0..r {
                keep.push(i + j * r);
            }
        }
        (keep, vec![r, kept_cols.len()])
    } else {
        // Linear deletion → a vector. Keep source orientation when it is a row.
        let keep: Vec<usize> = (0..total).filter(|p| !remove.contains(p)).collect();
        let n = keep.len();
        let new_dims = if dims.len() == 2 && dims[0] == 1 {
            vec![1, n]
        } else {
            vec![n, 1]
        };
        (keep, new_dims)
    };

    Ok(gather_unchecked(base, &keep, &new_dims))
}

/// Scatter a struct value into a struct array (`s(i) = structValue`), growing
/// the array and unioning field names as needed.
fn scatter_struct(base: &Array, plan: &IndexPlan, rhs: &Array) -> Flow<Array> {
    let rhs_struct = rhs.as_struct().ok_or_else(|| {
        Signal::Error(InterpError::msg(format!(
            "cannot assign a {} value into a struct array",
            rhs.class_name()
        )))
    })?;
    if !base.class().is_reference() && base.numel() == 0 {
        // Growing into `[]` — start from an empty struct.
    } else if !matches!(base, Array::Struct(_)) {
        return Err(Signal::Error(InterpError::msg(format!(
            "cannot assign a struct into a {} value",
            base.class_name()
        ))));
    }

    let needed: usize = plan.needed_dims.iter().product();
    // Field name union, base order first then any new rhs fields.
    let mut names: Vec<String> = match base {
        Array::Struct(s) => s.field_name_strings(),
        _ => Vec::new(),
    };
    for n in rhs_struct.field_name_strings() {
        if !names.contains(&n) {
            names.push(n);
        }
    }

    // Build each field's element vector grown to `needed`, default empty.
    let mut fields: Vec<(String, Vec<Array>)> = names
        .iter()
        .map(|name| {
            let mut col: Vec<Array> = match base {
                Array::Struct(s) => s.field(name).map(<[Array]>::to_vec).unwrap_or_default(),
                _ => Vec::new(),
            };
            col.resize(needed.max(col.len()), Array::empty());
            (name.clone(), col)
        })
        .collect();

    // rhs broadcasts: a scalar struct fills every target position.
    let rhs_count = rhs_struct.numel();
    for (i, &p) in plan.linear.iter().enumerate() {
        let src = if rhs_count == 1 { 0 } else { i };
        for (name, col) in &mut fields {
            let v = rhs_struct
                .field(name)
                .and_then(|vals| vals.get(src))
                .cloned()
                .unwrap_or_else(Array::empty);
            col[p] = v;
        }
    }

    Ok(Array::struct_array(StructArray::from_fields(
        plan.needed_dims.to_vec(),
        fields,
    )))
}

/// Promote class on assignment: empty base adopts rhs; otherwise keep base's
/// class (MATLAB keeps the target's class when assigning into it).
fn promote_assign(base: DataClass, rhs: DataClass) -> Flow<DataClass> {
    if base.is_reference() && base != rhs {
        return Err(Signal::Error(InterpError::msg(format!(
            "cannot assign {} into {}",
            rhs.name(),
            base.name()
        ))));
    }
    Ok(base)
}

// ---- Field access on structs ------------------------------------------------

/// Read field `name` from a scalar struct value.
pub fn field_read(base: &Array, name: &str) -> Flow<Array> {
    match base {
        Array::Struct(s) => s.scalar_field(name).cloned().ok_or_else(|| {
            Signal::Error(InterpError::msg(format!(
                "reference to non-existent field '{name}'"
            )))
        }),
        _ => Err(Signal::Error(InterpError::msg(format!(
            "cannot access field '{name}' of a {} value",
            base.class_name()
        )))),
    }
}

/// Assign `value` to field `name` of `base`, creating the struct/field if
/// needed (grow-on-assign for fields).
pub fn field_write(base: &Array, name: &str, value: Array) -> Flow<Array> {
    match base {
        Array::Struct(s) => {
            let mut fields: Vec<(String, Array)> = s
                .field_names()
                .iter()
                .filter(|n| **n != name)
                .map(|n| {
                    (
                        (*n).to_string(),
                        s.scalar_field(n).cloned().unwrap_or_else(Array::empty),
                    )
                })
                .collect();
            fields.push((name.to_string(), value));
            Ok(Array::struct_array(StructArray::scalar(fields)))
        }
        _ if base.numel() == 0 && !base.class().is_reference() => {
            // Assigning a field into `[]` creates a new scalar struct.
            Ok(Array::struct_array(StructArray::scalar([(
                name.to_string(),
                value,
            )])))
        }
        _ => Err(Signal::Error(InterpError::msg(format!(
            "cannot set field '{name}' on a {} value",
            base.class_name()
        )))),
    }
}
