//! `fm-doc` — the FreeMat-rs help system core (Phase P1).
//!
//! This crate provides the documentation **data model** (§2), the
//! [`register_doc!`]/[`register_section!`] **macros** (§7) used next to each
//! builtin, the [`Registry`] that collects and resolves them (§2.3, §6.1), and
//! the **forward markdown-dialect parser** (§3) that extracts the structured
//! view and the executable [`FragmentScript`] (§4.1) for an entry.
//!
//! The authoritative contract is `docs/HELP_SYSTEM.md`.
//!
//! # Registration
//! Docs are registered via the `inventory` crate's distributed collection
//! (locked decision §0 #1). A `register_doc!` next to a builtin is collected
//! automatically; [`Registry::global`] builds a deterministic, `(section,
//! name)`-sorted view on first access. The macros expand to
//! `inventory::submit!` of public wrapper types, so they are usable from other
//! crates (e.g. `fm-builtins`).
//!
//! ```ignore
//! use fm_doc::register_doc;
//! register_doc! {
//!     name: "cos",
//!     section: "mathfunctions",
//!     summary: "Trigonometric cosine of the argument.",
//!     body: r#"
//! ## Usage
//! Computes `cos(x)`.
//! "#,
//! }
//! ```
//!
//! What this crate does **not** do: it does not parse bodies at compile time
//! (kept cheap — parsing runs in `docgen`/`--check`), and it does not implement
//! the legacy `.doc`/`.m` converter (that is P6; see [`parser::legacy`]).

mod fragment;
mod macros;
mod model;
mod parser;
mod registry;

// Re-export `inventory` so the `register_doc!`/`register_section!` macros can
// refer to it as `$crate::inventory` from any downstream crate.
pub use inventory;

pub use fragment::{CapturedFragment, FRAGMENTS, Fragment, fragment_by_hash};
pub use macros::module_stem;
pub use model::{DocEntry, DocRegistration, SectionEntry, SectionRegistration, SourceKind};
pub use parser::{
    BlockKind, CodeBlock, CrossLink, FragmentScript, Heading, MathSpan, ParsedDoc, fragment_script,
    fragment_script_from_body, legacy, parse_body,
};
pub use registry::{DuplicateName, Registry, Resolution, damerau_levenshtein};

#[cfg(test)]
mod tests;
