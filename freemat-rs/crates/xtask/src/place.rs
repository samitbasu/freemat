//! `cargo xtask migrate-place` — place the converted legacy docs into
//! `fm-builtins` as compiled-in `register_doc!`/`register_section!` (help-system
//! phase P7).
//!
//! ```text
//! cargo xtask migrate-place                 # convert + place the whole corpus
//! cargo xtask migrate-place --staging <dir> # (no-op placeholder; conversion is
//!                                           #  always in-memory, see below)
//! ```
//!
//! Unlike P6's `migrate-docs` (which writes plain-text staging for review), this
//! task **converts the legacy corpus in memory** (the exact same path as
//! `migrate-docs`, via [`crate::migrate::convert_corpus`]) and emits the result
//! as real Rust source under `crates/fm-builtins/src/docs/`, one module per
//! section plus a `mod.rs`. The emitted modules are `#[path]`-free `register_*!`
//! sites collected by `inventory` at runtime.
//!
//! To keep `cargo xtask docgen --check` an HONEST green gate, placement
//! *neutralizes* problems at emit time (recording each change) rather than
//! weakening any validation:
//!
//! 1. **Global name dedup** — topic `name` must be unique across the registry
//!    (it panics on duplicates). On a collision we keep the first by
//!    `(section, name)` order and drop the rest.
//! 2. **Fragment probing** — every doc whose body has `fm-exec`/`fm-exec:figure`
//!    blocks is run through [`fm_cli::run_fragment`]. If it runs as declared
//!    (`error_count == expect_errors`) the blocks stay executable; otherwise (or
//!    on panic) every executable block in that doc is downgraded to a
//!    display-only ```` ```fm ```` block so docgen won't run it.
//! 3. **Cross-link resolution** — after dedup, any `[text](target)` whose
//!    `target` is not a placed topic is rewritten to plain `text`.
//! 4. **Compile-safety** — if an emitted body fails to round-trip through the
//!    doc parser / raw-string emitter, the doc is skipped and recorded.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::Command;

use fm_doc::{FragmentScript, fragment_script_from_body, parse_body};

use crate::migrate::{ConvertedDoc, ConvertedSection, convert_corpus, legacy_root};
use crate::{rust_str, workspace_root};

/// Run the `migrate-place` task. `_staging` is accepted for CLI symmetry with
/// `migrate-docs --out`, but conversion is always done in memory.
pub fn migrate_place(_staging: Option<&str>) -> Result<(), String> {
    let root = workspace_root()?;
    let legacy = legacy_root()?;
    let corpus = convert_corpus(&legacy)?;

    let mut report = PlaceReport::default();

    // Locate (building if needed) the `fm` binary used to probe fragments in an
    // isolated subprocess. A fragment can crash the interpreter HARD (panic, or
    // an abort from an out-of-bounds allocation that `catch_unwind` cannot
    // catch); probing in-process would take this whole tool down. Running each
    // fragment in a child means a crash is just a non-zero exit we record and
    // downgrade — and it proves the fragment is unsafe for docgen's in-process
    // capture too.
    let fm_bin = build_fm_binary(&root)?;

    // --- 1. Global name dedup (keep first by (section, name)) ----------------
    // `corpus.docs` is already sorted by (section, name). Any topic name seen a
    // second time is dropped, handled generically here. (`cos` previously had a
    // P3 smoke fixture in trig.rs; that fixture is removed separately, so the
    // migrated `cos` is the only one and wins.)
    let mut seen_names: BTreeSet<String> = BTreeSet::new();
    let mut docs: Vec<ConvertedDoc> = Vec::new();
    for d in corpus.docs {
        if seen_names.contains(&d.name) {
            report
                .deduped
                .push(format!("{} (section {})", d.name, d.section));
            continue;
        }
        seen_names.insert(d.name.clone());
        docs.push(d);
    }

    // --- 2. Fragment probing -------------------------------------------------
    // For every doc that declares executable blocks, run it and decide whether
    // to keep the blocks live or downgrade them to display-only `fm` blocks.
    for d in &mut docs {
        if !d.has_fragment {
            continue;
        }
        let Some(script) = fragment_script_from_body(&d.body_md) else {
            // Declared a fragment but the parser found none — downgrade to be
            // safe and record.
            d.body_md = downgrade_exec_blocks(&d.body_md);
            report.downgraded.push(format!(
                "{} (no parseable fragment script despite fm-exec marker)",
                d.name
            ));
            continue;
        };
        // Reject non-deterministic fragments up front. Even if the RNG is seeded
        // (capture runs `seed(1,0)`), wall-clock/timing builtins produce output
        // that changes every run, which would make `docgen --check` perpetually
        // dirty (HELP_SYSTEM.md §4.3: docs avoid tic/toc/clock/now). Downgrade so
        // the example still renders as display-only source.
        if let Some(call) = nondeterministic_call(&script) {
            d.body_md = downgrade_exec_blocks(&d.body_md);
            report
                .downgraded
                .push(format!("{} (non-deterministic: uses {call})", d.name));
            continue;
        }
        match probe_fragment(&fm_bin, &script) {
            ProbeResult::Ok { error_count } if error_count == script.expect_errors => {
                report.fragments_live += 1;
            }
            ProbeResult::Ok { error_count } => {
                d.body_md = downgrade_exec_blocks(&d.body_md);
                report.downgraded.push(format!(
                    "{} (raised {} error(s), declared {})",
                    d.name, error_count, script.expect_errors
                ));
            }
            ProbeResult::Crashed(why) => {
                d.body_md = downgrade_exec_blocks(&d.body_md);
                report.downgraded.push(format!("{} ({why})", d.name));
            }
        }
    }

    // --- 3. Cross-link resolution (after dedup) ------------------------------
    // The placed topic set = names + aliases of the surviving docs.
    let mut placed: BTreeSet<String> = BTreeSet::new();
    for d in &docs {
        placed.insert(d.name.clone());
        for a in &d.aliases {
            placed.insert(a.clone());
        }
    }
    for d in &mut docs {
        let parsed = parse_body(&d.body_md);
        let mut targets: BTreeSet<String> = BTreeSet::new();
        for link in &parsed.links {
            if !placed.contains(&link.target) {
                targets.insert(link.target.clone());
            }
        }
        if !targets.is_empty() {
            let before = d.body_md.clone();
            d.body_md = strip_dangling_links(&before, &placed);
            for t in &targets {
                report
                    .crosslinks_stripped
                    .push(format!("{} -> [{}]", d.name, t));
            }
        }
    }

    // --- 4. Compile-safety probe + group by section --------------------------
    // Drop any doc whose emitted raw-string would be malformed. In practice the
    // converter's `raw_str` escalation handles this, but we re-probe to honor
    // the "skip + record" contract.
    let mut by_section: BTreeMap<String, Vec<ConvertedDoc>> = BTreeMap::new();
    for d in docs {
        if let Err(why) = compile_safe(&d) {
            report.skipped.push(format!("{} ({why})", d.name));
            continue;
        }
        by_section.entry(d.section.clone()).or_default().push(d);
    }

    // Only keep sections that actually have at least one placed doc, plus ensure
    // every doc's section is among the emitted sections (it always is, since the
    // section list comes from the same corpus).
    let mut sections: Vec<ConvertedSection> = corpus.sections;
    sections.retain(|s| by_section.contains_key(&s.id));
    // A doc could reference a section id that has no ConvertedSection (shouldn't
    // happen, but be defensive): synthesize a minimal one.
    let have: BTreeSet<String> = sections.iter().map(|s| s.id.clone()).collect();
    for sec_id in by_section.keys() {
        if !have.contains(sec_id) {
            sections.push(ConvertedSection {
                id: sec_id.clone(),
                title: sec_id.clone(),
            });
        }
    }
    sections.sort_by(|a, b| a.id.cmp(&b.id));

    // --- 5. Emit ------------------------------------------------------------
    let docs_dir = root
        .join("crates")
        .join("fm-builtins")
        .join("src")
        .join("docs");
    // Clear any previously-emitted modules so removals are reflected.
    if docs_dir.is_dir() {
        for e in std::fs::read_dir(&docs_dir)
            .map_err(|e| e.to_string())?
            .flatten()
        {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "rs") {
                std::fs::remove_file(&p).map_err(|e| format!("removing {}: {e}", p.display()))?;
            }
        }
    }
    std::fs::create_dir_all(&docs_dir).map_err(|e| format!("creating docs dir: {e}"))?;

    let mut mod_entries: Vec<(String, String)> = Vec::new(); // (module ident, section id)
    for sec in &sections {
        let module = ident_for_section(&sec.id);
        let empty = Vec::new();
        let sec_docs = by_section.get(&sec.id).unwrap_or(&empty);
        let summary = section_summary(&sec.id, &sec.title);
        let rendered = render_section_module(&sec.id, &sec.title, &summary, sec_docs);
        // Pipe through rustfmt so the checked-in module is `cargo fmt --check`
        // clean regardless of our emitter's spacing (falls back to the raw text
        // if rustfmt is unavailable). If rustfmt rejects the file (malformed
        // macro / raw string), skip the whole section's docs rather than emit
        // something that won't compile — recorded below.
        let rendered = match crate::rustfmt(&rendered) {
            Ok(f) => f,
            Err(e) => {
                for d in sec_docs {
                    report.skipped.push(format!(
                        "{} (section {} failed rustfmt: {e})",
                        d.name, sec.id
                    ));
                }
                // Re-render an empty section so the module still compiles.
                render_section_module(&sec.id, &sec.title, &summary, &[])
            }
        };
        let path = docs_dir.join(format!("{module}.rs"));
        std::fs::write(&path, rendered).map_err(|e| format!("writing {}: {e}", path.display()))?;
        report.docs_placed += sec_docs.len();
        report.sections_emitted += 1;
        mod_entries.push((module, sec.id.clone()));
    }
    mod_entries.sort();

    // mod.rs
    let mut modrs = String::new();
    modrs.push_str(
        "//! Migrated help corpus (help-system phase P7), placed by\n\
         //! `cargo xtask migrate-place`. One module per section; each holds a\n\
         //! `fm_doc::register_section!` plus that section's `fm_doc::register_doc!`\n\
         //! blocks. GENERATED — re-run `cargo xtask migrate-place` to refresh.\n\n",
    );
    for (module, sec_id) in &mod_entries {
        modrs.push_str(&format!("pub(crate) mod {module}; // section {sec_id}\n"));
    }
    std::fs::write(docs_dir.join("mod.rs"), modrs).map_err(|e| format!("writing mod.rs: {e}"))?;

    // --- 6. Summary ---------------------------------------------------------
    report.print(&sections);
    Ok(())
}

/// Outcome of probing one fragment in the isolated `fm` subprocess.
enum ProbeResult {
    /// The subprocess ran to completion and reported this many errors.
    Ok { error_count: usize },
    /// The subprocess crashed (non-zero/abnormal exit) or its output was
    /// unparseable; `why` describes it.
    Crashed(String),
}

/// Build the `fm` binary (debug) so fragments can be probed in a child process,
/// returning its path. Building once up front avoids a cargo invocation per
/// fragment (cargo would no-op after the first, but the spawn cost adds up).
fn build_fm_binary(root: &std::path::Path) -> Result<PathBuf, String> {
    let status = Command::new(env!("CARGO"))
        .current_dir(root)
        .args(["build", "-p", "fm-cli", "--bin", "fm"])
        .status()
        .map_err(|e| format!("spawning cargo build for fm: {e}"))?;
    if !status.success() {
        return Err("cargo build -p fm-cli --bin fm failed".into());
    }
    let bin = root.join("target").join("debug").join("fm");
    if !bin.exists() {
        return Err(format!("fm binary not found at {}", bin.display()));
    }
    Ok(bin)
}

/// Run one fragment script through `fm --capture-fragment` in a child process,
/// returning whether it ran cleanly (and its error count) or crashed.
fn probe_fragment(fm_bin: &std::path::Path, script: &FragmentScript) -> ProbeResult {
    let json = match serde_json::to_string(script) {
        Ok(j) => j,
        Err(e) => return ProbeResult::Crashed(format!("serialize script: {e}")),
    };
    let mut child = match Command::new(fm_bin)
        .args(["--capture-fragment", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return ProbeResult::Crashed(format!("spawn fm: {e}")),
    };
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(json.as_bytes());
    }
    let out = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => return ProbeResult::Crashed(format!("waiting on fm: {e}")),
    };
    if !out.status.success() {
        return ProbeResult::Crashed(format!("fm exited abnormally ({})", out.status));
    }
    // The captured JSON has an `error_count` field.
    #[derive(serde::Deserialize)]
    struct Captured {
        error_count: usize,
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    match serde_json::from_str::<Captured>(stdout.trim()) {
        Ok(c) => ProbeResult::Ok {
            error_count: c.error_count,
        },
        Err(e) => ProbeResult::Crashed(format!("unparseable capture output: {e}")),
    }
}

/// Accumulated placement actions, for the closing summary.
#[derive(Default)]
struct PlaceReport {
    docs_placed: usize,
    sections_emitted: usize,
    fragments_live: usize,
    deduped: Vec<String>,
    downgraded: Vec<String>,
    crosslinks_stripped: Vec<String>,
    skipped: Vec<String>,
}

impl PlaceReport {
    fn print(&self, sections: &[ConvertedSection]) {
        println!("migrate-place summary:");
        println!("  docs placed:          {}", self.docs_placed);
        println!("  sections emitted:     {}", self.sections_emitted);
        println!("  fragments kept live:  {}", self.fragments_live);
        println!("  fragments downgraded: {}", self.downgraded.len());
        println!("  names deduped:        {}", self.deduped.len());
        println!("  cross-links stripped: {}", self.crosslinks_stripped.len());
        println!("  docs skipped:         {}", self.skipped.len());
        println!("  sections:");
        for s in sections {
            println!("    {} ({})", s.id, s.title);
        }
        if !self.deduped.is_empty() {
            println!("  deduped names:");
            for d in &self.deduped {
                println!("    - {d}");
            }
        }
        if !self.downgraded.is_empty() {
            println!("  downgraded fragments:");
            for d in &self.downgraded {
                println!("    - {d}");
            }
        }
        if !self.crosslinks_stripped.is_empty() {
            println!("  stripped cross-links:");
            for c in &self.crosslinks_stripped {
                println!("    - {c}");
            }
        }
        if !self.skipped.is_empty() {
            println!("  skipped docs:");
            for s in &self.skipped {
                println!("    - {s}");
            }
        }
    }
}

/// Map a legacy section id to a valid Rust module identifier. Non-ident
/// characters (e.g. hyphens) become `_`; a leading digit is prefixed with `s_`.
/// The original `id` string is preserved in `register_section!`, so this only
/// affects the module/file name.
fn ident_for_section(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    for (i, ch) in id.chars().enumerate() {
        let ok = ch.is_ascii_alphanumeric() || ch == '_';
        let ch = if ok { ch } else { '_' };
        if i == 0 && ch.is_ascii_digit() {
            out.push_str("s_");
        }
        out.push(ch.to_ascii_lowercase());
    }
    if out.is_empty() {
        out.push_str("sec");
    }
    out
}

/// A short, accurate one-line summary per section id (the converter only set
/// `summary` to the title, which is thin). Falls back to the title with a
/// trailing period for sections not in the table.
fn section_summary(id: &str, title: &str) -> String {
    let s = match id {
        "array" => "Generate, reshape, and manipulate numeric arrays.",
        "binary" => "Bitwise and binary operations on integer values.",
        "class" => "Object-oriented programming: classes, methods, and dispatch.",
        "constants" => "Built-in numeric and symbolic constants.",
        "curvefit" => "Optimization, root-finding, and curve-fitting routines.",
        "debug" => "Debugging, breakpoints, and execution inspection.",
        "elementary" => "Elementary mathematical functions on arrays.",
        "external" => "Interfacing FreeMat with external code and libraries.",
        "flow" => "Control flow: conditionals, loops, and branching.",
        "freemat" => "Core FreeMat environment and utility functions.",
        "function" => "Defining, querying, and manipulating functions.",
        "functions" => "Writing functions and scripts in the FreeMat language.",
        "glwin" => "OpenGL-based 3-D model display.",
        "handle" => "Handle-based graphics objects and properties.",
        "inspection" => "Inspect variables, types, sizes, and the workspace.",
        "introduction" => "Getting started with the FreeMat environment.",
        "io" => "Reading and writing files and formatted input/output.",
        "mathfunctions" => "Elementary and special mathematical functions.",
        "num" => "Numerical methods: integration, interpolation, and solvers.",
        "operators" => "Arithmetic, comparison, and matrix operators.",
        "os" => "Operating-system and environment interaction.",
        "random" => "Random number generation and seeding.",
        "signal" => "Signal-processing functions and filters.",
        "sparse" => "Sparse matrix construction and operations.",
        "string" => "String creation, formatting, and manipulation.",
        "thread" => "Multithreading and concurrent execution.",
        "transforms" => "Matrix transforms and decompositions.",
        "typecast" => "Convert between numeric and data types.",
        "variables" => "Create, assign, and manage variables and arrays.",
        _ => "",
    };
    if s.is_empty() {
        let t = title.trim().trim_end_matches('.');
        format!("{t}.")
    } else {
        s.to_string()
    }
}

/// Probe whether a doc will emit and compile: round-trip the body through the
/// raw-string emitter and re-check that a `register_doc!` name/section/summary
/// are non-empty. Returns `Err(reason)` if it should be skipped.
fn compile_safe(d: &ConvertedDoc) -> Result<(), String> {
    if d.name.trim().is_empty() {
        return Err("empty name".into());
    }
    if d.section.trim().is_empty() {
        return Err("empty section".into());
    }
    // The body must round-trip a raw-string emit. `raw_str` escalates `#`
    // counts, but bail (and record) if it had to fall back to an escaped string
    // (signaled by the literal not starting with `r`), since that path is rarer
    // and we'd rather skip than emit something surprising.
    let lit = raw_str(&d.body_md);
    if !lit.starts_with('r') {
        return Err("body needs escaped (non-raw) string literal".into());
    }
    Ok(())
}

/// Downgrade every executable fence in `body` to a display-only `fm` fence,
/// stripping the optional `# errors: N` first line of each. Leaves `text`,
/// `fm`, `fm-file:*` and prose untouched. Line-oriented; matches the converter's
/// emitted fences (` ```fm-exec ` / ` ```fm-exec:figure ` on their own line).
fn downgrade_exec_blocks(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut lines = body.lines().peekable();
    while let Some(line) = lines.next() {
        let t = line.trim_start();
        if t == "```fm-exec" || t == "```fm-exec:figure" {
            // Preserve any leading indentation of the fence.
            let indent = &line[..line.len() - t.len()];
            out.push_str(indent);
            out.push_str("```fm\n");
            // Drop a single immediately-following `# errors: N` line.
            if lines
                .peek()
                .is_some_and(|next| next.trim_start().starts_with("# errors:"))
            {
                lines.next();
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Builtins whose output depends on the environment (wall clock / timing, or the
/// working directory) and therefore make a captured transcript non-reproducible
/// across runs and machines (so `docgen --check` would never stay green). The RNG
/// is seeded for determinism, so `rand`/`randn` are *not* here. The directory
/// builtins matter because capture runs each fragment in a fresh temp dir (so a
/// printed `pwd` would vary every run) — and even without that, the path would be
/// machine-specific.
const NONDETERMINISTIC_BUILTINS: &[&str] = &[
    // Wall-clock / timing.
    "tic", "toc", "clock", "now", "time", "date", "cputime", "etime", "datestr", "datenum",
    // Working-directory / filesystem-listing (path or listing is environment-specific).
    "pwd", "cd", "dir", "ls", "tempdir", "tempname", "getpath", "what",
];

/// If any `fm-exec` input in `script` calls a non-deterministic builtin (as a
/// whole-word token), return the first such name. Whole-word so `tictac` or a
/// variable named `date` in a string doesn't trip it (a crude but effective
/// guard for this corpus).
fn nondeterministic_call(script: &FragmentScript) -> Option<&'static str> {
    for input in &script.inputs {
        let masked = mask_strings_and_comments(input);
        for &name in NONDETERMINISTIC_BUILTINS {
            if contains_word(&masked, name) {
                return Some(name);
            }
        }
    }
    None
}

/// Replace the contents of `'…'` string literals and `%`-to-EOL comments with
/// spaces (length-preserving) so a later word scan ignores names that only
/// appear inside a string (e.g. `strrep(…,'time',…)`) or a comment (`% now …`).
/// A doubled `''` is FreeMat's escaped quote inside a string.
fn mask_strings_and_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out: Vec<u8> = bytes.to_vec();
    let mut i = 0;
    let mut in_str = false;
    while i < bytes.len() {
        let c = bytes[i];
        if in_str {
            if c == b'\'' {
                // Escaped quote `''` stays inside the string.
                if i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                    out[i] = b' ';
                    out[i + 1] = b' ';
                    i += 2;
                    continue;
                }
                in_str = false; // closing quote
                i += 1;
                continue;
            }
            out[i] = b' ';
            i += 1;
        } else {
            match c {
                b'\'' => {
                    in_str = true;
                    i += 1;
                }
                b'%' => {
                    // Comment to end of line.
                    while i < bytes.len() && bytes[i] != b'\n' {
                        out[i] = b' ';
                        i += 1;
                    }
                }
                _ => i += 1,
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Whether `hay` contains `word` delimited by non-identifier characters on both
/// sides (so `now` matches `now` and `x=now;` but not `snowfall`).
fn contains_word(hay: &str, word: &str) -> bool {
    let bytes = hay.as_bytes();
    let wb = word.as_bytes();
    let mut i = 0;
    while let Some(pos) = hay[i..].find(word) {
        let start = i + pos;
        let end = start + wb.len();
        let before_ok = start == 0 || !is_ident_byte(bytes[start - 1]);
        let after_ok = end == bytes.len() || !is_ident_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        i = start + 1;
    }
    false
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Rewrite `[text](target)` to plain `text` for every `target` not in `placed`.
/// Other links (placed topics, `http(s)://`, anchors) are left intact.
fn strip_dangling_links(body: &str, placed: &BTreeSet<String>) -> String {
    let bytes = body.as_bytes();
    let mut out = String::with_capacity(body.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'['
            && let Some((text, target, end)) = parse_link(body, i)
        {
            let is_topic = !target.contains(':') && !target.starts_with('#');
            if is_topic && !placed.contains(&target) {
                out.push_str(&text);
                i = end;
                continue;
            }
        }
        // Push one UTF-8 char.
        let ch = body[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Parse a markdown `[text](target)` starting at byte `start` (which must be
/// `[`). Returns `(text, target, end_byte)` if it matches, else `None`. Does not
/// handle nested brackets (none in this corpus).
fn parse_link(body: &str, start: usize) -> Option<(String, String, usize)> {
    let after_lb = &body[start + 1..];
    let close_b = after_lb.find(']')?;
    let text = &after_lb[..close_b];
    let rest = &after_lb[close_b + 1..];
    if !rest.starts_with('(') {
        return None;
    }
    let close_p = rest[1..].find(')')?;
    let target = &rest[1..1 + close_p];
    let end = start + 1 + close_b + 1 + 1 + close_p + 1;
    Some((text.to_string(), target.to_string(), end))
}

/// Render one section module: header + `register_section!` + the section's
/// `register_doc!` blocks (sorted by name).
fn render_section_module(
    section: &str,
    title: &str,
    summary: &str,
    docs: &[ConvertedDoc],
) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "//! Migrated help docs for the `{section}` section (help-system P7).\n\
         //! GENERATED by `cargo xtask migrate-place` — do not edit by hand.\n\n"
    ));
    s.push_str("fm_doc::register_section! {\n");
    s.push_str(&format!("    id: {},\n", rust_str(section)));
    s.push_str(&format!("    title: {},\n", rust_str(title)));
    s.push_str(&format!("    summary: {},\n", rust_str(summary)));
    s.push_str("}\n\n");

    let mut sorted: Vec<&ConvertedDoc> = docs.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    for d in sorted {
        s.push_str("fm_doc::register_doc! {\n");
        s.push_str(&format!("    name: {},\n", rust_str(&d.name)));
        if d.aliases.is_empty() {
            s.push_str("    aliases: [],\n");
        } else {
            let al: Vec<String> = d.aliases.iter().map(|a| rust_str(a)).collect();
            s.push_str(&format!("    aliases: [{}],\n", al.join(", ")));
        }
        s.push_str(&format!("    section: {},\n", rust_str(&d.section)));
        s.push_str(&format!("    summary: {},\n", rust_str(&d.summary)));
        s.push_str(&format!("    body: {},\n", raw_str(&d.body_md)));
        s.push_str("}\n\n");
    }
    s
}

/// Emit a Rust raw string literal `r#"…"#` for a body, escalating the `#` count
/// if the body contains the closing delimiter. Falls back to a normal escaped
/// string (via [`rust_str`]) for pathological bodies (caught by `compile_safe`).
fn raw_str(body: &str) -> String {
    let mut hashes = 1;
    loop {
        let close = format!("\"{}", "#".repeat(hashes));
        if !body.contains(&close) && !body.ends_with('"') {
            let h = "#".repeat(hashes);
            return format!("r{h}\"{body}\"{h}");
        }
        hashes += 1;
        if hashes > 16 {
            return rust_str(body);
        }
    }
}
