//! `cargo xtask ci` — run the local verification gate end to end.
//!
//! ```text
//! cargo xtask ci          # fmt + clippy + tests + docgen --check + conformance
//! cargo xtask ci --fast   # skip clippy and the conformance regression check
//! ```
//!
//! One command that runs the same checks CI does, in order, stopping at the
//! first failure so the output ends on the thing that needs fixing:
//!
//! 1. `cargo fmt --all --check`               — formatting is clean
//! 2. `cargo clippy … -D warnings`            — no lints        (skipped by `--fast`)
//! 3. `cargo test --workspace`                — the test suite passes
//! 4. `docgen --check` (in-process)           — the help fragment DB is current
//! 5. `conformance --check` (in-process)      — no directory regressed
//!    (skipped when no baseline exists yet, or under `--fast`)

use std::path::Path;
use std::process::Command;

/// Entry point for `cargo xtask ci …`.
pub fn ci(fast: bool) -> Result<(), String> {
    run_step(
        "cargo fmt --all --check",
        "cargo",
        &["fmt", "--all", "--check"],
    )?;

    if !fast {
        run_step(
            "cargo clippy --workspace --all-targets -- -D warnings",
            "cargo",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        )?;
    }

    run_step("cargo test --workspace", "cargo", &["test", "--workspace"])?;

    // docgen --check runs in-process (no subprocess): the help fragment DB is up
    // to date with what the interpreter now produces.
    step_header("cargo xtask docgen --check");
    crate::docgen(true)?;

    // Conformance regression check, only if a baseline has been recorded and we
    // aren't in --fast mode.
    if !fast {
        let baseline = crate::workspace_root()?
            .join("docs")
            .join("conformance-baseline.json");
        if baseline.exists() {
            step_header("cargo xtask conformance --check");
            crate::conformance::conformance(&[], false, false, true)?;
        } else {
            println!(
                "\n== skip: conformance --check (no baseline at {}) ==",
                baseline.display()
            );
        }
    }

    println!("\nci: all checks passed.");
    Ok(())
}

/// Print a step header so the output is easy to scan.
fn step_header(label: &str) {
    println!("\n== {label} ==");
}

/// Run one subprocess step, streaming its output. Returns `Err` if it exits
/// non-zero (or fails to launch).
fn run_step(label: &str, program: &str, args: &[&str]) -> Result<(), String> {
    step_header(label);
    let root = crate::workspace_root()?;
    let status = Command::new(program)
        .args(args)
        .current_dir(Path::new(&root))
        .status()
        .map_err(|e| format!("ci: failed to launch `{label}`: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("ci: step failed: `{label}` ({status})"))
    }
}
