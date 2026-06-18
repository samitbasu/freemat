//! Function-handle values (`@f`, `@(x) expr`).
//!
//! A [`FunctionHandle`] is either a **named** reference to a function (`@sin`,
//! `@myfunc`) or an **anonymous** closure (`@(x) x.^2 + c`). Anonymous functions
//! **capture their free variables by value at definition time** (MATLAB/FreeMat
//! semantics): the captured workspace is a snapshot of the variables the closure
//! body references, frozen when the `@(...)` expression is evaluated.
//!
//! The closure body and parameter list are borrowed from the parser AST
//! (`fm-parser`), so this type lives in `fm-core` only because [`Array`] must be
//! able to hold it; the interpreter (`fm-interp`) is what actually *calls* it.

use std::collections::HashMap;
use std::sync::Arc;

use fm_parser::ast::Expr;

use crate::array::Array;

/// A function handle: a named function reference or an anonymous closure.
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionHandle {
    /// `@name` — a handle to the function called `name` (resolved at call time,
    /// in the workspace where the handle is *invoked*, like FreeMat).
    Named(String),
    /// `@(params) body` — an anonymous closure capturing its free variables by
    /// value at the point of definition.
    Anonymous(Arc<AnonClosure>),
}

/// The captured state of an anonymous function: its parameter names, body
/// expression (AST), and the snapshot of variables it closed over.
#[derive(Debug, Clone)]
pub struct AnonClosure {
    /// The formal parameter names (`@(x, y) ...` ⇒ `["x", "y"]`).
    pub params: Vec<String>,
    /// The body expression, evaluated against `params` + `captures` on each call.
    pub body: Expr,
    /// Variables captured by value at definition time (free variables of `body`
    /// that were bound in the defining workspace).
    pub captures: HashMap<String, Array>,
    /// A textual rendering of the closure for `func2str` / display
    /// (`@(x) x + 1`).
    pub text: String,
    /// The full source the `body`'s spans index into (so the body — and any
    /// nested anonymous functions inside it — can be re-evaluated with correct
    /// diagnostics).
    pub source: Arc<String>,
}

// `Expr` is not `PartialEq`-comparable in a meaningful runtime sense; two
// anonymous closures are considered equal only if they are the very same `Arc`.
impl PartialEq for AnonClosure {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl FunctionHandle {
    /// A named handle.
    #[must_use]
    pub fn named(name: impl Into<String>) -> Self {
        FunctionHandle::Named(name.into())
    }

    /// An anonymous closure.
    #[must_use]
    pub fn anonymous(
        params: Vec<String>,
        body: Expr,
        captures: HashMap<String, Array>,
        text: String,
        source: Arc<String>,
    ) -> Self {
        FunctionHandle::Anonymous(Arc::new(AnonClosure {
            params,
            body,
            captures,
            text,
            source,
        }))
    }

    /// The `func2str` rendering: `name` for a named handle (no `@`, matching
    /// FreeMat), `@(args) body` for an anonymous one.
    #[must_use]
    pub fn to_str(&self) -> String {
        match self {
            FunctionHandle::Named(n) => n.clone(),
            FunctionHandle::Anonymous(c) => c.text.clone(),
        }
    }

    /// The display rendering (always prefixed with `@`).
    #[must_use]
    pub fn display_str(&self) -> String {
        match self {
            FunctionHandle::Named(n) => format!("@{n}"),
            FunctionHandle::Anonymous(c) => c.text.clone(),
        }
    }
}
