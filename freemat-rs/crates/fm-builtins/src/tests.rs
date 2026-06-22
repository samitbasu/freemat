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
fn zeros_ones_class_arg_and_trailing_singleton() {
    let mut i = interp();
    // A trailing class string names the result class.
    i.run("a = zeros(2,3,'single');").unwrap();
    assert_eq!(i.context.lookup("a").unwrap().dims(), vec![2, 3]);
    assert_eq!(i.context.lookup("a").unwrap().class_name(), "single");
    i.run("b = ones(3,'int8');").unwrap();
    assert_eq!(i.context.lookup("b").unwrap().class_name(), "int8");
    // Trailing singleton dimensions are squeezed: `ones(2,2,1)` is 2x2.
    i.run("c = ones(2,2,1);").unwrap();
    assert_eq!(i.context.lookup("c").unwrap().dims(), vec![2, 2]);
}

#[test]
fn diag_with_offset() {
    let mut i = interp();
    // Extract the super-diagonal (k = 1) as a column vector.
    i.run("b = diag([1,2,3,4;5,6,7,8;9,10,11,12], 1);").unwrap();
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("b").unwrap()),
        vec![2.0, 7.0, 12.0]
    );
    // Extract the sub-diagonal (k = -1).
    i.run("c = diag([1,2,3,4;5,6,7,8;9,10,11,12], -1);")
        .unwrap();
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("c").unwrap()),
        vec![5.0, 10.0]
    );
    // Build a matrix placing a vector on the k = -1 diagonal.
    i.run("m = diag([2,3], -1);").unwrap();
    let m = i.context.lookup("m").unwrap();
    assert_eq!(m.dims(), vec![3, 3]);
    assert_eq!(
        fm_interp::value::to_f64_vec(m),
        // column-major of [0 0 0; 2 0 0; 0 3 0]
        vec![0.0, 2.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0]
    );
}

#[test]
fn float_double_casts_preserve_complex() {
    // `double`/`single`/`float`/`complex`/`dcomplex` of a complex value must
    // keep the imaginary part (regression: the casts took the real part only).
    let mut i = interp();
    for (expr, want_class) in [
        ("double(2+3i)", "double"),
        ("single(2+3i)", "single"),
        ("float(2+3i)", "single"),
        ("complex(2+3i)", "single"),
        ("dcomplex(2+3i)", "double"),
    ] {
        i.run(&format!("z = {expr}; im = imag(z); t = typeof(z);"))
            .unwrap();
        assert_eq!(
            i.context.lookup("im").unwrap().as_f64(),
            Some(3.0),
            "imag part dropped by {expr}"
        );
        assert_eq!(
            i.context.lookup("t").unwrap().as_string().as_deref(),
            Some(want_class),
            "wrong class for {expr}"
        );
    }
    // Integer cast of a complex takes the real part only (no imag).
    i.run("zi = int32(2+3i); imi = imag(zi);").unwrap();
    assert_eq!(i.context.lookup("imi").unwrap().as_f64(), Some(0.0));
}

#[test]
fn typeof_ignores_complexity_and_single_dominates() {
    let mut i = interp();
    // `typeof` is FreeMat's `className`: complexity is a flag, not a class.
    i.run("a = 2.0 + i; t1 = typeof(a);").unwrap();
    assert_eq!(
        i.context.lookup("t1").unwrap().as_string().as_deref(),
        Some("double")
    );
    i.run("b = 2.0f + i; t2 = typeof(b);").unwrap();
    assert_eq!(
        i.context.lookup("t2").unwrap().as_string().as_deref(),
        Some("single")
    );
    // single dominates double in arithmetic and concatenation.
    i.run("t3 = typeof(4f + 1.0);").unwrap();
    assert_eq!(
        i.context.lookup("t3").unwrap().as_string().as_deref(),
        Some("single")
    );
    i.run("t4 = typeof([1, 2; 3.0f, 4f + i]);").unwrap();
    assert_eq!(
        i.context.lookup("t4").unwrap().as_string().as_deref(),
        Some("single")
    );
    i.run("t5 = typeof([1, 2; 3.0, 4.0 + i]);").unwrap();
    assert_eq!(
        i.context.lookup("t5").unwrap().as_string().as_deref(),
        Some("double")
    );
}

#[test]
fn user_variable_shadows_builtin_constant() {
    // `e`, `pi`, `eps`, … are functions in FreeMat: a local variable of that
    // name shadows the constant (regression for `[q,r,e] = qr(a)` then using
    // `e` as a permutation matrix).
    let mut i = interp();
    i.run("e = [1 2; 3 4]; x = e(2,2);").unwrap();
    assert_eq!(i.context.lookup("x").unwrap().as_f64(), Some(4.0));
    // Without a binding, `e` is still Euler's number.
    i.run("clear e; y = e;").unwrap();
    let y = i.context.lookup("y").unwrap().as_f64().unwrap();
    assert!((y - std::f64::consts::E).abs() < 1e-12);
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

// ---- graphics (Stage 7) -----------------------------------------------------

#[test]
fn plot_builds_a_line_series() {
    use fm_graphics::Series;
    let mut i = interp();
    i.run("x = 0:.1:10; plot(x, sin(x));").unwrap();
    let fig = i.graphics.scene.figure(1).expect("figure 1 exists");
    let ax = &fig.axes[0];
    assert_eq!(ax.series.len(), 1);
    let Series::Line(l) = &ax.series[0] else {
        panic!("expected a line series");
    };
    assert_eq!(l.x.len(), 101);
    assert_eq!(l.y.len(), 101);
    assert!((l.x[0]).abs() < 1e-12);
    assert!((l.x[100] - 10.0).abs() < 1e-12);
    assert!((l.y[0] - 0.0_f64.sin()).abs() < 1e-12);
    assert_eq!(l.line_style, "-");
    assert_eq!(l.color, "rgb(0,0,255)");
}

#[test]
fn second_plot_replaces_without_hold() {
    let mut i = interp();
    i.run("plot(1:3, [1 2 3]);").unwrap();
    i.run("plot(1:3, [3 2 1]);").unwrap();
    let ax = &i.graphics.scene.figure(1).unwrap().axes[0];
    assert_eq!(ax.series.len(), 1); // replaced, not appended
}

#[test]
fn hold_on_appends_series() {
    let mut i = interp();
    i.run("plot(1:3, [1 2 3]); hold on; plot(1:3, [3 2 1]);")
        .unwrap();
    let ax = &i.graphics.scene.figure(1).unwrap().axes[0];
    assert_eq!(ax.series.len(), 2);
}

#[test]
fn title_labels_grid_and_linespec() {
    let mut i = interp();
    i.run("plot(1:3, [1 2 3], 'r--o'); title('T'); xlabel('X'); ylabel('Y'); grid on;")
        .unwrap();
    let ax = &i.graphics.scene.figure(1).unwrap().axes[0];
    assert_eq!(ax.title, "T");
    assert_eq!(ax.xlabel, "X");
    assert_eq!(ax.ylabel, "Y");
    assert!(ax.grid);
    let fm_graphics::Series::Line(l) = &ax.series[0] else {
        panic!();
    };
    assert_eq!(l.color, "rgb(255,0,0)");
    assert_eq!(l.line_style, "--");
    assert_eq!(l.marker, "o");
}

#[test]
fn figure_selects_new_figure() {
    let mut i = interp();
    i.run("plot(1:3, [1 2 3]); figure; plot(1:3, [3 2 1]);")
        .unwrap();
    assert_eq!(i.graphics.scene.figures.len(), 2);
}

// ---- Stage 7.5: handle property system, subplot, contour --------------------

#[test]
fn subplot_makes_two_axes_in_one_figure() {
    let mut i = interp();
    i.run("subplot(2,1,1); plot(1:3, [1 2 3]);").unwrap();
    i.run("subplot(2,1,2); plot(1:3, [3 2 1]);").unwrap();
    assert_eq!(i.graphics.scene.figures.len(), 1, "one figure");
    let fig = i.graphics.scene.figure(1).unwrap();
    assert_eq!(fig.axes.len(), 2, "two axes");
    // Distinct positions: top cell is the upper half, bottom cell the lower.
    let top = fig.axes[0].position;
    let bottom = fig.axes[1].position;
    assert!((top[1] - 0.5).abs() < 1e-9, "top axes bottom edge at 0.5");
    assert!((bottom[1] - 0.0).abs() < 1e-9, "bottom axes at 0.0");
    assert_ne!(top, bottom);
    // Each axes has its own single line series.
    assert_eq!(fig.axes[0].series.len(), 1);
    assert_eq!(fig.axes[1].series.len(), 1);
}

#[test]
fn subplot_reselects_existing_cell() {
    let mut i = interp();
    i.run("subplot(1,2,1); plot(1:3,[1 2 3]); subplot(1,2,2); plot(1:3,[3 2 1]);")
        .unwrap();
    // Re-selecting cell 1 must target the SAME axes, not create a third.
    i.run("subplot(1,2,1); plot(1:3,[2 2 2]);").unwrap();
    let fig = i.graphics.scene.figure(1).unwrap();
    assert_eq!(fig.axes.len(), 2, "no extra axes created");
}

#[test]
fn set_get_roundtrip_on_axes() {
    let mut i = interp();
    i.run("h = gca;").unwrap();
    i.run("set(h, 'title', 'Hello');").unwrap();
    i.run("t = get(h, 'title');").unwrap();
    let t = i.context.lookup("t").and_then(|a| a.as_string()).unwrap();
    assert_eq!(t, "Hello");
    // xlim round-trip (numeric vector property).
    i.run("set(h, 'xlim', [2 8]);").unwrap();
    i.run("xl = get(h, 'xlim');").unwrap();
    let xl = fm_interp::value::to_f64_vec(i.context.lookup("xl").unwrap());
    assert_eq!(xl, vec![2.0, 8.0]);
}

#[test]
fn set_get_unknown_property_echoes() {
    let mut i = interp();
    i.run("h = gca; set(h, 'MyTag', 42); v = get(h, 'MyTag');")
        .unwrap();
    let v = i
        .context
        .lookup("v")
        .and_then(fm_core::Array::as_f64)
        .unwrap();
    assert_eq!(v, 42.0);
}

#[test]
fn ishandle_and_gco_and_delete() {
    let mut i = interp();
    i.run("h = plot(1:3, [1 2 3]);").unwrap();
    i.run("ok = ishandle(h); is_line = ishandle(h, 'line');")
        .unwrap();
    assert!(
        i.context
            .lookup("ok")
            .and_then(fm_core::Array::as_f64)
            .unwrap()
            != 0.0
    );
    assert!(
        i.context
            .lookup("is_line")
            .and_then(fm_core::Array::as_f64)
            .unwrap()
            != 0.0
    );
    // gco is the most recently created object (the line).
    i.run("c = gco;").unwrap();
    assert_eq!(
        i.context.lookup("c").and_then(fm_core::Array::as_f64),
        i.context.lookup("h").and_then(fm_core::Array::as_f64)
    );
    // Deleting the line removes the only series.
    i.run("delete(h);").unwrap();
    assert_eq!(i.graphics.scene.figure(1).unwrap().axes[0].series.len(), 0);
    i.run("gone = ishandle(h);").unwrap();
    assert!(
        i.context
            .lookup("gone")
            .and_then(fm_core::Array::as_f64)
            .unwrap()
            == 0.0
    );
}

#[test]
fn root_object_and_hierarchy_navigation() {
    let mut i = interp();
    // The root (handle 0) always exists, even before any plotting.
    i.run("r = ishandle(0); rt = get(0, 'type');").unwrap();
    assert!(
        i.context
            .lookup("r")
            .and_then(fm_core::Array::as_f64)
            .unwrap()
            != 0.0
    );
    assert_eq!(
        i.context.lookup("rt").and_then(fm_core::Array::as_string),
        Some("root".to_string())
    );

    // Build a scene: one figure, one axes, one line.
    i.run("ln = plot(1:3, [1 2 3]);").unwrap();
    i.run("f = gcf; a = gca;").unwrap();
    let fig = i
        .context
        .lookup("f")
        .and_then(fm_core::Array::as_f64)
        .unwrap();
    let ax = i
        .context
        .lookup("a")
        .and_then(fm_core::Array::as_f64)
        .unwrap();
    let line = i
        .context
        .lookup("ln")
        .and_then(fm_core::Array::as_f64)
        .unwrap();

    // Types.
    i.run("ft = get(f,'type'); at = get(a,'type'); lt = get(ln,'type');")
        .unwrap();
    assert_eq!(
        i.context.lookup("ft").and_then(fm_core::Array::as_string),
        Some("figure".to_string())
    );
    assert_eq!(
        i.context.lookup("at").and_then(fm_core::Array::as_string),
        Some("axes".to_string())
    );
    assert_eq!(
        i.context.lookup("lt").and_then(fm_core::Array::as_string),
        Some("line".to_string())
    );

    // Parents: figure → 0, axes → figure, line → axes.
    i.run("fp = get(f,'parent'); ap = get(a,'parent'); lp = get(ln,'parent');")
        .unwrap();
    assert_eq!(
        i.context.lookup("fp").and_then(fm_core::Array::as_f64),
        Some(0.0)
    );
    assert_eq!(
        i.context.lookup("ap").and_then(fm_core::Array::as_f64),
        Some(fig)
    );
    assert_eq!(
        i.context.lookup("lp").and_then(fm_core::Array::as_f64),
        Some(ax)
    );

    // Children: root contains figure, figure contains axes, axes contains line.
    i.run("rc = get(0,'children'); fc = get(f,'children'); ac = get(a,'children');")
        .unwrap();
    let rc = fm_interp::value::to_f64_vec(i.context.lookup("rc").unwrap());
    let fc = fm_interp::value::to_f64_vec(i.context.lookup("fc").unwrap());
    let ac = fm_interp::value::to_f64_vec(i.context.lookup("ac").unwrap());
    assert!(
        rc.contains(&fig),
        "root children {rc:?} should contain {fig}"
    );
    assert!(
        fc.contains(&ax),
        "figure children {fc:?} should contain {ax}"
    );
    assert!(
        ac.contains(&line),
        "axes children {ac:?} should contain {line}"
    );

    // The root's currentfigure tracks gcf.
    i.run("cf = get(0,'currentfigure');").unwrap();
    assert_eq!(
        i.context.lookup("cf").and_then(fm_core::Array::as_f64),
        Some(fig)
    );
}

#[test]
fn contour_builds_a_contour_series() {
    use fm_graphics::Series;
    let mut i = interp();
    i.run("contour(zeros(5));").unwrap();
    let ax = &i.graphics.scene.figure(1).unwrap().axes[0];
    assert_eq!(ax.series.len(), 1);
    let Series::Contour(c) = &ax.series[0] else {
        panic!("expected a contour series");
    };
    assert_eq!(c.z.len(), 5);
    assert_eq!(c.z[0].len(), 5);
}

#[test]
fn close_all_clears_figures_and_handles() {
    let mut i = interp();
    i.run("plot(1:3,[1 2 3]); figure; plot(1:3,[3 2 1]); close all;")
        .unwrap();
    assert_eq!(i.graphics.scene.figures.len(), 0);
    assert!(i.graphics.all_handles().is_empty());
}

#[test]
fn drawnow_flushes_through_sink() {
    use fm_graphics::{GraphicsSink, Scene};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Capture(Arc<Mutex<Option<Scene>>>);
    impl GraphicsSink for Capture {
        fn publish(&self, scene: &Scene) {
            *self.0.lock().unwrap() = Some(scene.clone());
        }
    }

    let captured = Arc::new(Mutex::new(None));
    let mut i = interp();
    i.set_graphics_sink(Box::new(Capture(captured.clone())));
    i.run("plot(1:3, [1 2 3]);").unwrap(); // implicit draw flushes
    let scene = captured.lock().unwrap().clone().expect("scene published");
    assert_eq!(scene.figures.len(), 1);
    assert_eq!(scene.figures[0].axes[0].series.len(), 1);
}

#[test]
fn toc_after_tic_is_nonnegative_and_small() {
    let mut i = interp();
    i.run("tic; e = toc;").unwrap();
    let e = i
        .context
        .lookup("e")
        .and_then(fm_core::Array::as_f64)
        .unwrap();
    assert!(e >= 0.0, "elapsed should be non-negative, got {e}");
    assert!(e < 5.0, "elapsed should be small, got {e}");
}

#[test]
fn toc_handle_form_roundtrips() {
    let mut i = interp();
    i.run("id = tic; e = toc(id);").unwrap();
    let e = i
        .context
        .lookup("e")
        .and_then(fm_core::Array::as_f64)
        .unwrap();
    assert!(
        (0.0..5.0).contains(&e),
        "handle-form elapsed out of range: {e}"
    );
}

#[test]
fn clock_is_six_element_vector() {
    let mut i = interp();
    i.run("c = clock;").unwrap();
    let c = i.context.lookup("c").unwrap();
    assert_eq!(c.numel(), 6);
    let v = fm_interp::value::to_f64_vec(c);
    assert!(v[0] >= 2020.0, "year looks wrong: {}", v[0]);
    assert!((1.0..=12.0).contains(&v[1]), "month out of range: {}", v[1]);
}

#[test]
fn etime_of_known_clock_vectors() {
    // Two clock vectors one hour and 30 seconds apart on the same day.
    let mut i = interp();
    i.run("t1 = [2020 1 1 10 0 0]; t2 = [2020 1 1 11 0 30]; d = etime(t2, t1);")
        .unwrap();
    let d = i
        .context
        .lookup("d")
        .and_then(fm_core::Array::as_f64)
        .unwrap();
    assert!((d - 3630.0).abs() < 1e-6, "etime mismatch: {d}");
}

#[test]
fn clocktotime_known_value() {
    // 2000-01-01 00:00:00 UTC is exactly 946684800 seconds past the Unix epoch.
    let mut i = interp();
    i.run("t = clocktotime([2000 1 1 0 0 0]);").unwrap();
    let t = i
        .context
        .lookup("t")
        .and_then(fm_core::Array::as_f64)
        .unwrap();
    assert!(
        (t - 946_684_800.0).abs() < 1e-6,
        "clocktotime mismatch: {t}"
    );
    // Fractional seconds carry through.
    i.run("t2 = clocktotime([2000 1 1 0 0 0.5]);").unwrap();
    let t2 = i
        .context
        .lookup("t2")
        .and_then(fm_core::Array::as_f64)
        .unwrap();
    assert!((t2 - 946_684_800.5).abs() < 1e-6, "frac mismatch: {t2}");
}

#[test]
fn wav_write_then_read_roundtrip() {
    let mut i = interp();
    let path = std::env::temp_dir().join(format!(
        "fm_wav_roundtrip_{}_{}.wav",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let p = path.to_string_lossy().replace('\\', "/");
    // A short known mono signal in [-1, 1].
    i.run(&format!(
        "y = [0; 0.5; -0.5; 1; -1; 0.25]; wavwrite(y, 22050, '{p}');"
    ))
    .unwrap();
    i.run(&format!("[z, fs, nb] = wavread('{p}');")).unwrap();
    let z = fm_interp::value::to_f64_vec(i.context.lookup("z").unwrap());
    let expected = [0.0, 0.5, -0.5, 1.0, -1.0, 0.25];
    assert_eq!(z.len(), expected.len(), "sample count mismatch");
    for (a, b) in z.iter().zip(expected.iter()) {
        assert!((a - b).abs() < 1e-4, "sample mismatch: {a} vs {b}");
    }
    let fs = i
        .context
        .lookup("fs")
        .and_then(fm_core::Array::as_f64)
        .unwrap();
    let nb = i
        .context
        .lookup("nb")
        .and_then(fm_core::Array::as_f64)
        .unwrap();
    assert_eq!(fs, 22050.0);
    assert_eq!(nb, 16.0);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn license_returns_gpl() {
    let mut i = interp();
    i.run("l = license;").unwrap();
    let l = i.context.lookup("l").and_then(|a| a.as_string()).unwrap();
    assert_eq!(l, "GNU General Public License");
}

#[test]
fn diary_off_runs_clean() {
    let mut i = interp();
    i.run("diary('off');").unwrap();
    i.run("diary('on');").unwrap();
    i.run("s = diary;").unwrap();
    // After 'on', a value query reports logical true.
    let s = i.context.lookup("s").unwrap();
    assert_eq!(s.as_f64(), Some(1.0));
    i.run("diary('off');").unwrap();
}

#[test]
fn zlim_roundtrip() {
    let mut i = interp();
    i.run("zlim([0 5]);").unwrap();
    i.run("zl = zlim;").unwrap();
    let zl = fm_interp::value::to_f64_vec(i.context.lookup("zl").unwrap());
    assert_eq!(zl, vec![0.0, 5.0]);
}

#[test]
fn now_is_a_reasonable_serial_date() {
    let mut i = interp();
    i.run("n = now;").unwrap();
    let n = i
        .context
        .lookup("n")
        .and_then(fm_core::Array::as_f64)
        .unwrap();
    // 2020-01-01 is serial 737791; 2050-01-01 is ~748000-ish. Today is between.
    assert!(n > 737_791.0, "now too small: {n}");
    assert!(n < 800_000.0, "now too large: {n}");
}

#[test]
fn cputime_increases_or_holds() {
    let mut i = interp();
    i.run("a = cputime; b = cputime;").unwrap();
    let a = i
        .context
        .lookup("a")
        .and_then(fm_core::Array::as_f64)
        .unwrap();
    let b = i
        .context
        .lookup("b")
        .and_then(fm_core::Array::as_f64)
        .unwrap();
    assert!(b >= a, "cputime went backwards: {a} -> {b}");
}

#[test]
fn pause_zero_is_noop() {
    let mut i = interp();
    // Should return promptly and not error.
    i.run("pause(0);").unwrap();
}

// ---- Bit operations ---------------------------------------------------------

#[test]
fn bitand_bitor_bitxor() {
    let mut i = interp();
    i.run("a = bitand([1,5,42],3);").unwrap();
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("a").unwrap()),
        vec![1.0, 1.0, 2.0]
    );
    i.run("b = bitor([1,5,42],3);").unwrap();
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("b").unwrap()),
        vec![3.0, 7.0, 43.0]
    );
    i.run("c = bitxor([1,5,42],3);").unwrap();
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("c").unwrap()),
        vec![2.0, 6.0, 41.0]
    );
}

#[test]
fn bitshift_left_and_right() {
    assert_eq!(eval_scalar("bitshift(1, 4)"), 16.0);
    assert_eq!(eval_scalar("bitshift(16, -2)"), 4.0);
}

#[test]
fn bitcmp_uint8() {
    assert_eq!(eval_scalar("bitcmp(uint8(0))"), 255.0);
}

// ---- Base conversion --------------------------------------------------------

#[test]
fn dec2hex_and_hex2dec() {
    let mut i = interp();
    i.run("s = dec2hex(255);").unwrap();
    assert_eq!(i.context.lookup("s").unwrap().as_string().unwrap(), "FF");
    assert_eq!(eval_scalar("hex2dec('FF')"), 255.0);
}

#[test]
fn dec2bin_and_bin2dec() {
    let mut i = interp();
    i.run("s = dec2bin(10);").unwrap();
    assert_eq!(i.context.lookup("s").unwrap().as_string().unwrap(), "1010");
    assert_eq!(eval_scalar("bin2dec('1010')"), 10.0);
}

#[test]
fn num2hex_hex2num_roundtrip() {
    assert_eq!(eval_scalar("hex2num(num2hex(3.5))"), 3.5);
}

#[test]
fn int2bin_bin2int_roundtrip() {
    let mut i = interp();
    i.run("b = int2bin([4;3;2;1],3);").unwrap();
    // First row should be MSB-first of 4 -> [1 0 0].
    let v = fm_interp::value::to_f64_vec(i.context.lookup("b").unwrap());
    // Column-major 4x3: column 0 = [1;0;0;0], so v[0..4] = [1,0,0,0].
    assert_eq!(&v[0..4], &[1.0, 0.0, 0.0, 0.0]);
    i.run("c = bin2int(b);").unwrap();
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("c").unwrap()),
        vec![4.0, 3.0, 2.0, 1.0]
    );
}

/// N-D `int2bin`/`bin2int` round-trip preserves shape and values
/// (conformance `suite/test_bin2int1`): a 3-D array survives the round trip.
#[test]
fn int2bin_bin2int_nd_roundtrip() {
    let mut i = interp();
    i.run("A = floor(rand(4,4,3)*10);").unwrap();
    i.run("C = bin2int(int2bin(A,4));").unwrap();
    i.run("ok = isequal(A, C);").unwrap();
    assert_eq!(
        i.context.lookup("ok").and_then(fm_core::Array::as_f64),
        Some(1.0)
    );
    // int2bin appends the bits along a new trailing dimension.
    i.run("d = size(int2bin(A,4));").unwrap();
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("d").unwrap()),
        vec![4.0, 4.0, 3.0, 4.0]
    );
}

/// `fitfun` fits a linear model `a*x + b` to data, recovering the parameters
/// (conformance `suite/test_fitfun*`). Uses a function handle.
#[test]
fn fitfun_linear_fit() {
    let mut i = interp();
    i.run("x = 1:10;").unwrap();
    i.run("y = 3*x + 2;").unwrap();
    i.run("f = @(p) p(1)*x + p(2);").unwrap();
    i.run("[xopt, yopt] = fitfun(f, [0,0], y, ones(1,10), 1e-10);")
        .unwrap();
    let xopt = fm_interp::value::to_f64_vec(i.context.lookup("xopt").unwrap());
    assert!((xopt[0] - 3.0).abs() < 1e-5, "slope {}", xopt[0]);
    assert!((xopt[1] - 2.0).abs() < 1e-5, "intercept {}", xopt[1]);
}

/// `gausfit` (embedded toolbox M-function on top of `fitfun`) recovers the
/// mean and width of a Gaussian (conformance `suite/test_gausfit1`).
#[test]
fn gausfit_recovers_gaussian() {
    let mut i = interp();
    i.run("t = linspace(-1,1);").unwrap();
    i.run("y = exp(-t.^2/8);").unwrap();
    i.run("[mu,sigma,dc,gain,yhat] = gausfit(t,y);").unwrap();
    let mu = i
        .context
        .lookup("mu")
        .and_then(fm_core::Array::as_f64)
        .unwrap();
    let sigma = i
        .context
        .lookup("sigma")
        .and_then(fm_core::Array::as_f64)
        .unwrap();
    assert!(mu.abs() < 1e-4, "mu {mu}");
    assert!((sigma.abs() - 2.0).abs() < 1e-4, "sigma {sigma}");
}

/// Indexed assignment into a sparse matrix keeps it sparse, and an assigned
/// zero clears the entry (conformance `suite/test_sparse75` depends on this).
#[test]
fn sparse_assignment_preserves_sparsity() {
    let mut i = interp();
    i.run("z = sparse(eye(4));").unwrap();
    i.run("z(1,3) = 5;").unwrap();
    i.run("s1 = issparse(z);").unwrap();
    assert_eq!(
        i.context.lookup("s1").and_then(fm_core::Array::as_f64),
        Some(1.0)
    );
    i.run("v = full(z(1,3));").unwrap();
    assert_eq!(
        i.context.lookup("v").and_then(fm_core::Array::as_f64),
        Some(5.0)
    );
    // Assigning zero removes the structural nonzero.
    i.run("z(2,2) = 0; n = nnz(z);").unwrap();
    assert_eq!(
        i.context.lookup("n").and_then(fm_core::Array::as_f64),
        Some(4.0)
    );
}

/// `lu` on a non-square sparse matrix errors, matching FreeMat's
/// `SparseLUDecompose` (conformance `suite/test_sparse75`).
#[test]
fn lu_sparse_non_square_errors() {
    let mut i = interp();
    i.run("z = sparse(rand(3,5));").unwrap();
    assert!(i.run("[l,u] = lu(z);").is_err());
    // A square sparse matrix is fine.
    i.run("w = sparse(eye(4));").unwrap();
    assert!(i.run("[l,u] = lu(w);").is_ok());
}

/// Indexed assignment into a single-precision complex array preserves the
/// imaginary part (regression for the `class == Double`-only scatter branch).
#[test]
fn single_complex_scatter_keeps_imaginary() {
    let mut i = interp();
    i.run("a = complex(single([1,3]) + single([0,2])*i);")
        .unwrap();
    i.run("a(1) = complex(7+2*i);").unwrap();
    i.run("ok = isequal(imag(a), single([2,2]));").unwrap();
    assert_eq!(
        i.context.lookup("ok").and_then(fm_core::Array::as_f64),
        Some(1.0)
    );
}

// ---- Polynomials ------------------------------------------------------------

#[test]
fn polyval_horner() {
    // p(x) = x^2 + 2x + 3 at x=2 -> 11.
    assert_eq!(eval_scalar("polyval([1 2 3], 2)"), 11.0);
}

#[test]
fn roots_quadratic() {
    // x^2 - 3x + 2 -> roots 2 and 1.
    let mut i = interp();
    i.run("r = roots([1 -3 2]);").unwrap();
    let mut v = fm_interp::value::to_f64_vec(i.context.lookup("r").unwrap());
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert!((v[0] - 1.0).abs() < 1e-9);
    assert!((v[1] - 2.0).abs() < 1e-9);
}

#[test]
fn polyfit_recovers_line() {
    // y = 2x + 1 fit with degree 1.
    let mut i = interp();
    i.run("p = polyfit([0 1 2 3],[1 3 5 7],1);").unwrap();
    let v = fm_interp::value::to_f64_vec(i.context.lookup("p").unwrap());
    assert!((v[0] - 2.0).abs() < 1e-9);
    assert!((v[1] - 1.0).abs() < 1e-9);
}

#[test]
fn poly_from_roots() {
    // roots 1,2 -> x^2 -3x +2.
    let mut i = interp();
    i.run("p = poly([1 2]);").unwrap();
    let v = fm_interp::value::to_f64_vec(i.context.lookup("p").unwrap());
    assert!((v[0] - 1.0).abs() < 1e-12);
    assert!((v[1] + 3.0).abs() < 1e-12);
    assert!((v[2] - 2.0).abs() < 1e-12);
}

#[test]
fn polyder_polyint() {
    // d/dx (x^3) = 3x^2 -> [3 0 0].
    let mut i = interp();
    i.run("d = polyder([1 0 0 0]);").unwrap();
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("d").unwrap()),
        vec![3.0, 0.0, 0.0]
    );
}

#[test]
fn conv_polynomials() {
    // (x+1)(x+1) = x^2 + 2x + 1.
    let mut i = interp();
    i.run("c = conv([1 1],[1 1]);").unwrap();
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("c").unwrap()),
        vec![1.0, 2.0, 1.0]
    );
}

#[test]
fn deconv_polynomials() {
    let mut i = interp();
    i.run("[q,r] = deconv([1 2 1],[1 1]);").unwrap();
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("q").unwrap()),
        vec![1.0, 1.0]
    );
}

// ---- Linear-algebra extras --------------------------------------------------

#[test]
fn cond_identity_is_one() {
    assert!((eval_scalar("cond(eye(3))") - 1.0).abs() < 1e-9);
}

#[test]
fn rref_reduces() {
    let mut i = interp();
    i.run("r = rref([1 2; 3 4]);").unwrap();
    // Full-rank 2x2 -> identity. Column-major [1,0,0,1].
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("r").unwrap()),
        vec![1.0, 0.0, 0.0, 1.0]
    );
}

#[test]
fn kron_product() {
    let mut i = interp();
    i.run("k = kron([1 0; 0 1],[1 2; 3 4]);").unwrap();
    // Should be a 4x4 with two diagonal blocks of [1 2;3 4].
    assert_eq!(i.context.lookup("k").unwrap().dims(), vec![4, 4]);
}

#[test]
fn tril_triu() {
    let mut i = interp();
    i.run("l = tril([1 2; 3 4]);").unwrap();
    // tril: keep lower, zero upper. Column-major: [1,3,0,4].
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("l").unwrap()),
        vec![1.0, 3.0, 0.0, 4.0]
    );
    i.run("u = triu([1 2; 3 4]);").unwrap();
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("u").unwrap()),
        vec![1.0, 0.0, 2.0, 4.0]
    );
}

#[test]
fn null_of_singular() {
    // [1 1; 1 1] has a 1-D null space.
    let mut i = interp();
    i.run("n = null([1 1; 1 1]);").unwrap();
    assert_eq!(i.context.lookup("n").unwrap().dims(), vec![2, 1]);
}

// ---- Trig gaps --------------------------------------------------------------

#[test]
fn degree_trig() {
    assert!((eval_scalar("sind(30)") - 0.5).abs() < 1e-12);
    assert!((eval_scalar("cosd(60)") - 0.5).abs() < 1e-12);
    assert!((eval_scalar("tand(45)") - 1.0).abs() < 1e-12);
}

#[test]
fn hyperbolic_reciprocals() {
    assert!((eval_scalar("sech(0)") - 1.0).abs() < 1e-12);
}

// ---- Misc numeric -----------------------------------------------------------

#[test]
fn diff_vector() {
    let mut i = interp();
    i.run("d = diff([1 3 6 10]);").unwrap();
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("d").unwrap()),
        vec![2.0, 3.0, 4.0]
    );
}

#[test]
fn dot_and_cross() {
    assert_eq!(eval_scalar("dot([1 2 3],[4 5 6])"), 32.0);
    let mut i = interp();
    i.run("c = cross([1 0 0],[0 1 0]);").unwrap();
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("c").unwrap()),
        vec![0.0, 0.0, 1.0]
    );
}

#[test]
fn trapz_unit_spacing() {
    assert_eq!(eval_scalar("trapz([1 2 3])"), 4.0);
}

#[test]
fn trapz_with_x() {
    assert_eq!(eval_scalar("trapz([1 2 3],[1 2 3])"), 4.0);
}

#[test]
fn trapz_matrix_per_column() {
    let mut i = interp();
    i.run("r = trapz([1 2; 3 4]);").unwrap();
    let r = i.context.lookup("r").unwrap();
    assert_eq!(r.dims(), vec![1, 2]);
    assert_eq!(fm_interp::value::to_f64_vec(r), vec![2.0, 3.0]);
}

#[test]
fn cumtrapz_vector() {
    let mut i = interp();
    i.run("r = cumtrapz([1 2 3]);").unwrap();
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("r").unwrap()),
        vec![0.0, 1.5, 4.0]
    );
}

#[test]
fn interp2_bilinear_midpoint() {
    assert!((eval_scalar("interp2([1 2; 3 4], 1.5, 1.5)") - 2.5).abs() < 1e-12);
}

#[test]
fn interp2_nearest() {
    // Z=[1 2;3 4], query (1.4,1.4) rounds toward (row1,col1) => 1.
    assert_eq!(eval_scalar("interp2([1 2; 3 4], 1.4, 1.4, 'nearest')"), 1.0);
}

#[test]
fn interp2_out_of_range_nan() {
    assert!(eval_scalar("interp2([1 2; 3 4], 5, 5)").is_nan());
}

#[test]
fn interp2_full_grid_form() {
    // X=[10 20], Y=[100 200], midpoint should give bilinear average 2.5.
    let v = eval_scalar("interp2([10 20],[100 200],[1 2; 3 4], 15, 150)");
    assert!((v - 2.5).abs() < 1e-12);
}

#[test]
fn meshgrid_shapes() {
    let mut i = interp();
    i.run("[X,Y] = meshgrid([1 2 3],[4 5]);").unwrap();
    assert_eq!(i.context.lookup("X").unwrap().dims(), vec![2, 3]);
}

#[test]
fn vec_flattens() {
    let mut i = interp();
    i.run("v = vec([1 2; 3 4]);").unwrap();
    assert_eq!(i.context.lookup("v").unwrap().dims(), vec![4, 1]);
}

#[test]
fn erf_known_value() {
    assert!((eval_scalar("erf(0)")).abs() < 1e-6);
    assert!((eval_scalar("erf(10)") - 1.0).abs() < 1e-6);
}

#[test]
fn gamma_factorial() {
    // gamma(5) = 4! = 24.
    assert!((eval_scalar("gamma(5)") - 24.0).abs() < 1e-6);
}

// ---- eps as function --------------------------------------------------------

#[test]
fn eps_function_forms() {
    assert_eq!(eval_scalar("eps('double')"), f64::EPSILON);
    assert!((eval_scalar("eps(1.0)") - f64::EPSILON).abs() < 1e-30);
}

// ---- seed reproducibility ---------------------------------------------------

#[test]
fn seed_reproduces_sequence() {
    let mut i = interp();
    i.run("seed(32,41); a = rand(1,5); seed(32,41); b = rand(1,5); s = issame(a,b);")
        .unwrap();
    assert_eq!(i.context.lookup("s").unwrap().as_f64(), Some(1.0));
}

// ---- Stage 9: sparse matrices -----------------------------------------------

#[test]
fn sparse_full_round_trip() {
    // sparse(dense) then full() recovers the original.
    let orig = vec_of("a = [0 0 2; 1 0 0; 0 3 0];", "a");
    let back = vec_of("a = [0 0 2; 1 0 0; 0 3 0]; b = full(sparse(a));", "b");
    assert_eq!(orig, back);
}

#[test]
fn issparse_and_nnz() {
    let mut i = interp();
    i.run("A = sparse([0 0 2; 1 0 0; 0 3 0]); is = issparse(A); n = nnz(A);")
        .unwrap();
    assert_eq!(i.context.lookup("is").unwrap().as_f64(), Some(1.0));
    assert_eq!(i.context.lookup("n").unwrap().as_f64(), Some(3.0));
    // A dense matrix is not sparse.
    i.run("d = issparse([1 2 3]);").unwrap();
    assert_eq!(i.context.lookup("d").unwrap().as_f64(), Some(0.0));
}

#[test]
fn speye_full() {
    let v = vec_of("e = full(speye(3));", "e");
    assert_eq!(v, vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn spdiags_diagonal() {
    // spdiags of a length-3 column on the main diagonal of a 3x3.
    let v = vec_of("S = spdiags([1;2;3], 0, 3, 3); f = full(S);", "f");
    assert_eq!(v, vec![1.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0]);
}

#[test]
fn sparse_find_triplets() {
    // [i,j,v] = find(sparse(...)) then rebuild via sparse(i,j,v).
    let v = vec_of(
        "a = [0 0 2; 1 0 0; 0 3 0]; A = sparse(a); \
         [i,j,s] = find(A); B = sparse(i,j,s,3,3); f = full(B);",
        "f",
    );
    assert_eq!(v, vec![0.0, 1.0, 0.0, 0.0, 0.0, 3.0, 2.0, 0.0, 0.0]);
}

#[test]
fn sparse_add_and_scale() {
    // (sparse + sparse) and (scalar * sparse) stay correct vs dense.
    let v = vec_of(
        "a = [1 0; 0 2]; b = [0 3; 4 0]; \
         C = sparse(a) + sparse(b); f = full(2 * C);",
        "f",
    );
    // a+b = [1 3; 4 2]; 2*(a+b) = [2 6; 8 4], column-major.
    assert_eq!(v, vec![2.0, 8.0, 6.0, 4.0]);
}

#[test]
fn sparse_matmul() {
    // sparse * sparse, compared to the dense product.
    let dense = vec_of("a=[1 0 3; 0 2 0]; b=[1 0;0 1;1 0]; c = a*b;", "c");
    let sp = vec_of(
        "a=[1 0 3; 0 2 0]; b=[1 0;0 1;1 0]; C = sparse(a)*sparse(b); c = full(C);",
        "c",
    );
    assert_eq!(dense, sp);
}

#[test]
fn sparse_index_read() {
    // A(i,j) and A(linear) read the right values from a sparse matrix.
    let mut i = interp();
    i.run("a=[1 2 0 0 4;3 2 0 0 5;0 0 3 0 2]; A = sparse(a); x = A(2,5); y = A(7);")
        .unwrap();
    assert_eq!(i.context.lookup("x").unwrap().as_f64(), Some(5.0));
    // linear index 7 (column-major) into a 3x5: row 1, col 3 → 0.
    assert_eq!(i.context.lookup("y").unwrap().as_f64(), Some(0.0));
}

#[test]
fn sparse_transpose() {
    let v = vec_of("a=[1 0 3;0 2 0]; A = sparse(a); f = full(A');", "f");
    // transpose of [1 0 3;0 2 0] is [1 0;0 2;3 0], column-major.
    assert_eq!(v, vec![1.0, 0.0, 3.0, 0.0, 2.0, 0.0]);
}

#[test]
fn sparse_display_triplets() {
    use fm_core::FormatMode;
    let mut i = interp();
    i.run("A = sparse([0 0; 5 0]);").unwrap();
    let s = i.context.lookup("A").unwrap().format(FormatMode::Short);
    // (2,1)  5 in the body.
    assert!(s.contains("(2,1)"), "display was: {s}");
    assert!(s.contains('5'), "display was: {s}");
}

#[test]
fn sparse_solve_known_system() {
    // A \ b with a sparse A densifies to a correct solve.
    let v = vec_of("A = sparse([2 0;0 4]); b = [2;8]; x = A\\b;", "x");
    assert_eq!(v, vec![1.0, 2.0]);
}

#[test]
fn isset_requires_nonempty() {
    // FreeMat `isset` is true only for a defined, non-empty variable.
    let mut i = interp();
    i.run("a = []; b = 1; r1 = isset('c'); r2 = isset('a'); r3 = isset('b');")
        .unwrap();
    assert_eq!(i.context.lookup("r1").unwrap().as_f64(), Some(0.0));
    assert_eq!(i.context.lookup("r2").unwrap().as_f64(), Some(0.0));
    assert_eq!(i.context.lookup("r3").unwrap().as_f64(), Some(1.0));
}

#[test]
fn evalin_assignin_target_caller_scope() {
    // `assignin('caller', ...)` operates on the caller's scope (here `outer`),
    // not the helper subfunction's frame.
    let mut i = interp();
    i.define_source(
        "function y = outer()\n  y = 0;\n  helper();\nfunction helper()\n  assignin('caller','y',7);\n",
    )
    .unwrap();
    i.run("r = outer();").unwrap();
    assert_eq!(i.context.lookup("r").unwrap().as_f64(), Some(7.0));
}

#[test]
fn builtin_bypasses_user_shadow() {
    // `builtin('abs', x)` calls the native abs even when a same-named user
    // function shadows it.
    let mut i = interp();
    i.define_source(
        "function y = test_abs()\n  y = builtin('abs', -3);\nfunction y = abs(x)\n  y = x;\n",
    )
    .unwrap();
    i.run("r = test_abs();").unwrap();
    assert_eq!(i.context.lookup("r").unwrap().as_f64(), Some(3.0));
}

// ---- Function handles (feature pass) ----------------------------------------

#[test]
fn named_handle_calls_builtin() {
    assert!(eval_scalar("h = @sin; h(0)").abs() < 1e-12);
}

#[test]
fn anon_closure_captures_by_value() {
    let mut i = interp();
    // `c` is captured by value at definition; mutating it afterwards must not
    // change the closure's result.
    i.run("c = 3; g = @(x) x + c; c = 100; r = g(2);").unwrap();
    assert_eq!(i.context.lookup("r").unwrap().as_f64(), Some(5.0));
}

#[test]
fn anon_closure_multi_param() {
    assert_eq!(eval_scalar("f = @(x,y) x*y + 1; f(3,4)"), 13.0);
}

#[test]
fn feval_with_name_and_handle() {
    assert_eq!(eval_scalar("feval('mod', 7, 3)"), 1.0);
    assert_eq!(eval_scalar("h = @(a,b) a-b; feval(h, 9, 4)"), 5.0);
}

#[test]
fn func2str_and_str2func_roundtrip() {
    let mut i = interp();
    i.run("h = str2func('cos'); s = func2str(h); r = h(0);")
        .unwrap();
    assert_eq!(
        i.context.lookup("s").unwrap().as_string().as_deref(),
        Some("cos")
    );
    assert!((i.context.lookup("r").unwrap().as_f64().unwrap() - 1.0).abs() < 1e-12);
}

#[test]
fn is_function_handle_and_isa() {
    assert_eq!(eval_scalar("is_function_handle(@sin)"), 1.0);
    assert_eq!(eval_scalar("is_function_handle(3)"), 0.0);
    assert_eq!(eval_scalar("isa(@sin, 'function_handle')"), 1.0);
}

#[test]
fn handle_in_struct_field_is_callable() {
    // Regression for the test_fptr1 corpus case: `a.b = @cos; a.b(2.0)`.
    assert!((eval_scalar("a.b = @cos; a.b(2.0)") - (2.0_f64).cos()).abs() < 1e-12);
}

#[test]
fn arrayfun_with_handle() {
    let mut i = interp();
    i.run("r = arrayfun(@(x) x*x, [1 2 3 4]);").unwrap();
    let r = i.context.lookup("r").unwrap();
    assert_eq!(fm_interp::value::to_f64_vec(r), vec![1.0, 4.0, 9.0, 16.0]);
}

#[test]
fn cellfun_with_handle() {
    let mut i = interp();
    i.run("r = cellfun(@numel, {[1 2 3], [1 2], [1]});")
        .unwrap();
    let r = i.context.lookup("r").unwrap();
    assert_eq!(fm_interp::value::to_f64_vec(r), vec![3.0, 2.0, 1.0]);
}

// ---- N-D arrays (feature pass) ----------------------------------------------

#[test]
fn cat_along_third_dim() {
    let mut i = interp();
    i.run("a = [1 2; 3 4]; b = [5 6; 7 8]; c = cat(3, a, b); s = size(c);")
        .unwrap();
    let s = i.context.lookup("s").unwrap();
    assert_eq!(fm_interp::value::to_f64_vec(s), vec![2.0, 2.0, 2.0]);
    // c(:,:,2) should equal b.
    i.run("p = c(:,:,2);").unwrap();
    let p = i.context.lookup("p").unwrap();
    assert_eq!(fm_interp::value::to_f64_vec(p), vec![5.0, 7.0, 6.0, 8.0]);
}

#[test]
fn nd_vertical_concat_of_pages() {
    // `[a;a]` of a 1x2x2 array yields a 2x2x2 array (matcat2 corpus case).
    let mut i = interp();
    i.run("a = zeros(1,2,2); a(1,1,1)=1; a(1,2,1)=2; a(1,1,2)=5; a(1,2,2)=6; c=[a;a]; s=size(c);")
        .unwrap();
    let s = i.context.lookup("s").unwrap();
    assert_eq!(fm_interp::value::to_f64_vec(s), vec![2.0, 2.0, 2.0]);
    i.run("c1 = c(:,:,1); c2 = c(:,:,2);").unwrap();
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("c1").unwrap()),
        vec![1.0, 1.0, 2.0, 2.0]
    );
    assert_eq!(
        fm_interp::value::to_f64_vec(i.context.lookup("c2").unwrap()),
        vec![5.0, 5.0, 6.0, 6.0]
    );
}

#[test]
fn nd_assign_grows_to_3d() {
    // a(2,:,:) = r grows an undefined `a` to 2x2x2 (assign14 corpus case).
    let mut i = interp();
    i.run("r = [2,3;3,4]; a(2,:,:) = r; b = zeros(2,2,2); b(2,:,:) = r; t = all(a(:)==b(:)) && all(size(a)==size(b));")
        .unwrap();
    assert_eq!(i.context.lookup("t").unwrap().as_f64(), Some(1.0));
}

#[test]
fn repmat_nd() {
    let mut i = interp();
    i.run("s = ones(2,2,2); p = repmat(s,[2 2 1]); t = all(p(:) == 1) && all(size(p)==[4 4 2]);")
        .unwrap();
    assert_eq!(i.context.lookup("t").unwrap().as_f64(), Some(1.0));
}

#[test]
fn squeeze_nd() {
    let mut i = interp();
    i.run("a = ones(1,3,1); b = squeeze(a); s = size(b);")
        .unwrap();
    let s = i.context.lookup("s").unwrap();
    assert_eq!(fm_interp::value::to_f64_vec(s), vec![3.0, 1.0]);
}

// ---- conv2 / dlmread (feature pass) -----------------------------------------

#[test]
fn conv2_full_real() {
    let mut i = interp();
    i.run("c = conv2([1 1; 1 1], [1 0; 0 1]);").unwrap();
    let c = i.context.lookup("c").unwrap();
    // [1 1;1 1] * [1 0;0 1] full → 3x3.
    assert_eq!(c.dims(), vec![3, 3]);
    assert_eq!(
        fm_interp::value::to_f64_vec(c),
        vec![1.0, 1.0, 0.0, 1.0, 2.0, 1.0, 0.0, 1.0, 1.0]
    );
}

// ---- crash-fix & new-builtin regression tests (flip/sort/rand pass) ---------

/// Helper: column-major f64 data of variable `name`.
fn data_of(i: &fm_interp::Interpreter, name: &str) -> Vec<f64> {
    fm_interp::value::to_f64_vec(i.context.lookup(name).unwrap())
}

#[test]
fn fliplr_nd_integer_does_not_panic() {
    // Regression: fliplr/flipud on an N-D integer array used to panic.
    let mut i = interp();
    i.run("a = int32(reshape(1:12,2,2,3)); b = fliplr(a); s = size(b);")
        .unwrap();
    assert_eq!(data_of(&i, "s"), vec![2.0, 2.0, 3.0]);
    // First page of a [1 3;2 4] page flipped left-right is [3 1;4 2].
    i.run("p = b(:,:,1);").unwrap();
    assert_eq!(data_of(&i, "p"), vec![3.0, 4.0, 1.0, 2.0]);
    assert_eq!(
        i.context.lookup("b").unwrap().class(),
        fm_core::DataClass::Int32
    );
}

#[test]
fn flipud_nd_does_not_panic() {
    let mut i = interp();
    i.run("a = reshape(1:12,2,2,3); b = flipud(a); p = b(:,:,1);")
        .unwrap();
    // [1 3;2 4] flipped up-down -> [2 4;1 3].
    assert_eq!(data_of(&i, "p"), vec![2.0, 1.0, 4.0, 3.0]);
}

#[test]
fn circshift_nd_does_not_panic() {
    let mut i = interp();
    i.run("a = reshape(1:8,2,2,2); b = circshift(a,[1 0 0]); p = b(:,:,1);")
        .unwrap();
    // page1 = [1 3;2 4], shift rows down by 1 -> [2 4;1 3].
    assert_eq!(data_of(&i, "p"), vec![2.0, 1.0, 4.0, 3.0]);
}

#[test]
fn sort_cellstr_lexicographic() {
    let mut i = interp();
    i.run("[b,ix] = sort({'hello','abba','goodbye'});").unwrap();
    let b = i.context.lookup("b").unwrap();
    let cells = b.as_cell().unwrap();
    let got: Vec<String> = cells.iter().filter_map(fm_core::Array::as_string).collect();
    assert_eq!(got, vec!["abba", "goodbye", "hello"]);
    assert_eq!(data_of(&i, "ix"), vec![2.0, 3.0, 1.0]);
}

#[test]
fn sort_int_two_output_no_panic() {
    let mut i = interp();
    i.run("[b,ix] = sort(int32([3 1 2]));").unwrap();
    assert_eq!(data_of(&i, "b"), vec![1.0, 2.0, 3.0]);
    assert_eq!(data_of(&i, "ix"), vec![2.0, 3.0, 1.0]);
}

#[test]
fn rand_state_no_arg_returns_state_not_giant_alloc() {
    // Regression: rand('state') used to treat 'state' as a size and SIGABRT.
    let mut i = interp();
    i.run("rand('state',0); s = rand('state');").unwrap();
    // Returns a small numeric state vector (here a scalar), never a huge array.
    assert!(i.context.lookup("s").unwrap().numel() <= 4);
}

#[test]
fn rand_state_seed_is_deterministic() {
    let mut i = interp();
    i.run("rand('state',7); a = rand(1,3); rand('state',7); b = rand(1,3);")
        .unwrap();
    assert_eq!(data_of(&i, "a"), data_of(&i, "b"));
}

#[test]
fn randn_state_does_not_panic() {
    let mut i = interp();
    i.run("randn('state',0); x = randn(2,2); s = size(x);")
        .unwrap();
    assert_eq!(data_of(&i, "s"), vec![2.0, 2.0]);
}

#[test]
fn rand_size_vector_still_works() {
    let mut i = interp();
    i.run("s = size(rand([2,2,2]));").unwrap();
    assert_eq!(data_of(&i, "s"), vec![2.0, 2.0, 2.0]);
}

#[test]
fn flipdim_and_transpose() {
    let mut i = interp();
    i.run("b = flipdim(int32(reshape(1:8,2,2,2)),3); p = b(:,:,1);")
        .unwrap();
    // flip along dim 3 -> page1 becomes original page2 = [5 7;6 8].
    assert_eq!(data_of(&i, "p"), vec![5.0, 6.0, 7.0, 8.0]);
    i.run("t = transpose([1 2 3; 4 5 6]);").unwrap();
    assert_eq!(i.context.lookup("t").unwrap().dims(), vec![3, 2]);
}

#[test]
fn cellstr_strstr_char_predicates() {
    let mut i = interp();
    i.run("c = cellstr(['quick';'brown';'fox  ']);").unwrap();
    let got: Vec<String> = i
        .context
        .lookup("c")
        .unwrap()
        .as_cell()
        .unwrap()
        .iter()
        .filter_map(fm_core::Array::as_string)
        .collect();
    assert_eq!(got, vec!["quick", "brown", "fox"]);
    assert_eq!(eval_scalar("strstr('hello','lo')"), 4.0);
    assert_eq!(eval_scalar("strstr('free','lunch')"), 0.0);
    i.run("d = isdigit('a1b2');").unwrap();
    assert_eq!(data_of(&i, "d"), vec![0.0, 1.0, 0.0, 1.0]);
}

#[test]
fn idiv_erfinv_fullfile_norm_p() {
    assert_eq!(eval_scalar("idiv(27,6)"), 4.0);
    assert_eq!(eval_scalar("idiv(4,-2)"), -2.0);
    assert!((eval_scalar("erfinv(0.5)") - 0.476_936_27).abs() < 1e-4);
    let mut i = interp();
    i.run("f = fullfile('a','b','c.m');").unwrap();
    assert_eq!(
        i.context.lookup("f").unwrap().as_string().unwrap(),
        "a/b/c.m"
    );
    // p-norm of a vector.
    assert!((eval_scalar("norm([3;4],2)") - 5.0).abs() < 1e-12);
    assert!((eval_scalar("norm([1;2;3;4],-inf)") - 1.0).abs() < 1e-12);
}

#[test]
fn char_multi_arg_pads_matrix() {
    let mut i = interp();
    i.run("m = char('hello','to','the','world');").unwrap();
    assert_eq!(i.context.lookup("m").unwrap().dims(), vec![4, 5]);
    i.run("c = cast(pi,'uint8');").unwrap();
    assert_eq!(eval_scalar("cast(pi,'uint8')"), 3.0);
}

#[test]
fn isstr_alias_of_ischar() {
    assert_eq!(eval_scalar("isstr('x')"), 1.0);
    assert_eq!(eval_scalar("isstr(1)"), 0.0);
}

#[test]
fn issquare_predicate() {
    assert_eq!(eval_scalar("issquare(eye(3))"), 1.0);
    assert_eq!(eval_scalar("issquare([1 2 3])"), 0.0);
}

#[test]
fn isinttype_predicate() {
    assert_eq!(eval_scalar("isinttype(int8(1))"), 1.0);
    assert_eq!(eval_scalar("isinttype(1)"), 0.0);
}

#[test]
fn mpower_matches_operator() {
    let mut i = interp();
    i.run("a = mpower([1,2;3,4],2); b = [1,2;3,4]^2; d = isequal(a,b);")
        .unwrap();
    assert_eq!(i.context.lookup("d").unwrap().as_f64(), Some(1.0));
}

#[test]
fn isequalwithequalnans_treats_nans_equal() {
    assert_eq!(eval_scalar("isequalwithequalnans(nan,nan)"), 1.0);
    assert_eq!(eval_scalar("isequal(nan,nan)"), 0.0);
}

#[test]
fn maxdim_returns_index_of_largest_dim() {
    // FreeMat's maxdim returns the *index* of the largest dimension:
    // size([..]) = [2,5] -> max is dim 2.
    assert_eq!(eval_scalar("maxdim(zeros(2,5))"), 2.0);
    assert_eq!(eval_scalar("maxdim(zeros(7,3))"), 1.0);
}

#[test]
fn isinf_isnan_capitalized_aliases() {
    assert_eq!(eval_scalar("IsInf(inf)"), 1.0);
    assert_eq!(eval_scalar("IsNaN(nan)"), 1.0);
}

#[test]
fn computer_returns_nonempty_string() {
    let mut i = interp();
    i.run("c = computer;").unwrap();
    assert!(i.context.lookup("c").unwrap().is_char());
    assert!(
        !i.context
            .lookup("c")
            .unwrap()
            .as_string()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn mfilename_empty_at_repl() {
    let mut i = interp();
    i.run("m = mfilename;").unwrap();
    assert_eq!(i.context.lookup("m").unwrap().as_string().unwrap(), "");
}

#[test]
fn odeset_builds_options_struct() {
    let mut i = interp();
    i.run("o = odeset('RelTol',1e-4,'AbsTol',1e-7);").unwrap();
    let o = i.context.lookup("o").unwrap();
    let s = o.as_struct().expect("odeset returns a struct");
    let rt = s.scalar_field("RelTol").and_then(fm_core::Array::as_f64);
    let at = s.scalar_field("AbsTol").and_then(fm_core::Array::as_f64);
    assert_eq!(rt, Some(1e-4));
    assert_eq!(at, Some(1e-7));
}

#[test]
fn odeset_updates_existing_struct_case_insensitive() {
    let mut i = interp();
    i.run("o = odeset('reltol',1e-3); o = odeset(o,'AbsTol',1e-9);")
        .unwrap();
    let o = i.context.lookup("o").unwrap();
    let s = o.as_struct().unwrap();
    // 'reltol' normalises to the canonical 'RelTol'.
    assert_eq!(
        s.scalar_field("RelTol").and_then(fm_core::Array::as_f64),
        Some(1e-3)
    );
    assert_eq!(
        s.scalar_field("AbsTol").and_then(fm_core::Array::as_f64),
        Some(1e-9)
    );
}

#[test]
fn deval_interpolates_ode45_solution() {
    let mut i = interp();
    i.run("SOL = ode45(@(t,y) y,[0 3],1);").unwrap();
    i.run("a = deval(SOL, 3);").unwrap();
    i.run("b = deval(SOL, 0);").unwrap();
    let a = i.context.lookup("a").unwrap().as_f64().unwrap();
    let b = i.context.lookup("b").unwrap().as_f64().unwrap();
    assert!((a - 3.0_f64.exp()).abs() < 1e-2, "deval(SOL,3)={a}");
    assert!((b - 1.0).abs() < 1e-6, "deval(SOL,0)={b}");
}

#[test]
fn deval_vector_query_returns_states_by_n() {
    let mut i = interp();
    i.run("SOL = ode45(@(t,y) y,[0 3],1);").unwrap();
    i.run("v = deval(SOL, [0 1 2 3]);").unwrap();
    let v = i.context.lookup("v").unwrap();
    assert_eq!(v.dims(), vec![1, 4]);
    let data = fm_interp::value::to_f64_vec(v);
    assert!((data[0] - 1.0).abs() < 1e-6);
    assert!((data[3] - 3.0_f64.exp()).abs() < 1e-2);
}

#[test]
fn ode45_accepts_odeset_options() {
    let mut i = interp();
    // Tight tolerance via odeset should solve y'=y to high accuracy.
    i.run("o = odeset('RelTol',1e-8,'AbsTol',1e-10); [t,y]=ode45(@(t,y) y,[0 1],1,o);")
        .unwrap();
    let y = i.context.lookup("y").unwrap();
    let data = fm_interp::value::to_f64_vec(y);
    let last = *data.last().unwrap();
    assert!((last - 1.0_f64.exp()).abs() < 1e-6, "y(end)={last}");
}
