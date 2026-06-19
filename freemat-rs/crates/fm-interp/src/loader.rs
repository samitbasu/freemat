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
        self.load_file_main(path).map(|_| ())
    }

    /// Like [`Self::load_file`], but returns the name of the public (first
    /// top-level) function the file defined, if any.
    ///
    /// In FreeMat a function file's *public* function is the first top-level
    /// `function` in the file, and its name may differ from the filename (the
    /// corpus has several such cases, e.g. `array/test_resize5.m` defines
    /// `test_resize4`). Callers that want to invoke the defined function rather
    /// than guess from the filename use this entry point.
    ///
    /// # Errors
    /// Returns an [`InterpError`] if the file cannot be read or parsed.
    pub fn load_file_main(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<Option<String>, InterpError> {
        let path = path.as_ref();
        let src = std::fs::read_to_string(path)
            .map_err(|e| InterpError::msg(format!("cannot read '{}': {e}", path.display())))?;
        // Record the absolute path so `which` can report where a function lives.
        let abs = std::fs::canonicalize(path)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| path.to_string_lossy().into_owned());
        self.define_source_main_from(&src, Some(abs))
    }

    /// Parse `src` and register any function definitions it contains (the
    /// function-file path). Scripts are ignored here (use [`Self::run`]).
    ///
    /// # Errors
    /// Returns an [`InterpError`] if `src` fails to parse.
    pub fn define_source(&mut self, src: &str) -> Result<(), InterpError> {
        self.define_source_main(src).map(|_| ())
    }

    /// Like [`Self::define_source`], returning the public function's name.
    ///
    /// # Errors
    /// Returns an [`InterpError`] if `src` fails to parse.
    pub fn define_source_main(&mut self, src: &str) -> Result<Option<String>, InterpError> {
        self.define_source_main_from(src, None)
    }

    /// Like [`Self::define_source_main`], recording the originating file `path`
    /// (if any) so `which` can report where the functions live.
    ///
    /// # Errors
    /// Returns an [`InterpError`] if `src` fails to parse.
    pub fn define_source_main_from(
        &mut self,
        src: &str,
        path: Option<String>,
    ) -> Result<Option<String>, InterpError> {
        let program = parse_program(src).map_err(|e| InterpError::msg(e.to_string()))?;
        if let Program::Functions(defs) = program {
            let main = defs.first().map(|d| d.name.clone());
            self.load_functions_from(defs, src, path);
            Ok(main)
        } else {
            Ok(None)
        }
    }
}
