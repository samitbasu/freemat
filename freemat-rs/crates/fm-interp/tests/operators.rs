//! Operator dispatch: broadcasting, promotion, relational/logical, complex.

mod common;
use common::{eval_f64, run_var, run_vec};

use fm_core::DataClass;

#[test]
fn scalar_arithmetic() {
    assert_eq!(eval_f64("2 + 3"), 5.0);
    assert_eq!(eval_f64("10 - 4"), 6.0);
    assert_eq!(eval_f64("3 * 4"), 12.0);
    assert_eq!(eval_f64("12 / 4"), 3.0);
    assert_eq!(eval_f64("2 ^ 10"), 1024.0);
    assert_eq!(eval_f64("7 - 2*3"), 1.0);
}

#[test]
fn unary_ops() {
    assert_eq!(eval_f64("-5"), -5.0);
    assert_eq!(eval_f64("-2^2"), -4.0); // -(2^2)
    assert_eq!(eval_f64("~0"), 1.0);
    assert_eq!(eval_f64("~5"), 0.0);
}

#[test]
fn scalar_array_broadcast() {
    assert_eq!(run_vec("x = [1 2 3] + 10;", "x"), vec![11.0, 12.0, 13.0]);
    assert_eq!(run_vec("x = 2 .* [1 2 3];", "x"), vec![2.0, 4.0, 6.0]);
}

#[test]
fn array_array_elementwise() {
    assert_eq!(run_vec("x = [1 2 3] + [4 5 6];", "x"), vec![5.0, 7.0, 9.0]);
    assert_eq!(
        run_vec("x = [1 2 3] .* [4 5 6];", "x"),
        vec![4.0, 10.0, 18.0]
    );
}

#[test]
fn singleton_broadcast() {
    // Column + row → outer broadcast to a 2x3 matrix.
    let v = run_vec("x = [1;2] + [10 20 30];", "x");
    // Column-major: [11,12, 21,22, 31,32]
    assert_eq!(v, vec![11.0, 12.0, 21.0, 22.0, 31.0, 32.0]);
}

#[test]
fn relational_yields_logical() {
    let a = run_var("x = [1 2 3] > 2;", "x");
    assert_eq!(a.class(), DataClass::Bool);
    assert_eq!(run_vec("x = [1 2 3] > 2;", "x"), vec![0.0, 0.0, 1.0]);
    assert_eq!(run_vec("x = [1 2 3] == 2;", "x"), vec![0.0, 1.0, 0.0]);
}

#[test]
fn logical_ops() {
    assert_eq!(eval_f64("1 & 0"), 0.0);
    assert_eq!(eval_f64("1 | 0"), 1.0);
    assert_eq!(run_vec("x = [1 0 1] & [1 1 0];", "x"), vec![1.0, 0.0, 0.0]);
}

#[test]
fn integer_promotion_keeps_class() {
    let a = run_var("x = int32(5) + 2;", "x");
    assert_eq!(a.class(), DataClass::Int32);
    assert_eq!(a.as_f64(), Some(7.0));
}

#[test]
fn single_promotion() {
    let a = run_var("x = single(2) + single(3);", "x");
    assert_eq!(a.class(), DataClass::Float);
}

#[test]
fn complex_arithmetic() {
    // (3i) * (3i) = -9
    assert_eq!(eval_f64("3i * 3i"), -9.0);
    let a = run_var("x = 1 + 2i;", "x");
    assert!(a.is_complex());
}

#[test]
fn naive_matmul() {
    // [1 2;3 4] * [5 6;7 8] = [19 22; 43 50]
    let v = run_vec("x = [1 2;3 4] * [5 6;7 8];", "x");
    // column-major: [19,43, 22,50]
    assert_eq!(v, vec![19.0, 43.0, 22.0, 50.0]);
}

#[test]
fn matrix_solve_via_faer() {
    // Stage 5: `\` now routes through fm-linalg. A=[2 0;0 4], b=[2;8] => x=[1;2].
    let v = run_vec("x = [2 0;0 4] \\ [2;8];", "x");
    assert_eq!(v, vec![1.0, 2.0]);
}

#[test]
fn transpose_works() {
    // [1 2 3]' is a column.
    let v = run_vec("x = [1 2 3]';", "x");
    assert_eq!(v, vec![1.0, 2.0, 3.0]);
    let d = run_var("x = [1 2 3]';", "x").dims();
    assert_eq!(d, vec![3, 1]);
}
