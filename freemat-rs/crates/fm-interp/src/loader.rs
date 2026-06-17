//! Loading `.m` files into the interpreter.
//!
//! A `.m` file parses via [`fm_parser::parse_program`] and is executed/defined
//! through the **same evaluator** — function files register their definitions,
//! script files run. This is what lets the 317 `toolbox/*.m` files run unchanged
//! (Stage 4 wires up the conformance suite over this).

use std::path::Path;

use fm_parser::{Program, parse_program};

use crate::error::InterpError;
use crate::interp::Interpreter;

impl Interpreter {
    /// Load and *define* the functions in a `.m` file (without executing a
    /// script body). Function files register their definitions; a script file
    /// is a no-op for definitions and must instead be run with [`Self::run`].
    ///
    /// # Errors
    /// Returns an [`InterpError`] if the file cannot be read or parsed.
    pub fn load_file(&mut self, path: impl AsRef<Path>) -> Result<(), InterpError> {
        let path = path.as_ref();
        let src = std::fs::read_to_string(path)
            .map_err(|e| InterpError::msg(format!("cannot read '{}': {e}", path.display())))?;
        self.define_source(&src)
    }

    /// Parse `src` and register any function definitions it contains (the
    /// function-file path). Scripts are ignored here (use [`Self::run`]).
    ///
    /// # Errors
    /// Returns an [`InterpError`] if `src` fails to parse.
    pub fn define_source(&mut self, src: &str) -> Result<(), InterpError> {
        let program = parse_program(src).map_err(|e| InterpError::msg(e.to_string()))?;
        if let Program::Functions(defs) = program {
            self.load_functions(defs, src);
        }
        Ok(())
    }
}
