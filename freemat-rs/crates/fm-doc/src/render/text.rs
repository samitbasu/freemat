//! Markdown-dialect → terminal text rendering (§6.4).
//!
//! [`render_text`] turns one [`DocEntry`]'s `body_md` into the concise plain-text
//! page the `help <name>` builtin prints (the `.mdc` equivalent of the original
//! FreeMat). Like the HTML renderer ([`super::html`]) it is **pure** — it depends
//! only on the [`Registry`] and the generated fragment DB
//! ([`crate::fragment_by_hash`]); it has no env/TTY detection (the `help` builtin
//! decides the [`TextOpts`]). This keeps it unit-testable and keeps the policy
//! (TTY? OSC 8 support?) out of the renderer.
//!
//! ## How the dialect maps to text
//! We drive `pulldown_cmark` over `body_md` and fold its event stream into a
//! plain-text buffer:
//! - Title line `NAME — summary` (uppercase name), then a blank line.
//! - `## Heading` → the heading text, underlined with `-`.
//! - inline `` `code` `` / `**bold**` / `*italic*` → the bare text (or wrapped in
//!   ANSI SGR when `opts.ansi`).
//! - `text` / `fm` fenced blocks → indented 2 spaces, verbatim.
//! - `fm-exec` / `fm-exec:figure` blocks → the captured transcript (joined from
//!   the fragment DB by the entry's `fragment_script().content_hash()`), indented
//!   2 spaces; a `:figure` block adds a `[figure: <name>] (see helpwin <name>)`
//!   line.
//! - math `$…$` / `$$…$$` → the inner TeX, delimiters stripped.
//! - cross-links `[text](name)` → an **OSC 8** hyperlink to `{base}/help/<name>`
//!   when `opts.hyperlink_base` is set **and** the target resolves to a known
//!   topic; otherwise `text (see: name)`.
//! - prose paragraphs are width-wrapped to `opts.width` (fallback 80). OSC 8
//!   escapes are zero-width, so wrapping measures the visible label only.

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use crate::model::DocEntry;
use crate::parser::{BlockKind, ParsedDoc, fragment_script_from_body, parse_body};
use crate::registry::Registry;

/// Options controlling [`render_text`]. The `help` builtin populates these from
/// the runtime environment (terminal width, TTY/OSC 8 support, server URL); the
/// renderer itself does no detection.
#[derive(Debug, Clone)]
pub struct TextOpts {
    /// Wrap prose to this column count (fallback 80 if `0`).
    pub width: usize,
    /// Emit ANSI SGR escapes for emphasis/code (only when stdout is a TTY).
    pub ansi: bool,
    /// If `Some(base)`, render cross-links to known topics as OSC 8 hyperlinks to
    /// `{base}/help/<name>`. `None` → the plain `text (see: name)` form.
    pub hyperlink_base: Option<String>,
}

impl Default for TextOpts {
    fn default() -> Self {
        TextOpts {
            width: 80,
            ansi: false,
            hyperlink_base: None,
        }
    }
}

/// Render one topic to the concise terminal text page (§6.4).
#[must_use]
pub fn render_text(entry: &DocEntry, registry: &Registry, opts: &TextOpts) -> String {
    let mut out = String::new();

    // Title: `NAME — summary`, then a blank line.
    out.push_str(&entry.name.to_uppercase());
    out.push_str(" — ");
    out.push_str(entry.summary);
    out.push('\n');
    out.push('\n');

    let body = render_body(entry.body_md, registry, opts);
    // Fill in the figure-line topic name (render_body has no DocEntry, so it
    // emits a placeholder; substitute the real topic name here).
    out.push_str(&body.replace(FIGURE_NAME_PLACEHOLDER, entry.name));

    // Collapse any trailing blank lines to a single newline.
    while out.ends_with("\n\n") {
        out.pop();
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Wrap an OSC 8 hyperlink around `label` pointing at `url` (§0 #6). The escape
/// bytes are zero display width.
pub(crate) fn osc8(url: &str, label: &str) -> String {
    format!("\x1b]8;;{url}\x1b\\{label}\x1b]8;;\x1b\\")
}

const WIDTH_FALLBACK: usize = 80;

/// The render-time state threaded through the markdown event fold.
struct TextRender<'a> {
    registry: &'a Registry,
    opts: &'a TextOpts,
    /// The captured transcript+figure for this body, if present in the DB.
    fragment: Option<&'static crate::Fragment>,
    /// Parsed view (used to count exec blocks and find figure names).
    parsed: ParsedDoc,
    out: String,
}

/// Render the doc-body markdown to plain text, applying the dialect rules.
fn render_body(body_md: &str, registry: &Registry, opts: &TextOpts) -> String {
    let fragment = fragment_script_from_body(body_md)
        .as_ref()
        .map(crate::FragmentScript::content_hash)
        .and_then(|h| crate::fragment_by_hash(&h));

    let mut st = TextRender {
        registry,
        opts,
        fragment,
        parsed: parse_body(body_md),
        out: String::new(),
    };

    let total_exec = count_exec_blocks(&st.parsed);

    let cmark_opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let parser = Parser::new_ext(body_md, cmark_opts);

    // Inline-text accumulation: prose between block boundaries collects into
    // `para`, which is width-wrapped and flushed on paragraph/heading end.
    let mut para = String::new();
    let mut in_heading = false;
    let mut in_code = false;
    let mut code_info: Option<String> = None;
    let mut code_body = String::new();
    let mut exec_seen = 0usize;
    // Cross-link accumulation: (target, label-acc). Inline emphasis inside the
    // label is stripped to bare text.
    let mut link_stack: Vec<(String, String)> = Vec::new();
    let mut list_depth = 0usize;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                flush_para(&mut st, &mut para);
                in_heading = true;
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                let title = std::mem::take(&mut para);
                let title = title.trim();
                if !title.is_empty() {
                    st.out.push_str(title);
                    st.out.push('\n');
                    st.out.push_str(&"-".repeat(visible_width(title)));
                    st.out.push('\n');
                    st.out.push('\n');
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                flush_para(&mut st, &mut para);
                code_info = Some(match kind {
                    CodeBlockKind::Fenced(info) => info.to_string(),
                    CodeBlockKind::Indented => String::new(),
                });
                code_body.clear();
                in_code = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code = false;
                if let Some(info) = code_info.take() {
                    let kind = classify_info(&info);
                    let is_exec = matches!(
                        kind,
                        BlockKind::FmExec { .. } | BlockKind::FmExecFigure { .. }
                    );
                    if is_exec {
                        exec_seen += 1;
                        render_exec_block(&mut st, &kind, exec_seen, total_exec);
                    } else {
                        // text / fm / fm-file / other: show the source verbatim,
                        // indented two spaces.
                        push_indented(&mut st.out, &code_body, 2);
                        st.out.push('\n');
                    }
                }
                code_body.clear();
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                link_stack.push((dest_url.to_string(), String::new()));
            }
            Event::End(TagEnd::Link) => {
                if let Some((target, label)) = link_stack.pop() {
                    let rendered = render_link(&st, &target, &label);
                    push_inline(&mut para, &mut link_stack, &rendered);
                }
            }
            Event::Start(Tag::List(_)) => {
                flush_para(&mut st, &mut para);
                list_depth += 1;
            }
            Event::End(TagEnd::List(_)) => {
                list_depth = list_depth.saturating_sub(1);
                if list_depth == 0 {
                    st.out.push('\n');
                }
            }
            Event::Start(Tag::Item) => {
                flush_para(&mut st, &mut para);
                para.push_str("- ");
            }
            Event::End(TagEnd::Item) => {
                let item = std::mem::take(&mut para);
                let item = item.trim_end();
                if !item.is_empty() {
                    push_indented(&mut st.out, item, 2 * list_depth.saturating_sub(1));
                }
            }
            Event::Start(Tag::Emphasis | Tag::Strong) => {
                if st.opts.ansi {
                    push_inline(&mut para, &mut link_stack, "\x1b[1m");
                }
            }
            Event::End(TagEnd::Emphasis | TagEnd::Strong) => {
                if st.opts.ansi {
                    push_inline(&mut para, &mut link_stack, "\x1b[0m");
                }
            }
            Event::End(TagEnd::Paragraph) => {
                flush_para(&mut st, &mut para);
            }
            Event::Code(t) => {
                let code = if st.opts.ansi {
                    format!("\x1b[2m{t}\x1b[0m")
                } else {
                    t.to_string()
                };
                push_inline(&mut para, &mut link_stack, &code);
            }
            Event::Text(t) => {
                if in_code {
                    code_body.push_str(&t);
                } else if in_heading {
                    para.push_str(&t);
                } else {
                    push_inline(&mut para, &mut link_stack, &t);
                }
            }
            Event::SoftBreak | Event::HardBreak if !in_code && !in_heading => {
                push_inline(&mut para, &mut link_stack, " ");
            }
            _ => {}
        }
    }
    flush_para(&mut st, &mut para);

    // §6.4: math `$…$`/`$$…$$` → inner TeX verbatim, delimiters stripped. The
    // `$`-fenced runs pass through pulldown's text events as plain text (the
    // parser deliberately does not treat `$` specially), so we strip the paired
    // delimiters in a final pass over the assembled output. Display (`$$`) is
    // handled first so it isn't mistaken for two empty inline spans.
    let mut s = std::mem::take(&mut st.out);
    s = replace_math(&s, "$$");
    s = replace_math(&s, "$");
    st.out = s;

    st.out
}

/// Render an `fm-exec` / `fm-exec:figure` block: the captured transcript joined
/// from the DB (after the *last* exec block, per the v1 single-combined model,
/// §6.5), indented two spaces; a figure block adds a `[figure: <name>]` line.
fn render_exec_block(st: &mut TextRender<'_>, kind: &BlockKind, exec_seen: usize, total: usize) {
    // The combined transcript is attached after the final executable block.
    if exec_seen == total
        && let Some(frag) = st.fragment
        && !frag.transcript.is_empty()
    {
        push_indented(&mut st.out, frag.transcript, 2);
        st.out.push('\n');
    }
    if matches!(kind, BlockKind::FmExecFigure { .. }) {
        // `render_body` has no `DocEntry`, so it emits a placeholder for the
        // topic name; `render_text` substitutes the real name (the `helpwin`
        // target). The page's own topic is the most useful target.
        let name = FIGURE_NAME_PLACEHOLDER;
        st.out
            .push_str(&format!("  [figure: {name}] (see helpwin {name})\n\n"));
    }
}

/// Sentinel substituted with the real topic name by [`render_text`].
const FIGURE_NAME_PLACEHOLDER: &str = "\u{0}FIGNAME\u{0}";

/// Render a `[text](name)` cross-link per §6.4.
fn render_link(st: &TextRender<'_>, target: &str, label: &str) -> String {
    let known = is_topic_target(target)
        .then(|| st.registry.get(target))
        .flatten()
        .map(|e| e.name);
    match (known, st.opts.hyperlink_base.as_deref()) {
        (Some(name), Some(base)) => osc8(&format!("{base}/help/{name}"), label),
        (Some(name), None) => format!("{label} (see: {name})"),
        (None, _) => {
            // Unknown target: keep the visible text, no dangling link.
            label.to_string()
        }
    }
}

/// Append inline text either to the current paragraph or, if inside a link, to
/// the link's label accumulator (so emphasis inside a link strips to text).
fn push_inline(para: &mut String, link_stack: &mut [(String, String)], s: &str) {
    if let Some((_, label)) = link_stack.last_mut() {
        label.push_str(s);
    } else {
        para.push_str(s);
    }
}

/// Flush the accumulated prose paragraph: width-wrap and emit, then a blank line.
fn flush_para(st: &mut TextRender<'_>, para: &mut String) {
    let text = std::mem::take(para);
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    let width = if st.opts.width == 0 {
        WIDTH_FALLBACK
    } else {
        st.opts.width
    };
    st.out.push_str(&wrap(text, width));
    st.out.push('\n');
    st.out.push('\n');
}

/// Strip paired `delim`-fenced math runs, keeping the trimmed inner text.
fn replace_math(s: &str, delim: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(open) = rest.find(delim) {
        result.push_str(&rest[..open]);
        let after = &rest[open + delim.len()..];
        if let Some(close) = after.find(delim) {
            let inner = after[..close].trim();
            result.push_str(inner);
            rest = &after[close + delim.len()..];
        } else {
            // Unpaired delimiter: emit verbatim and stop.
            result.push_str(&rest[open..]);
            return result;
        }
    }
    result.push_str(rest);
    result
}

/// Count `fm-exec`/`fm-exec:figure` blocks in a parsed body.
fn count_exec_blocks(parsed: &ParsedDoc) -> usize {
    parsed
        .blocks
        .iter()
        .filter(|b| {
            matches!(
                b.kind,
                BlockKind::FmExec { .. } | BlockKind::FmExecFigure { .. }
            )
        })
        .count()
}

/// Classify a fence info string into a [`BlockKind`] (mirrors the parser).
fn classify_info(info: &str) -> BlockKind {
    let info = info.trim();
    match info {
        "text" => BlockKind::Text,
        "fm" => BlockKind::Fm,
        "fm-exec" => BlockKind::FmExec { expect_errors: 0 },
        "fm-exec:figure" => BlockKind::FmExecFigure { expect_errors: 0 },
        other => {
            if let Some(name) = other.strip_prefix("fm-file:") {
                BlockKind::FmFile {
                    name: name.trim().to_string(),
                }
            } else {
                BlockKind::Other {
                    info: other.to_string(),
                }
            }
        }
    }
}

/// Whether a link destination is a bare topic name (cross-link form).
fn is_topic_target(dest: &str) -> bool {
    !dest.is_empty() && !dest.contains(':') && !dest.contains('/') && !dest.starts_with('#')
}

/// Append `text` to `out`, prefixing each line with `n` spaces. Trailing
/// whitespace per line is trimmed; a single trailing newline is preserved.
fn push_indented(out: &mut String, text: &str, n: usize) {
    let pad = " ".repeat(n);
    for line in text.trim_end_matches('\n').split('\n') {
        let line = line.trim_end();
        if line.is_empty() {
            out.push('\n');
        } else {
            out.push_str(&pad);
            out.push_str(line);
            out.push('\n');
        }
    }
}

/// Greedy word-wrap `text` to `width` visible columns. OSC 8 escapes are
/// zero-width: we measure each word's *visible* width (label only), not its
/// bytes, so a hyperlinked word wraps on its label.
fn wrap(text: &str, width: usize) -> String {
    let mut out = String::new();
    let mut col = 0usize;
    for (i, word) in text.split_whitespace().enumerate() {
        let w = visible_width(word);
        if i == 0 {
            out.push_str(word);
            col = w;
        } else if col + 1 + w > width {
            out.push('\n');
            out.push_str(word);
            col = w;
        } else {
            out.push(' ');
            out.push_str(word);
            col += 1 + w;
        }
    }
    out
}

/// The visible (printed) width of `s`: counts characters, skipping ANSI SGR
/// (`\x1b[…m`) and OSC 8 (`\x1b]8;;…\x1b\\`) escape sequences, which take no
/// display columns.
fn visible_width(s: &str) -> usize {
    let bytes = s.as_bytes();
    let n = bytes.len();
    let mut i = 0usize;
    let mut width = 0usize;
    while i < n {
        if bytes[i] == 0x1b {
            // OSC: ESC ] … (terminated by ESC \ or BEL).
            if i + 1 < n && bytes[i + 1] == b']' {
                i += 2;
                while i < n {
                    if bytes[i] == 0x07 {
                        i += 1;
                        break;
                    }
                    if bytes[i] == 0x1b && i + 1 < n && bytes[i + 1] == b'\\' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
                continue;
            }
            // CSI: ESC [ … final-byte (SGR ends in `m`).
            if i + 1 < n && bytes[i + 1] == b'[' {
                i += 2;
                while i < n && !(0x40..=0x7e).contains(&bytes[i]) {
                    i += 1;
                }
                if i < n {
                    i += 1;
                }
                continue;
            }
            // Lone ST terminator `ESC \`.
            if i + 1 < n && bytes[i + 1] == b'\\' {
                i += 2;
                continue;
            }
            i += 1;
        } else {
            // Advance one UTF-8 scalar; count it as one column.
            let ch_len = utf8_len(bytes[i]);
            i += ch_len.max(1);
            width += 1;
        }
    }
    width
}

/// Byte-length of a UTF-8 scalar from its lead byte.
fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else if b >> 3 == 0b11110 {
        4
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{SectionEntry, SourceKind};

    fn entry(name: &'static str, body: &'static str) -> DocEntry {
        DocEntry {
            name,
            aliases: &[],
            section: "math",
            summary: "A summary.",
            body_md: body,
            source: SourceKind::RustBuiltin {
                krate: "test",
                module: "test",
            },
        }
    }

    fn registry_with(entries: Vec<DocEntry>) -> Registry {
        let section = SectionEntry {
            id: "math",
            title: "Mathematical Functions",
            summary: "Math stuff.",
        };
        Registry::try_build(entries, vec![section]).expect("no dup")
    }

    #[test]
    fn title_uppercased_with_summary() {
        let e = entry("cos", "## Usage\nComputes the cosine.\n");
        let reg = registry_with(vec![e]);
        let out = render_text(&e, &reg, &TextOpts::default());
        assert!(out.starts_with("COS — A summary.\n"), "title: {out:?}");
    }

    #[test]
    fn heading_underlined() {
        let e = entry("cos", "## Usage\nbody text here.\n");
        let reg = registry_with(vec![e]);
        let out = render_text(&e, &reg, &TextOpts::default());
        assert!(
            out.contains("Usage\n-----\n"),
            "underlined heading: {out:?}"
        );
    }

    #[test]
    fn inline_code_and_emphasis_stripped() {
        let e = entry("cos", "Computes `cos(x)` and **bold** and *italic*.\n");
        let reg = registry_with(vec![e]);
        let out = render_text(&e, &reg, &TextOpts::default());
        assert!(out.contains("cos(x)"), "code text kept: {out:?}");
        assert!(out.contains("bold"), "bold text kept: {out:?}");
        assert!(out.contains("italic"), "italic text kept: {out:?}");
        assert!(!out.contains('`'), "backticks stripped: {out:?}");
        assert!(!out.contains('*'), "asterisks stripped: {out:?}");
        assert!(!out.contains('\x1b'), "no ANSI without opts.ansi: {out:?}");
    }

    #[test]
    fn ansi_emphasis_when_enabled() {
        let e = entry("cos", "A `code` and **bold** word.\n");
        let reg = registry_with(vec![e]);
        let opts = TextOpts {
            ansi: true,
            ..TextOpts::default()
        };
        let out = render_text(&e, &reg, &opts);
        assert!(out.contains('\x1b'), "ANSI escapes present: {out:?}");
    }

    #[test]
    fn text_fence_indented() {
        let e = entry("cos", "```text\ny = cos(x)\n```\n");
        let reg = registry_with(vec![e]);
        let out = render_text(&e, &reg, &TextOpts::default());
        assert!(out.contains("  y = cos(x)"), "indented verbatim: {out:?}");
    }

    #[test]
    fn fm_exec_transcript_from_db() {
        // The trig.rs `cos` fixture's exact fragment is in the generated DB.
        let e = entry("cos", "## Example\n```fm-exec\ncos(0)\n```\n");
        let reg = registry_with(vec![e]);
        let out = render_text(&e, &reg, &TextOpts::default());
        assert!(
            out.contains("  --> cos(0)"),
            "transcript input echoed: {out:?}"
        );
        assert!(out.contains("ans ="), "transcript output present: {out:?}");
    }

    #[test]
    fn figure_block_adds_line() {
        // No real fragment for this body; the figure placeholder line still emits.
        let e = entry("plotme", "```fm-exec:figure\nplot(1)\n```\n");
        let reg = registry_with(vec![e]);
        let out = render_text(&e, &reg, &TextOpts::default());
        assert!(
            out.contains("[figure: plotme] (see helpwin plotme)"),
            "figure line names the topic: {out:?}"
        );
    }

    #[test]
    fn math_delimiters_stripped() {
        let e = entry(
            "cos",
            "## Internals\n$$ \\cos x = y $$\nand inline $x^2$ here.\n",
        );
        let reg = registry_with(vec![e]);
        let out = render_text(&e, &reg, &TextOpts::default());
        assert!(!out.contains('$'), "dollar delimiters stripped: {out:?}");
        assert!(out.contains("\\cos x = y"), "display TeX kept: {out:?}");
        assert!(out.contains("x^2"), "inline TeX kept: {out:?}");
    }

    #[test]
    fn cross_link_plain_when_no_base() {
        let cos = entry("cos", "x");
        let sin = entry("sin", "See [the cosine](cos) for details.\n");
        let reg = registry_with(vec![cos, sin]);
        let out = render_text(&sin, &reg, &TextOpts::default());
        assert!(
            out.contains("the cosine (see: cos)"),
            "plain cross-link form: {out:?}"
        );
        assert!(!out.contains('\x1b'), "no OSC 8 without base: {out:?}");
    }

    #[test]
    fn cross_link_osc8_when_base_and_known() {
        let cos = entry("cos", "x");
        let sin = entry("sin", "See [the cosine](cos) here.\n");
        let reg = registry_with(vec![cos, sin]);
        let opts = TextOpts {
            hyperlink_base: Some("http://127.0.0.1:9000".to_string()),
            ..TextOpts::default()
        };
        let out = render_text(&sin, &reg, &opts);
        assert!(
            out.contains("\x1b]8;;http://127.0.0.1:9000/help/cos\x1b\\the cosine\x1b]8;;\x1b\\"),
            "OSC 8 hyperlink emitted: {out:?}"
        );
    }

    #[test]
    fn cross_link_unknown_target_plain_text() {
        let e = entry("sin", "See [nope](doesnotexist) please.\n");
        let reg = registry_with(vec![e]);
        let opts = TextOpts {
            hyperlink_base: Some("http://x".to_string()),
            ..TextOpts::default()
        };
        let out = render_text(&e, &reg, &opts);
        assert!(out.contains("nope"), "label kept: {out:?}");
        assert!(!out.contains("doesnotexist"), "no dangling target: {out:?}");
        assert!(!out.contains('\x1b'), "no link for unknown target: {out:?}");
    }

    #[test]
    fn prose_is_width_wrapped() {
        let body = "one two three four five six seven eight nine ten eleven twelve.\n";
        let e = entry("cos", body);
        let reg = registry_with(vec![e]);
        let opts = TextOpts {
            width: 20,
            ..TextOpts::default()
        };
        let out = render_text(&e, &reg, &opts);
        for line in out.lines() {
            assert!(visible_width(line) <= 20, "line within width: {line:?}");
        }
    }

    #[test]
    fn visible_width_ignores_escapes() {
        let link = osc8("http://x/help/cos", "cos");
        assert_eq!(visible_width(&link), 3, "OSC 8 label width is 3");
        assert_eq!(visible_width("\x1b[1mbold\x1b[0m"), 4, "SGR is zero width");
    }
}
