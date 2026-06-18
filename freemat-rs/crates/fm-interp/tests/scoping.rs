//! Variable scoping: local / global / persistent, multi-return, nargin/nargout.

mod common;
use common::run_f64;

use fm_interp::Interpreter;

/// Run source and read a top-level variable as `f64`.
fn run(src: &str, var: &str) -> f64 {
    run_f64(src, var)
}

#[test]
fn locals_are_isolated_per_call() {
    let mut i = Interpreter::new();
    // Define a function that sets a local `t`; the caller's `t` is untouched.
    i.define_source("function y = f(x)\n  t = 100;\n  y = x + t;\n")
        .unwrap();
    i.run("t = 1; z = f(5);").unwrap();
    assert_eq!(i.context.lookup("t").unwrap().as_f64(), Some(1.0));
    assert_eq!(i.context.lookup("z").unwrap().as_f64(), Some(105.0));
}

#[test]
fn global_shared_across_scopes() {
    let mut i = Interpreter::new();
    i.define_source("function setg(v)\n  global G\n  G = v;\n")
        .unwrap();
    i.run("global G\n G = 0; setg(42);").unwrap();
    assert_eq!(i.context.lookup("G").unwrap().as_f64(), Some(42.0));
}

#[test]
fn persistent_retains_across_calls() {
    let mut i = Interpreter::new();
    i.define_source(
        "function y = counter()\n  persistent n\n  if isempty(n), n = 0; end\n  n = n + 1;\n  y = n;\n",
    )
    .unwrap();
    i.run("a = counter(); b = counter(); c = counter();")
        .unwrap();
    assert_eq!(i.context.lookup("a").unwrap().as_f64(), Some(1.0));
    assert_eq!(i.context.lookup("b").unwrap().as_f64(), Some(2.0));
    assert_eq!(i.context.lookup("c").unwrap().as_f64(), Some(3.0));
}

#[test]
fn subfunctions_are_file_local() {
    // A subfunction (a second top-level `function` in a file) is private to its
    // file: callable from the main function, but it must NOT leak into the
    // global table and clobber an identically named function from another file.
    let mut i = Interpreter::new();
    // File A: a public `helper/1` plus a private subfunction `priv` that
    // returns 1.
    i.define_source("function y = useA()\n  y = priv();\nfunction z = priv()\n  z = 1;\n")
        .unwrap();
    // File B: a different `priv` returning 2.
    i.define_source("function y = useB()\n  y = priv();\nfunction z = priv()\n  z = 2;\n")
        .unwrap();
    i.run("a = useA(); b = useB();").unwrap();
    assert_eq!(i.context.lookup("a").unwrap().as_f64(), Some(1.0));
    assert_eq!(i.context.lookup("b").unwrap().as_f64(), Some(2.0));
    // The private `priv` is not visible at the top level.
    assert!(!i.functions.contains("priv"));
}

#[test]
fn multi_return_assignment() {
    let mut i = Interpreter::new();
    i.define_source("function [a, b] = swap(x, y)\n  a = y;\n  b = x;\n")
        .unwrap();
    i.run("[p, q] = swap(1, 2);").unwrap();
    assert_eq!(i.context.lookup("p").unwrap().as_f64(), Some(2.0));
    assert_eq!(i.context.lookup("q").unwrap().as_f64(), Some(1.0));
}

#[test]
fn size_multi_return() {
    assert_eq!(run("A = [1 2 3;4 5 6]; [r, c] = size(A);", "r"), 2.0);
    assert_eq!(run("A = [1 2 3;4 5 6]; [r, c] = size(A);", "c"), 3.0);
}

#[test]
fn nargin_reported() {
    let mut i = Interpreter::new();
    i.define_source("function y = f(a, b, c)\n  y = nargin;\n")
        .unwrap();
    i.run("n = f(1, 2);").unwrap();
    assert_eq!(i.context.lookup("n").unwrap().as_f64(), Some(2.0));
}

#[test]
fn recursive_function() {
    let mut i = Interpreter::new();
    i.define_source(
        "function y = fact(n)\n  if n <= 1\n    y = 1;\n  else\n    y = n * fact(n - 1);\n  end\n",
    )
    .unwrap();
    i.run("f = fact(5);").unwrap();
    assert_eq!(i.context.lookup("f").unwrap().as_f64(), Some(120.0));
}

#[test]
fn fewer_outputs_requested() {
    // Requesting one output from a two-output function returns just the first.
    let mut i = Interpreter::new();
    i.define_source("function [a, b] = two()\n  a = 1;\n  b = 2;\n")
        .unwrap();
    i.run("x = two();").unwrap();
    assert_eq!(i.context.lookup("x").unwrap().as_f64(), Some(1.0));
}

#[test]
fn debug_seam_active_scope_switchable() {
    // The active-scope switch (basis for dbup/dbdown) is exercised directly.
    let mut i = Interpreter::new();
    i.run("x = 1;").unwrap();
    assert_eq!(i.context.active_index(), 0);
    i.context.push_scope("frame");
    assert_eq!(i.context.active_index(), 1);
    i.context.set_active(0);
    assert_eq!(i.context.active_index(), 0);
}

#[test]
fn multi_output_into_cell_contents() {
    // `[c{1:3}] = f` distributes the function's three outputs across the cell
    // slots (comma-list expansion), one value per indexed position.
    let mut i = Interpreter::new();
    i.define_source("function [a, b, c] = three()\n  a = 1;\n  b = 4;\n  c = 3;\n")
        .unwrap();
    i.run("d = {0,0,0,0}; [d{1:3}] = three();").unwrap();
    // Cell stays 1x4; first three slots get 1,4,3, the fourth is untouched.
    assert_eq!(i.context.lookup("d").unwrap().shape(), &[1, 4]);
    i.run("v1 = d{1}; v2 = d{2}; v3 = d{3}; v4 = d{4};")
        .unwrap();
    assert_eq!(i.context.lookup("v1").unwrap().as_f64(), Some(1.0));
    assert_eq!(i.context.lookup("v2").unwrap().as_f64(), Some(4.0));
    assert_eq!(i.context.lookup("v3").unwrap().as_f64(), Some(3.0));
    assert_eq!(i.context.lookup("v4").unwrap().as_f64(), Some(0.0));
}
