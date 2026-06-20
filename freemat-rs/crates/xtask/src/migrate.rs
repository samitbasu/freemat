//! `cargo xtask migrate-docs` — the legacy-doc → markdown-dialect converter
//! (help-system phase P6, implementing §1's legacy→dialect mapping against the
//! §3 dialect of `docs/HELP_SYSTEM.md`).
//!
//! ```text
//! cargo xtask migrate-docs                       # convert all sections
//! cargo xtask migrate-docs --section mathfunctions
//! cargo xtask migrate-docs --out some/dir
//! ```
//!
//! Reads `../FreeMat/doc/**` (Doxygen `.doc` pages, excluding any `vtk` path)
//! plus the 2 inline-`@Module` toolbox `.m` files, converts each page to the
//! new markdown dialect, and writes per-section staging files
//! (`<section>.rs`, each one `register_section!{…}` + the section's
//! `register_doc!{…}` blocks) plus a `REPORT.md` summary into the staging dir
//! (`docs/migration-staging/` by default).
//!
//! **The staging dir is plain text for P7 review — it is NOT compiled into any
//! crate.** Nothing here is placed into `fm-builtins`; that is P7's job.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::{rust_str, workspace_root};

// ============================================================================
// In-memory corpus model (shared by `migrate-docs` and `migrate-place`)
// ============================================================================

/// One converted help topic, in memory (the structural output of the
/// legacy→dialect conversion). Both `migrate-docs` (writes staging text) and
/// `migrate-place` (emits compiled `register_doc!` into `fm-builtins`) consume
/// this — the conversion runs exactly once, in [`convert_corpus`].
#[derive(Debug, Clone)]
pub struct ConvertedDoc {
    pub name: String,
    pub aliases: Vec<String>,
    pub section: String,
    pub summary: String,
    pub body_md: String,
    pub has_fragment: bool,
}

/// One converted section (id + title), in memory.
#[derive(Debug, Clone)]
pub struct ConvertedSection {
    pub id: String,
    pub title: String,
}

/// The full converted corpus: every section (sorted by id) and every doc
/// (sorted by `(section, name)`), already deduped of nothing — placement-time
/// concerns (name dedup, fragment probing, cross-link resolution) are the
/// caller's job.
pub struct Corpus {
    pub sections: Vec<ConvertedSection>,
    pub docs: Vec<ConvertedDoc>,
}

/// Convert the entire legacy corpus in memory (the same conversion path
/// `migrate-docs` writes to staging). Returns sections + docs, deterministically
/// sorted. `legacy` is the FreeMat checkout root (`<workspace>/../FreeMat`).
pub fn convert_corpus(legacy: &Path) -> Result<Corpus, String> {
    let doc_root = legacy.join("doc");
    if !doc_root.is_dir() {
        return Err(format!(
            "legacy doc dir not found: {} (expected the FreeMat checkout beside the rust port)",
            doc_root.display()
        ));
    }

    let descriptors = read_section_descriptors(legacy)?;
    let section_ids: Vec<String> = section_dirs(&doc_root)?;

    let mut sections: Vec<ConvertedSection> = Vec::new();
    let mut docs: Vec<ConvertedDoc> = Vec::new();

    for sec in &section_ids {
        let sec_dir = doc_root.join(sec);
        let title = descriptors.get(sec).cloned().unwrap_or_else(|| {
            section_title_from_sec_doc(&doc_root, sec).unwrap_or_else(|| sec.clone())
        });
        sections.push(ConvertedSection {
            id: sec.clone(),
            title,
        });

        let mut files: Vec<PathBuf> = std::fs::read_dir(&sec_dir)
            .map_err(|e| format!("reading {}: {e}", sec_dir.display()))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "doc"))
            .collect();
        files.sort();

        for f in &files {
            let text = std::fs::read_to_string(f)
                .map_err(|e| format!("{}: read error: {e}", f.display()))?;
            if let Ok(d) = convert_doc_page(&text, sec) {
                docs.push(ConvertedDoc {
                    name: d.name,
                    aliases: d.aliases,
                    section: d.section,
                    summary: d.summary,
                    body_md: d.body,
                    has_fragment: d.has_fragment,
                });
            }
        }
    }

    // Secondary: the inline-@Module toolbox `.m` files.
    let mut module_docs: Vec<ConvertedDoc> = Vec::new();
    let mut module_sections: BTreeMap<String, String> = BTreeMap::new();
    for path in find_module_m_files(legacy) {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if !text.contains("@Module") {
            continue;
        }
        if let Some(d) = convert_module_m(&text) {
            let sec_id = d.section.to_ascii_lowercase();
            module_sections
                .entry(sec_id.clone())
                .or_insert_with(|| sec_id.clone());
            module_docs.push(ConvertedDoc {
                name: d.name,
                aliases: d.aliases,
                section: sec_id,
                summary: d.summary,
                body_md: d.body,
                has_fragment: d.has_fragment,
            });
        }
    }
    // Only add a section for the `.m` modules if the `.doc` corpus didn't
    // already define one with that id (the `freemat` section overlaps).
    let known: std::collections::HashSet<String> = sections.iter().map(|s| s.id.clone()).collect();
    for (id, title) in module_sections {
        if !known.contains(&id) {
            sections.push(ConvertedSection { id, title });
        }
    }
    docs.extend(module_docs);

    sections.sort_by(|a, b| a.id.cmp(&b.id));
    docs.sort_by(|a, b| (&a.section, &a.name).cmp(&(&b.section, &b.name)));
    Ok(Corpus { sections, docs })
}

/// Locate the legacy FreeMat checkout (`<workspace>/../FreeMat`).
pub fn legacy_root() -> Result<PathBuf, String> {
    let root = workspace_root()?;
    Ok(root
        .parent()
        .ok_or("cannot find parent of workspace root")?
        .join("FreeMat"))
}

// ============================================================================
// CLI entry point
// ============================================================================

/// Run the `migrate-docs` task. `section` restricts to a single section id;
/// `out` overrides the staging directory (default `docs/migration-staging/`).
pub fn migrate_docs(section: Option<&str>, out: Option<&str>) -> Result<(), String> {
    let root = workspace_root()?;
    // The legacy FreeMat checkout sits next to the rust port: `<root>/../FreeMat`.
    let legacy = root
        .parent()
        .ok_or("cannot find parent of workspace root")?
        .join("FreeMat");
    let doc_root = legacy.join("doc");
    if !doc_root.is_dir() {
        return Err(format!(
            "legacy doc dir not found: {} (expected the FreeMat checkout beside the rust port)",
            doc_root.display()
        ));
    }

    let out_dir = match out {
        Some(o) => {
            let p = PathBuf::from(o);
            if p.is_absolute() { p } else { root.join(p) }
        }
        None => root.join("docs").join("migration-staging"),
    };

    let descriptors = read_section_descriptors(&legacy)?;
    let mut report = Report::default();

    // Discover the section ids to process.
    let mut section_ids: Vec<String> = section_dirs(&doc_root)?;
    if let Some(want) = section {
        section_ids.retain(|s| s == want);
        if section_ids.is_empty() {
            return Err(format!(
                "no such section directory under {}: {want}",
                doc_root.display()
            ));
        }
    }

    std::fs::create_dir_all(&out_dir)
        .map_err(|e| format!("creating {}: {e}", out_dir.display()))?;

    for sec in &section_ids {
        let sec_dir = doc_root.join(sec);
        let title = descriptors.get(sec).cloned().unwrap_or_else(|| {
            section_title_from_sec_doc(&doc_root, sec).unwrap_or_else(|| sec.clone())
        });

        let mut docs: Vec<DocOut> = Vec::new();
        let mut files: Vec<PathBuf> = std::fs::read_dir(&sec_dir)
            .map_err(|e| format!("reading {}: {e}", sec_dir.display()))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "doc"))
            .collect();
        files.sort();

        for f in &files {
            report.found += 1;
            let text = match std::fs::read_to_string(f) {
                Ok(t) => t,
                Err(e) => {
                    report.skipped += 1;
                    report
                        .failures
                        .push(format!("{}: read error: {e}", f.display()));
                    continue;
                }
            };
            match convert_doc_page(&text, sec) {
                Ok(mut doc) => {
                    for u in &doc.unknown {
                        report.unknown.push(format!("{}: {}", f.display(), u));
                    }
                    if let Some(mismatch) = doc.id_mismatch.take() {
                        report
                            .id_mismatches
                            .push(format!("{}: {mismatch}", f.display()));
                    }
                    if doc.has_fragment {
                        report.fragments += 1;
                    }
                    report.converted += 1;
                    *report.per_section.entry(sec.clone()).or_default() += 1;
                    docs.push(doc);
                }
                Err(e) => {
                    report.skipped += 1;
                    report.failures.push(format!("{}: {e}", f.display()));
                }
            }
        }

        docs.sort_by(|a, b| a.name.cmp(&b.name));
        let rendered = render_section_file(sec, &title, &docs);
        let path = out_dir.join(format!("{sec}.rs"));
        std::fs::write(&path, rendered).map_err(|e| format!("writing {}: {e}", path.display()))?;
    }

    // --- Secondary: the inline-@Module toolbox .m files ---------------------
    // (only when not restricted to a specific .doc section)
    if section.is_none() {
        migrate_module_m_files(&legacy, &out_dir, &mut report)?;
    }

    // --- REPORT.md ----------------------------------------------------------
    let report_path = out_dir.join("REPORT.md");
    std::fs::write(&report_path, report.render(&section_ids))
        .map_err(|e| format!("writing {}: {e}", report_path.display()))?;

    println!(
        "migrate-docs: {} page(s) found, {} converted, {} skipped → {}",
        report.found,
        report.converted,
        report.skipped,
        out_dir.display()
    );
    println!("  report: {}", report_path.display());
    Ok(())
}

// ============================================================================
// Report
// ============================================================================

#[derive(Default)]
struct Report {
    found: usize,
    converted: usize,
    skipped: usize,
    fragments: usize,
    per_section: BTreeMap<String, usize>,
    /// `"\command"`-style unknown markup, with the source path.
    unknown: Vec<String>,
    /// Pages that failed to parse.
    failures: Vec<String>,
    /// Pages whose `\page` id didn't match the `<section>_<name>` shape.
    id_mismatches: Vec<String>,
    /// Notes for the 2 inline-@Module .m files.
    module_notes: Vec<String>,
}

impl Report {
    fn render(&self, sections: &[String]) -> String {
        let mut s = String::new();
        s.push_str("# Migration staging report (P6 `cargo xtask migrate-docs`)\n\n");
        s.push_str("Generated by `cargo xtask migrate-docs`. This is a **staging** area for\n");
        s.push_str("P7 review — nothing here is compiled into any crate.\n\n");
        s.push_str("## Totals\n\n");
        s.push_str(&format!("- pages found: **{}**\n", self.found));
        s.push_str(&format!("- pages converted: **{}**\n", self.converted));
        s.push_str(&format!(
            "- pages skipped (parse failures): **{}**\n",
            self.skipped
        ));
        s.push_str(&format!(
            "- pages with `fm-exec` fragments: **{}**\n",
            self.fragments
        ));
        s.push_str(&format!("- sections processed: **{}**\n\n", sections.len()));

        s.push_str("## Per-section counts\n\n");
        s.push_str("| section | converted |\n|---------|-----------|\n");
        for sec in sections {
            let n = self.per_section.get(sec).copied().unwrap_or(0);
            s.push_str(&format!("| {sec} | {n} |\n"));
        }
        s.push('\n');

        s.push_str(&format!("## Unknown markup ({})\n\n", self.unknown.len()));
        s.push_str("Pages that hit a `\\command` the converter doesn't recognize. The literal\n");
        s.push_str("marker `[[unknown: \\cmd]]` is left in the emitted body for P7 to fix.\n\n");
        if self.unknown.is_empty() {
            s.push_str("_None._\n\n");
        } else {
            for u in &self.unknown {
                s.push_str(&format!("- {u}\n"));
            }
            s.push('\n');
        }

        s.push_str(&format!("## Parse failures ({})\n\n", self.failures.len()));
        if self.failures.is_empty() {
            s.push_str("_None._\n\n");
        } else {
            for f in &self.failures {
                s.push_str(&format!("- {f}\n"));
            }
            s.push('\n');
        }

        s.push_str(&format!(
            "## `\\page` id mismatches ({})\n\n",
            self.id_mismatches.len()
        ));
        s.push_str("Pages whose `\\page <id>` did not match the `<section>_<name>` shape\n");
        s.push_str("(section taken from the directory; name from the `<NAME>` token).\n\n");
        if self.id_mismatches.is_empty() {
            s.push_str("_None._\n\n");
        } else {
            for m in &self.id_mismatches {
                s.push_str(&format!("- {m}\n"));
            }
            s.push('\n');
        }

        s.push_str("## Inline `@Module` `.m` files\n\n");
        if self.module_notes.is_empty() {
            s.push_str("_None._\n\n");
        } else {
            for n in &self.module_notes {
                s.push_str(&format!("- {n}\n"));
            }
            s.push('\n');
        }
        s
    }
}

// ============================================================================
// Doxygen `.doc` page conversion
// ============================================================================

/// The converted output for one page.
struct DocOut {
    name: String,
    section: String,
    summary: String,
    aliases: Vec<String>,
    body: String,
    has_fragment: bool,
    unknown: Vec<String>,
    id_mismatch: Option<String>,
}

/// Convert one Doxygen `.doc` page body to a [`DocOut`]. `dir_section` is the
/// directory name (the authoritative section id).
fn convert_doc_page(text: &str, dir_section: &str) -> Result<DocOut, String> {
    // Strip the `/*!` … `*/` wrapper and any leading ` * ` / `//` comment
    // prefixes (the `.doc` pages are mostly bare, but be robust).
    let inner = strip_comment_wrapper(text);
    let lines: Vec<&str> = inner.lines().collect();

    // --- \page line ---------------------------------------------------------
    let page_idx = lines
        .iter()
        .position(|l| l.trim_start().starts_with("\\page"))
        .ok_or("no \\page directive found")?;
    let page_line = lines[page_idx].trim_start();
    let rest = page_line.strip_prefix("\\page").unwrap().trim_start();
    let mut it = rest.splitn(3, char::is_whitespace);
    let id = it.next().ok_or("\\page missing id")?.to_string();
    let name_tok = it.next().ok_or("\\page missing NAME")?.to_string();
    let summary_tail = it.next().unwrap_or("").trim().to_string();

    let name = name_tok.to_ascii_lowercase();

    // Section id = prefix before the first `_` of the id; validate it matches
    // the directory and that `<section>_<name>` holds.
    let mut id_mismatch = None;
    let id_section = id.split_once('_').map(|(s, _)| s.to_string());
    match &id_section {
        Some(s) if s == dir_section => {}
        Some(s) => {
            id_mismatch = Some(format!(
                "\\page id {id:?} section-prefix {s:?} != directory {dir_section:?}"
            ));
        }
        None => {
            id_mismatch = Some(format!("\\page id {id:?} has no `_` separator"));
        }
    }
    let section = dir_section.to_string();

    // The `\page` NAME might encode an alias list? In practice it does not; the
    // first token is the function name, alternate names are rare. We keep
    // aliases empty unless detected from the body (none in this corpus).
    let aliases: Vec<String> = Vec::new();

    // Summary = the trailing text of the \page line, plus following lines up to
    // the first \section / Section: / blank-with-content boundary. We take the
    // first sentence/line, trimmed (per §1).
    let mut summary = summary_tail;
    if summary.is_empty() {
        // Look ahead for a prose line before the first \section.
        for l in &lines[page_idx + 1..] {
            let t = l.trim();
            if t.is_empty() || t.starts_with("<p>") || t.starts_with("<P>") {
                continue;
            }
            if t.starts_with("\\section") || t.starts_with("Section:") {
                break;
            }
            summary = t.to_string();
            break;
        }
    }
    let summary = clean_summary(&summary);

    // --- Body ---------------------------------------------------------------
    let mut conv = BodyConverter::new();
    conv.run(&lines[page_idx + 1..]);
    let has_fragment = conv.has_fragment;
    let unknown = std::mem::take(&mut conv.unknown);
    let body = conv.finish();

    Ok(DocOut {
        name,
        section,
        summary,
        aliases,
        body,
        has_fragment,
        unknown,
        id_mismatch,
    })
}

/// Reduce a raw summary string to a clean one-liner: strip inline `<tt>`/markup,
/// collapse whitespace, take the first sentence, trim a trailing period-run.
fn clean_summary(raw: &str) -> String {
    let mut s = strip_inline(raw, &mut Vec::new());
    s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    // First sentence: cut at the first ". " (keep the period).
    if let Some(pos) = s.find(". ") {
        s.truncate(pos + 1);
    }
    s.trim().to_string()
}

/// Strip the Doxygen `/*!` … `*/` comment wrapper and any leading comment
/// prefixes (` * `, `//`, `%`) from each line.
fn strip_comment_wrapper(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for raw in text.lines() {
        let mut line = raw;
        let trimmed = line.trim_start();
        if trimmed == "/*!" || trimmed == "/*" || trimmed == "*/" {
            continue;
        }
        // Leading ` * ` block-comment continuation.
        let lt = line.trim_start();
        if let Some(stripped) = lt.strip_prefix("* ") {
            line = stripped;
        } else if lt == "*" {
            line = "";
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

// ============================================================================
// Body conversion (line-oriented state machine)
// ============================================================================

/// Streaming converter: walks the body lines, handling block constructs
/// (`\verbatim`, `\if FRAGMENT`, `\if FILE`, `\f[…\f]`) as multi-line modes and
/// applying inline rules everywhere else.
struct BodyConverter {
    out: String,
    unknown: Vec<String>,
    has_fragment: bool,
    /// Pending: did the previous fragment want a figure (followed by \image)?
    /// We resolve `\image` *after* the fragment, so we may need to upgrade the
    /// already-emitted fence. Track the byte offset of the last `fm-exec` fence
    /// opener so a trailing `\image`/`mprint` can upgrade it to `:figure`.
    last_fragment_fence: Option<usize>,
}

impl BodyConverter {
    fn new() -> Self {
        Self {
            out: String::new(),
            unknown: Vec::new(),
            has_fragment: false,
            last_fragment_fence: None,
        }
    }

    fn run(&mut self, lines: &[&str]) {
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];
            let t = line.trim_start();

            // \section X [more...] -> ## X [more...]
            if let Some(rest) = t.strip_prefix("\\section") {
                let heading = rest.trim();
                let heading = normalize_heading(heading);
                self.ensure_blank_before_block();
                self.out.push_str("## ");
                self.out.push_str(&heading);
                self.out.push('\n');
                i += 1;
                continue;
            }

            // \verbatim … \endverbatim -> ```text … ```
            if t == "\\verbatim" || t.starts_with("\\verbatim ") {
                i = self.emit_verbatim(lines, i);
                continue;
            }

            // \if FRAGMENT … \endif -> ```fm-exec``` (or :figure)
            if t == "\\if FRAGMENT" {
                i = self.emit_fragment(lines, i);
                continue;
            }

            // \if FILE … \endif -> ```fm-file:NAME```
            if t == "\\if FILE" {
                i = self.emit_file(lines, i);
                continue;
            }

            // \f[ … \f] -> $$ … $$
            if t == "\\f[" {
                i = self.emit_display_math(lines, i);
                continue;
            }

            // \verbinclude … -> DROP
            if t.starts_with("\\verbinclude") {
                i += 1;
                continue;
            }

            // \image … -> DROP (but upgrade the preceding fragment to :figure)
            if t.starts_with("\\image") {
                self.upgrade_last_fragment_to_figure();
                i += 1;
                continue;
            }

            // "Section: \ref sec_x ..." line -> DROP (section is a struct field).
            if t.starts_with("Section:") {
                i += 1;
                continue;
            }

            // HTML list openers/closers.
            if let Some(rest) = strip_html_tag(t, &["ul", "ol"]) {
                // Drop the tag; keep any trailing content on the line.
                if !rest.trim().is_empty() {
                    self.push_inline_line(&rest);
                }
                i += 1;
                continue;
            }
            if let Some(rest) = strip_html_open(t, "li") {
                self.out.push_str("- ");
                self.push_inline_line(rest.trim_start());
                i += 1;
                continue;
            }

            // Plain prose / inline line.
            self.push_inline_line(line);
            i += 1;
        }
    }

    fn ensure_blank_before_block(&mut self) {
        if !self.out.is_empty() && !self.out.ends_with("\n\n") {
            if !self.out.ends_with('\n') {
                self.out.push('\n');
            }
            self.out.push('\n');
        }
    }

    fn push_inline_line(&mut self, line: &str) {
        let converted = strip_inline(line, &mut self.unknown);
        self.out.push_str(&converted);
        self.out.push('\n');
    }

    /// Emit a `\verbatim … \endverbatim` block as a ```text fence. Returns the
    /// index just past `\endverbatim`.
    fn emit_verbatim(&mut self, lines: &[&str], start: usize) -> usize {
        self.ensure_blank_before_block();
        self.out.push_str("```text\n");
        let mut i = start + 1;
        while i < lines.len() && lines[i].trim_start() != "\\endverbatim" {
            // Verbatim content is literal; do NOT apply inline rules.
            self.out.push_str(lines[i]);
            self.out.push('\n');
            i += 1;
        }
        self.out.push_str("```\n");
        if i < lines.len() {
            i += 1; // skip \endverbatim
        }
        i
    }

    /// Emit a `\if FRAGMENT … \endif` block as a ```fm-exec fence. Drops the
    /// leading generated filename line; converts the error-count line into a
    /// `# errors: N` header only when N>0. Returns index past `\endif`.
    fn emit_fragment(&mut self, lines: &[&str], start: usize) -> usize {
        self.has_fragment = true;
        let mut i = start + 1;
        // line 1: generated filename -> DROP
        if i < lines.len() {
            i += 1;
        }
        // line 2: expected error count
        let mut errors = 0usize;
        if i < lines.len() {
            errors = lines[i].trim().parse().unwrap_or(0);
            i += 1;
        }
        // Remaining lines up to \endif: REPL input. Detect mprint/print for the
        // figure hint.
        let mut body: Vec<&str> = Vec::new();
        let mut figure = false;
        while i < lines.len() && lines[i].trim_start() != "\\endif" {
            let l = lines[i];
            let lt = l.trim_start();
            if lt.starts_with("mprint(") || lt.starts_with("print(") {
                figure = true;
            }
            body.push(l);
            i += 1;
        }
        if i < lines.len() {
            i += 1; // skip \endif
        }

        self.ensure_blank_before_block();
        // Record the fence-open offset so a trailing \image can upgrade it.
        let fence_offset = self.out.len();
        if figure {
            self.out.push_str("```fm-exec:figure\n");
        } else {
            self.out.push_str("```fm-exec\n");
        }
        self.last_fragment_fence = Some(fence_offset);
        if errors > 0 {
            self.out.push_str(&format!("# errors: {errors}\n"));
        }
        for l in body {
            // REPL input is literal FreeMat source — no inline markup.
            self.out.push_str(l);
            self.out.push('\n');
        }
        self.out.push_str("```\n");
        i
    }

    /// Emit a `\if FILE … \endif` block as a ```fm-file:NAME fence. The first
    /// line is the filename; the rest is the file contents. Returns index past
    /// `\endif`.
    fn emit_file(&mut self, lines: &[&str], start: usize) -> usize {
        let mut i = start + 1;
        let name = if i < lines.len() {
            let n = lines[i].trim().to_string();
            i += 1;
            n
        } else {
            String::new()
        };
        let mut body: Vec<&str> = Vec::new();
        while i < lines.len() && lines[i].trim_start() != "\\endif" {
            body.push(lines[i]);
            i += 1;
        }
        if i < lines.len() {
            i += 1; // skip \endif
        }
        self.ensure_blank_before_block();
        self.out.push_str(&format!("```fm-file:{name}\n"));
        for l in body {
            self.out.push_str(l);
            self.out.push('\n');
        }
        self.out.push_str("```\n");
        i
    }

    /// Emit a `\f[ … \f]` display-math block as `$$ … $$`. The TeX inside is
    /// kept verbatim. Returns index past `\f]`.
    fn emit_display_math(&mut self, lines: &[&str], start: usize) -> usize {
        let mut i = start + 1;
        let mut tex: Vec<&str> = Vec::new();
        while i < lines.len() && lines[i].trim_start() != "\\f]" {
            tex.push(lines[i]);
            i += 1;
        }
        if i < lines.len() {
            i += 1; // skip \f]
        }
        self.ensure_blank_before_block();
        self.out.push_str("$$\n");
        for l in tex {
            self.out.push_str(l.trim_end());
            self.out.push('\n');
        }
        self.out.push_str("$$\n");
        i
    }

    /// Upgrade the most recently emitted `fm-exec` fence to `fm-exec:figure`
    /// (called when a `\image` follows the fragment).
    fn upgrade_last_fragment_to_figure(&mut self) {
        if let Some(off) = self.last_fragment_fence {
            let marker = "```fm-exec\n";
            if self.out[off..].starts_with(marker) {
                self.out
                    .replace_range(off..off + marker.len(), "```fm-exec:figure\n");
            }
            // Already :figure — nothing to do.
            self.last_fragment_fence = None;
        }
    }

    fn finish(self) -> String {
        // Collapse 3+ consecutive newlines to a single blank line, and ensure a
        // single leading newline (matching the `body: r#"\n…"#` convention).
        let body = collapse_blank_lines(&self.out);
        let trimmed = body.trim_matches('\n');
        format!("\n{trimmed}\n")
    }
}

/// Title-case-ish normalization for a `\section` heading. Most are already
/// like `Usage`, `Function Internals`, `Example`. We only fix the all-caps
/// `USAGE` variant and trim.
fn normalize_heading(h: &str) -> String {
    let h = h.trim();
    if h == "USAGE" || h == "Usage" {
        return "Usage".to_string();
    }
    // Title-case each word for shouty headings, otherwise keep as-is.
    if h.chars().all(|c| !c.is_lowercase()) && h.chars().any(|c| c.is_alphabetic()) {
        return h
            .split_whitespace()
            .map(title_case_word)
            .collect::<Vec<_>>()
            .join(" ");
    }
    h.to_string()
}

fn title_case_word(w: &str) -> String {
    let mut c = w.chars();
    match c.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + &c.as_str().to_ascii_lowercase(),
    }
}

/// Collapse runs of 3+ newlines down to exactly 2 (one blank line).
fn collapse_blank_lines(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut nl = 0;
    for ch in s.chars() {
        if ch == '\n' {
            nl += 1;
            if nl <= 2 {
                out.push('\n');
            }
        } else {
            nl = 0;
            out.push(ch);
        }
    }
    out
}

// ============================================================================
// Inline markup
// ============================================================================

/// Apply inline conversion rules to a single line of prose:
/// - `<tt>…</tt>` → `` `…` ``
/// - `\f$ … \f$` → `$ … $`
/// - `\ref id "text"` / `\ref id` → cross-link
/// - `\b X`/`\e X`/`\c X` → `**X**`/`*X*`/`` `X` ``
/// - `\ldots`/`\n` housekeeping; strip `<p>`/`<strong>` tags
/// - unknown `\command` → `[[unknown: \cmd]]` marker (recorded in `unknown`)
fn strip_inline(line: &str, unknown: &mut Vec<String>) -> String {
    // Convert inline math `\f$ … \f$` → `$ … $` first, so its TeX backslashes
    // aren't treated as Doxygen commands. The resulting `$ … $` spans (and any
    // `<tt>…</tt>` literal-code spans) are then *shielded* from the Doxygen
    // command pass below — TeX (`\pi`, `\times`) and literal regex inside code
    // must survive verbatim.
    let math_done = replace_inline_math(line);

    // <tt>…</tt> -> `…`
    let mut s = replace_tag_pair(&math_done, "tt", "`", "`");
    // <strong>…</strong> / <b>…</b> -> **…**
    s = replace_tag_pair(&s, "strong", "**", "**");
    s = replace_tag_pair(&s, "em", "*", "*");
    s = replace_tag_pair(&s, "i", "*", "*");

    // Drop bare structural tags.
    for tag in [
        "<p>", "</p>", "<P>", "</P>", "<br>", "<br/>", "<hr>", "</li>", "</LI>", "</ul>", "</UL>",
        "</ol>", "</OL>",
    ] {
        s = s.replace(tag, "");
    }

    // \ref id "text" / \ref id (operates on the whole line; refs never appear
    // inside math/code spans in this corpus).
    s = replace_refs(&s);

    // Remaining Doxygen inline commands — but ONLY outside `$…$` math spans and
    // `` `…` `` code spans (their backslashes are content, not markup).
    s = apply_doxy_outside_spans(&s, unknown);

    s.trim_end().to_string()
}

/// Run [`replace_doxy_inline`] on the prose between `$…$` math spans and
/// `` `…` `` inline-code spans, leaving the spans themselves verbatim. This
/// keeps TeX (`\pi`, `\frac`) and literal regex (`\w`, `\s`) intact.
fn apply_doxy_outside_spans(s: &str, unknown: &mut Vec<String>) -> String {
    let mut out = String::with_capacity(s.len());
    let mut seg = String::new();
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        let delim = match ch {
            '`' => Some('`'),
            '$' => Some('$'),
            _ => None,
        };
        match delim {
            Some(d) => {
                // Flush prose segment through the Doxygen pass.
                out.push_str(&replace_doxy_inline(&seg, unknown));
                seg.clear();
                // Copy the span verbatim up to and including the closing delim.
                out.push(d);
                for c in chars.by_ref() {
                    out.push(c);
                    if c == d {
                        break;
                    }
                }
            }
            None => seg.push(ch),
        }
    }
    out.push_str(&replace_doxy_inline(&seg, unknown));
    out
}

/// Replace `\f$ … \f$` with `$ … $`, keeping the TeX verbatim.
fn replace_inline_math(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut in_math = false;
    while i < bytes.len() {
        if s[i..].starts_with("\\f$") {
            out.push_str(if in_math { " $" } else { "$ " });
            in_math = !in_math;
            i += 3;
        } else {
            // push one char
            let ch = s[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    // Normalize the `$ …$ ` spacing the simple way: trim double spaces around $.
    out.replace("$  ", "$ ").replace("  $", " $")
}

/// Replace `<tag>…</tag>` with `open`…`close`. Tag match is case-insensitive.
fn replace_tag_pair(s: &str, tag: &str, open: &str, close: &str) -> String {
    let lo = s.to_ascii_lowercase();
    let ot = format!("<{tag}>");
    let ct = format!("</{tag}>");
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if lo[i..].starts_with(&ot) {
            out.push_str(open);
            i += ot.len();
        } else if lo[i..].starts_with(&ct) {
            out.push_str(close);
            i += ct.len();
        } else {
            let ch = s[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

/// Replace all `\ref id "text"` / `\ref id` occurrences with `[text](name)` /
/// `[name](name)`, where `name` strips the `<section>_` prefix from `id`.
fn replace_refs(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(pos) = rest.find("\\ref") {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + 4..];
        let after = after.trim_start_matches(' ');
        // id = up to whitespace.
        let id_end = after.find(char::is_whitespace).unwrap_or(after.len());
        let id = &after[..id_end];
        let mut tail = &after[id_end..];
        // Optional "text"
        let label;
        let consumed_quote;
        let tt = tail.trim_start();
        if let Some(after_quote) = tt.strip_prefix('"') {
            if let Some(end) = after_quote.find('"') {
                label = after_quote[..end].to_string();
                consumed_quote = (tail.len() - tt.len()) + 1 + end + 1;
            } else {
                label = ref_name(id);
                consumed_quote = 0;
            }
        } else {
            label = ref_name(id);
            consumed_quote = 0;
        }
        let target = ref_name(id);
        out.push_str(&format!("[{label}]({target})"));
        tail = &tail[consumed_quote..];
        rest = tail;
    }
    out.push_str(rest);
    out
}

/// Topic name from a `\ref` id: strip the `<section>_` prefix and a leading
/// `sec_` for section refs.
fn ref_name(id: &str) -> String {
    let id = id.trim();
    if let Some(s) = id.strip_prefix("sec_") {
        return s.to_string();
    }
    match id.split_once('_') {
        Some((_, name)) => name.to_string(),
        None => id.to_string(),
    }
}

/// Handle remaining Doxygen inline commands. `\b X`/`\e X`/`\c X` consume the
/// next whitespace-delimited word; `\n`, `\ldots`, etc. map to text; unknown
/// `\command` tokens are left as `[[unknown: \cmd]]` and recorded.
fn replace_doxy_inline(s: &str, unknown: &mut Vec<String>) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        // Read the command word.
        let rest = &s[idx + 1..];
        let cmd_len = rest
            .find(|c: char| !c.is_ascii_alphabetic())
            .unwrap_or(rest.len());
        let cmd = &rest[..cmd_len];
        if cmd.is_empty() {
            // Lone backslash or escaped punctuation — keep literally.
            out.push('\\');
            continue;
        }
        // Advance the iterator past the command word.
        for _ in 0..cmd_len {
            chars.next();
        }
        match cmd {
            // Emphasis commands consuming the next word.
            "b" | "e" | "c" => {
                // Skip spaces, grab the next word.
                let after = &s[idx + 1 + cmd_len..];
                let after_trim = after.trim_start_matches(' ');
                let skipped = after.len() - after_trim.len();
                let word_len = after_trim
                    .find(char::is_whitespace)
                    .unwrap_or(after_trim.len());
                let word = &after_trim[..word_len];
                let (open, close) = match cmd {
                    "b" => ("**", "**"),
                    "e" => ("*", "*"),
                    _ => ("`", "`"),
                };
                out.push_str(open);
                out.push_str(word);
                out.push_str(close);
                for _ in 0..(skipped + word_len) {
                    chars.next();
                }
            }
            // Doxygen line-break / ellipsis we map to plain text.
            "n" => out.push(' '),
            "ldots" | "dots" | "hdots" => out.push_str("..."),
            // `\li` list item (rare).
            "li" => out.push_str("- "),
            // Escaped characters.
            "backslash" => out.push('\\'),
            _ => {
                out.push_str(&format!("[[unknown: \\{cmd}]]"));
                unknown.push(format!("\\{cmd}"));
            }
        }
    }
    out
}

/// If `t` starts with an open or close `<tag>` (any of `tags`), return the
/// remainder of the line after stripping it. Case-insensitive.
fn strip_html_tag(t: &str, tags: &[&str]) -> Option<String> {
    let lo = t.to_ascii_lowercase();
    for tag in tags {
        for form in [format!("<{tag}>"), format!("</{tag}>")] {
            if lo.starts_with(&form) {
                return Some(t[form.len()..].to_string());
            }
        }
    }
    None
}

/// If `t` starts with `<tag>` (open form, case-insensitive), return the rest.
fn strip_html_open(t: &str, tag: &str) -> Option<String> {
    let lo = t.to_ascii_lowercase();
    let form = format!("<{tag}>");
    if lo.starts_with(&form) {
        Some(t[form.len()..].to_string())
    } else {
        None
    }
}

// ============================================================================
// Sections
// ============================================================================

/// Read `toolbox/help/section_descriptors.txt` → map of id → title.
fn read_section_descriptors(legacy: &Path) -> Result<BTreeMap<String, String>, String> {
    let path = legacy
        .join("toolbox")
        .join("help")
        .join("section_descriptors.txt");
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((id, title)) = line.split_once(char::is_whitespace) {
            map.insert(id.to_string(), title.trim().to_string());
        }
    }
    Ok(map)
}

/// Fallback section title from `sections/sec_<id>.doc`'s `\page` line.
fn section_title_from_sec_doc(doc_root: &Path, sec: &str) -> Option<String> {
    let path = doc_root.join("sections").join(format!("sec_{sec}.doc"));
    let text = std::fs::read_to_string(path).ok()?;
    for line in text.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("\\page") {
            let rest = rest.trim_start();
            // skip the id token
            let (_id, after) = rest.split_once(char::is_whitespace)?;
            return Some(after.trim().to_string());
        }
    }
    None
}

/// List the section directories under `doc/` (excluding `sections`, `fragments`,
/// and any `vtk` path).
fn section_dirs(doc_root: &Path) -> Result<Vec<String>, String> {
    let mut v = Vec::new();
    for e in
        std::fs::read_dir(doc_root).map_err(|e| format!("reading {}: {e}", doc_root.display()))?
    {
        let e = e.map_err(|e| e.to_string())?;
        if !e.path().is_dir() {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_string();
        if name == "sections" || name == "fragments" || name.contains("vtk") {
            continue;
        }
        // Only keep dirs that actually contain .doc pages with a matching prefix.
        v.push(name);
    }
    v.sort();
    Ok(v)
}

// ============================================================================
// Rendering the staging `<section>.rs` file
// ============================================================================

fn render_section_file(section: &str, title: &str, docs: &[DocOut]) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "// MIGRATION STAGING — produced by `cargo xtask migrate-docs` (P6).\n\
         // Section: {section}. NOT compiled; for P7 review/placement into fm-builtins.\n\n"
    ));
    s.push_str("fm_doc::register_section! {\n");
    s.push_str(&format!("    id: {},\n", rust_str(section)));
    s.push_str(&format!("    title: {},\n", rust_str(title)));
    s.push_str(&format!("    summary: {},\n", rust_str(title)));
    s.push_str("}\n\n");

    for d in docs {
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
        s.push_str(&format!("    body: {},\n", raw_str(&d.body)));
        s.push_str("}\n\n");
    }
    s
}

/// Emit a Rust raw string literal `r#"…"#` for a body, escalating the `#` count
/// if the body contains the closing delimiter.
fn raw_str(body: &str) -> String {
    let mut hashes = 1;
    loop {
        let close = format!("\"{}", "#".repeat(hashes));
        if !body.contains(&close) {
            let h = "#".repeat(hashes);
            return format!("r{h}\"{body}\"{h}");
        }
        hashes += 1;
        if hashes > 8 {
            // Pathological; fall back to a normal escaped string.
            return rust_str(body);
        }
    }
}

// ============================================================================
// Inline `@Module` `.m` files (secondary path)
// ============================================================================

/// Convert the (few) toolbox `.m` files that still carry the old inline
/// `@Module`/`@@Section`/`@|code|`/`@[…@]`/`@<…@>`/`@{name…@}` doc grammar.
fn migrate_module_m_files(
    legacy: &Path,
    out_dir: &Path,
    report: &mut Report,
) -> Result<(), String> {
    let candidates = find_module_m_files(legacy);
    if candidates.is_empty() {
        return Ok(());
    }
    let mut by_section: BTreeMap<String, Vec<DocOut>> = BTreeMap::new();
    for path in candidates {
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                report
                    .module_notes
                    .push(format!("{}: read error: {e}", path.display()));
                continue;
            }
        };
        if !text.contains("@Module") {
            continue;
        }
        match convert_module_m(&text) {
            Some(doc) => {
                report.found += 1;
                report.converted += 1;
                if doc.has_fragment {
                    report.fragments += 1;
                }
                report.module_notes.push(format!(
                    "{}: converted (section {:?}, name {:?})",
                    path.display(),
                    doc.section,
                    doc.name
                ));
                by_section.entry(doc.section.clone()).or_default().push(doc);
            }
            None => {
                report.module_notes.push(format!(
                    "{}: contains @Module but could not parse — handle manually",
                    path.display()
                ));
            }
        }
    }
    for (sec, docs) in &by_section {
        // Lowercase the @@Section id (legacy uses uppercase like FREEMAT).
        let sec_id = sec.to_ascii_lowercase();
        let title = sec_id.clone();
        let path = out_dir.join(format!("module_m_{sec_id}.rs"));
        std::fs::write(&path, render_section_file(&sec_id, &title, docs))
            .map_err(|e| format!("writing {}: {e}", path.display()))?;
    }
    Ok(())
}

/// Find toolbox `.m` files containing `@Module`. We restrict to genuine inline
/// doc blocks (a `%@Module` comment line), not source that merely mentions the
/// token (e.g. `helpgen.m`'s regex).
fn find_module_m_files(legacy: &Path) -> Vec<PathBuf> {
    let toolbox = legacy.join("toolbox");
    let mut out = Vec::new();
    walk_m(&toolbox, &mut out);
    out.retain(|p| {
        std::fs::read_to_string(p)
            .map(|t| t.lines().any(|l| l.trim_start().starts_with("%@Module")))
            .unwrap_or(false)
    });
    out.sort();
    out
}

fn walk_m(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk_m(&p, out);
        } else if p.extension().is_some_and(|x| x == "m") {
            out.push(p);
        }
    }
}

/// Convert a legacy inline-`@Module` `.m` doc block to a [`DocOut`].
fn convert_module_m(text: &str) -> Option<DocOut> {
    // Collect the leading `%`-comment doc block, stripping the `%` prefix.
    let mut lines: Vec<String> = Vec::new();
    for raw in text.lines() {
        let t = raw.trim_start();
        if let Some(rest) = t.strip_prefix('%') {
            lines.push(rest.strip_prefix(' ').unwrap_or(rest).to_string());
        } else if t.starts_with("function") || t.starts_with("function ") {
            break;
        } else if t.is_empty() {
            lines.push(String::new());
        } else {
            break;
        }
    }

    // @Module NAME summary...
    let module_line = lines.iter().find(|l| l.starts_with("@Module"))?;
    let rest = module_line.strip_prefix("@Module").unwrap().trim_start();
    let mut it = rest.splitn(2, char::is_whitespace);
    let name = it.next()?.to_ascii_lowercase();
    let summary = clean_summary(it.next().unwrap_or(""));

    // @@Section X (legacy uses uppercase ids like FREEMAT; section ids in the
    // new system are lowercase, matching the directory-derived ids).
    let section = lines
        .iter()
        .find_map(|l| l.strip_prefix("@@Section"))
        .map(|s| s.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "misc".to_string());

    // Body: everything after the @Module line, converting @@Heading and the
    // @|…|, @[…@], @<…@>, @{…@} constructs.
    let mut body = String::new();
    let mut has_fragment = false;
    let mut unknown = Vec::new();
    let start = lines.iter().position(|l| l.starts_with("@Module")).unwrap() + 1;
    let body_lines: Vec<&str> = lines[start..].iter().map(String::as_str).collect();
    let mut i = 0;
    while i < body_lines.len() {
        let l = body_lines[i];
        let t = l.trim_start();
        if let Some(h) = t.strip_prefix("@@") {
            // @@Section already consumed; other @@X become ## X.
            let h = h.trim();
            if h.starts_with("Section") {
                i += 1;
                continue;
            }
            if !body.is_empty() && !body.ends_with("\n\n") {
                body.push('\n');
            }
            body.push_str(&format!("## {h}\n"));
            i += 1;
            continue;
        }
        // @[ … @] verbatim
        if t == "@[" {
            body.push_str("```text\n");
            i += 1;
            while i < body_lines.len() && body_lines[i].trim_start() != "@]" {
                body.push_str(body_lines[i]);
                body.push('\n');
                i += 1;
            }
            body.push_str("```\n");
            i += 1;
            continue;
        }
        // @< … @> executed fragment
        if t == "@<" {
            has_fragment = true;
            body.push_str("```fm-exec\n");
            i += 1;
            while i < body_lines.len() && body_lines[i].trim_start() != "@>" {
                body.push_str(body_lines[i]);
                body.push('\n');
                i += 1;
            }
            body.push_str("```\n");
            i += 1;
            continue;
        }
        // @{ name … @} file block
        if let Some(fname) = t.strip_prefix("@{") {
            let fname = fname.trim();
            body.push_str(&format!("```fm-file:{fname}\n"));
            i += 1;
            while i < body_lines.len() && body_lines[i].trim_start() != "@}" {
                body.push_str(body_lines[i]);
                body.push('\n');
                i += 1;
            }
            body.push_str("```\n");
            i += 1;
            continue;
        }
        // Inline: @|code| -> `code`; \[ \] display math.
        body.push_str(&convert_module_inline(l, &mut unknown));
        body.push('\n');
        i += 1;
    }

    let body = format!("\n{}\n", collapse_blank_lines(&body).trim_matches('\n'));
    Some(DocOut {
        name,
        section,
        summary,
        aliases: Vec::new(),
        body,
        has_fragment,
        unknown,
        id_mismatch: None,
    })
}

/// Inline conversion for the old `@Module` grammar: `@|code|` → `` `code` ``.
fn convert_module_inline(line: &str, _unknown: &mut [String]) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(pos) = rest.find("@|") {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + 2..];
        if let Some(end) = after.find('|') {
            out.push('`');
            out.push_str(&after[..end]);
            out.push('`');
            rest = &after[end + 1..];
        } else {
            out.push_str("@|");
            rest = after;
        }
    }
    out.push_str(rest);
    // \[ … \] display math on a single line (rare); leave block-level handling
    // to manual review.
    out.replace("\\[", "$$").replace("\\]", "$$")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_split_and_summary() {
        let page = "\
/*!
\\page mathfunctions_cos COS Trigonometric Cosine Function
Section: \\ref sec_mathfunctions \"Mathematical Functions\"
\\section Usage
Computes the <tt>cos</tt> function.
*/";
        let d = convert_doc_page(page, "mathfunctions").unwrap();
        assert_eq!(d.name, "cos");
        assert_eq!(d.section, "mathfunctions");
        assert_eq!(d.summary, "Trigonometric Cosine Function");
        assert!(d.id_mismatch.is_none());
        assert!(d.body.contains("## Usage"));
        assert!(d.body.contains("`cos`"));
    }

    #[test]
    fn id_mismatch_detected() {
        let page = "\
\\page wrongsec_foo FOO A foo.
\\section Usage
Body.";
        let d = convert_doc_page(page, "mathfunctions").unwrap();
        assert!(d.id_mismatch.is_some());
        // Section is taken from the directory, not the id.
        assert_eq!(d.section, "mathfunctions");
    }

    #[test]
    fn verbatim_block() {
        let page = "\
\\page array_x X A thing.
\\section Usage
\\verbatim
  y = x(a)
\\endverbatim
done";
        let d = convert_doc_page(page, "array").unwrap();
        assert!(d.body.contains("```text\n  y = x(a)\n```"), "{}", d.body);
    }

    #[test]
    fn inline_and_display_math() {
        let page = "\
\\page m_foo FOO Foo.
\\section Function Internals
The value \\f$ x^2 \\f$ matters.
\\f[
  \\cos x \\equiv 1
\\f]";
        let d = convert_doc_page(page, "m").unwrap();
        assert!(d.body.contains("$ x^2 $"), "inline math: {}", d.body);
        assert!(
            d.body.contains("$$\n  \\cos x \\equiv 1\n$$"),
            "display: {}",
            d.body
        );
    }

    #[test]
    fn fragment_with_figure() {
        let page = "\
\\page mathfunctions_cos COS Cosine.
\\section Example
\\if FRAGMENT
frag_mathfunctions_cos_000.m
0
x = linspace(0,1);
plot(x,cos(2*pi*x))
mprint('cosplot');
\\endif
\\verbinclude frag_mathfunctions_cos_000.m.out
\\image html cosplot.png
\\image latex cosplot.eps \"cosplot\" width=12cm";
        let d = convert_doc_page(page, "mathfunctions").unwrap();
        assert!(d.has_fragment);
        assert!(d.body.contains("```fm-exec:figure"), "{}", d.body);
        // dropped filename + zero error count -> no `# errors:` line
        assert!(!d.body.contains("frag_mathfunctions_cos_000"), "{}", d.body);
        assert!(!d.body.contains("# errors:"), "{}", d.body);
        assert!(d.body.contains("x = linspace(0,1);"));
        // \verbinclude and \image dropped
        assert!(!d.body.contains("verbinclude"));
        assert!(!d.body.contains("\\image"));
    }

    #[test]
    fn fragment_error_count_emitted_when_nonzero() {
        let page = "\
\\page flow_error ERROR Err.
\\section Example
\\if FRAGMENT
frag_flow_error_000.m
2
error('boom')
\\endif";
        let d = convert_doc_page(page, "flow").unwrap();
        assert!(d.body.contains("```fm-exec\n# errors: 2\n"), "{}", d.body);
    }

    #[test]
    fn fragment_with_mprint_is_figure_even_without_image() {
        let page = "\
\\page graphics_plot PLOT Plot.
\\section Example
\\if FRAGMENT
frag_x_000.m
0
plot(1:10)
mprint('p');
\\endif";
        let d = convert_doc_page(page, "graphics").unwrap();
        assert!(d.body.contains("```fm-exec:figure"), "{}", d.body);
    }

    #[test]
    fn file_block() {
        let page = "\
\\page flow_switch SWITCH Switch.
\\section Examples
\\if FILE
switch_test.m
function c = switch_test(a)
  c = a;
\\endif
\\verbinclude switch_test.m";
        let d = convert_doc_page(page, "flow").unwrap();
        assert!(d.body.contains("```fm-file:switch_test.m\n"), "{}", d.body);
        assert!(d.body.contains("function c = switch_test(a)"));
    }

    #[test]
    fn ref_crosslinks() {
        let out =
            replace_refs("see \\ref array_size \"the size function\" and \\ref array_zeros here");
        assert_eq!(out, "see [the size function](size) and [zeros](zeros) here");
    }

    #[test]
    fn html_lists() {
        let page = "\
\\page array_x X X.
\\section Usage
Possible values:
<UL>
<LI> <tt>'a'</tt> - the a value
</LI>
<LI> <tt>'b'</tt> - the b value
</LI>
</UL>";
        let d = convert_doc_page(page, "array").unwrap();
        assert!(d.body.contains("- `'a'` - the a value"), "{}", d.body);
        assert!(d.body.contains("- `'b'` - the b value"), "{}", d.body);
        // No double-space after the list marker.
        assert!(!d.body.contains("-  `"), "{}", d.body);
    }

    #[test]
    fn emphasis_commands() {
        let mut u = Vec::new();
        let s = strip_inline("This is \\b bold and \\e italic and \\c code here", &mut u);
        assert!(s.contains("**bold**"), "{s}");
        assert!(s.contains("*italic*"), "{s}");
        assert!(s.contains("`code`"), "{s}");
        assert!(u.is_empty());
    }

    #[test]
    fn unknown_command_recorded() {
        let mut u = Vec::new();
        let s = strip_inline("A \\frobnicate thing", &mut u);
        assert!(s.contains("[[unknown: \\frobnicate]]"), "{s}");
        assert_eq!(u, vec!["\\frobnicate".to_string()]);
    }

    #[test]
    fn raw_str_escalates_hashes() {
        let body = "contains \"# a hash-quote";
        let lit = raw_str(body);
        // Default r#"…"# would be broken by `"#`; must escalate to r##"…"##.
        assert!(lit.starts_with("r##\""), "{lit}");
        assert!(lit.ends_with("\"##"), "{lit}");
    }

    #[test]
    fn module_m_conversion() {
        let m = "\
%@Module BIND Bind Standalone Executable
%@@Section FREEMAT
%@@Usage
%The @|bind| function builds an executable.
%@[
%   bind(output)
%@]
%@<
%bind('hello')
%@>
function bind(output)
  x = 1;
";
        let d = convert_module_m(m).unwrap();
        assert_eq!(d.name, "bind");
        assert_eq!(d.section, "freemat");
        assert!(d.body.contains("## Usage"), "{}", d.body);
        assert!(d.body.contains("`bind`"), "{}", d.body);
        assert!(
            d.body.contains("```text\n  bind(output)\n```"),
            "{}",
            d.body
        );
        assert!(
            d.body.contains("```fm-exec\nbind('hello')\n```"),
            "{}",
            d.body
        );
        assert!(d.has_fragment);
    }
}
