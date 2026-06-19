//! Native sparse linear solves for `A\b` (`mldivide`) and `x A = b`
//! (`mrdivide`). The sparse matrix `A` is fed to faer's sparse LU (square) or
//! sparse QR least-squares (rectangular) **without densifying** — only the
//! dense right-hand side is materialised, and the result is dense (matching
//! FreeMat, whose sparse solve also returns a dense array).

use faer::linalg::solvers::{Solve, SolveLstsq};
use faer::sparse::{SparseColMat, Triplet};
use faer::{Mat, c64};
use fm_core::SparseMatrix;

use super::{LinalgError, MatData, Result, finish};

/// Build a faer complex sparse matrix from our canonical CSC arrays.
fn to_faer_c64(s: &SparseMatrix) -> Result<SparseColMat<usize, c64>> {
    let (rows, cols) = (s.rows(), s.cols());
    let (re, im, row_idx, col_ptr) = (s.re(), s.im(), s.row_idx(), s.col_ptr());
    let mut entries: Vec<Triplet<usize, usize, c64>> = Vec::with_capacity(re.len());
    for j in 0..cols {
        for k in col_ptr[j]..col_ptr[j + 1] {
            entries.push(Triplet::new(
                row_idx[k],
                j,
                c64::new(re[k], im.map_or(0.0, |m| m[k])),
            ));
        }
    }
    SparseColMat::try_new_from_triplets(rows, cols, &entries)
        .map_err(|_| LinalgError::new("sparse solve: failed to assemble matrix"))
}

/// Build a faer real sparse matrix from our canonical CSC arrays.
fn to_faer_f64(s: &SparseMatrix) -> Result<SparseColMat<usize, f64>> {
    let (rows, cols) = (s.rows(), s.cols());
    let (re, row_idx, col_ptr) = (s.re(), s.row_idx(), s.col_ptr());
    let mut entries: Vec<Triplet<usize, usize, f64>> = Vec::with_capacity(re.len());
    for j in 0..cols {
        for k in col_ptr[j]..col_ptr[j + 1] {
            entries.push(Triplet::new(row_idx[k], j, re[k]));
        }
    }
    SparseColMat::try_new_from_triplets(rows, cols, &entries)
        .map_err(|_| LinalgError::new("sparse solve: failed to assemble matrix"))
}

/// Solve `A x = b` with sparse `A` (kept sparse) and a dense right-hand side.
/// Square `A` uses sparse LU; rectangular `A` uses sparse QR least-squares.
pub(crate) fn sp_mldivide(a: &SparseMatrix, b: &super::Array) -> Result<super::Array> {
    let (rows, cols) = (a.rows(), a.cols());
    let bm = MatData::from_array(b)?;
    if rows != bm.rows {
        return Err(LinalgError::new(format!(
            "dimensions disagree in \\ ({}x{} \\ {}x{})",
            rows, cols, bm.rows, bm.cols
        )));
    }
    let square = rows == cols;
    let complex = a.is_complex() || bm.complex;

    if complex {
        let fa = to_faer_c64(a)?;
        let mut rhs = Mat::<c64>::from_fn(bm.rows, bm.cols, |i, j| bm.data[i + j * bm.rows]);
        if square {
            let lu = fa
                .sp_lu()
                .map_err(|_| LinalgError::new("sparse \\: matrix is singular or LU failed"))?;
            lu.solve_in_place(rhs.as_mut());
        } else {
            let qr = fa
                .sp_qr()
                .map_err(|_| LinalgError::new("sparse \\: QR factorization failed"))?;
            qr.solve_lstsq_in_place(rhs.as_mut());
        }
        // The solution occupies the first `cols` rows of each column.
        let mut data = Vec::with_capacity(cols * bm.cols);
        for j in 0..bm.cols {
            for i in 0..cols {
                data.push(rhs[(i, j)]);
            }
        }
        Ok(finish(cols, bm.cols, data, true))
    } else {
        let fa = to_faer_f64(a)?;
        let mut rhs = Mat::<f64>::from_fn(bm.rows, bm.cols, |i, j| bm.data[i + j * bm.rows].re);
        if square {
            let lu = fa
                .sp_lu()
                .map_err(|_| LinalgError::new("sparse \\: matrix is singular or LU failed"))?;
            lu.solve_in_place(rhs.as_mut());
        } else {
            let qr = fa
                .sp_qr()
                .map_err(|_| LinalgError::new("sparse \\: QR factorization failed"))?;
            qr.solve_lstsq_in_place(rhs.as_mut());
        }
        let mut data = Vec::with_capacity(cols * bm.cols);
        for j in 0..bm.cols {
            for i in 0..cols {
                data.push(c64::new(rhs[(i, j)], 0.0));
            }
        }
        Ok(finish(cols, bm.cols, data, false))
    }
}
