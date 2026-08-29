//! `cargo xtask conformance` — run the FreeMat `.m` conformance corpus and,
//! optionally, gate the result against a saved baseline so regressions fail CI.
//!
//! ```text
//! cargo xtask conformance                     # full covered corpus, pass table
//! cargo xtask conformance flow string         # only these directories
//! cargo xtask conformance --failures          # also list each failing/erroring test
//! cargo xtask conformance --save-baseline      # record the current pass counts
//! cargo xtask conformance --check              # fail if any directory regressed
//! ```
//!
//! The pass-rate is a tracked metric that should climb as the port matures; it
//! is not a hard gate on its own. The value this task adds over the
//! `fm-conformance` binary is the **baseline**: `--save-baseline` snapshots the
//! per-directory pass counts to `docs/conformance-baseline.json`, and `--check`
//! re-runs the corpus and fails if any directory's pass count dropped — a cheap
//! regression guard that can run in CI without pinning the (still-climbing)
//! absolute number.

use std::collections::BTreeMap;
use std::path::PathBuf;

use fm_conformance::{COVERED_DIRS, Outcome, Stats, run_dir, summarize};
use serde::{Deserialize, Serialize};

/// Per-directory pass/fail/error counts, keyed by directory name. Serialized as
/// the baseline artifact (stable, diff-friendly BTreeMap ordering).
#[derive(Debug, Default, Serialize, Deserialize)]
struct Baseline {
    dirs: BTreeMap<String, DirStat>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
struct DirStat {
    pass: usize,
    fail: usize,
    error: usize,
}

impl From<&Stats> for DirStat {
    fn from(s: &Stats) -> Self {
        DirStat {
            pass: s.pass,
            fail: s.fail,
            error: s.error,
        }
    }
}

/// Default location of the checked-in baseline snapshot.
fn baseline_path() -> Result<PathBuf, String> {
    Ok(crate::workspace_root()?
        .join("docs")
        .join("conformance-baseline.json"))
}

/// Entry point for `cargo xtask conformance …`.
pub fn conformance(
    dirs: &[String],
    show_failures: bool,
    save: bool,
    check: bool,
) -> Result<(), String> {
    // An empty `dirs` list means "the whole covered corpus".
    let dirs: Vec<&str> = if dirs.is_empty() {
        COVERED_DIRS.to_vec()
    } else {
        dirs.iter().map(String::as_str).collect()
    };

    // Interpreter bugs surface as caught panics (recorded as errors); keep the
    // report readable by suppressing the default panic message to stderr.
    fm_conformance::silence_panic_output();

    println!("FreeMat-rs conformance");
    println!(
        "{:<14} {:>6} {:>6} {:>6} {:>6} {:>8}",
        "directory", "total", "pass", "fail", "error", "rate"
    );
    println!("{}", "-".repeat(50));

    let mut overall = Stats::default();
    let mut current: BTreeMap<String, DirStat> = BTreeMap::new();
    let mut failure_lines: Vec<String> = Vec::new();

    for d in &dirs {
        let results = run_dir(d);
        if results.is_empty() {
            continue;
        }
        let s = summarize(&results);
        println!(
            "{:<14} {:>6} {:>6} {:>6} {:>6} {:>7.1}%",
            d,
            s.total(),
            s.pass,
            s.fail,
            s.error,
            s.rate() * 100.0
        );
        overall.pass += s.pass;
        overall.fail += s.fail;
        overall.error += s.error;
        current.insert((*d).to_string(), DirStat::from(&s));

        if show_failures {
            for r in &results {
                if r.outcome != Outcome::Pass {
                    let tag = if r.outcome == Outcome::Error {
                        "ERROR"
                    } else {
                        "FAIL "
                    };
                    failure_lines.push(format!("  {tag} {}/{}: {}", r.dir, r.name, r.detail));
                }
            }
        }
    }

    println!("{}", "-".repeat(50));
    println!(
        "{:<14} {:>6} {:>6} {:>6} {:>6} {:>7.1}%",
        "TOTAL",
        overall.total(),
        overall.pass,
        overall.fail,
        overall.error,
        overall.rate() * 100.0
    );

    if show_failures && !failure_lines.is_empty() {
        println!("\nFailures / errors:");
        for line in &failure_lines {
            println!("{line}");
        }
    }

    let path = baseline_path()?;
    if save {
        let baseline = Baseline { dirs: current };
        let json = serde_json::to_string_pretty(&baseline)
            .map_err(|e| format!("serializing baseline: {e}"))?;
        std::fs::write(&path, json).map_err(|e| format!("writing {}: {e}", path.display()))?;
        println!("\nconformance: saved baseline to {}", path.display());
        return Ok(());
    }

    if check {
        return check_regression(&current, &path);
    }

    Ok(())
}

/// Compare `current` per-directory pass counts against the saved baseline and
/// return `Err` if any directory regressed (fewer passes than before).
fn check_regression(current: &BTreeMap<String, DirStat>, path: &PathBuf) -> Result<(), String> {
    let raw = std::fs::read_to_string(path).map_err(|e| {
        format!(
            "conformance --check: cannot read baseline {} ({e}).\n\
             Create one first with `cargo xtask conformance --save-baseline`.",
            path.display()
        )
    })?;
    let baseline: Baseline = serde_json::from_str(&raw)
        .map_err(|e| format!("parsing baseline {}: {e}", path.display()))?;

    let mut regressions: Vec<String> = Vec::new();
    for (dir, base) in &baseline.dirs {
        // A directory that isn't in this run (e.g. a filtered run) is skipped.
        let Some(now) = current.get(dir) else {
            continue;
        };
        if now.pass < base.pass {
            regressions.push(format!(
                "  {dir}: {} → {} passing ({} test(s) regressed)",
                base.pass,
                now.pass,
                base.pass - now.pass
            ));
        }
    }

    if regressions.is_empty() {
        println!("\nconformance --check: OK — no directory regressed against the baseline.");
        Ok(())
    } else {
        let mut msg = format!(
            "conformance --check: {} directory/directories regressed:\n",
            regressions.len()
        );
        for r in &regressions {
            msg.push_str(r);
            msg.push('\n');
        }
        Err(msg)
    }
}
