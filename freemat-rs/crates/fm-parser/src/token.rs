//! Token kinds produced by the lexer.
//!
//! Mirrors the token set of FreeMat's `Token.hpp` (`TOK_*`), but expressed as a
//! Rust enum. Reserved words become dedicated variants; operators are explicit
//! variants rather than raw character codes so the parser matches on them
//! exhaustively.

use crate::span::Span;

/// A lexed token: a [`TokenKind`] plus its byte [`Span`] in the source.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// What kind of token this is (and any carried text/literal payload).
    pub kind: TokenKind,
    /// Byte range of this token in the source.
    pub span: Span,
}

impl Token {
    /// Construct a token from a kind and span.
    #[must_use]
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// The lexical classes FreeMat source decomposes into.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // --- literals & names ---
    /// An identifier (variable / function name).
    Ident(String),
    /// A real numeric literal, double precision (text preserved for the parser).
    Real(String),
    /// A real numeric literal, single precision (trailing `f`/`F`).
    RealF(String),
    /// An imaginary literal, double precision (trailing `i`/`j`).
    Imag(String),
    /// An imaginary literal, single precision (`f` + `i`/`j`).
    ImagF(String),
    /// A string literal (quotes stripped, `''` collapsed to `'`).
    Str(String),

    // --- reserved words ---
    /// `break`
    Break,
    /// `case`
    Case,
    /// `catch`
    Catch,
    /// `continue`
    Continue,
    /// `dbstep`
    DbStep,
    /// `dbtrace`
    DbTrace,
    /// `dbup`
    DbUp,
    /// `dbdown`
    DbDown,
    /// `else`
    Else,
    /// `elseif`
    ElseIf,
    /// `end`
    End,
    /// `for`
    For,
    /// `function`
    Function,
    /// `global`
    Global,
    /// `if`
    If,
    /// `keyboard`
    Keyboard,
    /// `otherwise`
    Otherwise,
    /// `persistent`
    Persistent,
    /// `quit`
    Quit,
    /// `retall`
    Retall,
    /// `return`
    Return,
    /// `switch`
    Switch,
    /// `try`
    Try,
    /// `while`
    While,

    // --- single-char operators / punctuation ---
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Times,
    /// `/`
    RDivide,
    /// `\`
    LDivide,
    /// `^`
    Power,
    /// `'` (matrix transpose; the lexer disambiguates from a string start)
    Transpose,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `&`
    Amp,
    /// `|`
    Pipe,
    /// `~`
    Not,
    /// `:`
    Colon,
    /// `@`
    At,
    /// `=`
    Assign,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `,`
    Comma,
    /// `;`
    Semicolon,
    /// `.` (field access; element-wise variants are their own tokens)
    Dot,

    // --- multi-char operators ---
    /// `.*`
    DotTimes,
    /// `./`
    DotRDivide,
    /// `.\`
    DotLDivide,
    /// `.^`
    DotPower,
    /// `.'`
    DotTranspose,
    /// `<=`
    Le,
    /// `>=`
    Ge,
    /// `==`
    Eq,
    /// `~=`
    Ne,
    /// `&&`
    AndAnd,
    /// `||`
    OrOr,

    // --- layout ---
    /// Run of blanks (spaces / tabs / carriage returns). Significant inside
    /// `[ ]` / `{ }`; the parser otherwise skips it.
    Space,
    /// A statement-terminating newline.
    Newline,
    /// End of input.
    Eof,
}

impl TokenKind {
    /// The reserved word for `ident`, or `None` if it is an ordinary identifier.
    #[must_use]
    pub fn reserved(ident: &str) -> Option<TokenKind> {
        Some(match ident {
            "break" => TokenKind::Break,
            "case" => TokenKind::Case,
            "catch" => TokenKind::Catch,
            "continue" => TokenKind::Continue,
            "dbtrace" => TokenKind::DbTrace,
            // NOTE: `dbstep`/`dbup`/`dbdown` are intentionally *not* reserved —
            // they lex as ordinary identifiers so they dispatch to the `db*`
            // builtins (`fm-builtins/src/debug.rs`) like `dbstop`/`dbcont`, which
            // implement them against the live debug session. (The `DbStep`/
            // `DbUp`/`DbDown` token kinds remain for back-compat but are unused.)
            "else" => TokenKind::Else,
            "elseif" => TokenKind::ElseIf,
            "end" => TokenKind::End,
            "for" => TokenKind::For,
            "function" => TokenKind::Function,
            "global" => TokenKind::Global,
            "if" => TokenKind::If,
            "keyboard" => TokenKind::Keyboard,
            "otherwise" => TokenKind::Otherwise,
            "persistent" => TokenKind::Persistent,
            "quit" => TokenKind::Quit,
            "retall" => TokenKind::Retall,
            "return" => TokenKind::Return,
            "switch" => TokenKind::Switch,
            "try" => TokenKind::Try,
            "while" => TokenKind::While,
            _ => return None,
        })
    }

    /// Whether this token can act as a binary operator in an expression.
    #[must_use]
    pub fn is_binary_operator(&self) -> bool {
        matches!(
            self,
            TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Times
                | TokenKind::RDivide
                | TokenKind::LDivide
                | TokenKind::Power
                | TokenKind::Lt
                | TokenKind::Gt
                | TokenKind::Colon
                | TokenKind::Le
                | TokenKind::Ge
                | TokenKind::Eq
                | TokenKind::Ne
                | TokenKind::OrOr
                | TokenKind::AndAnd
                | TokenKind::DotTimes
                | TokenKind::DotRDivide
                | TokenKind::DotLDivide
                | TokenKind::DotPower
                | TokenKind::Pipe
                | TokenKind::Amp
        )
    }

    /// Whether this token can start a unary (prefix) expression.
    #[must_use]
    pub fn is_unary_operator(&self) -> bool {
        matches!(self, TokenKind::Plus | TokenKind::Minus | TokenKind::Not)
    }

    /// A short human-readable name, used in diagnostics ("expected `)`").
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            TokenKind::Ident(s) => format!("identifier `{s}`"),
            TokenKind::Real(s) | TokenKind::RealF(s) | TokenKind::Imag(s) | TokenKind::ImagF(s) => {
                format!("number `{s}`")
            }
            TokenKind::Str(_) => "string literal".to_string(),
            TokenKind::Space => "whitespace".to_string(),
            TokenKind::Newline => "newline".to_string(),
            TokenKind::Eof => "end of input".to_string(),
            other => format!("`{}`", other.symbol()),
        }
    }

    /// The literal symbol for operator / keyword tokens (for diagnostics).
    fn symbol(&self) -> &'static str {
        match self {
            TokenKind::Plus => "+",
            TokenKind::Minus => "-",
            TokenKind::Times => "*",
            TokenKind::RDivide => "/",
            TokenKind::LDivide => "\\",
            TokenKind::Power => "^",
            TokenKind::Transpose => "'",
            TokenKind::Lt => "<",
            TokenKind::Gt => ">",
            TokenKind::Amp => "&",
            TokenKind::Pipe => "|",
            TokenKind::Not => "~",
            TokenKind::Colon => ":",
            TokenKind::At => "@",
            TokenKind::Assign => "=",
            TokenKind::LParen => "(",
            TokenKind::RParen => ")",
            TokenKind::LBracket => "[",
            TokenKind::RBracket => "]",
            TokenKind::LBrace => "{",
            TokenKind::RBrace => "}",
            TokenKind::Comma => ",",
            TokenKind::Semicolon => ";",
            TokenKind::Dot => ".",
            TokenKind::DotTimes => ".*",
            TokenKind::DotRDivide => "./",
            TokenKind::DotLDivide => ".\\",
            TokenKind::DotPower => ".^",
            TokenKind::DotTranspose => ".'",
            TokenKind::Le => "<=",
            TokenKind::Ge => ">=",
            TokenKind::Eq => "==",
            TokenKind::Ne => "~=",
            TokenKind::AndAnd => "&&",
            TokenKind::OrOr => "||",
            TokenKind::Break => "break",
            TokenKind::Case => "case",
            TokenKind::Catch => "catch",
            TokenKind::Continue => "continue",
            TokenKind::DbStep => "dbstep",
            TokenKind::DbTrace => "dbtrace",
            TokenKind::DbUp => "dbup",
            TokenKind::DbDown => "dbdown",
            TokenKind::Else => "else",
            TokenKind::ElseIf => "elseif",
            TokenKind::End => "end",
            TokenKind::For => "for",
            TokenKind::Function => "function",
            TokenKind::Global => "global",
            TokenKind::If => "if",
            TokenKind::Keyboard => "keyboard",
            TokenKind::Otherwise => "otherwise",
            TokenKind::Persistent => "persistent",
            TokenKind::Quit => "quit",
            TokenKind::Retall => "retall",
            TokenKind::Return => "return",
            TokenKind::Switch => "switch",
            TokenKind::Try => "try",
            TokenKind::While => "while",
            // Variants with payload are handled in `describe`.
            TokenKind::Ident(_)
            | TokenKind::Real(_)
            | TokenKind::RealF(_)
            | TokenKind::Imag(_)
            | TokenKind::ImagF(_)
            | TokenKind::Str(_)
            | TokenKind::Space
            | TokenKind::Newline
            | TokenKind::Eof => "?",
        }
    }
}
