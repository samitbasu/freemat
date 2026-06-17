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
    },
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

    /// Register an interpreted `.m` function.
    pub fn add_interpreted(&mut self, def: FunctionDef, src: Arc<String>) {
        self.funcs.insert(
            def.name.clone(),
            Function::Interpreted {
                def: Arc::new(def),
                src,
            },
        );
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
}
