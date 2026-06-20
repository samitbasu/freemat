//! HTML rendering of doc bodies for the browser help server (§6.5 of
//! `docs/HELP_SYSTEM.md`, phase P4).
//!
//! The renderer is **pure** — it depends only on the [`crate::Registry`] and the
//! generated fragment DB ([`crate::fragment_by_hash`]); it has no knowledge of
//! axum or any HTTP machinery. This keeps it unit-testable (see
//! [`html`]'s tests) and reusable: `fm-cli`'s server wraps the returned body
//! fragments in a shared HTML shell.
//!
//! Two entry points:
//! - [`html::render_page`] — the body HTML for one topic page.
//! - [`html::render_index`] — the body HTML for the section/topic index.
//!
//! Math (`$…$`/`$$…$$`) is left as literal delimiters in the output so the
//! browser's KaTeX auto-render extension can typeset it client-side (locked
//! decision §0 #5). Code blocks carry `language-…` classes for highlight.js.
//! Captured `fm-exec` transcripts and `fm-exec:figure` Plotly scenes are joined
//! in from the fragment DB by content hash (§6.5).

pub mod html;
pub mod text;
