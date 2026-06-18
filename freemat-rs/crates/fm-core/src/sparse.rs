//! Sparse matrix value — **CSC (compressed sparse column)** storage, matching
//! FreeMat's CCS (compressed-column) sparse layout.
//!
//! # Representation
//! A single [`SparseMatrix`] type covers every sparse class FreeMat supports
//! (real `double`/`single`, integer, `logical`, and complex). Values are stored
//! as `f64` (plus an optional parallel imaginary `f64` buffer for complex),
//! while the **logical element class** is tracked in [`SparseMatrix::class`] so
//! [`SparseMatrix::to_dense`] restores the correct dense class. This keeps one
//! storage path for all classes — the same trick FreeMat's templated CCS uses,
//! collapsed to `f64` because every numeric class round-trips losslessly within
//! the magnitudes the test corpus exercises.
//!
//! Stored entries are kept in **column order, ascending row within a column**,
//! with no explicit zeros (canonical CSC). Complex sparse and sparse
//! eigensolvers (ARPACK) are partly supported here for round-tripping; heavy
//! complex linear algebra is densified by the interpreter/builtins layer.

use crate::class::DataClass;
use crate::complex::C64;

/// A sparse matrix in CSC (compressed-sparse-column) form.
///
/// `col_ptr` has length `cols + 1`; entries of column `j` are the slice
/// `row_idx[col_ptr[j]..col_ptr[j+1]]` with values `re[..]` (+ `im[..]` when
/// complex). Rows within a column are ascending and unique.
#[derive(Debug, Clone, PartialEq)]
pub struct SparseMatrix {
    rows: usize,
    cols: usize,
    /// `[rows, cols]`, cached so [`Array::shape`] can borrow a slice.
    dims_cache: [usize; 2],
    /// The logical element class (`Double`, `Float`, `Int32`, `Bool`, …).
    class: DataClass,
    /// `cols + 1` column pointers.
    col_ptr: Vec<usize>,
    /// Row index of each stored entry.
    row_idx: Vec<usize>,
    /// Real part of each stored entry.
    re: Vec<f64>,
    /// Imaginary part of each stored entry (only `Some` for complex matrices).
    im: Option<Vec<f64>>,
}

impl SparseMatrix {
    /// The single private constructor — keeps `dims_cache` consistent.
    fn build(
        rows: usize,
        cols: usize,
        class: DataClass,
        col_ptr: Vec<usize>,
        row_idx: Vec<usize>,
        re: Vec<f64>,
        im: Option<Vec<f64>>,
    ) -> Self {
        Self {
            rows,
            cols,
            dims_cache: [rows, cols],
            class,
            col_ptr,
            row_idx,
            re,
            im,
        }
    }

    /// Number of rows.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Number of columns.
    #[must_use]
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// `[rows, cols]`.
    #[must_use]
    pub fn dims(&self) -> [usize; 2] {
        [self.rows, self.cols]
    }

    /// `[rows, cols]` as a borrowable slice (for zero-alloc shape access).
    #[must_use]
    pub fn dims_slice(&self) -> &[usize] {
        &self.dims_cache
    }

    /// The logical element class.
    #[must_use]
    pub fn class(&self) -> DataClass {
        self.class
    }

    /// Whether this matrix carries an imaginary part.
    #[must_use]
    pub fn is_complex(&self) -> bool {
        self.im.is_some()
    }

    /// Number of explicitly stored (structural) nonzeros.
    #[must_use]
    pub fn nnz(&self) -> usize {
        self.row_idx.len()
    }

    /// Column pointers (length `cols + 1`).
    #[must_use]
    pub fn col_ptr(&self) -> &[usize] {
        &self.col_ptr
    }

    /// Row indices of stored entries.
    #[must_use]
    pub fn row_idx(&self) -> &[usize] {
        &self.row_idx
    }

    /// Real parts of stored entries.
    #[must_use]
    pub fn re(&self) -> &[f64] {
        &self.re
    }

    /// Imaginary parts of stored entries, if complex.
    #[must_use]
    pub fn im(&self) -> Option<&[f64]> {
        self.im.as_deref()
    }

    /// Build a CSC matrix from raw, already-sorted CSC arrays. Drops any stored
    /// entry that is structurally zero (re == 0 and im == 0).
    fn from_csc_raw(
        rows: usize,
        cols: usize,
        class: DataClass,
        col_ptr: Vec<usize>,
        row_idx: Vec<usize>,
        re: Vec<f64>,
        im: Option<Vec<f64>>,
    ) -> Self {
        Self::build(rows, cols, class, col_ptr, row_idx, re, im).pruned()
    }

    /// Remove explicit zeros, keeping canonical CSC form.
    fn pruned(self) -> Self {
        let complex = self.im.is_some();
        let mut col_ptr = Vec::with_capacity(self.cols + 1);
        let mut row_idx = Vec::with_capacity(self.row_idx.len());
        let mut re = Vec::with_capacity(self.re.len());
        let mut im: Option<Vec<f64>> = if complex { Some(Vec::new()) } else { None };
        col_ptr.push(0);
        for j in 0..self.cols {
            for k in self.col_ptr[j]..self.col_ptr[j + 1] {
                let r = self.re[k];
                let m = self.im.as_ref().map_or(0.0, |v| v[k]);
                if r != 0.0 || m != 0.0 {
                    row_idx.push(self.row_idx[k]);
                    re.push(r);
                    if let Some(iv) = im.as_mut() {
                        iv.push(m);
                    }
                }
            }
            col_ptr.push(row_idx.len());
        }
        Self::build(self.rows, self.cols, self.class, col_ptr, row_idx, re, im)
    }

    /// Construct a sparse matrix from a **column-major dense** value buffer
    /// (`re` + optional `im`), of the given `class`. Stores only nonzeros.
    #[must_use]
    pub fn from_dense_cols(
        rows: usize,
        cols: usize,
        class: DataClass,
        re: &[f64],
        im: Option<&[f64]>,
    ) -> Self {
        let complex = im.is_some();
        let mut col_ptr = Vec::with_capacity(cols + 1);
        let mut row_idx = Vec::new();
        let mut vre = Vec::new();
        let mut vim: Option<Vec<f64>> = if complex { Some(Vec::new()) } else { None };
        col_ptr.push(0);
        for j in 0..cols {
            for r in 0..rows {
                let lin = j * rows + r;
                let a = re.get(lin).copied().unwrap_or(0.0);
                let b = im.and_then(|v| v.get(lin).copied()).unwrap_or(0.0);
                if a != 0.0 || b != 0.0 {
                    row_idx.push(r);
                    vre.push(a);
                    if let Some(iv) = vim.as_mut() {
                        iv.push(b);
                    }
                }
            }
            col_ptr.push(row_idx.len());
        }
        Self::build(rows, cols, class, col_ptr, row_idx, vre, vim)
    }

    /// Construct from `(i, j, v)` triplets (1-based row/col like MATLAB).
    /// **Duplicate `(i, j)` entries are summed** (MATLAB `sparse` semantics).
    /// `rows`/`cols` default to the max index when `None`.
    ///
    /// # Errors
    /// Returns `Err` if any index is < 1 or exceeds the explicit dimensions.
    pub fn from_triplets(
        i: &[usize],
        j: &[usize],
        re: &[f64],
        im: Option<&[f64]>,
        rows: Option<usize>,
        cols: Option<usize>,
        class: DataClass,
    ) -> Result<Self, String> {
        let n = re.len();
        if i.len() != n || j.len() != n {
            return Err("sparse: i, j, v must have the same length".into());
        }
        let rows = rows.unwrap_or_else(|| i.iter().copied().max().unwrap_or(0));
        let cols = cols.unwrap_or_else(|| j.iter().copied().max().unwrap_or(0));
        // Accumulate into a dense-ish per-column map keyed by row (sum dups).
        let complex = im.is_some();
        let mut cols_map: Vec<std::collections::BTreeMap<usize, (f64, f64)>> =
            vec![std::collections::BTreeMap::new(); cols];
        for k in 0..n {
            let ri = i[k];
            let cj = j[k];
            if ri < 1 || cj < 1 || ri > rows || cj > cols {
                return Err(format!(
                    "sparse: index ({ri},{cj}) out of range for {rows}x{cols}"
                ));
            }
            let entry = cols_map[cj - 1].entry(ri - 1).or_insert((0.0, 0.0));
            entry.0 += re[k];
            if let Some(iv) = im {
                entry.1 += iv[k];
            }
        }
        let mut col_ptr = Vec::with_capacity(cols + 1);
        let mut row_idx = Vec::new();
        let mut vre = Vec::new();
        let mut vim: Option<Vec<f64>> = if complex { Some(Vec::new()) } else { None };
        col_ptr.push(0);
        for col in &cols_map {
            for (&r, &(a, b)) in col {
                row_idx.push(r);
                vre.push(a);
                if let Some(iv) = vim.as_mut() {
                    iv.push(b);
                }
            }
            col_ptr.push(row_idx.len());
        }
        Ok(Self::from_csc_raw(
            rows, cols, class, col_ptr, row_idx, vre, vim,
        ))
    }

    /// Identity matrix `speye(n)` (`m`×`n` general form): ones on the diagonal.
    #[must_use]
    pub fn eye(rows: usize, cols: usize) -> Self {
        let d = rows.min(cols);
        let mut col_ptr = Vec::with_capacity(cols + 1);
        let mut row_idx = Vec::with_capacity(d);
        let mut re = Vec::with_capacity(d);
        col_ptr.push(0);
        for j in 0..cols {
            if j < d {
                row_idx.push(j);
                re.push(1.0);
            }
            col_ptr.push(row_idx.len());
        }
        Self::build(rows, cols, DataClass::Double, col_ptr, row_idx, re, None)
    }

    /// An all-zero sparse matrix.
    #[must_use]
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self::build(
            rows,
            cols,
            DataClass::Double,
            vec![0; cols + 1],
            Vec::new(),
            Vec::new(),
            None,
        )
    }

    /// The value at `(row, col)` (0-based) as `(re, im)`; `(0, 0)` if absent.
    #[must_use]
    pub fn get(&self, row: usize, col: usize) -> (f64, f64) {
        if row >= self.rows || col >= self.cols {
            return (0.0, 0.0);
        }
        let lo = self.col_ptr[col];
        let hi = self.col_ptr[col + 1];
        for k in lo..hi {
            if self.row_idx[k] == row {
                let m = self.im.as_ref().map_or(0.0, |v| v[k]);
                return (self.re[k], m);
            }
            if self.row_idx[k] > row {
                break;
            }
        }
        (0.0, 0.0)
    }

    /// The value at column-major linear index `lin` (0-based) as `(re, im)`.
    #[must_use]
    pub fn get_linear(&self, lin: usize) -> (f64, f64) {
        let col = lin / self.rows;
        let row = lin % self.rows;
        self.get(row, col)
    }

    /// Dense column-major real-and-imaginary buffers `(re, im)`. `im` is `None`
    /// for real matrices.
    #[must_use]
    pub fn to_dense_cols(&self) -> (Vec<f64>, Option<Vec<f64>>) {
        let n = self.rows * self.cols;
        let mut re = vec![0.0; n];
        let mut im = if self.im.is_some() {
            Some(vec![0.0; n])
        } else {
            None
        };
        for j in 0..self.cols {
            for k in self.col_ptr[j]..self.col_ptr[j + 1] {
                let lin = j * self.rows + self.row_idx[k];
                re[lin] = self.re[k];
                if let (Some(dst), Some(src)) = (im.as_mut(), self.im.as_ref()) {
                    dst[lin] = src[k];
                }
            }
        }
        (re, im)
    }

    /// `[i, j, v]` triplets for `find` (1-based, column-major order). Returns
    /// `(rows_vec, cols_vec, re_vec, im_vec)`.
    #[must_use]
    pub fn find_triplets(&self) -> (Vec<f64>, Vec<f64>, Vec<f64>, Option<Vec<f64>>) {
        let mut iv = Vec::with_capacity(self.nnz());
        let mut jv = Vec::with_capacity(self.nnz());
        let mut vv = Vec::with_capacity(self.nnz());
        let mut mv: Option<Vec<f64>> = if self.im.is_some() {
            Some(Vec::with_capacity(self.nnz()))
        } else {
            None
        };
        for j in 0..self.cols {
            for k in self.col_ptr[j]..self.col_ptr[j + 1] {
                iv.push((self.row_idx[k] + 1) as f64);
                jv.push((j + 1) as f64);
                vv.push(self.re[k]);
                if let (Some(dst), Some(src)) = (mv.as_mut(), self.im.as_ref()) {
                    dst.push(src[k]);
                }
            }
        }
        (iv, jv, vv, mv)
    }

    /// Stored nonzero values, column-major, as complex (for `nonzeros`).
    #[must_use]
    pub fn nonzeros(&self) -> Vec<C64> {
        (0..self.nnz())
            .map(|k| {
                let m = self.im.as_ref().map_or(0.0, |v| v[k]);
                C64::new(self.re[k], m)
            })
            .collect()
    }

    /// Transpose (CSC transpose = CSR-build then re-CSC). Non-conjugate.
    #[must_use]
    pub fn transpose(&self) -> Self {
        let complex = self.im.is_some();
        // Count entries per row (which become columns of the result).
        let mut counts = vec![0usize; self.rows + 1];
        for &r in &self.row_idx {
            counts[r + 1] += 1;
        }
        for r in 0..self.rows {
            counts[r + 1] += counts[r];
        }
        let nnz = self.nnz();
        let mut row_idx = vec![0usize; nnz];
        let mut re = vec![0.0; nnz];
        let mut im = if complex { Some(vec![0.0; nnz]) } else { None };
        let mut next = counts.clone();
        for j in 0..self.cols {
            for k in self.col_ptr[j]..self.col_ptr[j + 1] {
                let r = self.row_idx[k];
                let dst = next[r];
                row_idx[dst] = j;
                re[dst] = self.re[k];
                if let (Some(d), Some(s)) = (im.as_mut(), self.im.as_ref()) {
                    d[dst] = s[k];
                }
                next[r] += 1;
            }
        }
        Self::build(self.cols, self.rows, self.class, counts, row_idx, re, im)
    }

    /// Sparse · sparse matrix multiply (`A * B`). Result is complex iff either
    /// operand is. Result class follows `class`-promotion to `Double` unless
    /// both are the same non-double class.
    ///
    /// # Errors
    /// Returns `Err` on an inner-dimension mismatch.
    pub fn matmul(&self, b: &SparseMatrix) -> Result<SparseMatrix, String> {
        if self.cols != b.rows {
            return Err(format!(
                "matrix dimensions must agree: {}x{} * {}x{}",
                self.rows, self.cols, b.rows, b.cols
            ));
        }
        let complex = self.im.is_some() || b.im.is_some();
        let m = self.rows;
        let p = b.cols;
        let class = result_class(self.class, b.class);
        let mut col_ptr = Vec::with_capacity(p + 1);
        let mut row_idx = Vec::new();
        let mut vre = Vec::new();
        let mut vim: Option<Vec<f64>> = if complex { Some(Vec::new()) } else { None };
        col_ptr.push(0);
        // Accumulator over result rows for the current column.
        let mut acc_re = vec![0.0f64; m];
        let mut acc_im = vec![0.0f64; m];
        let mut touched = vec![false; m];
        for jb in 0..p {
            let mut rows_in_col: Vec<usize> = Vec::new();
            for kb in b.col_ptr[jb]..b.col_ptr[jb + 1] {
                let kcol = b.row_idx[kb]; // column index into `self`
                let (bre, bim) = (b.re[kb], b.im.as_ref().map_or(0.0, |v| v[kb]));
                for ka in self.col_ptr[kcol]..self.col_ptr[kcol + 1] {
                    let ar = self.row_idx[ka];
                    let (are, aim) = (self.re[ka], self.im.as_ref().map_or(0.0, |v| v[ka]));
                    // (are + i aim)(bre + i bim)
                    let pr = are * bre - aim * bim;
                    let pi = are * bim + aim * bre;
                    if !touched[ar] {
                        touched[ar] = true;
                        rows_in_col.push(ar);
                    }
                    acc_re[ar] += pr;
                    acc_im[ar] += pi;
                }
            }
            rows_in_col.sort_unstable();
            for &r in &rows_in_col {
                let a = acc_re[r];
                let bb = acc_im[r];
                if a != 0.0 || bb != 0.0 {
                    row_idx.push(r);
                    vre.push(a);
                    if let Some(iv) = vim.as_mut() {
                        iv.push(bb);
                    }
                }
                acc_re[r] = 0.0;
                acc_im[r] = 0.0;
                touched[r] = false;
            }
            col_ptr.push(row_idx.len());
        }
        Ok(Self::from_csc_raw(m, p, class, col_ptr, row_idx, vre, vim))
    }
}

impl SparseMatrix {
    /// Scale every stored value by a real scalar (`s * A`), staying sparse.
    #[must_use]
    pub fn scale_real(&self, s: f64) -> SparseMatrix {
        if s == 0.0 {
            return SparseMatrix::zeros(self.rows, self.cols);
        }
        let re = self.re.iter().map(|v| v * s).collect();
        let im = self
            .im
            .as_ref()
            .map(|iv| iv.iter().map(|v| v * s).collect());
        Self::build(
            self.rows,
            self.cols,
            self.class,
            self.col_ptr.clone(),
            self.row_idx.clone(),
            re,
            im,
        )
    }

    /// Scale by a complex scalar (`(sr + i·si) * A`), staying sparse.
    #[must_use]
    pub fn scale_complex(&self, sr: f64, si: f64) -> SparseMatrix {
        let nnz = self.nnz();
        let mut re = Vec::with_capacity(nnz);
        let mut im = Vec::with_capacity(nnz);
        for k in 0..nnz {
            let a = self.re[k];
            let b = self.im.as_ref().map_or(0.0, |v| v[k]);
            re.push(a * sr - b * si);
            im.push(a * si + b * sr);
        }
        Self::from_csc_raw(
            self.rows,
            self.cols,
            self.class,
            self.col_ptr.clone(),
            self.row_idx.clone(),
            re,
            Some(im),
        )
    }

    /// Element-wise add (`A + B`) or subtract (`A - B` when `sub`). Both must
    /// have equal dimensions. Result stays sparse.
    ///
    /// # Errors
    /// Returns `Err` on a dimension mismatch.
    pub fn add_sub(&self, b: &SparseMatrix, sub: bool) -> Result<SparseMatrix, String> {
        if self.rows != b.rows || self.cols != b.cols {
            return Err(format!(
                "matrix dimensions must agree: {}x{} vs {}x{}",
                self.rows, self.cols, b.rows, b.cols
            ));
        }
        let complex = self.im.is_some() || b.im.is_some();
        let class = result_class(self.class, b.class);
        let sgn = if sub { -1.0 } else { 1.0 };
        let mut col_ptr = Vec::with_capacity(self.cols + 1);
        let mut row_idx = Vec::new();
        let mut vre = Vec::new();
        let mut vim: Option<Vec<f64>> = if complex { Some(Vec::new()) } else { None };
        col_ptr.push(0);
        for j in 0..self.cols {
            // Merge the two sorted columns by row index.
            let mut ka = self.col_ptr[j];
            let mut kb = b.col_ptr[j];
            let ae = self.col_ptr[j + 1];
            let be = b.col_ptr[j + 1];
            while ka < ae || kb < be {
                let ra = if ka < ae {
                    self.row_idx[ka]
                } else {
                    usize::MAX
                };
                let rb = if kb < be { b.row_idx[kb] } else { usize::MAX };
                let (row, are, aim, bre, bim);
                if ra < rb {
                    row = ra;
                    are = self.re[ka];
                    aim = self.im.as_ref().map_or(0.0, |v| v[ka]);
                    bre = 0.0;
                    bim = 0.0;
                    ka += 1;
                } else if rb < ra {
                    row = rb;
                    are = 0.0;
                    aim = 0.0;
                    bre = b.re[kb];
                    bim = b.im.as_ref().map_or(0.0, |v| v[kb]);
                    kb += 1;
                } else {
                    row = ra;
                    are = self.re[ka];
                    aim = self.im.as_ref().map_or(0.0, |v| v[ka]);
                    bre = b.re[kb];
                    bim = b.im.as_ref().map_or(0.0, |v| v[kb]);
                    ka += 1;
                    kb += 1;
                }
                let r = are + sgn * bre;
                let m = aim + sgn * bim;
                if r != 0.0 || m != 0.0 {
                    row_idx.push(row);
                    vre.push(r);
                    if let Some(iv) = vim.as_mut() {
                        iv.push(m);
                    }
                }
            }
            col_ptr.push(row_idx.len());
        }
        Ok(Self::build(
            self.rows, self.cols, class, col_ptr, row_idx, vre, vim,
        ))
    }
}

/// Result class for a binary op between two sparse classes (double-dominant,
/// matching FreeMat's promotion for sparse results which are always float/double).
fn result_class(a: DataClass, b: DataClass) -> DataClass {
    if a == DataClass::Double || b == DataClass::Double {
        DataClass::Double
    } else if a == DataClass::Float || b == DataClass::Float {
        DataClass::Float
    } else {
        DataClass::Double
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dense_round_trip() {
        // 3x4 column-major dense buffer with a few nonzeros.
        let re = vec![
            0.0, 0.0, 1.0, // col 0
            0.0, 0.0, 0.0, // col 1
            2.0, 1.0, 0.0, // col 2
            0.0, 1.0, 0.0, // col 3
        ];
        let s = SparseMatrix::from_dense_cols(3, 4, DataClass::Double, &re, None);
        assert_eq!(s.nnz(), 4);
        let (out, _) = s.to_dense_cols();
        assert_eq!(out, re);
    }

    #[test]
    fn triplets_sum_duplicates() {
        let s = SparseMatrix::from_triplets(
            &[1, 1, 2],
            &[1, 1, 2],
            &[3.0, 4.0, 5.0],
            None,
            Some(2),
            Some(2),
            DataClass::Double,
        )
        .unwrap();
        assert_eq!(s.get(0, 0), (7.0, 0.0));
        assert_eq!(s.get(1, 1), (5.0, 0.0));
        assert_eq!(s.nnz(), 2);
    }

    #[test]
    fn eye_and_nnz() {
        let s = SparseMatrix::eye(3, 3);
        assert_eq!(s.nnz(), 3);
        assert_eq!(s.get(1, 1), (1.0, 0.0));
        assert_eq!(s.get(0, 1), (0.0, 0.0));
    }

    #[test]
    fn transpose_roundtrip() {
        let re = vec![1.0, 0.0, 0.0, 2.0, 3.0, 0.0]; // 3x2 col-major
        let s = SparseMatrix::from_dense_cols(3, 2, DataClass::Double, &re, None);
        let t = s.transpose();
        assert_eq!(t.dims(), [2, 3]);
        assert_eq!(t.get(0, 0), s.get(0, 0));
        assert_eq!(t.get(1, 0), s.get(0, 1));
    }

    #[test]
    fn matmul_matches_dense() {
        // A (2x3) * B (3x2)
        let a = SparseMatrix::from_dense_cols(
            2,
            3,
            DataClass::Double,
            &[1.0, 0.0, 0.0, 2.0, 3.0, 0.0],
            None,
        );
        let b = SparseMatrix::from_dense_cols(
            3,
            2,
            DataClass::Double,
            &[1.0, 0.0, 1.0, 0.0, 1.0, 0.0],
            None,
        );
        let c = a.matmul(&b).unwrap();
        let (out, _) = c.to_dense_cols();
        // Dense reference computed by hand.
        // A = [1 0 3; 0 2 0]; B = [1 0; 0 1; 1 0]; A*B = [4 0; 0 2]
        assert_eq!(c.dims(), [2, 2]);
        assert_eq!(out, vec![4.0, 0.0, 0.0, 2.0]);
    }

    #[test]
    fn find_triplets_column_major() {
        let re = vec![0.0, 5.0, 7.0, 0.0]; // 2x2 col-major: (2,1)=5, (1,2)=7
        let s = SparseMatrix::from_dense_cols(2, 2, DataClass::Double, &re, None);
        let (i, j, v, _) = s.find_triplets();
        assert_eq!(i, vec![2.0, 1.0]);
        assert_eq!(j, vec![1.0, 2.0]);
        assert_eq!(v, vec![5.0, 7.0]);
    }
}
