//! `fm-parser` — FreeMat-rs lexer, recursive-descent parser, and AST.
//!
//! Turns FreeMat / MATLAB source text into a span-carrying [`ast::Program`],
//! reporting syntax errors as `miette` [`error::ParseError`] diagnostics (each
//! anchored to a byte span in the source).
//!
//! # Pipeline
//! 1. [`lexer::Lexer`] — a hand-written, pull-based lexer. It handles the
//!    transpose-vs-string rule, command-syntax blob mode, numbers (incl. a
//!    Rust-port hex extension), strings with `''` escaping, `%` and `%{ %}`
//!    comments, `...` continuations, and bracket-nesting tracking.
//! 2. [`parser::Parser`] — a recursive-descent / precedence-climbing parser
//!    reproducing FreeMat's grammar and operator precedence.
//!
//! # Quick start
//! ```
//! use fm_parser::{parse_program, ast::Program};
//! let prog = parse_program("x = 1 + 2*3;").unwrap();
//! assert!(matches!(prog, Program::Script(_)));
//! ```
//!
//! Every AST node carries a byte [`span::Span`]; diagnostics use those spans so a
//! malformed snippet renders annotated and source-highlighted (with miette's
//! `fancy` feature, enabled by the consumer).

pub mod ast;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod token;

pub use ast::Program;
pub use error::{ParseError, Result};
pub use parser::Parser;
pub use span::Span;
pub use token::{Token, TokenKind};

/// Parse a complete source unit (a `.m` script or function file).
pub fn parse_program(src: &str) -> Result<Program> {
    Parser::new(src)?.parse_program()
}

/// Parse a statement list (e.g. a REPL line or a script body).
pub fn parse_statements(src: &str) -> Result<Vec<ast::Stmt>> {
    Parser::new(src)?.parse_statements()
}

/// Parse a single expression, requiring it to consume all of `src`.
pub fn parse_expression(src: &str) -> Result<ast::Expr> {
    Parser::new(src)?.parse_expression()
}

/// Lex `src` to completion, returning all tokens (including layout) plus `Eof`.
pub fn tokenize(src: &str) -> Result<Vec<Token>> {
    lexer::tokenize(src)
}
