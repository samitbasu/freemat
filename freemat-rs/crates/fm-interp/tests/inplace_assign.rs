//! In-place indexed-assignment fast path: copy-on-write correctness + value
//! correctness + fallback correctness.
//!
//! The hot-path optimisation (`scatter_into` + take/put-back) must be
//! *observationally identical* to the old materialise-and-rebuild `scatter`.
//! The most important guard is the **aliasing COW test**: `B = A; A(i,j) = x`
//! must not disturb `B`.

mod common;
use common::{run_var, run_vec};
use fm_interp::value::to_f64_vec;

/// **The single most important correctness guard.** After `B = A`, mutating `A`
/// in place via indexed assignment must *not* touch `B` — the shared backing
/// `Arc` has strong-count 2, so `make_mut_*` must deep-copy on the first write.
#[test]
fn cow_alias_not_disturbed_by_indexed_assign() {
    // Matrix subscript write.
    let interp = common::run("A = [1 2;3 4]; B = A; A(1,1) = 99;");
    assert_eq!(
        to_f64_vec(&interp.context.lookup("B").unwrap().clone()),
        vec![1.0, 3.0, 2.0, 4.0],
        "B must be the original matrix, unaffected by A(1,1)=99"
    );
    assert_eq!(
        to_f64_vec(&interp.context.lookup("A").unwrap().clone()),
        vec![99.0, 3.0, 2.0, 4.0],
        "A must reflect the in-place write"
    );

    // Linear write into a vector alias.
    let interp = common::run("a = [10 20 30]; b = a; a(2) = 0;");
    assert_eq!(
        to_f64_vec(&interp.context.lookup("b").unwrap().clone()),
        vec![10.0, 20.0, 30.0],
        "b must be unaffected by a(2)=0"
    );
    assert_eq!(
        to_f64_vec(&interp.context.lookup("a").unwrap().clone()),
        vec![10.0, 0.0, 30.0]
    );
}

/// Linear-index assignment past the end of a **2-D matrix** must not panic.
/// FreeMat flattens the matrix to a row vector and grows it (column-major
/// order preserved); a single past-end index also grows the same way.
#[test]
fn linear_grow_past_end_of_matrix_flattens_to_row() {
    // `a = [1 2;3 4]; a(7) = 9` → `1×7` row `[1 3 2 4 0 0 9]`.
    let interp = common::run("a = [1 2;3 4]; a(7) = 9;");
    let a = interp.context.lookup("a").unwrap().clone();
    assert_eq!(
        a.shape(),
        &[1, 7],
        "matrix flattens to a row on linear grow"
    );
    assert_eq!(
        to_f64_vec(&a),
        vec![1.0, 3.0, 2.0, 4.0, 0.0, 0.0, 9.0],
        "column-major order preserved, gaps zero-filled"
    );

    // Multi-position past-end write (regression for the documented panic in
    // sparse `a(p) = v` with `max(p) > numel(a)`).
    let interp = common::run("a = [1,0,3;6,2,3]; a([3;4;9]) = [4,6,3];");
    let a = interp.context.lookup("a").unwrap().clone();
    assert_eq!(a.shape(), &[1, 9]);
    assert_eq!(
        to_f64_vec(&a),
        vec![1.0, 6.0, 4.0, 6.0, 3.0, 3.0, 0.0, 0.0, 3.0],
    );
}

/// COW must also hold across many writes (the loop case): only the *first*
/// write after aliasing copies; subsequent writes mutate the now-unshared copy.
#[test]
fn cow_alias_holds_across_many_writes() {
    let src = "A = zeros(3); B = A; for k = 1:3; A(k,k) = k; end";
    assert_eq!(
        run_vec(src, "B"),
        vec![0.0; 9],
        "B must stay all-zeros while A is filled in place"
    );
    // A's diagonal is 1,2,3 (column-major positions 0,4,8).
    let a = to_f64_vec(&run_var(src, "A"));
    assert_eq!((a[0], a[4], a[8]), (1.0, 2.0, 3.0));
}

// ---- Value correctness across scatter kinds (fast path) ---------------------

#[test]
fn scalar_scatter() {
    assert_eq!(
        run_vec("A = [1 2;3 4]; A(2,2) = 7;", "A"),
        vec![1.0, 3.0, 2.0, 7.0]
    );
}

#[test]
fn vector_scatter_matches_count() {
    assert_eq!(
        run_vec("v = [1 2 3 4]; v(2:3) = [20 30];", "v"),
        vec![1.0, 20.0, 30.0, 4.0]
    );
}

#[test]
fn logical_mask_scatter() {
    assert_eq!(
        run_vec("v = [1 2 3 4]; v(logical([1 0 1 0])) = 0;", "v"),
        vec![0.0, 2.0, 0.0, 4.0]
    );
}

#[test]
fn range_scatter_scalar_fill() {
    assert_eq!(
        run_vec("v = [1 2 3 4 5]; v(2:4) = 9;", "v"),
        vec![1.0, 9.0, 9.0, 9.0, 5.0]
    );
}

#[test]
fn colon_scatter() {
    // A(:,1) = [7;8] replaces the first column.
    assert_eq!(
        run_vec("A = [1 2;3 4]; A(:,1) = [7;8];", "A"),
        vec![7.0, 8.0, 2.0, 4.0]
    );
    // A(:) = scalar fills everything.
    assert_eq!(run_vec("A = [1 2;3 4]; A(:) = 5;", "A"), vec![5.0; 4]);
}

#[test]
fn integer_class_scatter_in_place() {
    // int8 array, in-place write keeps the int8 class. The in-place fast path
    // must produce identical bytes to the rebuild path's `f64`→int8 cast: this
    // codebase truncates (`sat_i(v) as i8`), so `int8(200)` is the same value
    // whether assigned in place or built fresh — that consistency is the point.
    let a = run_var("A = int8([1 2 3]); A(2) = 200;", "A");
    assert_eq!(a.class_name(), "int8");
    let fresh = run_var("x = int8(200);", "x");
    assert_eq!(
        to_f64_vec(&a),
        vec![1.0, to_f64_vec(&fresh)[0], 3.0],
        "in-place int8 write must match the int8() cast"
    );
    // In-bounds value writes exactly.
    let b = run_var("A = int8([1 2 3]); A(2) = 50;", "A");
    assert_eq!(to_f64_vec(&b), vec![1.0, 50.0, 3.0]);
}

// ---- Fallback correctness (rebuild path) ------------------------------------

#[test]
fn grow_falls_back() {
    assert_eq!(
        run_vec("v = [1 2 3]; v(5) = 9;", "v"),
        vec![1.0, 2.0, 3.0, 0.0, 9.0]
    );
}

#[test]
fn type_promote_falls_back() {
    // Assigning into a double keeps double (value 5).
    assert_eq!(
        run_vec("A = [1 2 3]; A(1) = int8(5);", "A"),
        vec![5.0, 2.0, 3.0]
    );
}

#[test]
fn complex_promote_falls_back() {
    let v = run_var("v = [1 2 3]; v(2) = 1 + 2i;", "v");
    assert!(v.is_complex(), "assigning complex must promote the array");
}

#[test]
fn char_scatter_falls_back() {
    let s = run_var("s = 'abcd'; s(2) = 'X';", "s");
    assert_eq!(s.as_string().as_deref(), Some("aXcd"));
}

#[test]
fn cell_paren_assign_falls_back() {
    let c = run_var("c = {1, 2, 3}; c(2) = {99};", "c");
    assert_eq!(c.class_name(), "cell");
}

#[test]
fn deletion_falls_back() {
    assert_eq!(
        run_vec("v = [1 2 3 4]; v(2) = [];", "v"),
        vec![1.0, 3.0, 4.0]
    );
}

// ---- Read correctness after the gather change -------------------------------

#[test]
fn gather_after_optimization() {
    assert_eq!(run_vec("A = [1 2;3 4]; x = A([1 4]);", "x"), vec![1.0, 4.0]);
    assert_eq!(run_vec("A = [1 2;3 4]; x = A(:,2);", "x"), vec![2.0, 4.0]);
    assert_eq!(
        run_vec("v = [10 20 30]; x = v([3 1]);", "x"),
        vec![30.0, 10.0]
    );
}

// ---- Timing (informational, NOT gating) -------------------------------------

/// Non-gating timing harness for the indexed-assignment hot path. Run with
/// `cargo test -p fm-interp --release -- --ignored --nocapture inplace_bench`.
/// The C++ FreeMat reference for `A=zeros(1000); A(i,j)=i+j` over 1e6 writes is
/// ~0.78s; the Rust port is in the same ballpark with the in-place fast path
/// (it was effectively unbounded — O(N) per write — before the fix). This is
/// deliberately not asserted (wall-clock asserts are flaky in CI).
#[test]
#[ignore = "timing-only benchmark; run manually with --release --ignored"]
fn inplace_bench() {
    use std::time::Instant;
    let mut interp = fm_interp::Interpreter::new();
    interp.run("A = zeros(1000);").unwrap();
    let t = Instant::now();
    interp
        .run("for i=1:1000; for j=1:1000; A(i,j)=i+j; end; end")
        .unwrap();
    let dt = t.elapsed();
    let check = interp.run("disp(A(1000,1000))");
    assert!(check.is_ok());
    eprintln!("inplace_bench: 1e6 indexed writes in {dt:?}");
    // A(1000,1000) == 2000 sanity.
    let a = to_f64_vec(&interp.context.lookup("A").unwrap().clone());
    assert_eq!(a[999 + 999 * 1000], 2000.0);
}
