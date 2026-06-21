//! Markdown-dialect → HTML rendering (§6.5).
//!
//! [`render_page`] turns one [`DocEntry`]'s `body_md` into an HTML body
//! fragment; [`render_index`] turns the [`Registry`] into the section/topic
//! index. Both return only the page *body* (no `<html>`/`<head>`); the server
//! wraps them in the shared shell that pulls in KaTeX / highlight.js / Plotly.
//!
//! ## How the dialect is handled
//! We drive `pulldown_cmark` over `body_md`, then post-process its event stream
//! before serializing with `pulldown_cmark::html::push_html`:
//! - Fenced blocks keep their info string as a `language-…` class (highlight.js
//!   styles `language-fm`, `language-text`, `language-fm-exec`, …). The
//!   `# errors:` line is *not* stripped here because we re-tokenize the raw body
//!   via [`crate::parse_body`] only to decide where the transcript goes; the
//!   displayed source is the verbatim block text.
//! - The captured transcript for the entry's fragment script (looked up via
//!   [`crate::fragment_by_hash`] on the script's `content_hash`) is emitted in a
//!   `<pre class="fm-transcript">` immediately after the entry's **last**
//!   `fm-exec`/`fm-exec:figure` block (the known v1 single-combined-transcript
//!   limitation, §6.5).
//! - For a `fm-exec:figure` whose captured fragment carries a Plotly scene, a
//!   `<div class="fm-figure" data-plotly='…'>` is emitted; a small client script
//!   (in the server shell) calls `Plotly.newPlot` on it, mirroring
//!   `web/index.html`.
//! - `[text](name)` cross-links to known topics become `<a href="/help/name">`.
//!   Unknown targets fall back to plain text.
//! - `$…$` / `$$…$$` math is left as literal delimiters for KaTeX auto-render.

use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd};

use crate::model::DocEntry;
use crate::parser::{BlockKind, fragment_script_from_body};
use crate::registry::Registry;

/// Render one topic page to an HTML **body** fragment (no `<head>`/shell).
///
/// Layout: an `<h1>` title (`NAME — summary`), then the dialect-rendered
/// `body_md`. The captured `fm-exec` transcript and any `fm-exec:figure` Plotly
/// scene are joined in from the fragment DB (§6.5).
#[must_use]
pub fn render_page(entry: &DocEntry, registry: &Registry) -> String {
    let mut out = String::new();

    out.push_str("<article class=\"fm-help-page\">\n");
    out.push_str("<h1 class=\"fm-title\">");
    push_escaped(&mut out, &entry.name.to_uppercase());
    out.push_str(" <span class=\"fm-summary\">— ");
    push_escaped(&mut out, entry.summary);
    out.push_str("</span></h1>\n");

    out.push_str(&render_body(entry.body_md, registry));

    out.push_str("</article>\n");
    out
}

/// Render the doc-body markdown to HTML, applying the dialect rules.
fn render_body(body_md: &str, registry: &Registry) -> String {
    // Look up the captured fragment (transcript + figure) for this body, if any.
    let script = fragment_script_from_body(body_md);
    let fragment = script
        .as_ref()
        .map(crate::FragmentScript::content_hash)
        .and_then(|h| crate::fragment_by_hash(&h));
    // The clean, pasteable example script (input lines only — no `--> ` prompts
    // and no captured output) for the transcript's "Copy" button.
    let copy_script: Option<String> = script
        .as_ref()
        .map(|s| s.inputs.join("\n").trim_end().to_string());

    // Count executable blocks so we can attach the (single, combined) transcript
    // after the *last* one (§6.5 v1 limitation).
    let total_exec = count_exec_blocks(body_md);

    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let parser = Parser::new_ext(body_md, opts);

    // Transformed event stream. We intercept code-block starts (to record the
    // info string), code-block ends (to inject the transcript/figure after the
    // final exec block), and links (to rewrite cross-link destinations).
    let mut events: Vec<Event<'_>> = Vec::new();
    let mut cur_info: Option<String> = None;
    let mut exec_seen = 0usize;
    // While inside an executed (`fm-exec`/`:figure`) block, suppress the source
    // fence and its text — the captured transcript (injected after the block)
    // already echoes the input lines, so rendering the source too would show the
    // example code twice.
    let mut suppress_code = false;

    // Link rewriting needs to look ahead for the link text only to decide the
    // fallback; pulldown gives us Start(Link)/…text…/End(Link). We rewrite the
    // destination on Start and, for unknown targets, strip the anchor by
    // dropping the Start/End and keeping the inner text. Track that with a depth
    // stack of "is this link being kept as an anchor".
    let mut link_kept: Vec<bool> = Vec::new();

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref info))) => {
                cur_info = Some(info.to_string());
                if fragment.is_some()
                    && matches!(
                        classify_info(info),
                        BlockKind::FmExec { .. } | BlockKind::FmExecFigure { .. }
                    )
                {
                    // Executed block with a captured transcript: don't emit the
                    // source fence — the transcript (injected after the block)
                    // already echoes the input, and a "Copy" button on it provides
                    // the script. (If there's no captured fragment, fall through
                    // and keep the source so the example isn't blank.)
                    suppress_code = true;
                } else {
                    // Re-emit a fenced code block whose info string is the dialect
                    // language so highlight.js can style it. `language-` prefix is
                    // added by pulldown's html serializer from the info string.
                    let lang = code_language(info);
                    events.push(Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(
                        CowStr::from(lang),
                    ))));
                }
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Indented)) => {
                cur_info = Some(String::new());
                events.push(Event::Start(Tag::CodeBlock(CodeBlockKind::Indented)));
            }
            Event::Text(_) if suppress_code => {
                // Drop the source text of an executed block (shown via transcript).
            }
            Event::End(TagEnd::CodeBlock) => {
                if !suppress_code {
                    events.push(Event::End(TagEnd::CodeBlock));
                }
                suppress_code = false;
                if let Some(info) = cur_info.take() {
                    let kind = classify_info(&info);
                    if matches!(
                        kind,
                        BlockKind::FmExec { .. } | BlockKind::FmExecFigure { .. }
                    ) {
                        exec_seen += 1;
                        // Attach the combined transcript/figure after the final
                        // executable block.
                        if exec_seen == total_exec
                            && let Some(frag) = fragment
                        {
                            if !frag.transcript.is_empty() {
                                events.push(Event::Html(CowStr::from(render_transcript(
                                    frag.transcript,
                                    copy_script.as_deref(),
                                ))));
                            }
                            if let Some(fig) = frag.figure {
                                events.push(Event::Html(CowStr::from(render_figure(fig))));
                            }
                        }
                    }
                }
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => {
                if let Some(name) = resolve_cross_link(registry, &dest_url) {
                    link_kept.push(true);
                    events.push(Event::Start(Tag::Link {
                        link_type,
                        dest_url: CowStr::from(format!("/help/{name}")),
                        title,
                        id,
                    }));
                } else if is_topic_target(&dest_url) {
                    // Unknown bare-name target: drop the anchor, keep the text.
                    link_kept.push(false);
                } else {
                    // External / anchor link: pass through unchanged.
                    link_kept.push(true);
                    events.push(Event::Start(Tag::Link {
                        link_type,
                        dest_url,
                        title,
                        id,
                    }));
                }
            }
            Event::End(TagEnd::Link) => {
                if link_kept.pop().unwrap_or(true) {
                    events.push(Event::End(TagEnd::Link));
                }
            }
            other => events.push(other),
        }
    }

    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, events.into_iter());
    html
}

/// Render the section/topic index to an HTML **body** fragment.
///
/// Each section shows its title + summary and lists its topics as
/// `<a href="/help/<name>">name</a> — summary`. A search box (an `<input>` plus
/// a little client JS hitting `/help/search?q=`) sits at the top.
#[must_use]
pub fn render_index(registry: &Registry) -> String {
    let mut out = String::new();
    out.push_str("<article class=\"fm-help-index\">\n");
    out.push_str("<h1>FreeMat Help</h1>\n");

    // Search box + results container; the shell ships the search JS.
    out.push_str(
        "<div class=\"fm-search\">\n  \
         <input id=\"fm-search-input\" type=\"search\" \
         placeholder=\"Search help topics…\" autocomplete=\"off\" />\n  \
         <ul id=\"fm-search-results\" class=\"fm-search-results\"></ul>\n</div>\n",
    );

    for section in registry.sections() {
        let topics: Vec<&DocEntry> = registry.in_section(section.id).collect();
        if topics.is_empty() {
            continue;
        }
        out.push_str("<section class=\"fm-section\">\n  <h2>");
        push_escaped(&mut out, section.title);
        out.push_str("</h2>\n");
        if !section.summary.is_empty() {
            out.push_str("  <p class=\"fm-section-summary\">");
            push_escaped(&mut out, section.summary);
            out.push_str("</p>\n");
        }
        out.push_str("  <ul class=\"fm-topic-list\">\n");
        for e in topics {
            out.push_str("    <li><a href=\"/help/");
            push_escaped(&mut out, e.name);
            out.push_str("\">");
            push_escaped(&mut out, e.name);
            out.push_str("</a> — ");
            push_escaped(&mut out, e.summary);
            out.push_str("</li>\n");
        }
        out.push_str("  </ul>\n</section>\n");
    }

    out.push_str("</article>\n");
    out
}

/// Number of `fm-exec`/`fm-exec:figure` blocks in `body_md`.
fn count_exec_blocks(body_md: &str) -> usize {
    crate::parse_body(body_md)
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

/// The `language-…`-friendly class name for a fence info string. The dialect
/// info strings (`fm`, `text`, `fm-exec`, `fm-exec:figure`, `fm-file:NAME`) are
/// normalized so highlight.js sees a stable class; unknown infos pass through.
fn code_language(info: &str) -> String {
    let info = info.trim();
    match classify_info(info) {
        BlockKind::Text => "text".to_string(),
        BlockKind::Fm => "fm".to_string(),
        BlockKind::FmExec { .. } => "fm-exec".to_string(),
        BlockKind::FmExecFigure { .. } => "fm-exec".to_string(),
        BlockKind::FmFile { .. } => "fm".to_string(),
        BlockKind::Other { info } => info,
    }
}

/// Classify a fence info string into a [`BlockKind`] (mirrors the parser's
/// classification, without needing the block body).
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

/// Render a captured transcript into an `<pre class="fm-transcript">` block,
/// wrapped in a `.fm-example` container with a "Copy" button. The transcript
/// text is HTML-escaped (plain REPL output, not markup). `copy_script`, if
/// present, is the clean pasteable script (input lines only) embedded in the
/// button's `data-copy` attribute for the shell's clipboard handler.
fn render_transcript(transcript: &str, copy_script: Option<&str>) -> String {
    let mut s = String::from("<div class=\"fm-example\">");
    if let Some(script) = copy_script.filter(|c| !c.is_empty()) {
        // Single-quote-delimited attribute (FreeMat scripts use single quotes,
        // which `push_attr_escaped` turns into `&#39;`); newlines are encoded as
        // `&#10;` so the multi-line script survives as one attribute value.
        s.push_str("<button class=\"fm-copy\" type=\"button\" data-copy='");
        for (i, line) in script.split('\n').enumerate() {
            if i > 0 {
                s.push_str("&#10;");
            }
            push_attr_escaped(&mut s, line);
        }
        s.push_str("'>Copy</button>");
    }
    s.push_str("<pre class=\"fm-transcript\"><code>");
    push_escaped(&mut s, transcript);
    s.push_str("</code></pre></div>\n");
    s
}

/// Render a captured Plotly scene into a `<div class="fm-figure">` carrying the
/// JSON in a `data-plotly` attribute (single-quote delimited, attribute-escaped)
/// for the shell's client script to plot via `Plotly.newPlot`.
fn render_figure(scene_json: &str) -> String {
    let mut s = String::from("<div class=\"fm-figure\" data-plotly='");
    push_attr_escaped(&mut s, scene_json);
    s.push_str("'></div>\n");
    s
}

/// If `dest` is a bare topic name/alias that the registry resolves to a known
/// entry, return that entry's canonical `name` (so the link points at the
/// canonical `/help/<name>`). Otherwise `None`.
fn resolve_cross_link<'a>(registry: &'a Registry, dest: &str) -> Option<&'a str> {
    if !is_topic_target(dest) {
        return None;
    }
    registry.get(dest).map(|e| e.name)
}

/// Whether a link destination is a bare topic name (no scheme, no path, no
/// fragment) — the cross-link form `[text](name)`.
fn is_topic_target(dest: &str) -> bool {
    !dest.is_empty() && !dest.contains(':') && !dest.contains('/') && !dest.starts_with('#')
}

/// Append `text` to `out` with HTML text-content escaping.
fn push_escaped(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
}

/// Append `text` to `out` with escaping suitable for a single-quote-delimited
/// HTML attribute value (escapes `&`, `<`, `>`, and `'`).
fn push_attr_escaped(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DocEntry, SectionEntry, SourceKind};

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
    fn title_and_summary_rendered() {
        let e = entry("cos", "## Usage\nComputes `cos(x)`.\n");
        let reg = registry_with(vec![e]);
        let html = render_page(&e, &reg);
        assert!(html.contains("COS"), "uppercased name in title: {html}");
        assert!(html.contains("— A summary."), "summary in title: {html}");
        assert!(html.contains("<h2>Usage</h2>"), "heading rendered: {html}");
        assert!(html.contains("<code>cos(x)</code>"), "inline code: {html}");
    }

    #[test]
    fn fm_and_text_blocks_get_language_classes() {
        let body = "```fm\ny = cos(x)\n```\n\n```text\nverbatim\n```\n";
        let e = entry("cos", body);
        let reg = registry_with(vec![e]);
        let html = render_page(&e, &reg);
        assert!(
            html.contains("class=\"language-fm\""),
            "fm class present: {html}"
        );
        assert!(
            html.contains("class=\"language-text\""),
            "text class present: {html}"
        );
    }

    #[test]
    fn fm_exec_block_gets_transcript_when_in_db() {
        // The migrated `pi` topic's exact fragment (`pi` then `cos(pi)`) is in
        // the generated DB, so its captured transcript is rendered here.
        let body = "## Example\n```fm-exec\npi\ncos(pi)\n```\n";
        let e = entry("pi", body);
        let reg = registry_with(vec![e]);
        let html = render_page(&e, &reg);
        // The source fence is suppressed for an executed block with a captured
        // transcript (no duplicate code); the transcript stands in for it.
        assert!(
            !html.contains("class=\"language-fm-exec\""),
            "exec source fence should be suppressed: {html}"
        );
        assert!(
            html.contains("fm-transcript"),
            "transcript block present: {html}"
        );
        assert!(
            html.contains("--&gt; pi") && html.contains("ans ="),
            "captured transcript text present (escaped): {html}"
        );
        // A "Copy" button carries the clean script (input only, no `--> ` / output).
        assert!(
            html.contains("class=\"fm-copy\"") && html.contains("data-copy='pi&#10;cos(pi)'"),
            "copy button with clean script present: {html}"
        );
    }

    #[test]
    fn figure_div_emitted_for_figure_block() {
        // Synthesize a fragment-DB-independent check: a body whose figure block
        // has no captured fragment still renders the source; with a real capture
        // it would emit the div. Here we assert the figure *source* class and
        // that, absent a DB hit, no figure div is fabricated.
        let body = "```fm-exec:figure\nplot(1)\n```\n";
        let e = entry("plotme", body);
        let reg = registry_with(vec![e]);
        let html = render_page(&e, &reg);
        assert!(
            html.contains("class=\"language-fm-exec\""),
            "figure source highlighted as fm-exec: {html}"
        );
        // No fragment in the DB for this body → no fabricated figure/transcript.
        assert!(
            !html.contains("fm-figure"),
            "no figure div without a captured scene: {html}"
        );
    }

    #[test]
    fn figure_div_rendered_from_scene() {
        // Direct unit test of the figure renderer (DB-independent).
        let div = render_figure(r#"{"figures":[{"id":1}]}"#);
        assert!(div.contains("class=\"fm-figure\""), "{div}");
        assert!(div.contains("data-plotly='"), "{div}");
        assert!(div.contains("&#39;") || div.contains("figures"), "{div}");
    }

    #[test]
    fn cross_link_to_known_topic_becomes_anchor() {
        let cos = entry("cos", "x");
        let sin = entry("sin", "See [the cosine](cos) for details.\n");
        let reg = registry_with(vec![cos, sin]);
        let html = render_page(&sin, &reg);
        assert!(
            html.contains("<a href=\"/help/cos\">the cosine</a>"),
            "cross-link anchor: {html}"
        );
    }

    #[test]
    fn cross_link_to_unknown_topic_is_plain_text() {
        let e = entry("sin", "See [nope](doesnotexist).\n");
        let reg = registry_with(vec![e]);
        let html = render_page(&e, &reg);
        assert!(!html.contains("href=\"/help/doesnotexist\""), "{html}");
        assert!(!html.contains("doesnotexist"), "no dangling target: {html}");
        assert!(html.contains("nope"), "link text retained: {html}");
    }

    #[test]
    fn math_delimiters_preserved() {
        let e = entry(
            "cos",
            "## Internals\n$$ \\cos x = \\sum_{n} a_n $$\nand inline $x^2$.\n",
        );
        let reg = registry_with(vec![e]);
        let html = render_page(&e, &reg);
        assert!(html.contains("$$"), "display delimiters preserved: {html}");
        assert!(
            html.contains("\\cos x"),
            "TeX backslashes preserved: {html}"
        );
        assert!(
            html.contains("$x^2$"),
            "inline delimiters preserved: {html}"
        );
    }

    #[test]
    fn index_lists_sections_and_topics() {
        let cos = entry("cos", "x");
        let sin = entry("sin", "x");
        let reg = registry_with(vec![cos, sin]);
        let html = render_index(&reg);
        assert!(html.contains("Mathematical Functions"), "{html}");
        assert!(html.contains("Math stuff."), "section summary: {html}");
        assert!(html.contains("<a href=\"/help/cos\">cos</a>"), "{html}");
        assert!(html.contains("<a href=\"/help/sin\">sin</a>"), "{html}");
        assert!(html.contains("fm-search-input"), "search box: {html}");
    }
}
