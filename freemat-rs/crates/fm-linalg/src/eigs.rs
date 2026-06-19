//! Sparse eigenvalues (`eigs`) via a (non-restarted) **shift-invert Arnoldi**
//! iteration — the pure-Rust analogue of the ARPACK role, with no native
//! dependency. The sparse matrix is never densified: the operator is applied
//! either as a sparse mat-vec (`A·x`) or, for a shift `σ`, as a sparse solve
//! `(A − σI)⁻¹·x` using faer's sparse LU.
//!
//! For interior targets (`eigs(A, k, σ)`) shift-invert makes the eigenvalues
//! nearest `σ` dominant, so a single Arnoldi factorization of modest dimension
//! converges to them quickly. This is not implicitly-restarted ARPACK, so for
//! large clustered spectra in the plain-`A` modes it is best-effort; the
//! shift-invert path the corpus exercises is exact for small systems.

use faer::linalg::solvers::Solve;
use faer::{Mat, c64};
use fm_core::{Array, C64, SparseMatrix};

use super::{LinalgError, Result, build_from_c64, eig, to_c64};
use crate::sparse_solve::to_faer_c64;

/// A linear operator applied to a complex vector (the Arnoldi `OP`).
type OpFn<'a> = Box<dyn Fn(&[C64]) -> Vec<C64> + 'a>;

/// Which eigenvalues `eigs` should return.
#[derive(Clone, Copy)]
pub enum EigsWhich {
    /// The `k` eigenvalues nearest a real shift `σ` (shift-invert).
    Nearest(f64),
    /// Largest / smallest magnitude.
    LargeMag,
    SmallMag,
    /// Largest / smallest real part.
    LargeReal,
    SmallReal,
    /// Largest / smallest imaginary part.
    LargeImag,
    SmallImag,
}

impl EigsWhich {
    /// Sort key for an eigenvalue (ascending ⇒ selected first).
    fn key(self, l: C64) -> f64 {
        match self {
            EigsWhich::Nearest(s) => (l - C64::new(s, 0.0)).norm(),
            EigsWhich::LargeMag => -l.norm(),
            EigsWhich::SmallMag => l.norm(),
            EigsWhich::LargeReal => -l.re,
            EigsWhich::SmallReal => l.re,
            EigsWhich::LargeImag => -l.im,
            EigsWhich::SmallImag => l.im,
        }
    }
}

/// Hermitian inner product `⟨u, w⟩ = Σ conj(u) · w`.
fn cdot(u: &[C64], w: &[C64]) -> C64 {
    let mut s = C64::new(0.0, 0.0);
    for (a, b) in u.iter().zip(w) {
        s += a.conj() * b;
    }
    s
}

fn cnorm(w: &[C64]) -> f64 {
    cdot(w, w).re.max(0.0).sqrt()
}

/// `eigs(A, k, which)` — `k` selected eigenvalues of the sparse matrix `A`.
///
/// With one output returns the selected eigenvalues as a column vector; with two
/// returns `[V, D]` (selected eigenvectors in `V`'s columns, eigenvalues on the
/// diagonal of `D`).
///
/// # Errors
/// Returns [`LinalgError`] if `A` is not a square sparse matrix, or if a
/// shift-invert factorization of `A − σI` is singular.
pub fn eigs(a_arr: &Array, k: usize, which: EigsWhich, nargout: usize) -> Result<Vec<Array>> {
    let a = a_arr
        .as_sparse()
        .ok_or_else(|| LinalgError::new("eigs: input must be a sparse matrix"))?;
    let n = a.rows();
    if a.cols() != n {
        return Err(LinalgError::new("eigs: matrix must be square"));
    }
    if n == 0 {
        return Err(LinalgError::new("eigs: matrix must be non-empty"));
    }
    let k = k.clamp(1, n);
    // Krylov subspace dimension (ARPACK's default `ncv = max(2k+1, 20)`, capped
    // at n). For a small system this is the full space ⇒ exact eigenvalues.
    let m = (2 * k + 1).max(20).min(n);

    // Shift for the λ ↔ θ map: shift-invert for Nearest/SmallMag, plain A otherwise.
    let shift = match which {
        EigsWhich::Nearest(s) => Some(s),
        EigsWhich::SmallMag => Some(0.0),
        _ => None,
    };

    // Build the operator OP(x): (A − σI)⁻¹·x (shift-invert) or A·x.
    let op: OpFn<'_> = if let Some(sigma) = shift {
        let shifted = a
            .add_sub(&SparseMatrix::eye(n, n).scale_real(sigma), true)
            .map_err(LinalgError::new)?;
        let fa = to_faer_c64(&shifted)?;
        let lu = fa.sp_lu().map_err(|_| {
            LinalgError::new("eigs: (A - sigma*I) is singular; choose a different sigma")
        })?;
        Box::new(move |x: &[C64]| {
            let mut rhs = Mat::<c64>::from_fn(n, 1, |i, _| c64::new(x[i].re, x[i].im));
            lu.solve_in_place(rhs.as_mut());
            (0..n)
                .map(|i| C64::new(rhs[(i, 0)].re, rhs[(i, 0)].im))
                .collect()
        })
    } else {
        Box::new(move |x: &[C64]| a.matvec(x))
    };

    // Arnoldi factorization: orthonormal basis V (n×) and Hessenberg H (m×m).
    let inv = 1.0 / (n as f64).sqrt();
    let mut basis: Vec<Vec<C64>> = vec![vec![C64::new(inv, 0.0); n]];
    let mut h = vec![vec![C64::new(0.0, 0.0); m]; m];
    let mut dim = m;
    for j in 0..m {
        let mut w = op(&basis[j]);
        for (i, vi) in basis.iter().enumerate().take(j + 1) {
            let hij = cdot(vi, &w);
            h[i][j] = hij;
            for (wt, &vt) in w.iter_mut().zip(vi) {
                *wt -= hij * vt;
            }
        }
        let nrm = cnorm(&w);
        if j + 1 < m {
            if nrm < 1e-12 {
                // Invariant subspace found early.
                dim = j + 1;
                break;
            }
            h[j + 1][j] = C64::new(nrm, 0.0);
            let s = 1.0 / nrm;
            basis.push(w.iter().map(|&v| v * C64::new(s, 0.0)).collect());
        }
    }

    // Ritz values/vectors from the (small, dense) leading dim×dim Hessenberg.
    let mut hdata = vec![c64::new(0.0, 0.0); dim * dim];
    for (i, hi) in h.iter().enumerate().take(dim) {
        for (j, &v) in hi.iter().enumerate().take(dim) {
            hdata[i + j * dim] = c64::new(v.re, v.im);
        }
    }
    let h_arr = build_from_c64(dim, dim, hdata);

    let want_vecs = nargout >= 2;
    let parts = eig(&h_arr, if want_vecs { 2 } else { 1 })?;
    let theta: Vec<C64> = if want_vecs {
        // Diagonal of D.
        to_c64(&parts[1])
            .into_iter()
            .enumerate()
            .filter(|(idx, _)| idx % dim == idx / dim)
            .map(|(_, c)| C64::new(c.re, c.im))
            .collect()
    } else {
        to_c64(&parts[0])
            .into_iter()
            .map(|c| C64::new(c.re, c.im))
            .collect()
    };

    // Map Ritz value θ back to an eigenvalue λ of A.
    let lambda: Vec<C64> = theta
        .iter()
        .map(|&t| match shift {
            Some(s) if t.norm() > 0.0 => C64::new(s, 0.0) + C64::new(1.0, 0.0) / t,
            _ => t,
        })
        .collect();

    // Select k by the requested criterion.
    let mut order: Vec<usize> = (0..dim).collect();
    order.sort_by(|&x, &y| {
        which
            .key(lambda[x])
            .partial_cmp(&which.key(lambda[y]))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let kk = k.min(dim);
    order.truncate(kk);

    if !want_vecs {
        let picked: Vec<c64> = order
            .iter()
            .map(|&i| c64::new(lambda[i].re, lambda[i].im))
            .collect();
        return Ok(vec![build_from_c64(kk, 1, picked)]);
    }

    // [V, D]: back-transform the selected Ritz vectors y → V_arnoldi · y.
    let yvecs = to_c64(&parts[0]); // dim×dim, column-major
    let mut vdata = Vec::with_capacity(n * kk);
    for &col in &order {
        let mut acc = vec![C64::new(0.0, 0.0); n];
        for (r, vr) in basis.iter().enumerate().take(dim) {
            let y = yvecs[r + col * dim];
            let yc = C64::new(y.re, y.im);
            for (at, &vt) in acc.iter_mut().zip(vr) {
                *at += yc * vt;
            }
        }
        vdata.extend(acc.into_iter().map(|c| c64::new(c.re, c.im)));
    }
    let v_arr = build_from_c64(n, kk, vdata);
    let mut ddata = vec![c64::new(0.0, 0.0); kk * kk];
    for (i, &col) in order.iter().enumerate() {
        ddata[i + i * kk] = c64::new(lambda[col].re, lambda[col].im);
    }
    let d_arr = build_from_c64(kk, kk, ddata);
    Ok(vec![v_arr, d_arr])
}
