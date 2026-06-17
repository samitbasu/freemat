//! Integration tests: register the standard library and drive the interpreter.

use fm_interp::Interpreter;

/// Build an interpreter with the full standard library registered.
fn interp() -> Interpreter {
    let mut i = Interpreter::new();
    crate::register_standard_library(&mut i);
    i
}

/// Evaluate `expr`, returning the scalar `f64` value of `ans`.
fn eval_scalar(src: &str) -> f64 {
    let mut i = interp();
    i.run(src).expect("run ok");
    i.context
        .lookup("ans")
        .or_else(|| i.context.lookup("r"))
        .and_then(fm_core::Array::as_f64)
        .expect("scalar result")
}

#[test]
fn sqrt_of_four() {
    assert_eq!(eval_scalar("sqrt(4)"), 2.0);
}

#[test]
fn sqrt_negative_is_complex() {
    let mut i = interp();
    i.run("r = sqrt(-4);").unwrap();
    let r = i.context.lookup("r").unwrap();
    assert!(r.is_complex());
}

#[test]
fn exp_log_roundtrip() {
    assert!((eval_scalar("log(exp(1))") - 1.0).abs() < 1e-12);
}

#[test]
fn sin_pi_is_zero() {
    assert!(eval_scalar("sin(pi)").abs() < 1e-12);
}

#[test]
fn atan2_quadrant() {
    assert!((eval_scalar("atan2(1,1)") - std::f64::consts::FRAC_PI_4).abs() < 1e-12);
}

#[test]
fn sum_of_vector() {
    assert_eq!(eval_scalar("sum([1 2 3 4])"), 10.0);
}

#[test]
fn sum_columns() {
    let mut i = interp();
    i.run("r = sum([1 2; 3 4]);").unwrap();
    let r = i.context.lookup("r").unwrap();
    assert_eq!(fm_interp::value::to_f64_vec(r), vec![4.0, 6.0]);
}

#[test]
fn prod_of_vector() {
    assert_eq!(eval_scalar("prod([1 2 3 4])"), 24.0);
}

#[test]
fn mean_of_vector() {
    assert_eq!(eval_scalar("mean([2 4 6])"), 4.0);
}

#[test]
fn max_returns_value_and_index() {
    let mut i = interp();
    i.run("[v, idx] = max([3 7 2]);").unwrap();
    assert_eq!(i.context.lookup("v").unwrap().as_f64(), Some(7.0));
    assert_eq!(i.context.lookup("idx").unwrap().as_f64(), Some(2.0));
}

#[test]
fn min_returns_value_and_index() {
    let mut i = interp();
    i.run("[v, idx] = min([3 7 2]);").unwrap();
    assert_eq!(i.context.lookup("v").unwrap().as_f64(), Some(2.0));
    assert_eq!(i.context.lookup("idx").unwrap().as_f64(), Some(3.0));
}

#[test]
fn cumsum_vector() {
    let mut i = interp();
    i.run("r = cumsum([1 2 3]);").unwrap();
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("r").unwrap()),
        vec![1.0, 3.0, 6.0]
    );
}

#[test]
fn all_true() {
    assert_eq!(eval_scalar("all([1 1 1])"), 1.0);
    assert_eq!(eval_scalar("all([1 0 1])"), 0.0);
}

#[test]
fn any_true() {
    assert_eq!(eval_scalar("any([0 0 1])"), 1.0);
    assert_eq!(eval_scalar("any([0 0 0])"), 0.0);
}

#[test]
fn all_of_colon() {
    // The conformance `test()` helper shape: all(x(:)).
    let mut i = interp();
    i.run("r = all([1 1; 1 1](:));").unwrap();
    assert_eq!(i.context.lookup("r").unwrap().as_f64(), Some(1.0));
}

#[test]
fn eye_identity() {
    let mut i = interp();
    i.run("r = eye(3);").unwrap();
    let r = i.context.lookup("r").unwrap();
    assert_eq!(r.dims(), vec![3, 3]);
    assert_eq!(
        fm_interp::value::to_f64_vec(r),
        vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
    );
}

#[test]
fn linspace_endpoints() {
    let mut i = interp();
    i.run("r = linspace(0, 1, 5);").unwrap();
    let r = i.context.lookup("r").unwrap();
    let v = fm_interp::value::to_f64_vec(r);
    assert_eq!(v, vec![0.0, 0.25, 0.5, 0.75, 1.0]);
}

#[test]
fn repmat_tiles() {
    let mut i = interp();
    i.run("r = repmat([1 2], 2, 1);").unwrap();
    let r = i.context.lookup("r").unwrap();
    assert_eq!(r.dims(), vec![2, 2]);
}

#[test]
fn diag_builds_and_extracts() {
    let mut i = interp();
    i.run("r = diag([1 2 3]);").unwrap();
    assert_eq!(i.context.lookup("r").unwrap().dims(), vec![3, 3]);
    i.run("d = diag([1 2 3; 4 5 6; 7 8 9]);").unwrap();
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("d").unwrap()),
        vec![1.0, 5.0, 9.0]
    );
}

#[test]
fn rand_in_range() {
    let mut i = interp();
    i.run("r = rand(10);").unwrap();
    let v = fm_interp::value::to_f64_vec(i.context.lookup("r").unwrap());
    assert!(v.iter().all(|&x| (0.0..1.0).contains(&x)));
    assert_eq!(v.len(), 100);
}

// ---- Milestone 1 acceptance: matmul + linalg via the REPL path -------------

#[test]
fn milestone1_matmul_transpose() {
    // A=[1 2;3 4]; A*A' = [5 11; 11 25].
    let mut i = interp();
    i.run("A = [1 2; 3 4]; r = A*A';").unwrap();
    let r = i.context.lookup("r").unwrap();
    assert_eq!(fm_interp::value::to_f64_vec(r), vec![5.0, 11.0, 11.0, 25.0]);
}

#[test]
fn milestone1_solve_backslash() {
    // A=[2 0;0 4]; b=[2;8]; x = A\b => [1;2].
    let mut i = interp();
    i.run("A = [2 0; 0 4]; b = [2; 8]; x = A\\b;").unwrap();
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("x").unwrap()),
        vec![1.0, 2.0]
    );
}

#[test]
fn milestone1_eig() {
    let mut i = interp();
    i.run("e = eig([2 0; 0 3]);").unwrap();
    let mut v = fm_interp::value::to_f64_vec(i.context.lookup("e").unwrap());
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert!((v[0] - 2.0).abs() < 1e-9 && (v[1] - 3.0).abs() < 1e-9);
}

#[test]
fn milestone1_svd_lu() {
    let mut i = interp();
    i.run("s = svd([3 0; 0 4]);").unwrap();
    let v = fm_interp::value::to_f64_vec(i.context.lookup("s").unwrap());
    assert!((v[0] - 4.0).abs() < 1e-9 && (v[1] - 3.0).abs() < 1e-9);
    // lu reconstructs.
    i.run("A = [4 3; 6 3]; [L, U] = lu(A); P = L*U;").unwrap();
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("P").unwrap()),
        vec![4.0, 6.0, 3.0, 3.0]
    );
}

#[test]
fn det_and_inv() {
    assert_eq!(eval_scalar("det([1 2; 3 4])"), -2.0);
    let mut i = interp();
    i.run("r = inv([1 2; 3 4]);").unwrap();
    let v = fm_interp::value::to_f64_vec(i.context.lookup("r").unwrap());
    let expected = [-2.0, 1.5, 1.0, -0.5];
    assert!(v.iter().zip(expected).all(|(a, b)| (a - b).abs() < 1e-9));
}

#[test]
fn norm_default_two() {
    assert_eq!(eval_scalar("norm([3 4])"), 5.0);
}

#[test]
fn matrix_power() {
    let mut i = interp();
    i.run("r = [2 0; 0 3]^2;").unwrap();
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("r").unwrap()),
        vec![4.0, 0.0, 0.0, 9.0]
    );
}
