//! Curated conformance subset — the **must-pass** gate for `cargo test`.
//!
//! These are FreeMat `.m` tests that the Stage 1–3 interpreter already passes
//! (arithmetic, indexing/assignment, control flow, ranges, `nargin`,
//! persistents, struct/cell subsetting, typecasts). They run as a normal
//! integration test so the workspace test gate stays green and any regression
//! in `fm-core`/`fm-parser`/`fm-interp` is caught immediately.
//!
//! The *broad* failing corpus is reported by the `fm-conformance` binary (and
//! the `#[ignore]`d `full_suite_report` test below), which never fails the
//! build — many tests are expected to fail until later stages land.

use fm_conformance::{Outcome, run_full_suite, summarize, test_root};

/// Tests known to pass today, as `"dir/name"`. If a later change breaks one of
/// these, this test fails — that is the point.
const CURATED: &[&str] = &[
    // array — multiple assignment, struct/size assign, diag
    "array/test_assign7",
    "array/test_assign8",
    "array/test_assign10",
    "array/test_assign11",
    "array/test_assign13",
    "array/test_assign22",
    "array/test_diag5",
    // flow — if / switch / continue / error
    "flow/test_if1",
    "flow/test_if2",
    "flow/test_if3",
    "flow/test_if4",
    "flow/test_switch1",
    "flow/test_switch2",
    "flow/test_switch5",
    "flow/test_switch6",
    "flow/test_switch7",
    "flow/test_switch8",
    "flow/test_continue1",
    "flow/test_error1",
    // functions — nargin
    "functions/test_nargin1",
    "functions/test_nargin2",
    // operators — ranges
    "operators/test_range9",
    // elementary — the `test()` helper itself + all/any-backed checks
    "elementary/test_test1",
    "elementary/test_test2",
    "elementary/test_test5",
    // array — Stage-5 builtins (det, diag, repmat, ones, isfloat/isinteger)
    "array/test_det1",
    "array/test_diag1",
    "array/test_repmat2",
    "array/test_isfloat1",
    "array/test_isinteger1",
    "array/test_ones1",
    // Stage-6 — array manipulation builtins
    "array/test_reshape1",
    "array/test_reshape2",
    "array/test_sort",
    "array/test_cell1",
    "array/test_permute1",
    "array/test_permute2",
    // Stage-6 — bug fixes: struct-array concat + element deletion
    "suite/test_struct1",
    "suite/test_struct2",
    "suite/test_struct3",
    "suite/test_assign19",
    "suite/test_subset19",
    "array/test_assign19",
    // Stage-6 — cell/struct + inspection builtins
    "inspection/test_fieldnames1",
    "inspection/test_isfield1",
    "elementary/test_getfield1",
    // Stage-6 — eval / feval
    "freemat/test_eval1",
    "freemat/test_eval2",
    "freemat/test_feval1",
    // typecast — uint64 round trip
    "typecast/test_uint64_1",
    // suite — assignment, control flow, subsetting, matrix concat, persistents
    "suite/test_assign7",
    "suite/test_assign8",
    "suite/test_assign10",
    "suite/test_assign11",
    "suite/test_assign13",
    "suite/test_assign22",
    "suite/test_if1",
    "suite/test_switch1",
    "suite/test_continue1",
    "suite/test_error1",
    "suite/test_matcat7",
    "suite/test_matcat8",
    "suite/test_subset2",
    "suite/test_subset17",
    "suite/test_vec1",
    "suite/test_range9",
    "suite/test_nargin1",
    "suite/test_persistent2",
    "suite/test_ret",
    "suite/test_uint64_1",
    // variables — persistents, subsetting, matrix concat
    "variables/test_persistent2",
    "variables/test_subset2",
    "variables/test_subset17",
    "variables/test_vec1",
    "variables/test_matcat7",
    "variables/test_matcat8",
];

/// The corpus must be checked into the crate (self-contained, no `../FreeMat`).
#[test]
fn corpus_is_self_contained() {
    let root = test_root();
    assert!(
        root.join("flow/test_if1.m").exists(),
        "test corpus missing at {} — copy FreeMat/tests/*.m into data/tests",
        root.display()
    );
}

#[test]
fn curated_subset_passes() {
    let mut broken = Vec::new();
    for spec in CURATED {
        let (dir, name) = spec.split_once('/').expect("spec is dir/name");
        let path = test_root().join(dir).join(format!("{name}.m"));
        let (outcome, detail) = fm_conformance::run_test_file(&path);
        if outcome != Outcome::Pass {
            broken.push(format!("{spec}: {outcome:?} ({detail})"));
        }
    }
    assert!(
        broken.is_empty(),
        "curated conformance tests regressed:\n{}",
        broken.join("\n")
    );
}

/// Guard against silent regressions in the *aggregate* pass count: the full
/// covered corpus must keep passing at least this many tests. Raise the floor
/// as later stages improve the number (the binary prints the live total).
/// Stage 5 raised the live total to 198/603 (32.8%); Stage 6 to 250/603
/// (41.5%). The floor allows a small margin for PRNG-dependent tests.
const PASS_FLOOR: usize = 246;

#[test]
fn full_suite_pass_count_does_not_regress() {
    fm_conformance::silence_panic_output();
    let by_dir = run_full_suite();
    let pass: usize = by_dir.values().map(|rs| summarize(rs).pass).sum::<usize>();
    assert!(
        pass >= PASS_FLOOR,
        "full-suite pass count regressed: {pass} < floor {PASS_FLOOR}"
    );
}

/// **Milestone 1 acceptance:** the REPL path does real numeric work — matmul +
/// transpose, eigenvalues, SVD, LU, and a `\` solve give the known answers.
#[test]
fn milestone1_acceptance() {
    use fm_interp::Interpreter;
    let mut i = Interpreter::new();
    fm_builtins::register_standard_library(&mut i);

    let vec_of = |i: &Interpreter, name: &str| -> Vec<f64> {
        fm_interp::value::to_f64_vec(i.context.lookup(name).expect("var"))
    };

    // A*A'
    i.run("A = [1 2; 3 4]; m = A*A';").unwrap();
    assert_eq!(vec_of(&i, "m"), vec![5.0, 11.0, 11.0, 25.0]);

    // eig
    i.run("e = eig([2 0; 0 3]);").unwrap();
    let mut e = vec_of(&i, "e");
    e.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert!((e[0] - 2.0).abs() < 1e-9 && (e[1] - 3.0).abs() < 1e-9);

    // svd
    i.run("s = svd([3 0; 0 4]);").unwrap();
    let s = vec_of(&i, "s");
    assert!((s[0] - 4.0).abs() < 1e-9 && (s[1] - 3.0).abs() < 1e-9);

    // lu reconstructs
    i.run("[L, U] = lu([4 3; 6 3]); P = L*U;").unwrap();
    assert_eq!(vec_of(&i, "P"), vec![4.0, 6.0, 3.0, 3.0]);

    // backslash solve
    i.run("x = [2 0; 0 4] \\ [2; 8];").unwrap();
    assert_eq!(vec_of(&i, "x"), vec![1.0, 2.0]);
}

/// Non-gating full-suite reporter. Run with:
///   `cargo test -p fm-conformance -- --ignored --nocapture full_suite_report`
/// or the binary `cargo run -p fm-conformance`. Many tests fail today — that is
/// expected; this prints the tracked number, it does not fail the build.
#[test]
#[ignore = "reporter: prints the full-suite pass-rate, does not gate CI"]
fn full_suite_report() {
    fm_conformance::silence_panic_output();
    let by_dir = run_full_suite();
    let mut total = fm_conformance::Stats::default();
    println!(
        "\n{:<14} {:>6} {:>6} {:>6} {:>6}",
        "dir", "total", "pass", "fail", "error"
    );
    for (dir, results) in &by_dir {
        let s = summarize(results);
        println!(
            "{:<14} {:>6} {:>6} {:>6} {:>6}",
            dir,
            s.total(),
            s.pass,
            s.fail,
            s.error
        );
        total.pass += s.pass;
        total.fail += s.fail;
        total.error += s.error;
    }
    println!(
        "\nFull-suite pass-rate: {}/{} = {:.1}%",
        total.pass,
        total.total(),
        total.rate() * 100.0
    );
}
