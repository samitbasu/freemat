//! End-to-end: load and run a real `toolbox/*.m` function through the same
//! evaluator that will later run all 317 toolbox files unchanged.

mod common;

use fm_interp::Interpreter;

/// Path to the toolbox copy in the workspace.
fn toolbox(rel: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../toolbox")
        .join(rel)
}

#[test]
fn run_isvector_from_toolbox() {
    // `isvector.m`:
    //   function y = isvector(x)
    //     s = size(x);
    //     y = (prod(s) == (s(1)*s(2))) && ((s(1) == 1) || (s(2) == 1));
    // Needs: size, prod, paren-index, ==, *, &&, ||.
    let mut i = Interpreter::new();
    i.load_file(toolbox("array/isvector.m"))
        .expect("isvector.m should load");

    i.run("a = isvector([1 2 3 4]);").unwrap();
    assert_eq!(i.context.lookup("a").unwrap().as_f64(), Some(1.0));

    i.run("b = isvector([1 2;3 4]);").unwrap();
    assert_eq!(i.context.lookup("b").unwrap().as_f64(), Some(0.0));

    i.run("c = isvector([1;2;3]);").unwrap();
    assert_eq!(i.context.lookup("c").unwrap().as_f64(), Some(1.0));
}

#[test]
fn run_isscalar_from_toolbox() {
    // `isscalar.m`:  v = (numel(a) == 1);  — needs numel + ==.
    let mut i = Interpreter::new();
    i.load_file(toolbox("array/isscalar.m"))
        .expect("isscalar.m should load");
    i.run("a = isscalar(5);").unwrap();
    assert_eq!(i.context.lookup("a").unwrap().as_f64(), Some(1.0));
    i.run("b = isscalar([1 2 3]);").unwrap();
    assert_eq!(i.context.lookup("b").unwrap().as_f64(), Some(0.0));
}
