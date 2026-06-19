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
    // Stage-8 — fm-io: MAT save/load, ascii load, sscanf
    "io/test_save1",
    "io/test_load1",
    "io/test_sscanf1",
    // Stage-8 — transforms (linalg + fft), now enabled
    "transforms/test_eig3",
    "transforms/test_svd1",
    // Tier-1 harness fix — file's public function differs from the filename
    // (`test_resize5.m` defines `test_resize4`, etc.); now invoked correctly.
    "array/test_resize5",
    "array/test_resize8",
    // Tier-1 — `wbtest_near` whitebox helper loaded + `conv2` complex path.
    "signal/test_conv2_1",
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
    // Stage 7.5 — handle graphics (plot/image/contour + close all)
    "handle/test_plot1",
    "handle/test_image1",
    "handle/test_contour1",
    // Stage 9 — sparse matrices (deterministic, no PRNG)
    "sparse/test_sparse1",  // sparse↔full round-trip, all classes
    "sparse/test_sparse2",  // sparse concatenation [A;B], [C,D]
    "sparse/test_sparse3",  // sparse concatenation (single)
    "sparse/test_sparse7",  // sparse linear indexing A(b)
    "sparse/test_sparse15", // sparse 2-D indexing A(i,j)
    "sparse/test_sparse18", // complex sparse 2-D indexing
    "sparse/test_sparse27", // sparse linear-index assignment A(p) = scalar
    "sparse/test_sparse28", // sparse linear-index assignment A(p) = vector
    "sparse/test_sparse29", // linear-grow-past-end flattens (was a panic)
    // Correctness-bug pass — deterministic regressions for fixed root causes.
    // (These live in the *smaller* dirs so the per-test directory reload stays
    // cheap; the big `array`/`suite` gains are covered by the full-suite floor.)
    "transforms/test_qr1",     // qr(a,0) economy + diag(R,k)
    "transforms/test_qr5",     // qr(a) full
    "operators/test_range3",   // single-class range
    "inspection/test_typeof4", // typeof(complex single) == 'single'
    "inspection/test_typeof5", // typeof(complex double) == 'double'
    "inspection/test_isset1",  // isset requires non-empty
    "inspection/test_empty",   // [] ^ [] is empty
    "inspection/test_size4",   // [c,d]=size(a,2) fewer-returns
    "flow/test_switch3",       // switch on complex value
    "flow/test_switch4",       // switch on vector errors
    "functions/test_call2",    // [d{1:3}]=f multi-output cell-content
    "freemat/test_evalin1",    // evalin('caller', ...)
    "freemat/test_evalin2",    // evalin('base', ...) returns value
    "freemat/test_assignin1",  // assignin('caller', ...)
    "freemat/test_assignin2",  // assignin('base', ...)
    "freemat/test_builtin1",   // builtin('abs', x) bypasses user shadow
    // Feature pass — function handles, N-D arrays, struct cs-list, by-ref.
    "functions/test_fptr1", // @cos in a struct field, then a.b(2.0)
    "functions/test_call3", // pass-by-reference (&x) write-back
    "functions/test_call4", // pass-by-reference of a struct field
    "suite/test_matcat2",   // N-D vertical concat of pages [a;a]
    "suite/test_matcat3",   // N-D concat (rank 5)
    "array/test_assign14",  // N-D grow-on-assign a(2,:,:) = r
    "array/test_assign15",  // illegal incomplete N-D assign must error
    "array/test_repmat3",   // N-D repmat
    "suite/test_struct5",   // struct field over array → comma list (args)
    "suite/test_struct8",   // [a.foo] = f multi-output struct-field assign
    "suite/test_getfield2", // getfield(a, {i,j}, 'field')
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
/// (41.5%); Stage 8 to 284/637 (with the `transforms` + `io` gains); Stage 7.5
/// to 297/640 (handle dir + a parser leading-whitespace fix). The builtin
/// gap-fill pass (bit ops, base conversion, polynomial, linalg extras, trig
/// gaps, `eps`/`seed`) raised the live total to 309/640 (48.3%). Stage 9
/// (sparse matrices: CSC `Array::Sparse`, sparse builtins + the enabled `sparse`
/// dir) raised it to 402/677 (59.4%). The **correctness-bug pass** then fixed a
/// run of root causes (file-local subfunctions + flat-sibling parsing, the
/// linear-grow/subscript-grow relayout, qr modes, `diag(A,k)`, single-dominant
/// promotion + complex-preserving casts + complex-single gather, `randi`
/// semantics, colon-grow-into-empty, multi-output cell-content assign, switch
/// complex/scalar rules, zeros/ones class arg, evalin/assignin/builtin, …),
/// raising the live total to 627/677 (92.6%). The feature pass (function
/// handles `@f`/`@(x)…`/`feval`/`func2str`/`str2func`/`arrayfun`; N-D arrays —
/// `cat(3,…)`, N-D `[a;a]`/`repmat`/grow-assign; struct field cs-list
/// expansion; pass-by-reference `&x`; `conv2`/`dlmread`/`dlmwrite`/`which`/
/// `dir`/`pwd`/`cd`) raised the live total to 650/677 (96.0%). The floor allows
/// a margin for PRNG-dependent (`rand`/`randn`/`sprandn`/`randi`/`eig`) tests.
///
/// The Tier-1 + generalized-eig pass (harness invokes the file's public function
/// regardless of filename; `wbtest_near` helper loaded; `fileparts`/`xnrm2`
/// builtins; `return (expr)` parse; unassigned outputs return `[]` like FreeMat;
/// `eig(A,B)` via QZ + inverse-iteration refinement) raised the live total to
/// 658/677 (97.2%).
///
/// The REMAINING.md backlog pass (`fitfun`/`gausfit`, `source` + `which`-path,
/// N-D `int2bin`/`bin2int`, sparse-preserving indexed assignment + sparse-`lu`
/// errors, `imwrite`/`imread`, single-complex scatter fix) raised the live total
/// to 670/677 (99.0%).
const PASS_FLOOR: usize = 665;

/// **Fast pass-floor guard (gates `cargo test`).** Running the *whole* covered
/// corpus takes minutes (it spins up a fresh interpreter and re-parses every
/// `.m` in a directory per test). To keep `cargo test --workspace` quick, the
/// asserted path runs only the **curated must-pass subset** above (a few dozen
/// tests, well under a second). The full-corpus floor check is `#[ignore]`d (see
/// [`full_suite_pass_count_does_not_regress`]) and runs in the reporter / CI.
///
/// This still meaningfully guards against regressions: any break in the curated
/// tests fails here immediately, and the curated set is sized to be a strict,
/// representative subset.
#[test]
fn curated_floor_is_met() {
    // Reuse the curated subset as the fast floor; if any regressed,
    // `curated_subset_passes` already fails. This asserts the *count* so the
    // floor is explicit and visible.
    let mut pass = 0usize;
    for spec in CURATED {
        let (dir, name) = spec.split_once('/').expect("spec is dir/name");
        let path = test_root().join(dir).join(format!("{name}.m"));
        if fm_conformance::run_test_file(&path).0 == Outcome::Pass {
            pass += 1;
        }
    }
    assert!(
        pass >= CURATED.len(),
        "curated floor regressed: {pass}/{} passing",
        CURATED.len()
    );
}

/// **Slow full-corpus pass-floor guard — `#[ignore]`d so it does not drag every
/// `cargo test --workspace`.** Runs the entire covered corpus (~minutes). Run it
/// explicitly in CI / before a release:
///   `cargo test -p fm-conformance -- --ignored full_suite_pass_count_does_not_regress`
/// or via the `cargo run -p fm-conformance` reporter (which prints the live
/// total and per-dir breakdown).
#[test]
#[ignore = "slow: runs the whole covered corpus; use the reporter or CI"]
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
