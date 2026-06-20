//! The `help` and `helpwin` builtins (help-system P5, §6.1–6.3 of
//! `docs/HELP_SYSTEM.md`).
//!
//! - `help` (no args) lists the documentation sections.
//! - `help <section-id>` lists the topics in that section.
//! - `help <name>` resolves the topic (§6.1) and prints the terminal text
//!   rendering ([`fm_doc::render::text::render_text`]), followed by a `Browser:`
//!   line linking the `{base}/help/<name>` page — as an OSC 8 clickable link when
//!   stdout is a TTY that supports hyperlinks, else the bare URL. With no server
//!   running it prints a `helpwin` hint instead. `help` never opens a browser.
//! - `helpwin <name>` opens the browser help page (`helpwin` alone opens the
//!   index). It needs the graphics/help server to be running.
//!
//! # Where the server URL comes from
//! The builtins reach the running server's base URL through the interpreter's
//! **graphics sink**: `fm-cli`'s `ServerHandle` implements
//! [`fm_graphics::GraphicsSink::base_url`] (a defaulted trait method, `None` for
//! non-server sinks). So no new interpreter field or `main.rs` change is needed —
//! the same sink the graphics builtins publish through carries the URL. When no
//! sink is installed (library/conformance runs, `--no-gfx`), `base_url()` is
//! `None` and the no-server branches apply.
//!
//! # Output path
//! All output goes through [`Interpreter::emit`] — the same buffer `disp`/value
//! echoes use and that the REPL flushes via `take_output()`. We never write to
//! raw stdout, so `help` output is captured/displayed like any other builtin's.

use std::io::IsTerminal;

use fm_core::Array;
use fm_doc::Registry;
use fm_doc::render::text::{TextOpts, render_text};
use fm_interp::error::Flow;
use fm_interp::{FunctionTable, Interpreter};
use supports_hyperlinks::Stream;

use crate::util::need;

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("help", b_help);
    table.add_builtin("helpwin", b_helpwin);
}

/// The running server's base URL (`http://host:port`), via the interpreter's
/// graphics sink (`None` when no server sink is installed).
fn server_base(i: &Interpreter) -> Option<String> {
    i.graphics.sink.as_ref().and_then(|s| s.base_url())
}

/// First string argument, if `args[0]` is a char-array string.
fn first_string(args: &[Array]) -> Option<String> {
    args.first().and_then(Array::as_string)
}

/// `help` (§6.2). With no argument, list the sections; with a section id, list
/// that section's topics; otherwise resolve the query as a topic name.
fn b_help(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let reg = Registry::global();
    match first_string(args) {
        None => help_sections(i, reg),
        Some(query) => {
            // A section id takes precedence over a topic of the same string only
            // when it actually names a section.
            if reg.section(&query).is_some() {
                help_section_topics(i, reg, &query);
            } else {
                help_topic(i, reg, &query);
            }
        }
    }
    Ok(vec![])
}

/// `help` with no args: list each section's title + summary, then a usage hint.
fn help_sections(i: &mut Interpreter, reg: &Registry) {
    i.emit("FreeMat help — documentation sections:\n\n");
    for s in reg.sections() {
        // Skip empty sections (no registered topics yet).
        if reg.in_section(s.id).next().is_none() {
            continue;
        }
        if s.summary.is_empty() {
            i.emit(&format!("  {}  ({})\n", s.title, s.id));
        } else {
            i.emit(&format!("  {}  ({}) — {}\n", s.title, s.id, s.summary));
        }
    }
    i.emit("\nRun `help <section-id>` to list a section's topics, or ");
    i.emit("`help <name>` for one topic.\n");
}

/// `help <section-id>`: list the topics in the section (name + summary).
fn help_section_topics(i: &mut Interpreter, reg: &Registry, id: &str) {
    let Some(section) = reg.section(id) else {
        // Defensive: only reached if called with a non-section.
        help_topic(i, reg, id);
        return;
    };
    i.emit(&format!("{} ({}) — topics:\n\n", section.title, section.id));
    let mut any = false;
    for e in reg.in_section(id) {
        any = true;
        i.emit(&format!("  {:<16} {}\n", e.name, e.summary));
    }
    if !any {
        i.emit("  (no topics registered in this section yet)\n");
    }
    i.emit("\nRun `help <name>` for one topic.\n");
}

/// `help <name>`: resolve and render one topic, or a "did you mean" / not-found.
fn help_topic(i: &mut Interpreter, reg: &Registry, query: &str) {
    use fm_doc::Resolution;
    match reg.resolve(query) {
        Resolution::Exact(entry) => {
            let base = server_base(i);
            // Terminal text rendering. The renderer is pure; we feed it the
            // runtime policy: ANSI + OSC 8 cross-links only on a hyperlink-capable
            // TTY with a server URL.
            let tty = std::io::stdout().is_terminal();
            let hyperlinks = base.is_some() && tty && supports_hyperlinks::on(Stream::Stdout);
            let opts = TextOpts {
                width: term_width(),
                ansi: tty,
                hyperlink_base: hyperlinks.then(|| base.clone()).flatten(),
            };
            i.emit(&render_text(entry, reg, &opts));

            // Final browser line.
            match &base {
                Some(base) => {
                    let url = format!("{base}/help/{}", entry.name);
                    if hyperlinks {
                        i.emit(&format!(
                            "\nBrowser: {}\n",
                            osc8(&url, &format!("open {} ↗", entry.name))
                        ));
                    } else {
                        i.emit(&format!("\nBrowser: {url}\n"));
                    }
                }
                None => {
                    i.emit(&format!(
                        "\n(run `helpwin {}` for the rich browser page)\n",
                        entry.name
                    ));
                }
            }
        }
        Resolution::Suggestions(names) => {
            i.emit(&format!("No help found for `{query}`. Did you mean:\n"));
            for n in names {
                i.emit(&format!("  {n}\n"));
            }
        }
        Resolution::None => {
            i.emit(&format!(
                "No help found for `{query}`. Run `help` to list sections.\n"
            ));
        }
    }
}

/// `helpwin` (§6.3): open the browser help page (`helpwin <name>` opens the
/// topic page, `helpwin` alone opens the index). Requires a running server.
fn b_helpwin(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 0, "helpwin")?;
    let reg = Registry::global();
    let Some(base) = server_base(i) else {
        i.emit("helpwin: graphics/help server is not running (try without --no-gfx).\n");
        return Ok(vec![]);
    };

    let url = match first_string(args) {
        None => format!("{base}/help"),
        Some(query) => {
            use fm_doc::Resolution;
            match reg.resolve(&query) {
                Resolution::Exact(entry) => format!("{base}/help/{}", entry.name),
                Resolution::Suggestions(names) => {
                    i.emit(&format!("No help found for `{query}`. Did you mean:\n"));
                    for n in names {
                        i.emit(&format!("  {n}\n"));
                    }
                    return Ok(vec![]);
                }
                Resolution::None => {
                    i.emit(&format!("No help found for `{query}`.\n"));
                    return Ok(vec![]);
                }
            }
        }
    };

    if let Err(e) = webbrowser::open(&url) {
        i.emit(&format!(
            "helpwin: could not open browser ({e}); URL: {url}\n"
        ));
    }
    Ok(vec![])
}

/// Wrap an OSC 8 hyperlink around `label` (zero display width escapes, §0 #6).
fn osc8(url: &str, label: &str) -> String {
    format!("\x1b]8;;{url}\x1b\\{label}\x1b]8;;\x1b\\")
}

/// The terminal column width, from `$COLUMNS` if set, else 80. (No extra crate:
/// the renderer's fallback is 80 anyway.)
fn term_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|c| c.trim().parse::<usize>().ok())
        .filter(|&w| w > 0)
        .unwrap_or(80)
}

#[cfg(test)]
mod tests {
    use fm_interp::Interpreter;

    /// Build an interpreter with the standard library registered, then run a
    /// `help`/`helpwin` line and return its captured output.
    fn run(line: &str) -> String {
        let mut interp = Interpreter::new();
        crate::register_standard_library(&mut interp);
        interp.run(line).expect("help should not error");
        interp.take_output()
    }

    #[test]
    fn help_no_args_lists_sections() {
        let out = run("help");
        assert!(
            out.contains("Mathematical Functions"),
            "lists mathfunctions section: {out:?}"
        );
        assert!(out.contains("help <section-id>"), "usage hint: {out:?}");
    }

    #[test]
    fn help_section_lists_topics() {
        let out = run("help mathfunctions");
        assert!(out.contains("cos"), "lists cos topic: {out:?}");
        assert!(
            out.contains("Trigonometric cosine"),
            "topic summary: {out:?}"
        );
    }

    #[test]
    fn help_topic_renders_text_and_transcript() {
        let out = run("help cos");
        assert!(out.contains("COS — Trigonometric cosine"), "title: {out:?}");
        assert!(out.contains("--> cos(0)"), "captured transcript: {out:?}");
    }

    #[test]
    fn help_topic_no_server_shows_helpwin_hint() {
        // No graphics sink installed in this test interpreter → no base URL.
        let out = run("help cos");
        assert!(
            out.contains("helpwin cos"),
            "no-server browser hint: {out:?}"
        );
    }

    #[test]
    fn help_typo_suggests_did_you_mean() {
        let out = run("help coss");
        assert!(out.contains("Did you mean"), "did-you-mean header: {out:?}");
        assert!(out.contains("cos"), "suggests cos: {out:?}");
    }

    #[test]
    fn help_unknown_not_found() {
        let out = run("help zzznotathing");
        assert!(out.contains("No help found"), "not-found message: {out:?}");
    }

    #[test]
    fn helpwin_no_server_reports_disabled() {
        let out = run("helpwin cos");
        assert!(
            out.contains("server is not running"),
            "helpwin disabled message: {out:?}"
        );
    }
}
