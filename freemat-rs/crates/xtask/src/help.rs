//! `cargo xtask help` — query the embedded help/doc registry from the terminal.
//!
//! ```text
//! cargo xtask help sin           # print the doc topic for `sin`
//! cargo xtask help --list        # list every topic with its one-line summary
//! cargo xtask help --search fft  # topics whose name/summary mentions "fft"
//! ```
//!
//! This reads the same [`Registry`] the browser help panel and the docs pipeline
//! use, so a topic lookup here matches what a user sees typing `help <name>` in
//! the REPL — including alias resolution and `did you mean` suggestions.

use fm_doc::{Registry, Resolution};

/// Entry point for `cargo xtask help …`.
pub fn help(topic: Option<&str>, list_all: bool, search_term: Option<&str>) -> Result<(), String> {
    let registry = Registry::global();

    if list_all {
        return list(registry);
    }
    if let Some(term) = search_term {
        return search(registry, term);
    }
    let Some(topic) = topic else {
        return Err(
            "help: expected a topic name, `--list`, or `--search <term>`\n  \
             e.g. cargo xtask help sin"
                .to_string(),
        );
    };
    show(registry, topic)
}

/// Print every topic name with its one-line summary, sorted by name.
fn list(registry: &Registry) -> Result<(), String> {
    let mut rows: Vec<(&str, &str)> = registry.iter().map(|e| (e.name, e.summary)).collect();
    rows.sort_by(|a, b| a.0.cmp(b.0));
    for (name, summary) in &rows {
        println!("{name:<24} {summary}");
    }
    println!("\n{} topic(s).", rows.len());
    Ok(())
}

/// Print topics whose name or summary contains `term` (case-insensitive).
fn search(registry: &Registry, term: &str) -> Result<(), String> {
    let needle = term.to_lowercase();
    let mut hits: Vec<(&str, &str)> = registry
        .iter()
        .filter(|e| {
            e.name.to_lowercase().contains(&needle) || e.summary.to_lowercase().contains(&needle)
        })
        .map(|e| (e.name, e.summary))
        .collect();
    hits.sort_by(|a, b| a.0.cmp(b.0));
    for (name, summary) in &hits {
        println!("{name:<24} {summary}");
    }
    println!("\n{} match(es) for {term:?}.", hits.len());
    Ok(())
}

/// Resolve and print a single topic's documentation (name, section, summary, and
/// raw markdown body), or `did you mean` suggestions if it isn't found.
fn show(registry: &Registry, topic: &str) -> Result<(), String> {
    match registry.resolve(topic) {
        Resolution::Exact(e) => {
            let section = registry
                .section(e.section)
                .map(|s| s.title)
                .unwrap_or(e.section);
            println!("# {}", e.name);
            if !e.aliases.is_empty() {
                println!("aliases: {}", e.aliases.join(", "));
            }
            println!("section: {section}");
            println!("\n{}\n", e.summary);
            println!("{}", e.body_md);
            Ok(())
        }
        Resolution::Suggestions(names) => Err(format!(
            "help: no topic {topic:?}. Did you mean: {}?",
            names.join(", ")
        )),
        Resolution::None => Err(format!("help: no topic {topic:?} and no close matches.")),
    }
}
