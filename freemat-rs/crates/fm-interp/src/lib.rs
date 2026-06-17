//! `fm-interp` — FreeMat-rs tree-walking interpreter, scopes, and builtin
//! registry.
//!
//! This crate turns a parsed [`fm_parser`] AST into running code over
//! [`fm_core::Array`] values. It mirrors FreeMat's `Interpreter` / `Context` /
//! `Scope` design but in idiomatic Rust.
//!
//! # Pieces
//! - [`Context`] — the scope stack, global & per-function persistent tables, and
//!   the call stack (with a **switchable active scope** for future
//!   `dbup`/`dbdown`).
//! - [`Scope`] — one frame: locals + global/persistent declarations + the
//!   **current statement's span/line** (a debug seam).
//! - [`Interpreter`] — the evaluator. Every statement flows through the single
//!   chokepoint [`Interpreter::exec_statement`] (the breakpoint hook site).
//! - [`FunctionTable`] / [`Function`] — the builtin/`.m` registry (analogous to
//!   `addFunction` / `addSpecialFunction`).
//! - Operator dispatch ([`ops`]) with broadcasting + promotion, and indexing
//!   ([`index`]) with grow-on-assign.
//! - The `.m` loader (`loader`), parsing + running files through the same
//!   evaluator.
//!
//! # Errors & control flow
//! Runtime errors are [`InterpError`] (`miette` diagnostics with spans);
//! `break`/`continue`/`return` and caught errors are an idiomatic [`Signal`]
//! enum threaded through `Result`, never panics.
//!
//! # Quick start
//! ```
//! use fm_interp::Interpreter;
//! let mut interp = Interpreter::new();
//! interp.run("x = 2 + 3;").unwrap();
//! let x = interp.context.lookup("x").unwrap();
//! assert_eq!(x.as_f64(), Some(5.0));
//! ```

pub mod builtins;
pub mod context;
pub mod error;
pub mod function;
pub mod index;
pub mod interp;
pub mod loader;
pub mod ops;
pub mod scope;
pub mod value;

pub use context::Context;
pub use error::{Flow, InterpError, Signal};
pub use function::{BuiltinFn, Function, FunctionTable};
pub use interp::Interpreter;
pub use scope::Scope;
