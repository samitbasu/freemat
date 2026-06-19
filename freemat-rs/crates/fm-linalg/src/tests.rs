//! Unit tests for the faer-backed linalg routines against known matrices.

use super::*;

/// Build a real 2-D `double` matrix from a column-major data vector.
fn mat(rows: usize, cols: usize, data: &[f64]) -> Array {
    Array::double_matrix(&[rows, cols], data.to_vec())
}

/// Approximate equality for a flat f64 read of an Array (column-major).
fn approx(a: &Array, expected: &[f64]) {
    let got = to_f64(a);
    assert_eq!(got.len(), expected.len(), "length mismatch: {got:?}");
    for (g, e) in got.iter().zip(expected) {
        assert!(
            (g - e).abs() < 1e-9,
            "mismatch: got {got:?} want {expected:?}"
        );
    }
}

#[test]
fn matmul_2x2() {
    // A = [1 2; 3 4] (column-major: 1,3,2,4). A*A = [7 10; 15 22].
    let a = mat(2, 2, &[1.0, 3.0, 2.0, 4.0]);
    let p = mtimes(&a, &a).unwrap();
    // column-major [7,15,10,22]
    approx(&p, &[7.0, 15.0, 10.0, 22.0]);
}

#[test]
fn matmul_at_a() {
    // A = [1 2; 3 4]; A*A' = [5 11; 11 25].
    let a = mat(2, 2, &[1.0, 3.0, 2.0, 4.0]);
    let at = mat(2, 2, &[1.0, 2.0, 3.0, 4.0]); // A'
    let p = mtimes(&a, &at).unwrap();
    approx(&p, &[5.0, 11.0, 11.0, 25.0]);
}

#[test]
fn det_2x2() {
    let a = mat(2, 2, &[1.0, 3.0, 2.0, 4.0]);
    let d = det(&a).unwrap();
    approx(&d, &[-2.0]);
}

#[test]
fn inv_2x2() {
    let a = mat(2, 2, &[1.0, 3.0, 2.0, 4.0]);
    let inverse = inv(&a).unwrap();
    // inv([1 2;3 4]) = [-2 1; 1.5 -0.5] column-major: -2, 1.5, 1, -0.5
    approx(&inverse, &[-2.0, 1.5, 1.0, -0.5]);
}

#[test]
fn solve_square() {
    // A x = b, A = [2 0; 0 4], b = [2; 8] => x = [1; 2].
    let a = mat(2, 2, &[2.0, 0.0, 0.0, 4.0]);
    let b = mat(2, 1, &[2.0, 8.0]);
    let x = mldivide(&a, &b).unwrap();
    approx(&x, &[1.0, 2.0]);
}

#[test]
fn lu_reconstructs() {
    let a = mat(3, 3, &[4.0, 6.0, 10.0, 3.0, 3.0, 4.0, 1.0, 2.0, 3.0]);
    let out = lu(&a, 2).unwrap();
    let (l, u) = (&out[0], &out[1]);
    // L*U should equal A.
    let prod = mtimes(l, u).unwrap();
    approx(&prod, &to_f64(&a));
}

#[test]
fn lu_three_outputs() {
    let a = mat(2, 2, &[0.0, 1.0, 1.0, 0.0]);
    let out = lu(&a, 3).unwrap();
    assert_eq!(out.len(), 3);
    // P*A == L*U
    let pa = mtimes(&out[2], &a).unwrap();
    let lu_ = mtimes(&out[0], &out[1]).unwrap();
    approx(&pa, &to_f64(&lu_));
}

#[test]
fn qr_reconstructs() {
    // 3x2 (m > n): full mode yields Q 3x3, R 3x2; Q*R == A.
    let a = mat(3, 2, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    let out = qr(&a, 2, false).unwrap();
    assert_eq!(out[0].shape(), &[3, 3]);
    assert_eq!(out[1].shape(), &[3, 2]);
    let prod = mtimes(&out[0], &out[1]).unwrap();
    approx(&prod, &to_f64(&a));
}

#[test]
fn qr_economy_dims() {
    // 4x3 economy: Q 4x3, R 3x3, Q*R == A.
    let a = mat(
        4,
        3,
        &[1.0, 4.0, 7.0, 10.0, 2.0, 5.0, 8.0, 0.0, 3.0, 6.0, 9.0, 5.0],
    );
    let out = qr(&a, 2, true).unwrap();
    assert_eq!(out[0].shape(), &[4, 3]);
    assert_eq!(out[1].shape(), &[3, 3]);
    let prod = mtimes(&out[0], &out[1]).unwrap();
    approx(&prod, &to_f64(&a));
}

#[test]
fn qr_pivoted_reconstructs() {
    // [Q,R,E] = qr(A): A == Q*R*E' with E an n×n permutation matrix.
    let a = mat(
        4,
        3,
        &[1.0, 4.0, 7.0, 10.0, 2.0, 5.0, 8.0, 0.0, 3.0, 6.0, 9.0, 5.0],
    );
    let out = qr(&a, 3, false).unwrap();
    assert_eq!(out[0].shape(), &[4, 4]);
    assert_eq!(out[1].shape(), &[4, 3]);
    assert_eq!(out[2].shape(), &[3, 3]);
    let qr_ = mtimes(&out[0], &out[1]).unwrap();
    // E' (transpose of the 3×3 permutation matrix).
    let e = to_f64(&out[2]);
    let mut et = vec![0.0; 9];
    for i in 0..3 {
        for j in 0..3 {
            et[j + i * 3] = e[i + j * 3];
        }
    }
    let prod = mtimes(&qr_, &mat(3, 3, &et)).unwrap();
    approx(&prod, &to_f64(&a));
}

#[test]
fn svd_values() {
    // diag(3,4) singular values are {4,3} (descending).
    let a = mat(2, 2, &[3.0, 0.0, 0.0, 4.0]);
    let out = svd(&a, 1).unwrap();
    approx(&out[0], &[4.0, 3.0]);
}

#[test]
fn svd_reconstructs() {
    let a = mat(2, 2, &[1.0, 3.0, 2.0, 4.0]);
    let out = svd(&a, 3).unwrap();
    let (u, s, v) = (&out[0], &out[1], &out[2]);
    let us = mtimes(u, s).unwrap();
    let vt = super::transpose(v).unwrap();
    let prod = mtimes(&us, &vt).unwrap();
    approx(&prod, &to_f64(&a));
}

#[test]
fn eig_symmetric() {
    // [2 0; 0 3] eigenvalues 2, 3.
    let a = mat(2, 2, &[2.0, 0.0, 0.0, 3.0]);
    let out = eig(&a, 1).unwrap();
    let mut vals = to_f64(&out[0]);
    vals.sort_by(|x, y| x.partial_cmp(y).unwrap());
    approx(&Array::double_matrix(&[2, 1], vals), &[2.0, 3.0]);
}

/// Max-abs of the flat (column-major) read of an Array.
fn max_abs(a: &Array) -> f64 {
    to_f64(a).iter().fold(0.0_f64, |m, &v| m.max(v.abs()))
}

#[test]
fn eig_gen_eigenvalues_match_and_residual_small() {
    // General real pencil (A, B). The generalized eigenvalues are the standard
    // eigenvalues of B\A; the eigenvectors satisfy A*V = B*V*D.
    let a = mat(3, 3, &[4.0, 1.0, 0.0, 2.0, 3.0, 1.0, 1.0, 0.0, 5.0]);
    let b = mat(3, 3, &[2.0, 0.0, 0.0, 0.0, 3.0, 0.0, 1.0, 0.0, 4.0]);

    // Eigenvalues-only form equals eig(B\A).
    let g = eig_gen(&a, &b, 1).unwrap();
    let m = mldivide(&b, &a).unwrap();
    let std = eig(&m, 1).unwrap();
    let mut gv = to_f64(&g[0]);
    let mut sv = to_f64(&std[0]);
    gv.sort_by(|x, y| x.partial_cmp(y).unwrap());
    sv.sort_by(|x, y| x.partial_cmp(y).unwrap());
    for (x, y) in gv.iter().zip(&sv) {
        assert!((x - y).abs() < 1e-9, "eigenvalue mismatch {gv:?} vs {sv:?}");
    }

    // [V,D] residual: norm(A*V - B*V*D) within ~8*max|d|*eps*n.
    let vd = eig_gen(&a, &b, 2).unwrap();
    let (v, d) = (&vd[0], &vd[1]);
    let av = mtimes(&a, v).unwrap();
    let bv = mtimes(&b, v).unwrap();
    let bvd = mtimes(&bv, d).unwrap();
    // residual = A*V - B*V*D
    let avv = to_f64(&av);
    let bvdv = to_f64(&bvd);
    let er = avv
        .iter()
        .zip(&bvdv)
        .fold(0.0_f64, |m, (x, y)| m.max((x - y).abs()));
    let maxd = max_abs(d);
    let bnd = 8.0 * maxd * f64::EPSILON * 3.0;
    assert!(er < bnd.max(1e-12), "residual {er} exceeds bound {bnd}");
}

#[test]
fn eig_gen_symmetric_definite_residual_small() {
    // Symmetric A, SPD B (B = C*C').
    let a = mat(3, 3, &[2.0, -1.0, 0.0, -1.0, 2.0, -1.0, 0.0, -1.0, 2.0]);
    let c = mat(3, 3, &[3.0, 1.0, 0.0, 1.0, 4.0, 1.0, 0.0, 1.0, 5.0]);
    let ct = super::transpose(&c).unwrap();
    let b = mtimes(&c, &ct).unwrap(); // SPD

    let vd = eig_gen(&a, &b, 2).unwrap();
    let (v, d) = (&vd[0], &vd[1]);
    let av = mtimes(&a, v).unwrap();
    let bv = mtimes(&b, v).unwrap();
    let bvd = mtimes(&bv, d).unwrap();
    let avv = to_f64(&av);
    let bvdv = to_f64(&bvd);
    let er = avv
        .iter()
        .zip(&bvdv)
        .fold(0.0_f64, |m, (x, y)| m.max((x - y).abs()));
    let maxd = max_abs(d);
    let bnd = 10.0 * max_abs(&eig(&b, 1).unwrap()[0]) * maxd * f64::EPSILON * 3.0;
    assert!(er < bnd.max(1e-12), "residual {er} exceeds bound {bnd}");
}

#[test]
fn chol_upper() {
    // A = [4 2; 2 3] (SPD). R'*R = A, R upper triangular.
    let a = mat(2, 2, &[4.0, 2.0, 2.0, 3.0]);
    let r = chol(&a).unwrap();
    let rt = super::transpose(&r).unwrap();
    let prod = mtimes(&rt, &r).unwrap();
    approx(&prod, &[4.0, 2.0, 2.0, 3.0]);
}

#[test]
fn norm_vector_two() {
    let v = mat(3, 1, &[3.0, 4.0, 0.0]);
    let n = norm(&v, NormKind::Two).unwrap();
    approx(&n, &[5.0]);
}

#[test]
fn norm_matrix_one() {
    // [1 2; 3 4] 1-norm = max col abs sum = max(4, 6) = 6.
    let a = mat(2, 2, &[1.0, 3.0, 2.0, 4.0]);
    let n = norm(&a, NormKind::One).unwrap();
    approx(&n, &[6.0]);
}

#[test]
fn rank_full() {
    let a = mat(2, 2, &[1.0, 3.0, 2.0, 4.0]);
    let r = rank(&a).unwrap();
    approx(&r, &[2.0]);
}

#[test]
fn rank_deficient() {
    // [1 2; 2 4] rank 1.
    let a = mat(2, 2, &[1.0, 2.0, 2.0, 4.0]);
    let r = rank(&a).unwrap();
    approx(&r, &[1.0]);
}

#[test]
fn pinv_reconstructs_lstsq() {
    // For full column-rank A, A * pinv(A) * A == A.
    let a = mat(3, 2, &[1.0, 2.0, 3.0, 4.0, 5.0, 7.0]);
    let p = pinv(&a).unwrap();
    let ap = mtimes(&a, &p).unwrap();
    let apa = mtimes(&ap, &a).unwrap();
    approx(&apa, &to_f64(&a));
}

#[test]
fn mpower_square() {
    // [2 0; 0 3]^2 = [4 0; 0 9].
    let a = mat(2, 2, &[2.0, 0.0, 0.0, 3.0]);
    let p = mpower(&a, 2.0).unwrap();
    approx(&p, &[4.0, 0.0, 0.0, 9.0]);
}

#[test]
fn cond_identity() {
    let i3 = mat(3, 3, &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
    let c = cond(&i3).unwrap();
    assert!((c.as_f64().unwrap() - 1.0).abs() < 1e-9);
}

#[test]
fn rref_full_rank_2x2() {
    // [1 2; 3 4] column-major.
    let a = mat(2, 2, &[1.0, 3.0, 2.0, 4.0]);
    let r = rref(&a).unwrap();
    approx(&r, &[1.0, 0.0, 0.0, 1.0]);
}

#[test]
fn rref_rank_deficient() {
    // [1 2; 2 4] column-major: rref is [1 2; 0 0].
    let a = mat(2, 2, &[1.0, 2.0, 2.0, 4.0]);
    let r = rref(&a).unwrap();
    // column-major [1,0,2,0]
    approx(&r, &[1.0, 0.0, 2.0, 0.0]);
}

#[test]
fn kron_identity_block() {
    let i2 = mat(2, 2, &[1.0, 0.0, 0.0, 1.0]);
    let b = mat(2, 2, &[1.0, 3.0, 2.0, 4.0]);
    let k = kron(&i2, &b).unwrap();
    assert_eq!(k.dims(), vec![4, 4]);
}

#[test]
fn roots_quadratic() {
    // x^2 - 3x + 2 -> roots {1, 2}.
    let r = roots(&[1.0, -3.0, 2.0]).unwrap();
    let mut v = to_f64(&r);
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert!((v[0] - 1.0).abs() < 1e-9);
    assert!((v[1] - 2.0).abs() < 1e-9);
}

#[test]
fn tril_triu_2x2() {
    let a = mat(2, 2, &[1.0, 3.0, 2.0, 4.0]);
    approx(&tril(&a, 0).unwrap(), &[1.0, 3.0, 0.0, 4.0]);
    approx(&triu(&a, 0).unwrap(), &[1.0, 0.0, 2.0, 4.0]);
}

#[test]
fn null_dimension() {
    let a = mat(2, 2, &[1.0, 1.0, 1.0, 1.0]);
    let n = null(&a).unwrap();
    assert_eq!(n.dims(), vec![2, 1]);
}

// ---- Sparse solves (native sp_lu / sp_qr, no densification) -----------------

/// Wrap a dense column-major real matrix as a sparse `Array`.
fn sparse_real(rows: usize, cols: usize, data: &[f64]) -> Array {
    let s =
        fm_core::SparseMatrix::from_dense_cols(rows, cols, fm_core::DataClass::Double, data, None);
    Array::sparse(s)
}

#[test]
fn sparse_mldivide_matches_dense() {
    // SPD-ish tridiagonal 3x3; solve against a 3x1 rhs.
    let dense = mat(3, 3, &[4.0, 1.0, 0.0, 1.0, 3.0, 1.0, 0.0, 1.0, 2.0]);
    let sp = sparse_real(3, 3, &[4.0, 1.0, 0.0, 1.0, 3.0, 1.0, 0.0, 1.0, 2.0]);
    let b = mat(3, 1, &[1.0, 2.0, 3.0]);
    let xd = mldivide(&dense, &b).unwrap();
    let xs = mldivide(&sp, &b).unwrap();
    let (gd, gs) = (to_f64(&xd), to_f64(&xs));
    for (d, s) in gd.iter().zip(&gs) {
        assert!(
            (d - s).abs() < 1e-9,
            "sparse vs dense solve mismatch: {gd:?} {gs:?}"
        );
    }
    // The result is dense (FreeMat returns dense from a sparse solve).
    assert!(!xs.is_sparse());
}

#[test]
fn sparse_mldivide_complex_residual_zero() {
    // C = [2, i; i, 4] (column-major re=[2,0,0,4], im=[0,1,1,0]); b = [1+i; 2].
    let re = [2.0, 0.0, 0.0, 4.0];
    let im = [0.0, 1.0, 1.0, 0.0];
    let sp = Array::sparse(fm_core::SparseMatrix::from_dense_cols(
        2,
        2,
        fm_core::DataClass::Double,
        &re,
        Some(&im),
    ));
    let dense = Array::complex64_matrix(
        &[2, 2],
        vec![
            C64::new(2.0, 0.0),
            C64::new(0.0, 1.0),
            C64::new(0.0, 1.0),
            C64::new(4.0, 0.0),
        ],
    );
    let b = Array::complex64_matrix(&[2, 1], vec![C64::new(1.0, 1.0), C64::new(2.0, 0.0)]);
    let x = mldivide(&sp, &b).unwrap();
    // Residual C*x - b must vanish.
    let r = mtimes(&dense, &x).unwrap();
    let (rv, bv) = (to_c64(&r), to_c64(&b));
    for (ri, bi) in rv.iter().zip(&bv) {
        assert!(
            (ri - bi).norm() < 1e-9,
            "complex sparse solve residual too large"
        );
    }
}

#[test]
fn sparse_mldivide_singular_errors() {
    // A singular sparse matrix (zero column) must error, not return garbage.
    let sp = sparse_real(2, 2, &[0.0, 0.0, 1.0, 1.0]); // column 0 all zero
    let b = mat(2, 1, &[1.0, 2.0]);
    assert!(mldivide(&sp, &b).is_err());
}
