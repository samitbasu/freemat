//! `fft` / `ifft` / `fft2` / `ifft2` via [`rustfft`].
//!
//! MATLAB semantics: a 1-D transform operates along the first non-singleton
//! dimension (or a given dimension), independently over the other dimensions.
//! `fft` is unnormalized; `ifft` divides by `n`. An optional length argument
//! truncates or zero-pads each segment to `n` points.

use fm_core::{Array, C64};
use fm_interp::error::{Flow, InterpError, Signal};
use rustfft::FftPlanner;

fn fft_err<T>(msg: impl Into<String>) -> Flow<T> {
    Err(Signal::Error(InterpError::msg(msg)))
}

/// `fft(x [, n [, dim]])` (or `ifft`). `inverse` selects the inverse transform.
pub fn fft1(args: &[Array], inverse: bool) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return fft_err("fft: input required");
    }
    let x = &args[0];
    let mut dims = x.dims();
    while dims.len() < 2 {
        dims.push(1);
    }
    // Determine the transform dimension (1-based arg → 0-based), default = first
    // non-singleton dimension.
    let dim = match args.get(2).and_then(Array::as_f64) {
        Some(d) => (d as usize).saturating_sub(1),
        None => dims.iter().position(|&d| d > 1).unwrap_or(0),
    };
    if dim >= dims.len() {
        return Ok(vec![x.clone()]);
    }
    // Transform length.
    let n = args
        .get(1)
        .and_then(Array::as_f64)
        .map(|v| v as usize)
        .filter(|&v| v > 0)
        .unwrap_or(dims[dim]);

    let data = fm_interp::value::to_c64_vec(x);
    let result = transform_along(&data, &dims, dim, n, inverse);
    let mut out_dims = dims.clone();
    out_dims[dim] = n;
    Ok(vec![build(out_dims, result)])
}

/// `fft2(x)` / `ifft2(x)` — 2-D transform over the first two dimensions.
pub fn fft2(args: &[Array], inverse: bool) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return fft_err("fft2: input required");
    }
    // fft2 = fft along dim 1 then dim 2.
    let r1 = fft1(args, inverse)?;
    let mut next = vec![r1[0].clone()];
    next.push(Array::empty()); // placeholder for n (use full length)
    next.push(Array::double(2.0)); // dim 2
    // Re-run along dim 2 using the natural length.
    let x = &next[0];
    let mut dims = x.dims();
    while dims.len() < 2 {
        dims.push(1);
    }
    let data = fm_interp::value::to_c64_vec(x);
    let result = transform_along(&data, &dims, 1, dims[1], inverse);
    Ok(vec![build(dims, result)])
}

/// Transform `data` (column-major, shape `dims`) along `dim`, resizing that axis
/// to `n` points.
fn transform_along(data: &[C64], dims: &[usize], dim: usize, n: usize, inverse: bool) -> Vec<C64> {
    let old_len = dims[dim];
    let mut out_dims = dims.to_vec();
    out_dims[dim] = n;
    let out_total: usize = out_dims.iter().product();
    let mut out = vec![C64::new(0.0, 0.0); out_total];
    if out_total == 0 || old_len == 0 {
        return out;
    }

    // Column-major strides for input and output.
    let in_strides = strides(dims);
    let out_strides = strides(&out_dims);

    let mut planner = FftPlanner::new();
    let fft = if inverse {
        planner.plan_fft_inverse(n)
    } else {
        planner.plan_fft_forward(n)
    };

    // Iterate over every "line" along `dim`: all combinations of the other axes.
    let other_count: usize = dims
        .iter()
        .enumerate()
        .filter(|&(i, _)| i != dim)
        .map(|(_, &d)| d)
        .product();

    let mut buf = vec![C64::new(0.0, 0.0); n];
    for line in 0..other_count {
        // Decompose `line` into multi-index over the non-`dim` axes.
        let mut rem = line;
        let mut in_base = 0usize;
        let mut out_base = 0usize;
        for (ax, &d) in dims.iter().enumerate() {
            if ax == dim {
                continue;
            }
            let idx = rem % d;
            rem /= d;
            in_base += idx * in_strides[ax];
            out_base += idx * out_strides[ax];
        }
        // Gather the segment (truncate or zero-pad to n).
        for (k, slot) in buf.iter_mut().enumerate() {
            *slot = if k < old_len {
                data[in_base + k * in_strides[dim]]
            } else {
                C64::new(0.0, 0.0)
            };
        }
        fft.process(&mut buf);
        let scale = if inverse { 1.0 / n as f64 } else { 1.0 };
        for (k, &v) in buf.iter().enumerate() {
            out[out_base + k * out_strides[dim]] = if inverse { v * scale } else { v };
        }
    }
    out
}

/// `fftn(x [, dims])` / `ifftn(x [, dims])` — N-dimensional transform. Applies
/// the 1-D FFT along every dimension in turn (FreeMat's `fftn.m`). For a 2-D
/// input this equals `fft2`. The optional `dims` vector gives the transform
/// length along each axis.
pub fn fftn(args: &[Array], inverse: bool) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return fft_err("fftn: input required");
    }
    let x = &args[0];
    let mut dims = x.dims();
    while dims.len() < 2 {
        dims.push(1);
    }
    // Optional length vector: pad with the natural sizes for the remaining axes.
    let mut sizes: Vec<usize> = dims.clone();
    if let Some(arg) = args.get(1) {
        let req = fm_interp::value::to_f64_vec(arg);
        if !req.is_empty() {
            // Extend dims if more lengths than axes were requested.
            while dims.len() < req.len() {
                dims.push(1);
            }
            sizes = dims.clone();
            for (k, &v) in req.iter().enumerate() {
                sizes[k] = v.max(0.0) as usize;
            }
        }
    }

    let total: usize = dims.iter().product();
    if total == 0 {
        return Ok(vec![x.clone()]);
    }

    let mut data = fm_interp::value::to_c64_vec(x);
    let mut cur_dims = dims.clone();
    for (axis, &n) in sizes.iter().enumerate() {
        if n == 0 {
            // Zero-length transform along this axis collapses it.
            let mut d = cur_dims.clone();
            d[axis] = 0;
            return Ok(vec![build(d, Vec::new())]);
        }
        data = transform_along(&data, &cur_dims, axis, n, inverse);
        cur_dims[axis] = n;
    }
    Ok(vec![build(cur_dims, data)])
}

/// `hilbert(x [, n])` — analytic signal via the FFT method (FreeMat's
/// `hilbert.m`). Operates along columns (a row vector is treated as a column).
/// `real(hilbert(x)) == x`; the imaginary part is the Hilbert transform.
pub fn hilbert(args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return fft_err("hilbert: input required");
    }
    let x = &args[0];
    let mut dims = x.dims();
    while dims.len() < 2 {
        dims.push(1);
    }
    // Treat a row vector as a column (shiftdim semantics for the common case).
    let row_vector = dims.len() == 2 && dims[0] == 1 && dims[1] > 1;
    let (rows, cols) = if row_vector {
        (dims[1], 1usize)
    } else {
        (dims[0], dims[1..].iter().product::<usize>().max(1))
    };

    let n = args
        .get(1)
        .and_then(Array::as_f64)
        .map(|v| v as usize)
        .filter(|&v| v > 0)
        .unwrap_or(rows);
    if n == 0 {
        return Ok(vec![x.clone()]);
    }

    let data = fm_interp::value::to_c64_vec(x);
    let mut planner = FftPlanner::new();
    let fwd = planner.plan_fft_forward(n);
    let inv = planner.plan_fft_inverse(n);

    // Multiplier h: doubles positive frequencies, zeros negatives, keeps DC and
    // (for even n) the Nyquist bin.
    let mut h = vec![0.0f64; n];
    h[0] = 1.0;
    if n.is_multiple_of(2) {
        h[n / 2] = 1.0;
        for hk in h.iter_mut().take(n / 2).skip(1) {
            *hk = 2.0;
        }
    } else {
        for hk in h.iter_mut().take(n.div_ceil(2)).skip(1) {
            *hk = 2.0;
        }
    }

    let mut out = vec![C64::new(0.0, 0.0); n * cols];
    let mut buf = vec![C64::new(0.0, 0.0); n];
    for col in 0..cols {
        let base = col * rows;
        for (k, slot) in buf.iter_mut().enumerate() {
            *slot = if k < rows {
                data[base + k]
            } else {
                C64::new(0.0, 0.0)
            };
        }
        fwd.process(&mut buf);
        for (k, slot) in buf.iter_mut().enumerate() {
            *slot *= h[k];
        }
        inv.process(&mut buf);
        let scale = 1.0 / n as f64;
        for (k, &v) in buf.iter().enumerate() {
            out[col * n + k] = v * scale;
        }
    }

    let out_dims = if row_vector {
        vec![1, n]
    } else {
        let mut d = dims.clone();
        d[0] = n;
        d
    };
    Ok(vec![build(out_dims, out)])
}

/// Circular shift of every axis (or one axis) by `floor(size/2)`. `forward`
/// selects `fftshift` (positive shift); otherwise `ifftshift` (negative shift).
pub fn fftshift(args: &[Array], forward: bool) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return fft_err("fftshift: input required");
    }
    let x = &args[0];
    let mut dims = x.dims();
    while dims.len() < 2 {
        dims.push(1);
    }
    // Per-axis shift amount (signed): floor(size/2), negated for ifftshift.
    let mut shift = vec![0i64; dims.len()];
    match args.get(1).and_then(Array::as_f64) {
        Some(d) => {
            let dim = (d as usize).saturating_sub(1);
            if dim < dims.len() {
                let s = (dims[dim] / 2) as i64;
                shift[dim] = if forward { s } else { -s };
            }
        }
        None => {
            for (ax, &sz) in dims.iter().enumerate() {
                let s = (sz / 2) as i64;
                shift[ax] = if forward { s } else { -s };
            }
        }
    }

    let total: usize = dims.iter().product();
    if total == 0 {
        return Ok(vec![x.clone()]);
    }
    let data = fm_interp::value::to_c64_vec(x);
    let st = strides(&dims);
    let mut out = vec![C64::new(0.0, 0.0); total];
    for (idx, &val) in data.iter().enumerate().take(total) {
        // Decompose the linear index into a multi-index and apply the shift.
        let mut rem = idx;
        let mut dst = 0usize;
        for (ax, &sz) in dims.iter().enumerate() {
            let coord = rem % sz;
            rem /= sz;
            let szi = sz as i64;
            let shifted = (((coord as i64 + shift[ax]) % szi) + szi) % szi;
            dst += shifted as usize * st[ax];
        }
        out[dst] = val;
    }
    // Preserve real-ness / class of the input by routing back through the
    // class-aware complex builder.
    Ok(vec![fm_interp::value::build_complex_class(
        x.class(),
        &dims,
        out,
    )])
}

fn strides(dims: &[usize]) -> Vec<usize> {
    let mut s = vec![1usize; dims.len()];
    for i in 1..dims.len() {
        s[i] = s[i - 1] * dims[i - 1];
    }
    s
}

fn build(dims: Vec<usize>, data: Vec<C64>) -> Array {
    fm_interp::value::build_complex(&dims, data)
}
