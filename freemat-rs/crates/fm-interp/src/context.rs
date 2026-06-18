//! The execution [`Context`]: the scope stack, the global and persistent
//! variable tables, and the call stack (FreeMat's `Context`).
//!
//! Variable resolution mirrors FreeMat: a name declared `global` in the active
//! scope resolves to the shared [`Context::globals`] table; a name declared
//! `persistent` resolves to a per-function [`Context::persistents`] table keyed
//! by `function@name`; otherwise it is a plain local in the active scope.
//!
//! **Debug seam (Stage 3):** the *active* scope index is switchable
//! ([`Context::set_active`] / [`Context::active_index`]) so a future debugger's
//! `dbup`/`dbdown` can walk the call stack while paused, inspecting variables in
//! a frame other than the executing top one.

use fm_core::Array;
use fm_parser::Span;
use rustc_hash::FxHashMap;

use crate::scope::Scope;

/// Holds the scope stack + global/persistent tables; the runtime "world".
#[derive(Debug)]
pub struct Context {
    /// The scope stack. `scopes[0]` is the base (REPL/script) scope; each call
    /// pushes a new scope. The *executing* scope is always the last one.
    scopes: Vec<Scope>,
    /// The active scope index for variable inspection. Normally `scopes.len()-1`
    /// (the top), but a debugger may point it at an outer frame.
    active: usize,
    /// Shared global variables (declared `global`).
    globals: FxHashMap<String, Array>,
    /// Persistent variables, keyed `function\0name`.
    persistents: FxHashMap<String, Array>,
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    /// A fresh context with a single base scope.
    #[must_use]
    pub fn new() -> Self {
        Context {
            scopes: vec![Scope::new("")],
            active: 0,
            globals: FxHashMap::default(),
            persistents: FxHashMap::default(),
        }
    }

    /// Push a new call frame named `name`; the new scope becomes top & active.
    pub fn push_scope(&mut self, name: impl Into<String>) {
        self.scopes.push(Scope::new(name));
        self.active = self.scopes.len() - 1;
    }

    /// Pop the top call frame (returns it). The base scope is never popped.
    pub fn pop_scope(&mut self) -> Option<Scope> {
        if self.scopes.len() <= 1 {
            return None;
        }
        let s = self.scopes.pop();
        self.active = self.scopes.len() - 1;
        s
    }

    /// Push a previously-popped frame back onto the call stack (used to restore
    /// the executing frame after `evalin`/`assignin('caller', …)` temporarily
    /// pops it to run in the caller's scope).
    pub fn restore_scope(&mut self, scope: Scope) {
        self.scopes.push(scope);
        self.active = self.scopes.len() - 1;
    }

    /// The number of frames on the call stack (including the base scope).
    #[must_use]
    pub fn depth(&self) -> usize {
        self.scopes.len()
    }

    /// The *executing* (top) scope, mutably.
    #[must_use]
    pub fn top_mut(&mut self) -> &mut Scope {
        let i = self.scopes.len() - 1;
        &mut self.scopes[i]
    }

    /// The *executing* (top) scope.
    #[must_use]
    pub fn top(&self) -> &Scope {
        self.scopes.last().expect("base scope always present")
    }

    // ---- Debug seam: switchable active scope (dbup / dbdown) ----------------

    /// The index of the active (inspected) scope.
    #[must_use]
    pub fn active_index(&self) -> usize {
        self.active
    }

    /// Point variable inspection at scope `idx` (clamped). Basis for
    /// `dbup`/`dbdown`; does not change which scope *executes*.
    pub fn set_active(&mut self, idx: usize) {
        self.active = idx.min(self.scopes.len() - 1);
    }

    /// The active (inspected) scope.
    #[must_use]
    pub fn active(&self) -> &Scope {
        &self.scopes[self.active]
    }

    /// The active (inspected) scope, mutably.
    #[must_use]
    pub fn active_mut(&mut self) -> &mut Scope {
        &mut self.scopes[self.active]
    }

    /// **Debug seam:** record the span/line of the statement now executing in
    /// the top scope, so a debugger / `dbstack` knows the current location.
    pub fn set_current_span(&mut self, span: Span, line: usize) {
        let top = self.top_mut();
        top.current_span = Some(span);
        top.current_line = Some(line);
    }

    // ---- Variable resolution (global / persistent / local) ------------------

    /// The persistent-table key for `name` in the top scope's function.
    fn persistent_key(&self, name: &str) -> String {
        format!("{}\0{}", self.top().name, name)
    }

    /// Look up a variable by name, honouring global/persistent declarations of
    /// the **top** scope (the executing one).
    #[must_use]
    pub fn lookup(&self, name: &str) -> Option<&Array> {
        let top = self.top();
        if top.is_global(name) {
            return self.globals.get(name);
        }
        if top.is_persistent(name) {
            return self.persistents.get(&self.persistent_key(name));
        }
        top.get_local(name)
    }

    /// Mutable access to the variable `name` **in place** (no remove/insert),
    /// honouring the top scope's global/persistent declarations. Returns `None`
    /// if the variable is not currently bound.
    ///
    /// This is the in-place indexed-assignment fast path: scattering through the
    /// returned `&mut Array` keeps the backing `Arc` owned solely by the symbol
    /// table (strong-count 1 when not aliased), so `make_mut_*` mutates in place;
    /// it also avoids the per-write map mutation and the `String` key realloc the
    /// old take → mutate → put-back dance incurred. Copy-on-write stays correct:
    /// an aliased `Arc` (`B = A`) still deep-copies on the first write.
    #[must_use]
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Array> {
        let top = self.scopes.last().expect("base scope always present");
        if top.is_global(name) {
            self.globals.get_mut(name)
        } else if top.is_persistent(name) {
            let key = self.persistent_key(name);
            self.persistents.get_mut(&key)
        } else {
            self.top_mut().get_local_mut(name)
        }
    }

    /// Assign `value` to `name`, honouring global/persistent declarations.
    pub fn assign(&mut self, name: &str, value: Array) {
        let top = self.top();
        if top.is_global(name) {
            self.globals.insert(name.to_string(), value);
        } else if top.is_persistent(name) {
            let key = self.persistent_key(name);
            self.persistents.insert(key, value);
        } else {
            self.top_mut().set_local(name, value);
        }
    }

    /// Remove and **return ownership** of the variable `name` (local, global, or
    /// persistent, honouring the top scope's declarations), leaving it unbound.
    ///
    /// This is the "take" half of the in-place indexed-assignment fast path: by
    /// owning the `Array` out of the table (rather than `lookup().cloned()`), the
    /// backing `Arc` has strong-count 1 in the non-aliased case, so
    /// `make_mut_*` mutates in place instead of deep-copying. The caller must
    /// [`set`](Self::set) the (possibly mutated) value back afterwards.
    pub fn take(&mut self, name: &str) -> Option<Array> {
        let top = self.top();
        if top.is_global(name) {
            self.globals.remove(name)
        } else if top.is_persistent(name) {
            let key = self.persistent_key(name);
            self.persistents.remove(&key)
        } else {
            self.top_mut().take_local(name)
        }
    }

    /// Bind `value` to `name`, honouring global/persistent declarations. Alias of
    /// [`assign`](Self::assign); the name pairs with [`take`](Self::take) to make
    /// the take → mutate → put-back idiom read clearly at call sites.
    pub fn set(&mut self, name: &str, value: Array) {
        self.assign(name, value);
    }

    /// Whether a variable named `name` is currently visible in the top scope.
    #[must_use]
    pub fn exists(&self, name: &str) -> bool {
        self.lookup(name).is_some()
    }

    /// Remove a local/global/persistent binding (`clear name`).
    pub fn remove(&mut self, name: &str) {
        let top = self.top();
        if top.is_global(name) {
            self.globals.remove(name);
        } else if top.is_persistent(name) {
            let key = self.persistent_key(name);
            self.persistents.remove(&key);
        } else {
            self.top_mut().remove_local(name);
        }
    }

    /// Declare `name` global in the top scope; seed the global table with `[]`
    /// if it has no value yet (FreeMat initialises a new global to empty).
    pub fn declare_global(&mut self, name: &str) {
        self.top_mut().add_global(name);
        self.globals
            .entry(name.to_string())
            .or_insert_with(Array::empty);
    }

    /// Declare `name` persistent in the top scope; seed it with `[]` if unset.
    pub fn declare_persistent(&mut self, name: &str) {
        self.top_mut().add_persistent(name);
        let key = self.persistent_key(name);
        self.persistents.entry(key).or_insert_with(Array::empty);
    }

    /// A snapshot of the call stack as `(function-name, line)` from base → top,
    /// for `dbstack`-style reporting (Stage 10 consumes this).
    #[must_use]
    pub fn stack_trace(&self) -> Vec<(String, Option<usize>)> {
        self.scopes
            .iter()
            .map(|s| (s.name.clone(), s.current_line))
            .collect()
    }
}
