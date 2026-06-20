//! The doc [`Registry`] (§2.3, §6.1 of `docs/HELP_SYSTEM.md`).
//!
//! Built lazily from the `inventory` collections, sorted deterministically by
//! `(section, name)`. A duplicate primary `name` across two `register_doc!`
//! sites is a **hard error at build** (see [`Registry::try_build`] /
//! [`Registry::global`]).

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::model::{DocEntry, DocRegistration, SectionEntry, SectionRegistration, SourceKind};

/// The compiled-in help registry. Holds all [`DocEntry`]/[`SectionEntry`]
/// collected from `register_doc!`/`register_section!` sites, sorted by
/// `(section, name)`.
#[derive(Debug)]
pub struct Registry {
    /// Entries sorted by `(section, name)`.
    entries: Vec<DocEntry>,
    /// Sections sorted by `id`.
    sections: Vec<SectionEntry>,
    /// `name`/`alias` (lowercased) -> index into `entries`. Primary names take
    /// precedence over aliases on collision.
    by_name: HashMap<String, usize>,
    /// `id` -> index into `sections`.
    section_by_id: HashMap<String, usize>,
}

/// Result of [`Registry::resolve`] (§6.1).
#[derive(Debug)]
pub enum Resolution<'a> {
    /// An exact (or alias / case-insensitive) hit.
    Exact(&'a DocEntry),
    /// No exact hit, but these names are close ("did you mean ...").
    Suggestions(Vec<&'a str>),
    /// Nothing close enough.
    None,
}

/// Error raised when two `register_doc!` sites declare the same primary `name`.
#[derive(Debug, Clone)]
pub struct DuplicateName {
    /// The clashing primary name.
    pub name: String,
    /// Source of the first registration.
    pub first: SourceKind,
    /// Source of the second registration.
    pub second: SourceKind,
}

impl std::fmt::Display for DuplicateName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "duplicate doc entry name {:?}: first registered at {:?}, then again at {:?}",
            self.name, self.first, self.second
        )
    }
}

impl std::error::Error for DuplicateName {}

/// Maximum number of "did you mean" suggestions returned (§6.1).
const MAX_SUGGESTIONS: usize = 8;
/// Maximum Damerau-Levenshtein distance for a suggestion (§6.1).
const MAX_EDIT_DISTANCE: usize = 2;

impl Registry {
    /// Build a registry from explicit iterators of entries/sections.
    ///
    /// Returns [`DuplicateName`] if two entries share a primary `name`. This is
    /// the testable core that both [`Registry::global`] and unit tests use.
    pub fn try_build<D, S>(entries: D, sections: S) -> Result<Registry, DuplicateName>
    where
        D: IntoIterator<Item = DocEntry>,
        S: IntoIterator<Item = SectionEntry>,
    {
        let mut entries: Vec<DocEntry> = entries.into_iter().collect();
        let mut sections: Vec<SectionEntry> = sections.into_iter().collect();

        // Deterministic order: (section, name) for entries, id for sections.
        entries.sort_by(|a, b| (a.section, a.name).cmp(&(b.section, b.name)));
        sections.sort_by(|a, b| a.id.cmp(b.id));

        // Detect duplicate primary names (after sort, equal names are adjacent
        // only within a section; scan the whole set via a map to be safe).
        let mut primary_seen: HashMap<&str, SourceKind> = HashMap::new();
        for e in &entries {
            if let Some(&first) = primary_seen.get(e.name) {
                return Err(DuplicateName {
                    name: e.name.to_string(),
                    first,
                    second: e.source,
                });
            }
            primary_seen.insert(e.name, e.source);
        }

        // Build the lookup map. Primary names win over aliases.
        let mut by_name: HashMap<String, usize> = HashMap::new();
        for (i, e) in entries.iter().enumerate() {
            for a in e.aliases {
                by_name.entry(a.to_lowercase()).or_insert(i);
            }
        }
        for (i, e) in entries.iter().enumerate() {
            by_name.insert(e.name.to_lowercase(), i);
        }

        let mut section_by_id: HashMap<String, usize> = HashMap::new();
        for (i, s) in sections.iter().enumerate() {
            section_by_id.insert(s.id.to_string(), i);
        }

        Ok(Registry {
            entries,
            sections,
            by_name,
            section_by_id,
        })
    }

    /// The lazily-built, process-global registry collected from `inventory`.
    ///
    /// # Panics
    /// Panics (naming both source locations) if a duplicate primary `name` is
    /// detected — a programming error that must be fixed, and which is also
    /// surfaced as a checkable error by [`Registry::try_build`] for tests.
    pub fn global() -> &'static Registry {
        static GLOBAL: OnceLock<Registry> = OnceLock::new();
        GLOBAL.get_or_init(|| {
            let entries = inventory::iter::<DocRegistration>.into_iter().map(|r| r.0);
            let sections = inventory::iter::<SectionRegistration>
                .into_iter()
                .map(|r| r.0);
            match Registry::try_build(entries, sections) {
                Ok(reg) => reg,
                Err(dup) => panic!("{dup}"),
            }
        })
    }

    /// Exact name -> alias lookup (case-sensitive primary, then any registered
    /// name/alias case-insensitively via the map). Returns the entry whose
    /// `name` or `alias` matches `name`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&DocEntry> {
        // Case-sensitive exact primary name first.
        if let Some(&i) = self.by_name.get(name) {
            // by_name is lowercased; a case-sensitive caller may pass an
            // already-lowercase name. Confirm a true match below in resolve();
            // here `get` follows the spec "exact -> alias".
            return Some(&self.entries[i]);
        }
        self.by_name
            .get(&name.to_lowercase())
            .map(|&i| &self.entries[i])
    }

    /// Resolve a query per §6.1: exact name -> alias -> case-insensitive ->
    /// "did you mean" (Damerau-Levenshtein <= 2 or prefix, capped).
    #[must_use]
    pub fn resolve(&self, query: &str) -> Resolution<'_> {
        // 1. Exact primary name (case-sensitive).
        for e in &self.entries {
            if e.name == query {
                return Resolution::Exact(e);
            }
        }
        // 2. Alias (case-sensitive).
        for e in &self.entries {
            if e.aliases.contains(&query) {
                return Resolution::Exact(e);
            }
        }
        // 3. Case-insensitive name/alias.
        let lower = query.to_lowercase();
        if let Some(&i) = self.by_name.get(&lower) {
            return Resolution::Exact(&self.entries[i]);
        }
        // 4. Did you mean: edit distance <= 2 or prefix match.
        let mut scored: Vec<(usize, &str)> = Vec::new();
        for e in &self.entries {
            let name_l = e.name.to_lowercase();
            if name_l.starts_with(&lower) || lower.starts_with(&name_l) {
                scored.push((0, e.name));
                continue;
            }
            let d = damerau_levenshtein(&lower, &name_l);
            if d <= MAX_EDIT_DISTANCE {
                scored.push((d, e.name));
            }
        }
        if scored.is_empty() {
            return Resolution::None;
        }
        // Sort by (distance, name) for determinism, then cap.
        scored.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(b.1)));
        scored.truncate(MAX_SUGGESTIONS);
        Resolution::Suggestions(scored.into_iter().map(|(_, n)| n).collect())
    }

    /// Look up a section by id.
    #[must_use]
    pub fn section(&self, id: &str) -> Option<&SectionEntry> {
        self.section_by_id.get(id).map(|&i| &self.sections[i])
    }

    /// All sections, sorted by id.
    pub fn sections(&self) -> impl Iterator<Item = &SectionEntry> {
        self.sections.iter()
    }

    /// All entries in a section, sorted by name (the entries are globally
    /// sorted by `(section, name)`, so the in-section slice is name-sorted).
    pub fn in_section(&self, id: &str) -> impl Iterator<Item = &DocEntry> {
        self.entries.iter().filter(move |e| e.section == id)
    }

    /// All entries, sorted by `(section, name)`.
    pub fn iter(&self) -> impl Iterator<Item = &DocEntry> {
        self.entries.iter()
    }
}

/// Damerau-Levenshtein edit distance (with adjacent transpositions) over
/// Unicode scalar values. Used for "did you mean" suggestions (§6.1).
#[must_use]
pub fn damerau_levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (n, m) = (a.len(), b.len());
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }
    // d[i][j] = distance between a[..i] and b[..j].
    let mut d = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(d[i - 2][j - 2] + 1);
            }
            d[i][j] = best;
        }
    }
    d[n][m]
}
