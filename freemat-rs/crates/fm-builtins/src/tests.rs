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

// ---- Stage 6 builtins -------------------------------------------------------

use fm_interp::value::to_f64_vec;

fn vec_of(src: &str, name: &str) -> Vec<f64> {
    let mut i = interp();
    i.run(src).expect("run ok");
    to_f64_vec(i.context.lookup(name).expect("var"))
}

fn str_of(src: &str, name: &str) -> String {
    let mut i = interp();
    i.run(src).expect("run ok");
    i.context.lookup(name).unwrap().as_string().expect("string")
}

#[test]
fn reshape_column_major() {
    // reshape preserves column-major order.
    assert_eq!(
        vec_of("r = reshape([1 2 3 4 5 6], 2, 3);", "r"),
        vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
    );
}

#[test]
fn reshape_infers_placeholder_dim() {
    let mut i = interp();
    i.run("r = reshape(1:12, 3, []);").unwrap();
    assert_eq!(i.context.lookup("r").unwrap().dims(), vec![3, 4]);
}

#[test]
fn sort_with_index() {
    let mut i = interp();
    i.run("[s, idx] = sort([3 1 2]);").unwrap();
    assert_eq!(
        to_f64_vec(i.context.lookup("s").unwrap()),
        vec![1.0, 2.0, 3.0]
    );
    assert_eq!(
        to_f64_vec(i.context.lookup("idx").unwrap()),
        vec![2.0, 3.0, 1.0]
    );
}

#[test]
fn sort_descending() {
    assert_eq!(
        vec_of("r = sort([1 3 2], 'descend');", "r"),
        vec![3.0, 2.0, 1.0]
    );
}

#[test]
fn unique_sorted_dedup() {
    assert_eq!(vec_of("r = unique([3 1 2 1 3]);", "r"), vec![1.0, 2.0, 3.0]);
}

#[test]
fn fliplr_and_flipud() {
    assert_eq!(vec_of("r = fliplr([1 2 3]);", "r"), vec![3.0, 2.0, 1.0]);
    assert_eq!(vec_of("r = flipud([1;2;3]);", "r"), vec![3.0, 2.0, 1.0]);
}

#[test]
fn circshift_vector() {
    assert_eq!(
        vec_of("r = circshift([1 2 3 4], 1);", "r"),
        vec![4.0, 1.0, 2.0, 3.0]
    );
}

#[test]
fn cat_horizontal_and_vertical() {
    assert_eq!(
        vec_of("r = horzcat([1 2], [3 4]);", "r"),
        vec![1.0, 2.0, 3.0, 4.0]
    );
    // vertcat of two rows → column-major [1 2;3 4] = [1,3,2,4].
    assert_eq!(
        vec_of("r = vertcat([1 2], [3 4]);", "r"),
        vec![1.0, 3.0, 2.0, 4.0]
    );
}

#[test]
fn sub2ind_ind2sub_roundtrip() {
    assert_eq!(vec_of("r = sub2ind([3 4], 2, 3);", "r"), vec![8.0]);
    let mut i = interp();
    i.run("[a, b] = ind2sub([3 4], 8);").unwrap();
    assert_eq!(to_f64_vec(i.context.lookup("a").unwrap()), vec![2.0]);
    assert_eq!(to_f64_vec(i.context.lookup("b").unwrap()), vec![3.0]);
}

#[test]
fn string_case_and_trim() {
    assert_eq!(str_of("r = upper('abc');", "r"), "ABC");
    assert_eq!(str_of("r = lower('ABC');", "r"), "abc");
    assert_eq!(str_of("r = strtrim('  hi  ');", "r"), "hi");
    assert_eq!(str_of("r = strrep('a-b-c', '-', '+');", "r"), "a+b+c");
}

#[test]
fn strfind_positions() {
    assert_eq!(vec_of("r = strfind('abcabc', 'bc');", "r"), vec![2.0, 5.0]);
}

#[test]
fn strsplit_strjoin_roundtrip() {
    assert_eq!(
        str_of("c = strsplit('a,b,c', ','); r = strjoin(c, '-');", "r"),
        "a-b-c"
    );
}

#[test]
fn sprintf_conversions() {
    assert_eq!(str_of("r = sprintf('%d-%s', 7, 'x');", "r"), "7-x");
    assert_eq!(str_of("r = sprintf('%.2f', 3.14159);", "r"), "3.14");
    assert_eq!(str_of("r = sprintf('%05d', 42);", "r"), "00042");
    // Argument recycling over a vector.
    assert_eq!(str_of("r = sprintf('%d ', [1 2 3]);", "r"), "1 2 3 ");
}

#[test]
fn set_operations() {
    assert_eq!(vec_of("r = union([1 2], [2 3]);", "r"), vec![1.0, 2.0, 3.0]);
    assert_eq!(
        vec_of("r = intersect([1 2 3], [2 3 4]);", "r"),
        vec![2.0, 3.0]
    );
    assert_eq!(vec_of("r = setdiff([1 2 3], [2]);", "r"), vec![1.0, 3.0]);
}

#[test]
fn ismember_mask_and_loc() {
    let mut i = interp();
    i.run("[tf, loc] = ismember([1 5 2], [2 1]);").unwrap();
    assert_eq!(
        to_f64_vec(i.context.lookup("tf").unwrap()),
        vec![1.0, 0.0, 1.0]
    );
    assert_eq!(
        to_f64_vec(i.context.lookup("loc").unwrap()),
        vec![2.0, 0.0, 1.0]
    );
}

#[test]
fn struct_builtin_and_fields() {
    let mut i = interp();
    i.run("s = struct('a', 1, 'b', 2);").unwrap();
    let s = i.context.lookup("s").unwrap();
    assert_eq!(s.class_name(), "struct");
    i.run("f = fieldnames(s);").unwrap();
    let f = i.context.lookup("f").unwrap();
    assert_eq!(f.as_cell().unwrap().len(), 2);
    i.run("t = isfield(s, 'a');").unwrap();
    assert_eq!(i.context.lookup("t").unwrap().as_f64(), Some(1.0));
}

#[test]
fn cell_builtin_shape() {
    let mut i = interp();
    i.run("c = cell(1, 2, 4);").unwrap();
    assert_eq!(i.context.lookup("c").unwrap().dims(), vec![1, 2, 4]);
}

#[test]
fn cellfun_uniform() {
    assert_eq!(
        vec_of("r = cellfun('numel', {[1 2], [1 2 3], 5});", "r"),
        vec![2.0, 3.0, 1.0]
    );
}

#[test]
fn struct_array_concat_and_index() {
    // The Stage-6 bug fix: concatenating scalar structs.
    let mut i = interp();
    i.run("a.foo = 1; b.foo = 4; c = [a, b]; r = c(2).foo;")
        .unwrap();
    assert_eq!(i.context.lookup("r").unwrap().as_f64(), Some(4.0));
}

#[test]
fn element_deletion() {
    // The Stage-6 bug fix: x(i) = [] removes the element.
    assert_eq!(
        vec_of("x = [1 2 3 4]; x(2) = []; r = x;", "r"),
        vec![1.0, 3.0, 4.0]
    );
}

#[test]
fn row_deletion() {
    // x(i, :) = [] removes a whole row.
    let mut i = interp();
    i.run("x = [1 2; 3 4; 5 6]; x(2, :) = []; r = x;").unwrap();
    assert_eq!(i.context.lookup("r").unwrap().dims(), vec![2, 2]);
    assert_eq!(
        to_f64_vec(i.context.lookup("r").unwrap()),
        vec![1.0, 5.0, 2.0, 6.0]
    );
}

#[test]
fn eval_runs_in_scope() {
    assert_eq!(eval_scalar("eval('r = 6 + 1');"), 7.0);
}

#[test]
fn eval_with_output() {
    let mut i = interp();
    i.run("r = eval('3 * 4');").unwrap();
    assert_eq!(i.context.lookup("r").unwrap().as_f64(), Some(12.0));
}

#[test]
fn num2str_and_str2double() {
    assert_eq!(str_of("r = num2str(42);", "r"), "42");
    assert_eq!(eval_scalar("str2double('3.5')"), 3.5);
}
