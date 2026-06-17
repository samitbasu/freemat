//! A single variable scope (FreeMat's `Scope`).
//!
//! Each scope holds its own local variables plus the *names* it has declared
//! `global` or `persistent`; lookups for those names resolve against the
//! [`Context`](crate::context::Context)'s global / per-function persistent
//! tables rather than the local map. The scope also records the **source span /
//! line of the statement currently executing** — a debug seam (Stage 3) that a
//! future debugger / `dbstack` reads to know the active line.

use std::collections::HashMap;

use fm_core::Array;
use fm_parser::Span;

/// A lexical scope: locals + global/persistent declarations + the active line.
#[derive(Debug, Clone)]
pub struct Scope {
    /// The name of the function this scope belongs to (`""` for the base/REPL).
    pub name: String,
    /// Local variables.
    locals: HashMap<String, Array>,
    /// Names declared `global` in this scope (resolved from the global table).
    globals: Vec<String>,
    /// Names declared `persistent` in this scope.
    persistents: Vec<String>,
    /// **Debug seam:** span of the statement currently executing in this scope.
    pub current_span: Option<Span>,
    /// **Debug seam:** 1-based source line of the current statement, if known.
    pub current_line: Option<usize>,
}

impl Scope {
    /// A fresh scope named `name`.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Scope {
            name: name.into(),
            locals: HashMap::new(),
            globals: Vec::new(),
            persistents: Vec::new(),
            current_span: None,
            current_line: None,
        }
    }

    /// Whether `name` is declared global in this scope.
    #[must_use]
    pub fn is_global(&self, name: &str) -> bool {
        self.globals.iter().any(|n| n == name)
    }

    /// Whether `name` is declared persistent in this scope.
    #[must_use]
    pub fn is_persistent(&self, name: &str) -> bool {
        self.persistents.iter().any(|n| n == name)
    }

    /// Declare `name` global in this scope.
    pub fn add_global(&mut self, name: &str) {
        if !self.is_global(name) {
            self.globals.push(name.to_string());
        }
    }

    /// Declare `name` persistent in this scope.
    pub fn add_persistent(&mut self, name: &str) {
        if !self.is_persistent(name) {
            self.persistents.push(name.to_string());
        }
    }

    /// Look up a *local* variable.
    #[must_use]
    pub fn get_local(&self, name: &str) -> Option<&Array> {
        self.locals.get(name)
    }

    /// Insert / overwrite a *local* variable.
    pub fn set_local(&mut self, name: &str, value: Array) {
        self.locals.insert(name.to_string(), value);
    }

    /// Remove a local variable, returning it.
    pub fn remove_local(&mut self, name: &str) -> Option<Array> {
        self.locals.remove(name)
    }

    /// Whether a local variable named `name` exists.
    #[must_use]
    pub fn has_local(&self, name: &str) -> bool {
        self.locals.contains_key(name)
    }

    /// All local variable names (for `who`/`whos`-style introspection).
    #[must_use]
    pub fn local_names(&self) -> Vec<&str> {
        self.locals.keys().map(String::as_str).collect()
    }
}
