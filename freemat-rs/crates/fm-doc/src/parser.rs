//! Forward markdown-dialect parser (§3 of `docs/HELP_SYSTEM.md`).
//!
//! Takes a doc body (`body_md`) written in the FreeMat help dialect and
//! produces a structured [`ParsedDoc`]: classified fenced blocks, `##` section
//! headings, recognized math spans, and `[text](name)` cross-links. It also
//! extracts the ordered [`FragmentScript`] (the executable part) that P2/P3
//! consume.
//!
//! This parser handles only the **forward** dialect that authors write. The
//! legacy `.doc`/`.m` -> dialect converter is P6 (see [`legacy`]).
//!
//! The CommonMark layer is provided by `pulldown-cmark`; the dialect-specific
//! classification of fence info strings, `# errors:` lines, math spans, and
//! cross-links is layered on top.

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};

/// The shared executable model for a [`crate::DocEntry`] (§4.1). Defined here so
/// it is the single source of truth; P2 (capture, via `fm-cli`) and P3 (docgen,
/// via `xtask`) both consume *this* type.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FragmentScript {
    /// `fm-file` blocks, in declaration order: `(name, contents)`.
    pub files: Vec<(String, String)>,
    /// One entry per `fm-exec` / `fm-exec:figure` block (multi-line), in order.
    pub inputs: Vec<String>,
    /// Sum of `# errors: N` declarations across `fm-exec` blocks.
    pub expect_errors: usize,
    /// `true` if any `fm-exec:figure` block is present.
    pub want_figure: bool,
}

impl FragmentScript {
    /// Content hash keying the generated fragment DB (§4.3): the lowercase hex
    /// `blake3` digest of a canonical, stable serialization of the script.
    ///
    /// Canonicalization uses `serde_json` over the field-ordered struct (serde
    /// preserves declaration order, and `Vec`s preserve element order), so the
    /// same script always hashes identically across runs and machines, giving
    /// stable git diffs and a reliable cache key. `expect_errors` /
    /// `want_figure` participate so a doc edit that only changes the expected
    /// error count or figure flag invalidates the cache.
    #[must_use]
    pub fn content_hash(&self) -> String {
        // serde_json::to_vec on this plain struct is infallible in practice;
        // fall back to a stable debug rendering if it ever weren't.
        let canon = serde_json::to_vec(self).unwrap_or_else(|_| format!("{self:?}").into_bytes());
        blake3::hash(&canon).to_hex().to_string()
    }
}

/// How a fenced code block is classified by its info string (§3.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockKind {
    /// ```` ```text ```` — verbatim, rendered as-is, not executed.
    Text,
    /// ```` ```fm ```` — FreeMat source for display only (highlighted).
    Fm,
    /// ```` ```fm-exec ```` — executed; `expect_errors` from `# errors: N`.
    FmExec {
        /// Declared expected error count (`# errors:` line, default 0).
        expect_errors: usize,
    },
    /// ```` ```fm-exec:figure ```` — executed and the resulting figure captured.
    FmExecFigure {
        /// Declared expected error count (`# errors:` line, default 0).
        expect_errors: usize,
    },
    /// ```` ```fm-file:NAME ```` — declares an auxiliary file `NAME`.
    FmFile {
        /// The declared file name (e.g. `"hello.m"`).
        name: String,
    },
    /// A fenced block with some other (or no) info string — passed through.
    Other {
        /// The raw info string (may be empty).
        info: String,
    },
}

/// A classified fenced code block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlock {
    /// The block's classification.
    pub kind: BlockKind,
    /// Block contents. For `fm-exec`/`fm-exec:figure` the leading
    /// `# errors:` line is **stripped** (its value lives in `kind`).
    pub body: String,
}

/// A `## Heading` (level-2) found in the body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    /// The heading text, e.g. `"Usage"`.
    pub text: String,
}

/// A `[text](name)` cross-link to another topic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossLink {
    /// The visible link text.
    pub text: String,
    /// The target topic `name`/alias.
    pub target: String,
}

/// A recognized math span (`$…$` inline or `$$…$$` display).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MathSpan {
    /// `true` for `$$…$$` display math, `false` for `$…$` inline math.
    pub display: bool,
    /// The inner TeX (delimiters stripped).
    pub tex: String,
}

/// The structured view of a parsed doc body.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedDoc {
    /// `##` headings in document order.
    pub headings: Vec<Heading>,
    /// Fenced code blocks in document order.
    pub blocks: Vec<CodeBlock>,
    /// Cross-links in document order.
    pub links: Vec<CrossLink>,
    /// Math spans in document order.
    pub math: Vec<MathSpan>,
}

/// Parse `body` (the §3 dialect) into a [`ParsedDoc`].
#[must_use]
pub fn parse_body(body: &str) -> ParsedDoc {
    let mut doc = ParsedDoc::default();

    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let parser = Parser::new_ext(body, opts);

    // Heading accumulation state.
    let mut in_h2 = false;
    let mut h2_text = String::new();

    // Code-block accumulation state.
    let mut cur_info: Option<String> = None;
    let mut cur_body = String::new();

    // Cross-link accumulation state.
    let mut link_stack: Vec<(String, String)> = Vec::new(); // (target, text-acc)

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                if level == pulldown_cmark::HeadingLevel::H2 {
                    in_h2 = true;
                    h2_text.clear();
                }
            }
            Event::End(TagEnd::Heading(level)) => {
                if level == pulldown_cmark::HeadingLevel::H2 && in_h2 {
                    doc.headings.push(Heading {
                        text: h2_text.trim().to_string(),
                    });
                    in_h2 = false;
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                cur_info = Some(match kind {
                    CodeBlockKind::Fenced(info) => info.into_string(),
                    CodeBlockKind::Indented => String::new(),
                });
                cur_body.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(info) = cur_info.take() {
                    doc.blocks.push(classify_block(&info, &cur_body));
                }
                cur_body.clear();
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                link_stack.push((dest_url.into_string(), String::new()));
            }
            Event::End(TagEnd::Link) => {
                if let Some((target, text)) = link_stack.pop() {
                    // Only treat scheme-less, fragment-less destinations as
                    // topic cross-links (not http(s):// or anchors).
                    if is_topic_target(&target) {
                        doc.links.push(CrossLink { text, target });
                    }
                }
            }
            Event::Text(t) | Event::Code(t) => {
                if cur_info.is_some() {
                    cur_body.push_str(&t);
                } else {
                    if let Some((_, text)) = link_stack.last_mut() {
                        text.push_str(&t);
                    }
                    if in_h2 {
                        h2_text.push_str(&t);
                    }
                }
            }
            _ => {}
        }
    }

    // Math is recognized from the *raw* source rather than from
    // pulldown-cmark's text events: CommonMark mangles the backslash escapes
    // (`\,`, `\int`, …) that TeX relies on. We mask code regions first so a `$`
    // inside a ```fenced``` block or an inline `code` span is not mistaken for
    // math, then scan the surviving text.
    scan_math(&mask_code_regions(body), &mut doc.math);

    doc
}

/// Replace the bytes of fenced code blocks and inline-code spans with spaces so
/// a later raw scan ignores them, while preserving byte offsets and newlines.
fn mask_code_regions(body: &str) -> String {
    let mut out: Vec<u8> = body.bytes().collect();
    let bytes = body.as_bytes();
    let n = bytes.len();

    // Mask fenced blocks: lines from a ``` (or ~~~) fence to the matching close.
    let mut i = 0;
    let mut in_fence = false;
    let mut fence_char = b'`';
    while i < n {
        let line_start = i;
        // Find end of line.
        let mut j = i;
        while j < n && bytes[j] != b'\n' {
            j += 1;
        }
        let line = &body[line_start..j];
        let trimmed = line.trim_start();
        let is_fence = trimmed.starts_with("```") || trimmed.starts_with("~~~");
        if !in_fence {
            if is_fence {
                in_fence = true;
                fence_char = trimmed.as_bytes()[0];
            }
        } else {
            let closes = (fence_char == b'`' && trimmed.starts_with("```"))
                || (fence_char == b'~' && trimmed.starts_with("~~~"));
            // Mask the entire content line (and fence lines) except newlines.
            for b in &mut out[line_start..j] {
                if *b != b'\n' {
                    *b = b' ';
                }
            }
            if closes {
                in_fence = false;
            }
            i = j + 1;
            continue;
        }
        if in_fence {
            // Also blank out the opening fence line itself.
            for b in &mut out[line_start..j] {
                if *b != b'\n' {
                    *b = b' ';
                }
            }
        }
        i = j + 1;
    }

    // Mask inline code spans (`code`, ``co`de``, …) in the non-fenced regions.
    // Work over the already fence-masked buffer.
    let buf = out.clone();
    let mut k = 0;
    while k < n {
        if buf[k] == b'`' {
            // Count run of backticks (fence opener length).
            let mut run = 0;
            while k + run < n && buf[k + run] == b'`' {
                run += 1;
            }
            // Find a closing run of the same length.
            let mut m = k + run;
            let close = loop {
                if m >= n {
                    break None;
                }
                if buf[m] == b'`' {
                    let mut r2 = 0;
                    while m + r2 < n && buf[m + r2] == b'`' {
                        r2 += 1;
                    }
                    if r2 == run {
                        break Some(m);
                    }
                    m += r2;
                } else {
                    m += 1;
                }
            };
            if let Some(close) = close {
                for b in &mut out[k..close + run] {
                    if *b != b'\n' {
                        *b = b' ';
                    }
                }
                k = close + run;
                continue;
            }
        }
        k += 1;
    }

    String::from_utf8(out).unwrap_or_else(|_| body.to_string())
}

/// Build the [`FragmentScript`] for an entry's body, or `None` if it contains
/// no executable (`fm-exec`/`fm-exec:figure`) blocks and no `fm-file` blocks.
#[must_use]
pub fn fragment_script(entry: &crate::DocEntry) -> Option<FragmentScript> {
    fragment_script_from_body(entry.body_md)
}

/// Build a [`FragmentScript`] directly from a body string (testable without a
/// full [`crate::DocEntry`]).
#[must_use]
pub fn fragment_script_from_body(body: &str) -> Option<FragmentScript> {
    let parsed = parse_body(body);
    let mut script = FragmentScript::default();
    for block in &parsed.blocks {
        match &block.kind {
            BlockKind::FmFile { name } => {
                script.files.push((name.clone(), block.body.clone()));
            }
            BlockKind::FmExec { expect_errors } => {
                script.inputs.push(block.body.clone());
                script.expect_errors += expect_errors;
            }
            BlockKind::FmExecFigure { expect_errors } => {
                script.inputs.push(block.body.clone());
                script.expect_errors += expect_errors;
                script.want_figure = true;
            }
            BlockKind::Text | BlockKind::Fm | BlockKind::Other { .. } => {}
        }
    }
    if script.files.is_empty() && script.inputs.is_empty() {
        None
    } else {
        Some(script)
    }
}

/// Classify a fenced block by its info string and strip the optional
/// `# errors: N` first line from executable blocks.
fn classify_block(info: &str, raw_body: &str) -> CodeBlock {
    let info = info.trim();
    match info {
        "text" => CodeBlock {
            kind: BlockKind::Text,
            body: raw_body.to_string(),
        },
        "fm" => CodeBlock {
            kind: BlockKind::Fm,
            body: raw_body.to_string(),
        },
        "fm-exec" => {
            let (errors, body) = split_errors_line(raw_body);
            CodeBlock {
                kind: BlockKind::FmExec {
                    expect_errors: errors,
                },
                body,
            }
        }
        "fm-exec:figure" => {
            let (errors, body) = split_errors_line(raw_body);
            CodeBlock {
                kind: BlockKind::FmExecFigure {
                    expect_errors: errors,
                },
                body,
            }
        }
        other => {
            if let Some(name) = other.strip_prefix("fm-file:") {
                CodeBlock {
                    kind: BlockKind::FmFile {
                        name: name.trim().to_string(),
                    },
                    body: raw_body.to_string(),
                }
            } else {
                CodeBlock {
                    kind: BlockKind::Other {
                        info: other.to_string(),
                    },
                    body: raw_body.to_string(),
                }
            }
        }
    }
}

/// Parse an optional leading `# errors: N` line. Returns `(N, remaining body)`;
/// defaults to `0` and the unchanged body if the line is absent.
fn split_errors_line(body: &str) -> (usize, String) {
    if let Some(first) = body.lines().next() {
        let trimmed = first.trim();
        if let Some(rest) = trimmed.strip_prefix('#').map(str::trim)
            && let Some(num) = rest.strip_prefix("errors:")
            && let Ok(n) = num.trim().parse::<usize>()
        {
            // Drop the first line; keep the remainder verbatim.
            let remaining: String = body.split_inclusive('\n').skip(1).collect();
            return (n, remaining);
        }
    }
    (0, body.to_string())
}

/// Whether a link destination should be treated as a topic cross-link: no URL
/// scheme, no `/` path, no `#` fragment, non-empty.
fn is_topic_target(dest: &str) -> bool {
    !dest.is_empty() && !dest.contains(':') && !dest.contains('/') && !dest.starts_with('#')
}

/// Scan a text run for `$$…$$` (display) and `$…$` (inline) math spans and push
/// them to `out`. Display spans are matched first so they are not mistaken for
/// two empty inline spans.
fn scan_math(text: &str, out: &mut Vec<MathSpan>) {
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        if bytes[i] == b'$' {
            // Display math: $$ ... $$
            if i + 1 < n
                && bytes[i + 1] == b'$'
                && let Some(end) = find(text, "$$", i + 2)
            {
                out.push(MathSpan {
                    display: true,
                    tex: text[i + 2..end].trim().to_string(),
                });
                i = end + 2;
                continue;
            }
            // Inline math: $ ... $ (non-empty; empty `$$` handled above).
            if let Some(end) = find(text, "$", i + 1)
                && end > i + 1
            {
                out.push(MathSpan {
                    display: false,
                    tex: text[i + 1..end].trim().to_string(),
                });
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }
}

/// Byte index of the next occurrence of `needle` in `hay` at or after `from`.
fn find(hay: &str, needle: &str, from: usize) -> Option<usize> {
    hay.get(from..)
        .and_then(|s| s.find(needle))
        .map(|p| p + from)
}

/// Placeholder for the legacy `.doc`/`.m` -> dialect converter.
///
/// **Not implemented in P1.** The forward parser above handles the dialect that
/// authors write; the legacy converter (parsing Doxygen `.doc` pages and the
/// remaining inline `@Module`/`%!` `.m` blocks into this dialect) is P6. This
/// stub documents that boundary so callers do not expect it here.
pub mod legacy {}
