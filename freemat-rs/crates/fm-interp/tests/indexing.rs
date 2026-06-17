//! Indexing: read + grow-on-assign, logical, `end`, subscript, cell, struct.

mod common;
use common::{eval_f64, run_var, run_vec};

#[test]
fn linear_read() {
    assert_eq!(eval_f64("v = [10 20 30]; v(2)"), 20.0);
    assert_eq!(eval_f64("v = [10 20 30]; v(end)"), 30.0);
}

#[test]
fn subscript_read() {
    // A = [1 2;3 4]; A(2,1) == 3
    assert_eq!(eval_f64("A = [1 2;3 4]; A(2,1)"), 3.0);
    assert_eq!(eval_f64("A = [1 2;3 4]; A(1,2)"), 2.0);
    assert_eq!(eval_f64("A = [1 2;3 4]; A(end,end)"), 4.0);
}

#[test]
fn range_index() {
    assert_eq!(
        run_vec("v = [10 20 30 40]; x = v(2:3);", "x"),
        vec![20.0, 30.0]
    );
    assert_eq!(
        run_vec("v = [10 20 30 40]; x = v(2:end);", "x"),
        vec![20.0, 30.0, 40.0]
    );
}

#[test]
fn colon_index() {
    assert_eq!(
        run_vec("A = [1 2;3 4]; x = A(:);", "x"),
        vec![1.0, 3.0, 2.0, 4.0]
    );
    // A(:,1) → first column.
    assert_eq!(run_vec("A = [1 2;3 4]; x = A(:,1);", "x"), vec![1.0, 3.0]);
}

#[test]
fn logical_index() {
    assert_eq!(
        run_vec("v = [10 20 30]; x = v(v > 15);", "x"),
        vec![20.0, 30.0]
    );
    assert_eq!(
        run_vec("v = [1 2 3 4]; m = logical([1 0 1 0]); x = v(m);", "x"),
        vec![1.0, 3.0]
    );
}

#[test]
fn assign_in_place() {
    assert_eq!(
        run_vec("v = [1 2 3]; v(2) = 99;", "v"),
        vec![1.0, 99.0, 3.0]
    );
}

#[test]
fn grow_on_assign_vector() {
    // Assigning past the end grows the vector with zeros.
    assert_eq!(
        run_vec("v = [1 2 3]; v(5) = 9;", "v"),
        vec![1.0, 2.0, 3.0, 0.0, 9.0]
    );
}

#[test]
fn grow_from_nothing() {
    // No prior variable: v(3) = 7 creates a 1x3 row.
    assert_eq!(run_vec("v(3) = 7;", "v"), vec![0.0, 0.0, 7.0]);
}

#[test]
fn subscript_assign_grow() {
    // A(2,3) = 5 on a fresh variable grows to 2x3.
    let v = run_var("A(2,3) = 5;", "A");
    assert_eq!(v.dims(), vec![2, 3]);
    assert_eq!(
        common::run_vec("A(2,3) = 5;", "A"),
        vec![0.0, 0.0, 0.0, 0.0, 0.0, 5.0]
    );
}

#[test]
fn logical_assign() {
    // v(v>2) = 0
    assert_eq!(
        run_vec("v = [1 2 3 4]; v(v>2) = 0;", "v"),
        vec![1.0, 2.0, 0.0, 0.0]
    );
}

#[test]
fn cell_brace_read_write() {
    assert_eq!(eval_f64("c = {1, 'hi', 3}; c{1}"), 1.0);
    let v = run_var("c = {1,2}; c{3} = 7;", "c");
    assert_eq!(v.numel(), 3);
}

#[test]
fn cell_brace_string() {
    let v = run_var("c = {'hello'}; x = c{1};", "x");
    assert_eq!(v.as_string().as_deref(), Some("hello"));
}

#[test]
fn struct_field_read_write() {
    let v = run_var("s.a = 5; s.b = 7;", "s");
    assert_eq!(v.class_name(), "struct");
    assert_eq!(eval_f64("s.a = 5; s.b = 7; s.a + s.b"), 12.0);
}

#[test]
fn dynamic_field() {
    assert_eq!(eval_f64("s.x = 42; f = 'x'; s.(f)"), 42.0);
}
