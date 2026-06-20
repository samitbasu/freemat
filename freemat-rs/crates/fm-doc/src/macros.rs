//! The `register_doc!` and `register_section!` macros (§7 of
//! `docs/HELP_SYSTEM.md`).
//!
//! Both expand to an `inventory::submit!` of the public wrapper types in
//! [`crate::model`], so they can be invoked from *other* crates (e.g.
//! `fm-builtins`) and still be collected into the global [`crate::Registry`].
//!
//! The body markdown is **not** parsed at compile time — that happens in
//! `docgen` / `--check` (§7). The macros only stash the `&'static str` body.

/// Extract the trailing `::`-delimited segment of a `module_path!()` string at
/// compile time (the "file stem", e.g. `"elementary"` from
/// `"fm_builtins::elementary"`). Falls back to the whole string if there is no
/// `::` separator.
#[must_use]
pub const fn module_stem(path: &'static str) -> &'static str {
    let bytes = path.as_bytes();
    let mut last = 0usize; // index just past the last ':' we saw
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b':' {
            last = i + 1;
        }
        i += 1;
    }
    // Safety: `last` lands on a UTF-8 boundary because ':' is ASCII and only
    // appears as part of the ASCII `::` separator in a module path.
    let (_, stem) = path.split_at(last);
    stem
}

/// Register a [`DocEntry`](crate::DocEntry) next to a builtin (§7).
///
/// `aliases` may be omitted (defaults to `[]`). `source` is filled with
/// `RustBuiltin { krate: env!("CARGO_PKG_NAME"), module: <file stem> }`.
///
/// # Example
/// ```ignore
/// register_doc! {
///     name: "cos",
///     aliases: ["arg"],            // optional; omit for none
///     section: "mathfunctions",
///     summary: "Trigonometric cosine of the argument.",
///     body: r#"
/// ## Usage
/// Computes `cos(x)`.
/// "#,
/// }
/// ```
#[macro_export]
macro_rules! register_doc {
    // With explicit aliases.
    (
        name: $name:expr,
        aliases: [ $($alias:expr),* $(,)? ],
        section: $section:expr,
        summary: $summary:expr,
        body: $body:expr $(,)?
    ) => {
        $crate::inventory::submit! {
            $crate::DocRegistration($crate::DocEntry {
                name: $name,
                aliases: &[ $($alias),* ],
                section: $section,
                summary: $summary,
                body_md: $body,
                source: $crate::SourceKind::RustBuiltin {
                    krate: env!("CARGO_PKG_NAME"),
                    module: $crate::module_stem(module_path!()),
                },
            })
        }
    };

    // Aliases omitted -> default to [].
    (
        name: $name:expr,
        section: $section:expr,
        summary: $summary:expr,
        body: $body:expr $(,)?
    ) => {
        $crate::register_doc! {
            name: $name,
            aliases: [],
            section: $section,
            summary: $summary,
            body: $body,
        }
    };
}

/// Register a [`SectionEntry`](crate::SectionEntry) (§7).
///
/// # Example
/// ```ignore
/// register_section! {
///     id: "mathfunctions",
///     title: "Mathematical Functions",
///     summary: "Elementary and special mathematical functions.",
/// }
/// ```
#[macro_export]
macro_rules! register_section {
    (
        id: $id:expr,
        title: $title:expr,
        summary: $summary:expr $(,)?
    ) => {
        $crate::inventory::submit! {
            $crate::SectionRegistration($crate::SectionEntry {
                id: $id,
                title: $title,
                summary: $summary,
            })
        }
    };
}
