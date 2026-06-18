//! Sparse-matrix builtins: `sparse`, `full`, `speye`, `spzeros`/`sparse(m,n)`,
//! `spones`, `spdiags`, `sprand`/`sprandn`, `nnz`, `nonzeros`, `find` (sparse,
//! 3-output `[i,j,v]`), `issparse`, `nzmax`, `spy`.
//!
//! Storage is CSC (see [`fm_core::SparseMatrix`]). Most numeric operations on
//! sparse values densify (correct; conformance compares via `full`), but
//! construction, `full`, `find`, `nnz`, and matrix `*` stay genuinely sparse.

use fm_core::{Array, C64, DataClass, SparseMatrix};
use fm_interp::error::Flow;
use fm_interp::value::{build_complex, build_real, densify, sparsify, to_c64_vec, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};
use rand::Rng;
use rand_distr::{Distribution, StandardNormal};

use crate::util::{err, need, scalar_arg};

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("sparse", b_sparse);
    table.add_builtin("full", b_full);
    table.add_builtin("speye", b_speye);
    table.add_builtin("spzeros", b_spzeros);
    table.add_builtin("spones", b_spones);
    table.add_builtin("spdiags", b_spdiags);
    table.add_builtin("sprand", b_sprand);
    table.add_builtin("sprandn", b_sprandn);
    table.add_builtin("nnz", b_nnz);
    table.add_builtin("nonzeros", b_nonzeros);
    table.add_builtin("issparse", b_issparse);
    table.add_builtin("nzmax", b_nzmax);
    table.add_builtin("spy", b_spy);
    // `find` and `full` override the dense versions to be sparse-aware. They are
    // registered after the stdlib's `logical::find`, so this shadows it.
    table.add_builtin("find", b_find);
}

/// `sparse(X)` / `sparse(m,n)` / `sparse(i,j,s[,m,n])`.
fn b_sparse(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "sparse")?;
    match args.len() {
        // sparse(X): convert a (dense or already-sparse) matrix to sparse.
        1 => Ok(vec![sparsify(&args[0])]),
        // sparse(m, n): an m-by-n all-zero sparse matrix.
        2 => {
            let m = scalar_arg(args, 0, "sparse")? as usize;
            let n = scalar_arg(args, 1, "sparse")? as usize;
            Ok(vec![Array::sparse(SparseMatrix::zeros(m, n))])
        }
        // sparse(i, j, s[, m, n[, nzmax]]): build from triplets (sum dups).
        _ => {
            let iv: Vec<usize> = to_f64_vec(&args[0])
                .iter()
                .map(|&x| x.round() as usize)
                .collect();
            let jv: Vec<usize> = to_f64_vec(&args[1])
                .iter()
                .map(|&x| x.round() as usize)
                .collect();
            let complex = args[2].is_complex();
            let class = if args[2].class() == DataClass::Float {
                DataClass::Float
            } else {
                DataClass::Double
            };
            let m = args.get(3).and_then(Array::as_f64).map(|x| x as usize);
            let n = args.get(4).and_then(Array::as_f64).map(|x| x as usize);
            // `s` may be a scalar (broadcast over all (i,j)) or a vector.
            let nij = iv.len().max(jv.len());
            let result = if complex {
                let cv = to_c64_vec(&args[2]);
                let re: Vec<f64> = broadcast(&cv.iter().map(|c| c.re).collect::<Vec<_>>(), nij);
                let im: Vec<f64> = broadcast(&cv.iter().map(|c| c.im).collect::<Vec<_>>(), nij);
                SparseMatrix::from_triplets(&iv, &jv, &re, Some(&im), m, n, class)
            } else {
                let sv = broadcast(&to_f64_vec(&args[2]), nij);
                SparseMatrix::from_triplets(&iv, &jv, &sv, None, m, n, class)
            };
            result
                .map(|s| vec![Array::sparse(s)])
                .map_err(|e| fm_interp::error::Signal::Error(fm_interp::error::InterpError::msg(e)))
        }
    }
}

/// Broadcast a length-1 vector to length `n`; pass others through.
fn broadcast(v: &[f64], n: usize) -> Vec<f64> {
    if v.len() == 1 && n > 1 {
        vec![v[0]; n]
    } else {
        v.to_vec()
    }
}

/// `full(X)`: convert sparse to dense (or return dense unchanged).
fn b_full(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "full")?;
    Ok(vec![densify(&args[0])])
}

/// `speye(n)` / `speye(m,n)`: sparse identity.
fn b_speye(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "speye")?;
    let m = scalar_arg(args, 0, "speye")? as usize;
    let n = if args.len() >= 2 {
        scalar_arg(args, 1, "speye")? as usize
    } else {
        m
    };
    Ok(vec![Array::sparse(SparseMatrix::eye(m, n))])
}

/// `spzeros(m[,n])`: an all-zero sparse matrix.
fn b_spzeros(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "spzeros")?;
    let m = scalar_arg(args, 0, "spzeros")? as usize;
    let n = if args.len() >= 2 {
        scalar_arg(args, 1, "spzeros")? as usize
    } else {
        m
    };
    Ok(vec![Array::sparse(SparseMatrix::zeros(m, n))])
}

/// `spones(S)`: sparse matrix with 1s in place of every structural nonzero.
fn b_spones(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "spones")?;
    let s = sparsify(&args[0]);
    let sm = s.as_sparse().expect("sparsify yields sparse");
    let (i, j, _v, _im) = sm.find_triplets();
    let iu: Vec<usize> = i.iter().map(|&x| x as usize).collect();
    let ju: Vec<usize> = j.iter().map(|&x| x as usize).collect();
    let ones = vec![1.0; iu.len()];
    let out = SparseMatrix::from_triplets(
        &iu,
        &ju,
        &ones,
        None,
        Some(sm.rows()),
        Some(sm.cols()),
        DataClass::Double,
    )
    .map_err(|e| fm_interp::error::Signal::Error(fm_interp::error::InterpError::msg(e)))?;
    Ok(vec![Array::sparse(out)])
}

/// `spdiags` — minimal `spdiags(B, d, m, n)` form that builds an `m`-by-`n`
/// sparse matrix whose diagonals `d` are the columns of `B`.
fn b_spdiags(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 4, "spdiags")?;
    let bdims = args[0].dims();
    let brows = bdims.first().copied().unwrap_or(0);
    let bdata = to_f64_vec(&args[0]); // column-major
    let diags: Vec<isize> = to_f64_vec(&args[1])
        .iter()
        .map(|&x| x.round() as isize)
        .collect();
    let m = scalar_arg(args, 2, "spdiags")? as usize;
    let n = scalar_arg(args, 3, "spdiags")? as usize;
    let mut iv = Vec::new();
    let mut jv = Vec::new();
    let mut sv = Vec::new();
    for (col, &d) in diags.iter().enumerate() {
        // Entries on diagonal d: (i, i+d) for valid i.
        for r in 0..brows {
            // MATLAB takes column `col` of B as the diagonal; element B(p, col)
            // maps to A(p - max(0, d) ... ). Use the standard placement.
            let (row, column) = if d >= 0 {
                (r, r + d as usize)
            } else {
                (r + (-d) as usize, r)
            };
            if row < m && column < n {
                let bval = bdata.get(col * brows + r).copied().unwrap_or(0.0);
                if bval != 0.0 {
                    iv.push(row + 1);
                    jv.push(column + 1);
                    sv.push(bval);
                }
            }
        }
    }
    let out = SparseMatrix::from_triplets(&iv, &jv, &sv, None, Some(m), Some(n), DataClass::Double)
        .map_err(|e| fm_interp::error::Signal::Error(fm_interp::error::InterpError::msg(e)))?;
    Ok(vec![Array::sparse(out)])
}

/// `sprandn(m, n, density)`: sparse matrix of standard-normal entries.
fn b_sprandn(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    sprand_impl(args, true)
}

/// `sprand(m, n, density)`: sparse matrix of uniform [0,1) entries.
fn b_sprand(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    sprand_impl(args, false)
}

fn sprand_impl(args: &[Array], normal: bool) -> Flow<Vec<Array>> {
    need(args, 1, "sprand")?;
    // sprand(S): match the sparsity pattern of S with new random values.
    if args.len() == 1 {
        if let Some(s) = args[0].as_sparse() {
            let (i, j, _v, _im) = s.find_triplets();
            let mut rng = rand::rng();
            let vals: Vec<f64> = (0..i.len())
                .map(|_| {
                    if normal {
                        StandardNormal.sample(&mut rng)
                    } else {
                        rng.random::<f64>()
                    }
                })
                .collect();
            let iu: Vec<usize> = i.iter().map(|&x| x as usize).collect();
            let ju: Vec<usize> = j.iter().map(|&x| x as usize).collect();
            let out = SparseMatrix::from_triplets(
                &iu,
                &ju,
                &vals,
                None,
                Some(s.rows()),
                Some(s.cols()),
                DataClass::Double,
            )
            .map_err(|e| fm_interp::error::Signal::Error(fm_interp::error::InterpError::msg(e)))?;
            return Ok(vec![Array::sparse(out)]);
        }
        return err("sprand: single-argument form requires a sparse matrix");
    }
    let m = scalar_arg(args, 0, "sprand")? as usize;
    let n = scalar_arg(args, 1, "sprand")? as usize;
    let density = args
        .get(2)
        .and_then(Array::as_f64)
        .unwrap_or(0.1)
        .clamp(0.0, 1.0);
    let target = ((m * n) as f64 * density).round() as usize;
    let mut rng = rand::rng();
    // Sample `target` distinct linear positions, set random values.
    let mut iv = Vec::with_capacity(target);
    let mut jv = Vec::with_capacity(target);
    let mut sv = Vec::with_capacity(target);
    let total = m * n;
    if total == 0 {
        return Ok(vec![Array::sparse(SparseMatrix::zeros(m, n))]);
    }
    let mut seen = std::collections::HashSet::new();
    let mut attempts = 0;
    while seen.len() < target && attempts < target * 20 + 100 {
        attempts += 1;
        let lin = rng.random_range(0..total);
        if !seen.insert(lin) {
            continue;
        }
        let row = lin % m;
        let col = lin / m;
        let val: f64 = if normal {
            StandardNormal.sample(&mut rng)
        } else {
            rng.random::<f64>()
        };
        iv.push(row + 1);
        jv.push(col + 1);
        sv.push(val);
    }
    let out = SparseMatrix::from_triplets(&iv, &jv, &sv, None, Some(m), Some(n), DataClass::Double)
        .map_err(|e| fm_interp::error::Signal::Error(fm_interp::error::InterpError::msg(e)))?;
    Ok(vec![Array::sparse(out)])
}

/// `nnz(X)`: number of nonzeros (works on sparse and dense).
fn b_nnz(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "nnz")?;
    let count = if let Some(s) = args[0].as_sparse() {
        s.nnz()
    } else if args[0].is_complex() {
        to_c64_vec(&args[0])
            .iter()
            .filter(|c| c.re != 0.0 || c.im != 0.0)
            .count()
    } else {
        to_f64_vec(&args[0]).iter().filter(|&&v| v != 0.0).count()
    };
    Ok(vec![Array::double(count as f64)])
}

/// `nzmax(X)`: storage capacity (== nnz for our model).
fn b_nzmax(i: &mut Interpreter, args: &[Array], n: usize) -> Flow<Vec<Array>> {
    b_nnz(i, args, n)
}

/// `nonzeros(X)`: the structural nonzero values as a column vector.
fn b_nonzeros(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "nonzeros")?;
    let s = sparsify(&args[0]);
    let sm = s.as_sparse().expect("sparsify yields sparse");
    let vals = sm.nonzeros();
    let k = vals.len();
    if sm.is_complex() {
        Ok(vec![build_complex(&[k, 1], vals)])
    } else {
        let re: Vec<f64> = vals.iter().map(|c| c.re).collect();
        Ok(vec![build_real(DataClass::Double, &[k, 1], re)])
    }
}

/// `issparse(X)`: true iff `X` is a sparse matrix.
fn b_issparse(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "issparse")?;
    Ok(vec![Array::bool(args[0].is_sparse())])
}

/// `spy(X)`: a no-op placeholder (returns nnz). A real plot is deferred.
fn b_spy(i: &mut Interpreter, args: &[Array], n: usize) -> Flow<Vec<Array>> {
    // No graphics for spy yet; behave like `nnz` so a script doesn't error.
    b_nnz(i, args, n)
}

/// `find` — sparse-aware, with the 1-output (linear indices), 2-output
/// (`[i,j]`), and 3-output (`[i,j,v]`) forms.
fn b_find(_i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "find")?;
    let a = &args[0];

    // Sparse: enumerate stored nonzeros directly in column-major order.
    if let Some(s) = a.as_sparse() {
        let (iv, jv, vv, im) = s.find_triplets();
        let k = iv.len();
        return Ok(match nargout {
            0 | 1 => {
                // Linear indices (column-major, 1-based).
                let rows = s.rows();
                let lin: Vec<f64> = iv
                    .iter()
                    .zip(&jv)
                    .map(|(&i, &j)| ((j as usize - 1) * rows + (i as usize - 1) + 1) as f64)
                    .collect();
                vec![build_real(DataClass::Double, &[k, 1], lin)]
            }
            2 => vec![
                build_real(DataClass::Double, &[k, 1], iv),
                build_real(DataClass::Double, &[k, 1], jv),
            ],
            _ => {
                let vals = if let Some(iiv) = im {
                    build_complex(
                        &[k, 1],
                        vv.iter().zip(iiv).map(|(&r, m)| C64::new(r, m)).collect(),
                    )
                } else {
                    build_real(DataClass::Double, &[k, 1], vv)
                };
                vec![
                    build_real(DataClass::Double, &[k, 1], iv),
                    build_real(DataClass::Double, &[k, 1], jv),
                    vals,
                ]
            }
        });
    }

    // Dense path: replicate the stdlib find, extended with the 2/3-output forms.
    let dims = a.dims();
    let rows = dims.first().copied().unwrap_or(0).max(1);
    let row_vec = dims.len() == 2 && dims[0] == 1;
    let (lin, vals_re, vals_im): (Vec<f64>, Vec<f64>, Option<Vec<f64>>) = if a.is_complex() {
        let cv = to_c64_vec(a);
        let mut lin = Vec::new();
        let mut re = Vec::new();
        let mut im = Vec::new();
        for (idx, c) in cv.iter().enumerate() {
            if c.re != 0.0 || c.im != 0.0 {
                lin.push((idx + 1) as f64);
                re.push(c.re);
                im.push(c.im);
            }
        }
        (lin, re, Some(im))
    } else {
        let dv = to_f64_vec(a);
        let mut lin = Vec::new();
        let mut re = Vec::new();
        for (idx, &v) in dv.iter().enumerate() {
            if v != 0.0 {
                lin.push((idx + 1) as f64);
                re.push(v);
            }
        }
        (lin, re, None)
    };
    let k = lin.len();
    let out_dims = if row_vec { vec![1, k] } else { vec![k, 1] };
    Ok(match nargout {
        0 | 1 => vec![build_real(DataClass::Double, &out_dims, lin)],
        2 => {
            let (i, j): (Vec<f64>, Vec<f64>) = lin
                .iter()
                .map(|&l| {
                    let l0 = l as usize - 1;
                    ((l0 % rows + 1) as f64, (l0 / rows + 1) as f64)
                })
                .unzip();
            vec![
                build_real(DataClass::Double, &out_dims, i),
                build_real(DataClass::Double, &out_dims, j),
            ]
        }
        _ => {
            let (i, j): (Vec<f64>, Vec<f64>) = lin
                .iter()
                .map(|&l| {
                    let l0 = l as usize - 1;
                    ((l0 % rows + 1) as f64, (l0 / rows + 1) as f64)
                })
                .unzip();
            let vals = match vals_im {
                Some(im) => build_complex(
                    &out_dims,
                    vals_re
                        .iter()
                        .zip(im)
                        .map(|(&r, m)| C64::new(r, m))
                        .collect(),
                ),
                None => build_real(DataClass::Double, &out_dims, vals_re),
            };
            vec![
                build_real(DataClass::Double, &out_dims, i),
                build_real(DataClass::Double, &out_dims, j),
                vals,
            ]
        }
    })
}
