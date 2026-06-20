//! Core data model for the help system (§2 of `docs/HELP_SYSTEM.md`).
//!
//! Every field is `&'static` because [`DocEntry`]/[`SectionEntry`] are produced
//! by the [`crate::register_doc!`] / [`crate::register_section!`] macros at
//! compile time and collected via the `inventory` crate.

/// One help topic. The compiled-in registry holds these; fragment transcripts
/// are stored separately (see §4) and joined at render time by content hash.
#[derive(Debug, Clone, Copy)]
pub struct DocEntry {
    /// Primary topic name, e.g. `"cos"`. Lowercase, the canonical key.
    pub name: &'static str,
    /// Alternate names resolving here, e.g. `["arg"]` for `"angle"`.
    pub aliases: &'static [&'static str],
    /// Section id, e.g. `"mathfunctions"`. Must match a [`SectionEntry`].
    pub section: &'static str,
    /// One-line summary, used in indexes and the `help` header.
    pub summary: &'static str,
    /// Markdown body in the dialect of §3.
    pub body_md: &'static str,
    /// Provenance, for tooling and the "writing docs" workflow.
    pub source: SourceKind,
}

/// Where a [`DocEntry`] came from — for tooling and the docs workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// Native Rust builtin. `module` is the file-stem, e.g. `"trig"`.
    RustBuiltin {
        /// The crate that registered the entry (`env!("CARGO_PKG_NAME")`).
        krate: &'static str,
        /// The file-stem of the registering module (from `module_path!()`).
        module: &'static str,
    },
    /// Embedded/toolbox `.m` function. `path` is repo-relative.
    Toolbox {
        /// Repo-relative path to the `.m` source.
        path: &'static str,
    },
}

/// A documentation section, e.g. "Mathematical Functions".
#[derive(Debug, Clone, Copy)]
pub struct SectionEntry {
    /// Stable section id, e.g. `"mathfunctions"`.
    pub id: &'static str,
    /// Human-readable title, e.g. `"Mathematical Functions"`.
    pub title: &'static str,
    /// One paragraph describing the section (from `sec_<id>.doc`).
    pub summary: &'static str,
}

// ---------------------------------------------------------------------------
// Inventory collection types.
//
// These wrappers are the concrete types collected by `inventory`. The
// `register_doc!`/`register_section!` macros expand to `inventory::submit!` of
// these, so the types (and the `inventory::collect!` invocations) must be
// public for the macros to be usable from other crates (e.g. `fm-builtins`).
// ---------------------------------------------------------------------------

/// Inventory wrapper for a registered [`DocEntry`]. Emitted by
/// [`crate::register_doc!`]; do not construct directly.
pub struct DocRegistration(pub DocEntry);

/// Inventory wrapper for a registered [`SectionEntry`]. Emitted by
/// [`crate::register_section!`]; do not construct directly.
pub struct SectionRegistration(pub SectionEntry);

inventory::collect!(DocRegistration);
inventory::collect!(SectionRegistration);
