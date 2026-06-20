//! `cargo xtask` — FreeMat-rs developer tasks.
//!
//! The only task today is `docgen` (help-system phase P3, implementing §5 of
//! `docs/HELP_SYSTEM.md`):
//!
//! ```text
//! cargo xtask docgen          # regenerate crates/fm-doc/src/generated/fragments.rs
//! cargo xtask docgen --check  # CI gate: fail if regenerating would change it,
//!                             # or if any validation error is found
//! ```
//!
//! # Pipeline (§5)
//! 1. **Collect** the registry ([`fm_doc::Registry::global`]). `inventory`
//!    collects whatever `register_doc!`/`register_section!` sites are *linked*
//!    into this binary; depending on (and referencing) `fm-builtins` forces
//!    those sites in — see [`force_builtin_linkage`].
//! 2. **Validate** the registry: unknown `section`, dangling `[text](name)`
//!    cross-links (target not a known topic). (Duplicate names already error at
//!    registry build.)
//! 3. **Extract** each entry's [`fm_doc::FragmentScript`] and **run** it via the
//!    in-process [`fm_cli::run_fragment`] (no subprocess, §4.1). Every fragment
//!    is re-run on each invocation so `--check` is an honest gate (the generated
//!    artifact is the only store, so reusing it as a cache would let stale output
//!    self-validate — see the note in [`docgen`]).
//! 4. **Validate `# errors:`**: the captured `error_count` must equal the
//!    script's declared `expect_errors`.
//! 5. **Emit** the fragment DB as deterministic, rustfmt-clean generated Rust at
//!    `crates/fm-doc/src/generated/fragments.rs` (sorted by hash).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use fm_doc::{CapturedFragment, Fragment, Registry};

/// Force the linker to keep `fm-builtins` (and therefore its
/// `register_doc!`/`register_section!` `inventory::submit!` sites) so
/// [`fm_doc::Registry::global`] collects them.
///
/// `inventory` registration relies on each submission site's object code being
/// linked into the final binary. A crate that is only a Cargo dependency but is
/// never *referenced* can be dropped by the linker, taking its inventory sites
/// with it. Taking a function pointer to a public `fm-builtins` symbol creates a
/// hard reference that pins the whole crate (and thus every `register_doc!` in
/// it) into the `xtask` binary. (`fm_cli::run_fragment` also calls into
/// `fm-builtins`, so it would be linked regardless — this makes the guarantee
/// explicit and self-documenting.)
fn force_builtin_linkage() {
    // Cast the fn item to a usize address; this references the symbol (pinning
    // the crate's object code, hence its inventory sites) without needing to
    // name the `Interpreter` parameter type.
    let addr = fm_builtins::register_standard_library as *const () as usize;
    std::hint::black_box(addr);
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let task = args.next();
    let check = args.any(|a| a == "--check");

    match task.as_deref() {
        Some("docgen") | None => match docgen(check) {
            Ok(()) => ExitCode::SUCCESS,
            Err(report) => {
                eprintln!("{report}");
                ExitCode::FAILURE
            }
        },
        Some(other) => {
            eprintln!("unknown xtask: {other:?}\n\nusage: cargo xtask docgen [--check]");
            ExitCode::FAILURE
        }
    }
}

/// Errors accumulated during docgen; rendered together so authors see every
/// problem in one run, not one-at-a-time.
#[derive(Default)]
struct Report {
    errors: Vec<String>,
}

impl Report {
    fn push(&mut self, msg: impl Into<String>) {
        self.errors.push(msg.into());
    }
    fn into_result(self) -> Result<(), String> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            let mut s = format!("docgen: {} validation error(s):\n", self.errors.len());
            for e in &self.errors {
                s.push_str("  - ");
                s.push_str(e);
                s.push('\n');
            }
            Err(s)
        }
    }
}

/// Run the docgen pipeline. With `check == true`, do not write the file; instead
/// fail if regenerating would change it.
fn docgen(check: bool) -> Result<(), String> {
    force_builtin_linkage();

    let registry = Registry::global();
    let mut report = Report::default();

    // --- Validate registry-level invariants (§5.1) ---------------------------
    // Known section ids and known topic names/aliases for cross-link checks.
    let known_sections: std::collections::HashSet<&str> =
        registry.sections().map(|s| s.id).collect();
    let mut known_topics: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for e in registry.iter() {
        known_topics.insert(e.name);
        for a in e.aliases {
            known_topics.insert(a);
        }
    }

    for e in registry.iter() {
        if !known_sections.contains(e.section) {
            report.push(format!(
                "topic {:?}: unknown section {:?} (no register_section! with that id)",
                e.name, e.section
            ));
        }
        let parsed = fm_doc::parse_body(e.body_md);
        for link in &parsed.links {
            if !known_topics.contains(link.target.as_str()) {
                report.push(format!(
                    "topic {:?}: dangling cross-link [{}]({}) — target is not a known topic",
                    e.name, link.text, link.target
                ));
            }
        }
    }

    let out_path = generated_path()?;

    // --- Extract + capture each entry's fragment (§5.2–5.4) ------------------
    // Always re-run every fragment through the interpreter. The generated
    // artifact is the only store, so using it as a cache would let a stale or
    // hand-corrupted transcript self-validate, and would mask interpreter output
    // drift: an unchanged input hash would reuse the old transcript forever, so
    // `--check` would stay green while the committed docs no longer match what
    // the interpreter produces (and `docgen` could never converge). Re-running
    // keeps `--check` an honest gate. A separate, engine-version-keyed cache can
    // be reintroduced as an optimization once the fragment surface is large —
    // see HELP_SYSTEM.md §4.3.
    // Keyed by content hash; BTreeMap gives deterministic (sorted) output.
    let mut db: BTreeMap<String, CapturedLite> = BTreeMap::new();
    for e in registry.iter() {
        let Some(script) = fm_doc::fragment_script(e) else {
            continue;
        };
        let hash = script.content_hash();
        let cap: CapturedFragment = fm_cli::run_fragment(&script);
        // Validate `# errors:` declaration vs. actual (§5.4).
        if cap.error_count != script.expect_errors {
            report.push(format!(
                "topic {:?}: fragment raised {} error(s) but `# errors:` declares {}",
                e.name, cap.error_count, script.expect_errors
            ));
        }
        db.insert(
            hash,
            CapturedLite {
                transcript: cap.transcript,
                figure: cap.figure,
            },
        );
    }

    // Surface validation errors before deciding to write/diff.
    report.into_result()?;

    // --- Emit deterministic, rustfmt-clean generated Rust (§5.5) -------------
    // Render, then run it through rustfmt so the checked-in file is always
    // `cargo fmt --check`-clean regardless of our emitter's spacing choices.
    let generated = rustfmt(&render_db(&db))?;

    if check {
        let current = std::fs::read_to_string(&out_path).unwrap_or_default();
        if current != generated {
            return Err(format!(
                "docgen --check: {} is out of date.\n\
                 Run `cargo xtask docgen` and commit the result.\n\n{}",
                out_path.display(),
                diff_summary(&current, &generated)
            ));
        }
        println!(
            "docgen --check: OK — {} is up to date ({} fragment(s)).",
            out_path.display(),
            db.len()
        );
    } else {
        std::fs::write(&out_path, &generated)
            .map_err(|e| format!("writing {}: {e}", out_path.display()))?;
        println!(
            "docgen: wrote {} ({} fragment(s)).",
            out_path.display(),
            db.len()
        );
    }
    Ok(())
}

/// The render-relevant captured data, mirroring the generated [`Fragment`] but
/// owned (so it can be cached/compared as an in-memory value).
#[derive(Clone, PartialEq, Eq)]
struct CapturedLite {
    transcript: String,
    figure: Option<String>,
}

/// Absolute path to the checked-in generated fragment DB.
fn generated_path() -> Result<PathBuf, String> {
    Ok(workspace_root()?
        .join("crates")
        .join("fm-doc")
        .join("src")
        .join("generated")
        .join("fragments.rs"))
}

/// Workspace root: `CARGO_MANIFEST_DIR` of `xtask` is `<root>/crates/xtask`.
fn workspace_root() -> Result<PathBuf, String> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("cannot derive workspace root from {}", manifest.display()))
}

/// Render the fragment DB to the exact generated-file text (deterministic,
/// rustfmt-clean). Entries are emitted in `BTreeMap` (hash-sorted) order.
fn render_db(db: &BTreeMap<String, CapturedLite>) -> String {
    let mut s = String::new();
    s.push_str(GENERATED_HEADER);
    if db.is_empty() {
        s.push_str("\npub static FRAGMENTS: &[(&str, Fragment)] = &[];\n");
        return s;
    }
    s.push_str("\npub static FRAGMENTS: &[(&str, Fragment)] = &[\n");
    for (hash, cap) in db {
        s.push_str("    (\n");
        s.push_str(&format!("        {},\n", rust_str(hash)));
        s.push_str("        Fragment {\n");
        s.push_str(&format!(
            "            transcript: {},\n",
            rust_str(&cap.transcript)
        ));
        match &cap.figure {
            None => s.push_str("            figure: None,\n"),
            Some(fig) => {
                s.push_str(&format!("            figure: Some({}),\n", rust_str(fig)));
            }
        }
        s.push_str("        },\n");
        s.push_str("    ),\n");
    }
    s.push_str("];\n");
    s
}

/// Pipe `src` through `rustfmt` (stdin -> stdout) so the generated file matches
/// exactly what `cargo fmt` would produce. Falls back to the unformatted text if
/// `rustfmt` is unavailable, so docgen still works in a minimal environment
/// (CI's `cargo fmt --check` would then catch any divergence).
fn rustfmt(src: &str) -> Result<String, String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = match Command::new("rustfmt")
        .arg("--edition")
        .arg("2024")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return Ok(src.to_string()),
    };
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(src.as_bytes())
            .map_err(|e| format!("writing to rustfmt: {e}"))?;
    }
    let out = child
        .wait_with_output()
        .map_err(|e| format!("running rustfmt: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "rustfmt failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    String::from_utf8(out.stdout).map_err(|e| format!("rustfmt produced non-UTF8: {e}"))
}

/// The header comment block at the top of the generated file. Kept identical to
/// the committed initial file so the empty-DB output is byte-stable.
const GENERATED_HEADER: &str = "// GENERATED FILE — do not edit by hand.\n\
//\n\
// Produced by `cargo xtask docgen` (help-system phase P3). Holds the captured\n\
// `fm-exec` fragment transcripts keyed by their script content hash\n\
// (`FragmentScript::content_hash`). Regenerate with `cargo xtask docgen`; CI\n\
// verifies it is up to date with `cargo xtask docgen --check`.\n\
//\n\
// Included by `crate::fragment` via `include!`. Entries are sorted by hash so\n\
// the file has a stable, reviewable git diff and supports binary-search lookup.\n";

/// Emit a Rust double-quoted string literal for `s`, escaping the characters
/// rustfmt would also escape so the output is fmt-clean as written.
fn rust_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\0' => out.push_str("\\0"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A compact human-readable diff summary for `--check` failures.
fn diff_summary(current: &str, generated: &str) -> String {
    let cur: Vec<&str> = current.lines().collect();
    let new: Vec<&str> = generated.lines().collect();
    let mut s = String::new();
    let max = cur.len().max(new.len());
    let mut shown = 0;
    for i in 0..max {
        let a = cur.get(i).copied().unwrap_or("");
        let b = new.get(i).copied().unwrap_or("");
        if a != b {
            if shown < 20 {
                s.push_str(&format!("  L{}: - {a}\n", i + 1));
                s.push_str(&format!("  L{}: + {b}\n", i + 1));
            }
            shown += 1;
        }
    }
    if shown == 0 {
        s.push_str("  (files differ but no line-level diff — check trailing bytes)\n");
    } else if shown > 20 {
        s.push_str(&format!("  … and {} more differing line(s)\n", shown - 20));
    }
    s
}

/// Touch the generated `Fragment` type so a refactor of its shape forces a
/// compile error here (the emitter must stay in sync with the struct).
#[allow(dead_code)]
fn _shape_check(f: Fragment) -> (&'static str, Option<&'static str>) {
    (f.transcript, f.figure)
}
