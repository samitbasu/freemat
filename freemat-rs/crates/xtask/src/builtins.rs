//! `cargo xtask builtins` — enumerate the registered standard library and
//! cross-reference it against the help registry to surface undocumented
//! builtins.
//!
//! ```text
//! cargo xtask builtins                 # every registered builtin + doc status
//! cargo xtask builtins --search fft    # only names containing "fft"
//! cargo xtask builtins --undocumented  # only builtins with no help entry
//! ```
//!
//! The registered set is the authoritative list (the same one `fm
//! --list-builtins` prints, and what `docs/COVERAGE.md` is generated from). Each
//! name is tagged `[doc]` if the help registry has an entry (by name or alias)
//! and `[   ]` otherwise, and a summary line reports documentation coverage — a
//! quick way to see where the help backlog is.

use std::collections::HashSet;

use fm_doc::Registry;
use fm_interp::Interpreter;

/// Entry point for `cargo xtask builtins …`.
pub fn builtins(search: Option<&str>, undocumented_only: bool) -> Result<(), String> {
    let mut interp = Interpreter::new();
    fm_builtins::register_standard_library(&mut interp);
    let mut names = interp.functions.names();
    names.sort();

    // Documented set: every help-registry topic name and alias.
    let registry = Registry::global();
    let mut documented: HashSet<&str> = HashSet::new();
    for e in registry.iter() {
        documented.insert(e.name);
        for a in e.aliases {
            documented.insert(a);
        }
    }

    let mut shown = 0usize;
    let mut doc_count = 0usize;
    for name in &names {
        let has_doc = documented.contains(name.as_str());
        if has_doc {
            doc_count += 1;
        }
        if let Some(term) = search
            && !name.contains(term)
        {
            continue;
        }
        if undocumented_only && has_doc {
            continue;
        }
        let tag = if has_doc { "[doc]" } else { "[   ]" };
        println!("{tag} {name}");
        shown += 1;
    }

    let total = names.len();
    let pct = if total == 0 {
        0.0
    } else {
        doc_count as f64 / total as f64 * 100.0
    };
    println!(
        "\n{shown} shown — {total} builtins registered, {doc_count} documented ({pct:.1}% help coverage)."
    );
    Ok(())
}
