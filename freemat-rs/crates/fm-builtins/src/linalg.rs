//! Linear-algebra builtins wrapping [`fm_linalg`]: `inv`, `det`, `eig`, `svd`,
//! `lu`, `qr`, `chol`, `norm`, `rank`, `pinv`, `trace`, `transpose`, `mtimes`.

use fm_core::{Array, DataClass, SparseMatrix};
use fm_interp::error::{Flow, InterpError, Signal};
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};
use fm_linalg::{LinalgError, NormKind};

use crate::util::need;

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("inv", b_inv);
    table.add_builtin("det", b_det);
    table.add_builtin("eig", b_eig);
    table.add_builtin("eigs", b_eigs);
    table.add_builtin("svd", b_svd);
    table.add_builtin("lu", b_lu);
    table.add_builtin("qr", b_qr);
    table.add_builtin("chol", b_chol);
    table.add_builtin("norm", b_norm);
    table.add_builtin("xnrm2", b_xnrm2);
    table.add_builtin("rank", b_rank);
    table.add_builtin("pinv", b_pinv);
    table.add_builtin("trace", b_trace);
    table.add_builtin("cond", b_cond);
    table.add_builtin("rcond", b_rcond);
    table.add_builtin("rref", b_rref);
    table.add_builtin("kron", b_kron);
    table.add_builtin("null", b_null);
    table.add_builtin("orth", b_orth);
    table.add_builtin("tril", b_tril);
    table.add_builtin("triu", b_triu);
    table.add_builtin("expm", b_expm);
}

fn b_expm(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "expm")?;
    Ok(vec![fm_linalg::expm(&args[0]).map_err(wrap)?])
}

fn b_cond(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "cond")?;
    Ok(vec![fm_linalg::cond(&args[0]).map_err(wrap)?])
}

fn b_rcond(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "rcond")?;
    Ok(vec![fm_linalg::rcond(&args[0]).map_err(wrap)?])
}

fn b_rref(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "rref")?;
    Ok(vec![fm_linalg::rref(&args[0]).map_err(wrap)?])
}

fn b_kron(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "kron")?;
    Ok(vec![fm_linalg::kron(&args[0], &args[1]).map_err(wrap)?])
}

fn b_null(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "null")?;
    Ok(vec![fm_linalg::null(&args[0]).map_err(wrap)?])
}

fn b_orth(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "orth")?;
    Ok(vec![fm_linalg::orth(&args[0]).map_err(wrap)?])
}

fn b_tril(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "tril")?;
    let k = args.get(1).and_then(Array::as_f64).unwrap_or(0.0) as i64;
    Ok(vec![fm_linalg::tril(&args[0], k).map_err(wrap)?])
}

fn b_triu(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "triu")?;
    let k = args.get(1).and_then(Array::as_f64).unwrap_or(0.0) as i64;
    Ok(vec![fm_linalg::triu(&args[0], k).map_err(wrap)?])
}

fn wrap(e: LinalgError) -> Signal {
    Signal::Error(InterpError::msg(e.message))
}

fn b_inv(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "inv")?;
    Ok(vec![fm_linalg::inv(&args[0]).map_err(wrap)?])
}

fn b_det(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "det")?;
    Ok(vec![fm_linalg::det(&args[0]).map_err(wrap)?])
}

fn b_eig(_i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "eig")?;
    // `eig(A, B)` — generalized eigenproblem of the pencil (A, B) — but only when
    // the second argument is a numeric matrix. A char second argument is an
    // option string (e.g. `eig(A, 'nobalance')`), handled by the standard path.
    if let Some(b) = args.get(1)
        && b.class() != DataClass::Char
    {
        return fm_linalg::eig_gen(&args[0], b, nargout).map_err(wrap);
    }
    fm_linalg::eig(&args[0], nargout).map_err(wrap)
}

fn b_svd(_i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "svd")?;
    fm_linalg::svd(&args[0], nargout).map_err(wrap)
}

/// `eigs(A, k, sigma)` — a subset of the (sparse) matrix `A`'s eigenvalues.
/// `k` defaults to 6; the third argument is either a numeric shift `sigma`
/// (eigenvalues nearest it) or a mode string (`'lm'`/`'sm'`/`'lr'`/`'sr'`/
/// `'li'`/`'si'`), defaulting to largest magnitude (`'lm'`).
fn b_eigs(_i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    use fm_linalg::EigsWhich;
    need(args, 1, "eigs")?;
    let k = args
        .get(1)
        .and_then(Array::as_f64)
        .map_or(6, |v| v as usize);
    let which = match args.get(2) {
        None => EigsWhich::LargeMag,
        Some(s) if s.class() == DataClass::Char => {
            match s.as_string().unwrap_or_default().to_lowercase().as_str() {
                "sm" => EigsWhich::SmallMag,
                "lr" | "la" => EigsWhich::LargeReal,
                "sr" | "sa" => EigsWhich::SmallReal,
                "li" => EigsWhich::LargeImag,
                "si" => EigsWhich::SmallImag,
                _ => EigsWhich::LargeMag,
            }
        }
        Some(s) => EigsWhich::Nearest(s.as_f64().unwrap_or(0.0)),
    };
    fm_linalg::eigs(&args[0], k, which, nargout.max(1)).map_err(wrap)
}

fn b_lu(_i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "lu")?;
    let a = &args[0];
    // FreeMat's sparse LU (SparseLUDecompose) only handles square double /
    // dcomplex matrices; everything else errors out (the dense path, by
    // contrast, supports rectangular and all numeric classes).
    if a.is_sparse() {
        if a.class() != DataClass::Double {
            return Err(Signal::Error(InterpError::msg(
                "lu: sparse LU is only supported for double and dcomplex matrices",
            )));
        }
        let dims = a.dims();
        if dims.len() != 2 || dims[0] != dims[1] {
            return Err(Signal::Error(InterpError::msg(
                "lu: sparse LU is only supported for square matrices",
            )));
        }
        let sp = a.as_sparse().expect("is_sparse() but as_sparse() is None");
        if sp.is_complex() {
            return Err(Signal::Error(InterpError::msg(
                "lu: complex sparse LU is not yet supported",
            )));
        }
        return sparse_lu(sp, nargout);
    }
    fm_linalg::lu(a, nargout).map_err(wrap)
}

/// Sparse LU factorization for square, real, double sparse matrices.
///
/// Produces `[L, U, P, Q, R]` such that `L*U = P*R*A*Q`, matching FreeMat's
/// (UMFPACK-backed) output conventions where `P` and `Q` are permutation
/// **vectors** (1-based) and `R` is a diagonal row-scaling matrix.
///
/// Convention used here (deterministic):
/// - `R` = identity row scaling (`speye(n)`).
/// - `Q` = identity column permutation (`1:n`); no fill-reducing reorder.
/// - `P` = the partial-pivoting row permutation (largest-magnitude pivot,
///   ties broken by smallest row index — fully deterministic).
///
/// With `R = I` and `Q = I` the identity collapses to `L*U = P*A`, so in the
/// doc example `b = r*a == a` and `full(b(p,q)) == full(P*A) == full(L*U)`.
///
/// The factorization is left-looking (Gilbert–Peierls style): it keeps `L` and
/// `U` sparse, using a single O(n) dense workspace per column (the matrix
/// itself is never densified).
fn sparse_lu(a: &SparseMatrix, nargout: usize) -> Flow<Vec<Array>> {
    let n = a.rows();
    // `perm[i]` = original row index currently sitting in position `i`.
    let mut perm: Vec<usize> = (0..n).collect();
    // `pos[r]` = current position of original row `r` (inverse of `perm`).
    let mut pos: Vec<usize> = (0..n).collect();

    // L and U accumulated as triplets in *final* (permuted) row order.
    // L is unit-lower-triangular (implicit unit diagonal stored explicitly).
    let mut l_i: Vec<usize> = Vec::new();
    let mut l_j: Vec<usize> = Vec::new();
    let mut l_v: Vec<f64> = Vec::new();
    let mut u_i: Vec<usize> = Vec::new();
    let mut u_j: Vec<usize> = Vec::new();
    let mut u_v: Vec<f64> = Vec::new();

    let col_ptr = a.col_ptr();
    let row_idx = a.row_idx();
    let re = a.re();

    // Dense workspace `x`, indexed by *position* (permuted row index).
    let mut x = vec![0.0f64; n];

    for j in 0..n {
        // Scatter column j of A into x, indexed by current position.
        for v in x.iter_mut() {
            *v = 0.0;
        }
        for k in col_ptr[j]..col_ptr[j + 1] {
            let orig_row = row_idx[k];
            x[pos[orig_row]] = re[k];
        }

        // Forward-substitution against the already-computed columns of L:
        // for each pivot row r < j (in position order), eliminate using the
        // multipliers stored in L's column r.
        for r in 0..j {
            let xr = x[r];
            if xr == 0.0 {
                continue;
            }
            // x[i] -= L(i, r) * x[r] for i > r.
            for (idx, &lj) in l_j.iter().enumerate() {
                if lj == r && l_i[idx] > r {
                    x[l_i[idx]] -= l_v[idx] * xr;
                }
            }
        }

        // Partial pivoting: pick the largest |x[i]| among i >= j.
        let mut piv = j;
        let mut best = x[j].abs();
        for (off, &xi) in x[(j + 1)..n].iter().enumerate() {
            if xi.abs() > best {
                best = xi.abs();
                piv = j + 1 + off;
            }
        }
        if best == 0.0 {
            return Err(Signal::Error(InterpError::msg(
                "lu: sparse matrix is singular to working precision",
            )));
        }

        // Swap positions j and piv (both the value workspace and the
        // permutation bookkeeping, plus already-emitted L entries).
        if piv != j {
            x.swap(j, piv);
            let (oj, op) = (perm[j], perm[piv]);
            perm.swap(j, piv);
            pos[oj] = piv;
            pos[op] = j;
            for ri in l_i.iter_mut() {
                if *ri == j {
                    *ri = piv;
                } else if *ri == piv {
                    *ri = j;
                }
            }
        }

        // Emit U's column j (rows 0..=j) and L's column j (rows j..n, divided
        // by the pivot, with a unit diagonal entry).
        let pivot = x[j];
        for (i, &xi) in x[..=j].iter().enumerate() {
            if xi != 0.0 {
                u_i.push(i);
                u_j.push(j);
                u_v.push(xi);
            }
        }
        l_i.push(j);
        l_j.push(j);
        l_v.push(1.0);
        for (off, &xi) in x[(j + 1)..n].iter().enumerate() {
            if xi != 0.0 {
                l_i.push(j + 1 + off);
                l_j.push(j);
                l_v.push(xi / pivot);
            }
        }
    }

    // Build sparse outputs. Triplet indices are 0-based positions; convert to
    // 1-based for `from_triplets`.
    let to_one = |v: &[usize]| -> Vec<usize> { v.iter().map(|&x| x + 1).collect() };
    let l = SparseMatrix::from_triplets(
        &to_one(&l_i),
        &to_one(&l_j),
        &l_v,
        None,
        Some(n),
        Some(n),
        DataClass::Double,
    )
    .map_err(|e| Signal::Error(InterpError::msg(e)))?;
    let u = SparseMatrix::from_triplets(
        &to_one(&u_i),
        &to_one(&u_j),
        &u_v,
        None,
        Some(n),
        Some(n),
        DataClass::Double,
    )
    .map_err(|e| Signal::Error(InterpError::msg(e)))?;

    // P as a 1-based column permutation vector: P*A takes row perm[i] to row i,
    // i.e. p(i) = perm[i] + 1.
    let p_vec: Vec<f64> = perm.iter().map(|&r| (r + 1) as f64).collect();
    // Q = identity, R = identity (see convention above).
    let q_vec: Vec<f64> = (1..=n).map(|i| i as f64).collect();

    let mut out: Vec<Array> = Vec::with_capacity(nargout.max(1));
    out.push(Array::sparse(l));
    if nargout >= 2 {
        out.push(Array::sparse(u));
    }
    if nargout >= 3 {
        out.push(Array::double_matrix(&[n, 1], p_vec));
    }
    if nargout >= 4 {
        out.push(Array::double_matrix(&[n, 1], q_vec));
    }
    if nargout >= 5 {
        out.push(Array::sparse(SparseMatrix::eye(n, n)));
    }
    Ok(out)
}

fn b_qr(_i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "qr")?;
    // `qr(a, 0)` requests the economy (compact) decomposition.
    let economy = args.get(1).and_then(Array::as_f64) == Some(0.0);
    fm_linalg::qr(&args[0], nargout, economy).map_err(wrap)
}

fn b_chol(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "chol")?;
    Ok(vec![fm_linalg::chol(&args[0]).map_err(wrap)?])
}

/// `norm(A)` / `norm(A, p)` where `p` is 1, 2, `inf`, or `'fro'`.
fn b_norm(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "norm")?;
    let kind = match args.get(1) {
        None => NormKind::Two,
        Some(p) => {
            if let Some(s) = p.as_string() {
                match s.to_lowercase().as_str() {
                    "fro" => NormKind::Fro,
                    "inf" => NormKind::Inf,
                    _ => return Err(Signal::Error(InterpError::msg("norm: unknown norm type"))),
                }
            } else {
                let v = p.as_f64().unwrap_or(2.0);
                if v == f64::INFINITY {
                    NormKind::Inf
                } else if v == f64::NEG_INFINITY {
                    NormKind::NegInf
                } else if v == 1.0 {
                    NormKind::One
                } else if v == 2.0 {
                    NormKind::Two
                } else {
                    // General p-norm (vectors only; matrices error in fm_linalg).
                    NormKind::P(v)
                }
            }
        }
    };
    Ok(vec![fm_linalg::norm(&args[0], kind).map_err(wrap)?])
}

/// `xnrm2(A)` — Euclidean (2-)norm of the flattened array (FreeMat's BLAS
/// `?nrm2` wrapper, used by the `wbtest_near` whitebox helper). Equivalent to
/// `norm(A(:))` / the Frobenius norm; real for real input.
fn b_xnrm2(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "xnrm2")?;
    Ok(vec![
        fm_linalg::norm(&args[0], NormKind::Fro).map_err(wrap)?,
    ])
}

fn b_rank(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "rank")?;
    Ok(vec![fm_linalg::rank(&args[0]).map_err(wrap)?])
}

fn b_pinv(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "pinv")?;
    Ok(vec![fm_linalg::pinv(&args[0]).map_err(wrap)?])
}

/// `trace(A)` — sum of the main diagonal.
fn b_trace(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "trace")?;
    let dims = args[0].dims();
    if dims.len() != 2 {
        return Err(Signal::Error(InterpError::msg("trace: input must be 2-D")));
    }
    let (r, c) = (dims[0], dims[1]);
    let data = to_f64_vec(&args[0]);
    let k = r.min(c);
    let t: f64 = (0..k).map(|i| data[i + i * r]).sum();
    Ok(vec![build_real(DataClass::Double, &[1, 1], vec![t])])
}
