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
    let a = mat(3, 2, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    let out = qr(&a, 2).unwrap();
    let prod = mtimes(&out[0], &out[1]).unwrap();
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
