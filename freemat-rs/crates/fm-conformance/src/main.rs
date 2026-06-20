//! `fm-conformance` binary — runs the full covered FreeMat conformance corpus
//! against the Rust interpreter and prints a pass-rate with a per-directory
//! breakdown. This does **not** gate CI; it is a tracked metric that should
//! climb as later stages land. (The curated, must-pass subset lives behind
//! `cargo test -p fm-conformance`.)

use fm_conformance::{COVERED_DIRS, DEFERRED, Outcome, Stats, run_dir, summarize};

fn main() {
    // Interpreter bugs surface as caught panics (recorded as errors); keep the
    // report readable by suppressing the default panic message to stderr.
    fm_conformance::silence_panic_output();

    // Optional first arg: only run a single directory (e.g. `flow`); with
    // `--failures`, also list the failing/erroring tests for triage.
    let args: Vec<String> = std::env::args().skip(1).collect();
    let show_failures = args.iter().any(|a| a == "--failures");
    let show_passes = args.iter().any(|a| a == "--passes");
    if args.iter().any(|a| a == "--timing") {
        run_timing(&args);
        return;
    }
    let only: Vec<&str> = args
        .iter()
        .filter(|a| !a.starts_with("--"))
        .map(String::as_str)
        .collect();
    let dirs: Vec<&str> = if only.is_empty() {
        COVERED_DIRS.to_vec()
    } else {
        only
    };

    println!("FreeMat-rs conformance report");
    println!("=============================\n");
    println!(
        "{:<14} {:>6} {:>6} {:>6} {:>6} {:>8}",
        "directory", "total", "pass", "fail", "error", "rate"
    );
    println!("{}", "-".repeat(50));

    let mut overall = Stats::default();
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

        if show_passes {
            for r in &results {
                if r.outcome == Outcome::Pass {
                    println!("  PASS  {}/{}", r.dir, r.name);
                }
            }
        }

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

    println!(
        "\nFull-suite pass-rate: {}/{} = {:.1}%",
        overall.pass,
        overall.total(),
        overall.rate() * 100.0
    );

    println!("\nDeferred directories (not run here):");
    for (dir, why) in DEFERRED {
        println!("  {dir:<32} {why}");
    }

    if show_failures && !failure_lines.is_empty() {
        println!("\nFailures / errors:");
        for line in failure_lines {
            println!("{line}");
        }
    }
}

/// `--timing` mode: measure per-test execution time and emit machine-readable
/// CSV on stdout (`dir,name,outcome,reps,ns_per_iter`), one row per test, so an
/// external script can join it against the original-FreeMat timings.
///
/// Usage: `fm-conformance --timing [--budget-ms N] [--max-reps N] [DIR...]`.
fn run_timing(args: &[String]) {
    let parse_opt = |flag: &str, default: u64| -> u64 {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    let budget_ms = parse_opt("--budget-ms", 100);
    let max_reps = parse_opt("--max-reps", 200_000);
    let budget = std::time::Duration::from_millis(budget_ms);

    let skip_next: std::collections::HashSet<usize> = args
        .iter()
        .enumerate()
        .filter(|(_, a)| *a == "--budget-ms" || *a == "--max-reps")
        .map(|(i, _)| i + 1)
        .collect();
    let only: Vec<&str> = args
        .iter()
        .enumerate()
        .filter(|(i, a)| !a.starts_with("--") && !skip_next.contains(i))
        .map(|(_, a)| a.as_str())
        .collect();
    let dirs: Vec<&str> = if only.is_empty() {
        COVERED_DIRS.to_vec()
    } else {
        only
    };

    eprintln!(
        "# fm-conformance timing: budget={budget_ms}ms max_reps={max_reps} dirs={}",
        dirs.len()
    );
    println!("dir,name,outcome,reps,ns_per_iter");
    for d in &dirs {
        for t in fm_conformance::time_dir(d, budget, max_reps) {
            let outcome = match t.outcome {
                Outcome::Pass => "pass",
                Outcome::Fail => "fail",
                Outcome::Error => "error",
            };
            println!(
                "{},{},{},{},{:.1}",
                t.dir,
                t.name,
                outcome,
                t.reps,
                t.ns_per_iter()
            );
        }
        eprintln!("# done {d}");
    }
}
