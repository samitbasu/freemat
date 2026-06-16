//! `fm-interp` — FreeMat-rs tree-walking interpreter, scopes, and builtin registry.
//!
//! Scaffold only (Stage 0). The evaluator, `Context`/`Scope`, builtin registry,
//! and `.m` loader arrive in Stage 3.

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_builds() {
        assert_eq!(2 + 2, 4);
    }
}
