//! Fragment capture-output model + the generated fragment DB (§4.1, §5).
//!
//! [`CapturedFragment`] is the shared result of running a
//! [`crate::FragmentScript`] through the capture engine (`fm-cli`'s
//! `run_fragment`, driven in-process by `xtask docgen`). It is the canonical
//! capture-output model — it lives here in `fm-doc` (not `fm-cli`) because it is
//! what gets serialized into the generated artifact and because `fm-doc` is the
//! single owner of the fragment data model.
//!
//! [`Fragment`] is the trimmed-down, `&'static` shape actually embedded in the
//! generated `src/generated/fragments.rs` (only the render-relevant fields:
//! transcript + optional figure JSON). The `FRAGMENTS` table maps a
//! [`FragmentScript::content_hash`](crate::FragmentScript::content_hash) to a
//! `Fragment`; [`fragment_by_hash`] is the runtime lookup the browser (P4) and
//! terminal renderer (P5) use to join a parsed doc body to its captured output.

use serde::{Deserialize, Serialize};

/// The captured result of running a [`crate::FragmentScript`] (§4.1).
///
/// Produced by the capture engine (`fm_cli::run_fragment`); consumed by
/// `xtask docgen` which validates `error_count` against the script's declared
/// `expect_errors` and emits the render-relevant parts into the generated
/// [`Fragment`] table. Serde-serializable for the `fm --capture-fragment` JSON
/// CLI path.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedFragment {
    /// The exact terminal-format transcript (§4.2): each input line echoed as
    /// `--> <line>`, followed by the captured value displays / error text.
    pub transcript: String,
    /// The number of errors actually raised by the session (cross-checked
    /// against the script's `expect_errors` by docgen).
    pub error_count: usize,
    /// The captured Plotly `Scene` JSON, if a figure was requested and one was
    /// produced. Rendered only in the browser (P4); the terminal prints a
    /// `[figure: …]` placeholder.
    pub figure: Option<String>,
}

/// One entry in the generated fragment DB — the render-relevant subset of a
/// [`CapturedFragment`], with `&'static` fields so the whole table is embedded
/// in the binary with no startup parse (§5, locked decision §0 #2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fragment {
    /// The captured terminal-format transcript (§4.2).
    pub transcript: &'static str,
    /// The captured Plotly `Scene` JSON, if the fragment produced a figure.
    pub figure: Option<&'static str>,
}

// The checked-in, docgen-generated table. `generated/fragments.rs` defines
// `pub static FRAGMENTS: &[(&str, Fragment)]`, sorted by hash for stable diffs.
include!("generated/fragments.rs");

/// Look up a captured [`Fragment`] by its script content hash (§4.3).
///
/// The key is [`FragmentScript::content_hash`](crate::FragmentScript::content_hash).
/// Returns `None` if no captured fragment exists for that hash (e.g. the doc
/// was edited but `cargo xtask docgen` has not been re-run).
#[must_use]
pub fn fragment_by_hash(hash: &str) -> Option<&'static Fragment> {
    // FRAGMENTS is sorted by hash (docgen guarantees it), so binary search.
    FRAGMENTS
        .binary_search_by(|(h, _)| (*h).cmp(hash))
        .ok()
        .map(|i| &FRAGMENTS[i].1)
}
