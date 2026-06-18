//! `fm-conformance` — runs FreeMat's own `.m` tests against the Rust
//! interpreter and reports a pass-rate.
//!
//! # FreeMat's pass/fail convention (reproduced faithfully)
//!
//! FreeMat's CMake suite invokes the interpreter in *test mode* (`-t`), which
//! seeds the process exit code to **1 (fail)**. Each test is a function file
//! named `test_NAME.m` containing `function test_val = test_NAME` (the output
//! variable name varies, but there is exactly one output). The CMake wrapper
//! either calls `wrap_test('test_NAME')` — which runs the function inside a
//! `try`/`catch` and only flips the exit code to 0 on success — or `~test_NAME`
//! in the `suite/` directory. The test **passes** when the function runs
//! without raising an error **and returns an all-nonzero (true) value**; it
//! **fails** if it errors or returns a false/zero/empty result.
//!
//! We reproduce that here: load every `.m` file in a directory so the test
//! function and its `.m` helpers (`test`, `testeq`, ...) are all defined, call
//! the test function requesting one output, and classify the result:
//! [`Outcome::Pass`] (true value), [`Outcome::Fail`] (false/empty value or a
//! genuinely wrong answer), or [`Outcome::Error`] (the interpreter raised — a
//! missing builtin or unsupported feature). For honest reporting, an `Error`
//! is **not** a pass — pass-rate counts only `Pass`.

use std::collections::BTreeMap;
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};

use fm_interp::{Interpreter, Signal};
use fm_parser::Span;

/// The result of running one test function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The test function returned an all-nonzero (true) value.
    Pass,
    /// The test function returned a false/zero/empty value (a real failure).
    Fail,
    /// The interpreter raised an error (missing builtin / unsupported feature)
    /// or panicked. Counted as a non-pass, never inflated into a pass.
    Error,
}

/// One executed test: its name, its directory, and the outcome (plus a short
/// detail string for errors/failures, useful when triaging).
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Directory (relative to the test root), e.g. `"flow"`.
    pub dir: String,
    /// Test function name, e.g. `"test_for1"`.
    pub name: String,
    /// What happened.
    pub outcome: Outcome,
    /// A short human-readable detail (error message, or "false result").
    pub detail: String,
}

/// Aggregated pass/fail/error counts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// Number of [`Outcome::Pass`].
    pub pass: usize,
    /// Number of [`Outcome::Fail`].
    pub fail: usize,
    /// Number of [`Outcome::Error`].
    pub error: usize,
}

impl Stats {
    /// Total number of tests counted.
    #[must_use]
    pub fn total(&self) -> usize {
        self.pass + self.fail + self.error
    }

    /// Pass-rate in `[0, 1]` (0 when no tests ran).
    #[must_use]
    pub fn rate(&self) -> f64 {
        if self.total() == 0 {
            0.0
        } else {
            self.pass as f64 / self.total() as f64
        }
    }

    fn record(&mut self, outcome: Outcome) {
        match outcome {
            Outcome::Pass => self.pass += 1,
            Outcome::Fail => self.fail += 1,
            Outcome::Error => self.error += 1,
        }
    }
}

/// Install a no-op panic hook so interpreter bugs (which the harness catches
/// via [`std::panic::catch_unwind`] and records as [`Outcome::Error`]) do not
/// spam stderr during a run. Call once before running the corpus.
pub fn silence_panic_output() {
    std::panic::set_hook(Box::new(|_| {}));
}

/// The root of the version-controlled, self-contained test corpus copied from
/// `FreeMat/tests/` into this crate's `data/tests` directory.
#[must_use]
pub fn test_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("data/tests")
}

/// Run a single test file (`path`) and return its [`Outcome`] + detail.
///
/// All `.m` files in the same directory (and the shared `_helpers` directory)
/// are defined first so helper functions resolve, then the function whose name
/// matches the file stem is called with one requested output.
#[must_use]
pub fn run_test_file(path: &Path) -> (Outcome, String) {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();

    // A fresh interpreter per test (matches FreeMat's process-per-test
    // isolation and avoids cross-test state leakage), with the full Stage-5
    // standard library registered on top of the minimal defaults.
    let mut interp = Interpreter::new();
    fm_builtins::register_standard_library(&mut interp);

    // Define shared helpers first, then every sibling `.m` (so the test fn and
    // its in-directory helpers are all registered).
    let helpers = test_root().join("_helpers");
    define_dir(&mut interp, &helpers);
    define_dir(&mut interp, &dir);

    // Call the test function, requesting one output, guarding against panics
    // from interpreter bugs (treated as Error, not a crash of the whole run).
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        interp.call_function(&stem, &[], 1, "", Span::empty(0))
    }));

    match result {
        Err(_) => (Outcome::Error, "panic in interpreter".to_string()),
        Ok(Err(Signal::Error(e))) => (Outcome::Error, e.message),
        Ok(Err(other)) => (Outcome::Error, format!("stray control flow: {other:?}")),
        Ok(Ok(outputs)) => match outputs.first() {
            None => (Outcome::Fail, "no return value".to_string()),
            Some(v) => match fm_interp::value::truth(v) {
                Ok(true) => (Outcome::Pass, String::new()),
                Ok(false) => (Outcome::Fail, "false result".to_string()),
                Err(_) => (Outcome::Fail, "non-truthy result".to_string()),
            },
        },
    }
}

/// Define every `.m` file in a directory (ignoring read/parse errors so one
/// bad helper does not abort the whole directory).
fn define_dir(interp: &mut Interpreter, dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) == Some("m") {
            let _ = interp.load_file(&p);
        }
    }
}

/// All test files under a directory of the corpus, sorted by name. A "test
/// file" is any `.m` whose stem starts with `test_` (FreeMat's convention).
#[must_use]
pub fn test_files_in(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("m"))
        .filter(|p| {
            p.file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.starts_with("test_"))
        })
        .collect();
    files.sort();
    files
}

/// Run every test in a single named directory of the corpus.
#[must_use]
pub fn run_dir(name: &str) -> Vec<TestResult> {
    let dir = test_root().join(name);
    test_files_in(&dir)
        .into_iter()
        .map(|p| {
            let (outcome, detail) = run_test_file(&p);
            TestResult {
                dir: name.to_string(),
                name: p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string(),
                outcome,
                detail,
            }
        })
        .collect()
}

/// The covered (runnable) directories of the corpus, in report order.
///
/// Deferred directories (graphics/`.mat`-fixture/sparse/jit/etc.) are listed in
/// [`DEFERRED`] with the reason; they are not run here.
pub const COVERED_DIRS: &[&str] = &[
    "array",
    "binary",
    "constants",
    "elementary",
    "flow",
    "freemat",
    "functions",
    "handle",
    "inspection",
    "io",
    "operators",
    "random",
    "signal",
    "sparse",
    "string",
    "suite",
    "transforms",
    "typecast",
    "variables",
];

/// Directories deferred to later stages, with the reason.
pub const DEFERRED: &[(&str, &str)] = &[
    // `reference`/`matcompat` hold .mat *fixtures* + whitebox driver scripts but
    // **no `test_*.m`** functions, so there is nothing the harness can run there
    // (the Stage-8 fm-io reader is exercised by the fm-io unit tests + the now-
    // enabled `io` save/load tests instead).
    (
        "reference",
        "367 .mat fixtures + whitebox drivers — no test_*.m to run",
    ),
    (
        "matcompat",
        ".mat fixtures + mcompat driver — no test_*.m to run",
    ),
    ("class", "user-defined classes/OOP — later stage"),
    ("curvefit", "curve fitting (fitfun/gausfit) — Stage 9"),
    ("glwin", "OpenGL graphics — Stage 7"),
    ("jit", "JIT compiler — dropped per plan"),
    ("itk", "ITK image toolkit — dropped per plan"),
    ("parse", "negative parse tests — different convention"),
    ("debug", "debugger — Stage 10"),
    ("vtk*", "VTK 3D — dropped per plan"),
    ("thread", "threading — out of scope"),
    ("external/os/num/mathfunctions", "empty or native-only"),
];

/// Run the full covered corpus and return per-directory results.
#[must_use]
pub fn run_full_suite() -> BTreeMap<String, Vec<TestResult>> {
    COVERED_DIRS
        .iter()
        .map(|d| ((*d).to_string(), run_dir(d)))
        .collect()
}

/// Summarize results into a [`Stats`].
#[must_use]
pub fn summarize(results: &[TestResult]) -> Stats {
    let mut stats = Stats::default();
    for r in results {
        stats.record(r.outcome);
    }
    stats
}
