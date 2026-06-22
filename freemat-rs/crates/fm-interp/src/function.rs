//! Callable functions and the function table.
//!
//! A [`Function`] is either a **builtin** (a Rust function with access to the
//! [`Interpreter`](crate::interp::Interpreter) and the requested `nargout`,
//! mirroring FreeMat's `addFunction` / `addSpecialFunction`) or an
//! **interpreted** `.m` function (a parsed [`FunctionDef`] run by the same
//! evaluator — this is what lets the 317 `toolbox/*.m` files execute unchanged).

use std::collections::HashMap;
use std::sync::Arc;

use fm_core::Array;
use fm_parser::ast::FunctionDef;

use crate::error::Flow;
use crate::interp::Interpreter;

/// The signature of a builtin: gets the interpreter (for re-entrant evaluation
/// and special functions), the evaluated arguments, and the number of outputs
/// requested (`nargout`). Returns the output values.
pub type BuiltinFn = fn(&mut Interpreter, &[Array], usize) -> Flow<Vec<Array>>;

/// A callable function: a Rust builtin or an interpreted `.m` function.
#[derive(Clone)]
pub enum Function {
    /// A native builtin.
    Builtin {
        /// The function's name.
        name: String,
        /// The Rust implementation.
        func: BuiltinFn,
    },
    /// An interpreted function loaded from `.m` source (shared, ref-counted).
    Interpreted {
        /// The parsed definition.
        def: Arc<FunctionDef>,
        /// The source text the def was parsed from (for diagnostics).
        src: Arc<String>,
        /// The file-local function table: every function defined in the same
        /// `.m` file (the public/main function plus its subfunctions and nested
        /// functions). These are visible only while a function from this file is
        /// executing, mirroring FreeMat's file-private subfunction scoping.
        locals: Arc<FileLocals>,
    },
}

/// The set of functions private to a single `.m` file (the main function plus
/// its subfunctions / nested functions), plus the shared source text they were
/// parsed from (for diagnostics).
#[derive(Debug, Default)]
pub struct FileLocals {
    /// File-private functions keyed by name.
    pub funcs: HashMap<String, Arc<FunctionDef>>,
    /// The source text shared by every function in the file.
    pub src: Arc<String>,
    /// The absolute path of the `.m` file these functions were loaded from, if
    /// they came from a file (vs. embedded/REPL source). Surfaced by `which`.
    pub path: Option<String>,
}

impl FileLocals {
    /// Look up a file-local function by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Arc<FunctionDef>> {
        self.funcs.get(name)
    }
}

impl Function {
    /// The function's name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Function::Builtin { name, .. } => name,
            Function::Interpreted { def, .. } => &def.name,
        }
    }
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Function::Builtin { name, .. } => write!(f, "Builtin({name})"),
            Function::Interpreted { def, .. } => write!(f, "Interpreted({})", def.name),
        }
    }
}

/// The interpreter's function table (name → [`Function`]).
#[derive(Debug, Default, Clone)]
pub struct FunctionTable {
    funcs: HashMap<String, Function>,
}

impl FunctionTable {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        FunctionTable {
            funcs: HashMap::new(),
        }
    }

    /// Register a builtin (analogous to FreeMat's `addFunction`).
    pub fn add_builtin(&mut self, name: &str, func: BuiltinFn) {
        self.funcs.insert(
            name.to_string(),
            Function::Builtin {
                name: name.to_string(),
                func,
            },
        );
    }

    /// Register an interpreted `.m` function with its file-local function set.
    /// The source text is taken from `locals.src` (shared across the file).
    pub fn add_interpreted(&mut self, def: Arc<FunctionDef>, locals: Arc<FileLocals>) {
        let src = locals.src.clone();
        self.funcs
            .insert(def.name.clone(), Function::Interpreted { def, src, locals });
    }

    /// Register an interpreted function under an *alias* name (e.g. the file
    /// stem) distinct from the function's declared name, sharing the same
    /// file-local table. Used so a function file is callable by its filename as
    /// well as by its declared name (FreeMat's filename-based invocation).
    pub fn add_interpreted_as(
        &mut self,
        name: &str,
        def: Arc<FunctionDef>,
        locals: Arc<FileLocals>,
    ) {
        let src = locals.src.clone();
        self.funcs
            .insert(name.to_string(), Function::Interpreted { def, src, locals });
    }

    /// Look up a function by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Function> {
        self.funcs.get(name)
    }

    /// Whether a function named `name` is registered.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.funcs.contains_key(name)
    }

    /// The absolute path of the `.m` file a function was loaded from, if it is
    /// an interpreted function loaded from a file (used by `which`).
    #[must_use]
    pub fn source_path(&self, name: &str) -> Option<String> {
        match self.funcs.get(name) {
            Some(Function::Interpreted { locals, .. }) => locals.path.clone(),
            _ => None,
        }
    }

    /// The `.m` source text a function was parsed from, if it is an interpreted
    /// function (used by `type` to display a function's contents).
    #[must_use]
    pub fn source_text(&self, name: &str) -> Option<String> {
        match self.funcs.get(name) {
            Some(Function::Interpreted { src, .. }) => Some(src.as_str().to_string()),
            _ => None,
        }
    }

    /// Every registered function name, **sorted** — the authoritative builtin
    /// set for `fm --list-builtins` / coverage reporting.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.funcs.keys().cloned().collect();
        names.sort();
        names
    }
}
