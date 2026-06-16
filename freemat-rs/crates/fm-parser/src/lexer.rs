//! Hand-written lexer.
//!
//! Reproduces FreeMat's `Scanner` semantics in idiomatic Rust:
//! - numbers (int / float / scientific / `f` single-precision / `i`,`j`
//!   imaginary), and a Rust-port addition: hex literals `0x...`;
//! - single-quoted strings with `''` → `'` un-escaping;
//! - `%` line comments and `%{ ... %}` block comments (a MATLAB feature absent
//!   from FreeMat's C++ scanner — see `PROGRESS.md`);
//! - `...` line continuations;
//! - bracket-nesting tracking (`[ ] { } ( )`);
//! - the transpose-vs-string rule: `'` is a transpose operator when it follows
//!   an identifier / number / `) ] }` / another `'`, and a string literal start
//!   otherwise;
//! - **blob mode** for command syntax (`hold on`), toggled by the parser.
//!
//! The lexer is pull-based: the parser calls [`Lexer::next_token`] (and toggles
//! [`Lexer::set_blob_mode`]). Whitespace and newlines are emitted as tokens; the
//! parser decides when they are significant (they are, inside matrix/cell
//! literals). Byte offsets are tracked for every token's [`Span`].

use crate::error::{ParseError, Result};
use crate::span::Span;
use crate::token::{Token, TokenKind};

/// A pull-based lexer over a UTF-8 source string.
#[derive(Debug, Clone)]
pub struct Lexer<'src> {
    src: &'src str,
    bytes: &'src [u8],
    /// Current byte offset.
    pos: usize,
    /// `[ ] { }` nesting depth (clamped at 0), used for matrix-literal context.
    bracket_depth: i32,
    /// When set, identifiers/numbers after a command name are lexed as blobs.
    blob_mode: bool,
}

fn is_blank(b: u8) -> bool {
    b == b' ' || b == b'\t' || b == b'\r'
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

impl<'src> Lexer<'src> {
    /// Construct a lexer over `src`.
    #[must_use]
    pub fn new(src: &'src str) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
            bracket_depth: 0,
            blob_mode: false,
        }
    }

    /// The source text (for building diagnostics).
    #[must_use]
    pub fn source(&self) -> &'src str {
        self.src
    }

    /// Toggle command-syntax blob mode. The parser enables this once it has
    /// committed to a command-syntax statement.
    pub fn set_blob_mode(&mut self, on: bool) {
        self.blob_mode = on;
    }

    /// Whether the lexer is currently inside `[ ]` / `{ }`.
    #[must_use]
    pub fn in_bracket(&self) -> bool {
        self.bracket_depth > 0
    }

    /// The current byte offset (the parser snapshots this to backtrack).
    #[must_use]
    pub fn position(&self) -> usize {
        self.pos
    }

    fn byte_at(&self, idx: usize) -> u8 {
        self.bytes.get(idx).copied().unwrap_or(0)
    }

    fn cur(&self) -> u8 {
        self.byte_at(self.pos)
    }

    fn ahead(&self, n: usize) -> u8 {
        self.byte_at(self.pos + n)
    }

    fn prev(&self) -> u8 {
        if self.pos == 0 {
            0
        } else {
            self.byte_at(self.pos - 1)
        }
    }

    fn err(&self, span: Span, message: &str, label: &str) -> ParseError {
        ParseError::new(self.src.to_string(), span, message, label)
    }

    /// Lex and return the next token.
    pub fn next_token(&mut self) -> Result<Token> {
        loop {
            if self.pos >= self.bytes.len() {
                return Ok(Token::new(TokenKind::Eof, Span::empty(self.pos)));
            }
            let c = self.cur();
            // Line / block comments.
            if c == b'%' {
                if self.ahead(1) == b'{' && self.only_blank_before_on_line(self.pos) {
                    self.skip_block_comment()?;
                    continue;
                }
                self.skip_line_comment();
                continue;
            }
            // Line continuation `...`.
            if c == b'.' && self.ahead(1) == b'.' && self.ahead(2) == b'.' {
                self.skip_continuation();
                continue;
            }
            // Blob mode: grab command-syntax arguments verbatim.
            if self.blob_mode
                && !is_blank(c)
                && c != b'\n'
                && c != b';'
                && c != b','
                && c != b'\''
                && c != b'%'
            {
                return Ok(self.fetch_blob());
            }
            if c == b'\n' {
                let span = Span::new(self.pos, self.pos + 1);
                self.pos += 1;
                return Ok(Token::new(TokenKind::Newline, span));
            }
            if is_blank(c) {
                return Ok(self.fetch_whitespace());
            }
            if is_ident_start(c) {
                return Ok(self.fetch_identifier());
            }
            if c.is_ascii_digit() || (c == b'.' && self.ahead(1).is_ascii_digit()) {
                return self.fetch_number();
            }
            // Transpose-vs-string disambiguation.
            if c == b'\'' && !self.transpose_context() {
                return self.fetch_string();
            }
            return self.fetch_other();
        }
    }

    /// `'` is a transpose operator (not a string start) when the previous byte
    /// is `'`, `)`, `]`, `}`, or an identifier character.
    fn transpose_context(&self) -> bool {
        let p = self.prev();
        p == b'\'' || p == b')' || p == b']' || p == b'}' || is_ident_continue(p)
    }

    fn skip_line_comment(&mut self) {
        while self.pos < self.bytes.len() && self.cur() != b'\n' {
            self.pos += 1;
        }
    }

    /// True if only blanks separate the line start from `at` (for `%{`).
    fn only_blank_before_on_line(&self, at: usize) -> bool {
        let mut i = at;
        while i > 0 {
            let b = self.byte_at(i - 1);
            if b == b'\n' {
                break;
            }
            if !is_blank(b) {
                return false;
            }
            i -= 1;
        }
        // After `%{` only blanks may follow until the newline.
        let mut j = at + 2;
        while j < self.bytes.len() && self.byte_at(j) != b'\n' {
            if !is_blank(self.byte_at(j)) {
                return false;
            }
            j += 1;
        }
        true
    }

    fn skip_block_comment(&mut self) -> Result<()> {
        let start = self.pos;
        // Skip the opening `%{` line.
        self.pos += 2;
        let mut depth = 1usize;
        while self.pos < self.bytes.len() {
            if self.cur() == b'\n' {
                self.pos += 1;
                // Examine the next line for `%{` (nest) or `%}` (close).
                let line_start = self.pos;
                while self.pos < self.bytes.len() && is_blank(self.cur()) {
                    self.pos += 1;
                }
                if self.cur() == b'%'
                    && self.ahead(1) == b'}'
                    && self.rest_of_line_blank(self.pos + 2)
                {
                    depth -= 1;
                    // consume `%}` and the rest of its line.
                    self.pos += 2;
                    self.skip_line_comment();
                    if depth == 0 {
                        return Ok(());
                    }
                } else if self.cur() == b'%'
                    && self.ahead(1) == b'{'
                    && self.rest_of_line_blank(self.pos + 2)
                {
                    depth += 1;
                    self.pos += 2;
                } else {
                    // Ordinary content line; rewind to line start and skip it.
                    self.pos = line_start;
                    self.skip_line_comment();
                }
            } else {
                self.pos += 1;
            }
        }
        Err(self.err(
            Span::new(start, start + 2),
            "unterminated block comment",
            "no matching `%}` for this `%{`",
        ))
    }

    fn rest_of_line_blank(&self, from: usize) -> bool {
        let mut j = from;
        while j < self.bytes.len() && self.byte_at(j) != b'\n' {
            if !is_blank(self.byte_at(j)) {
                return false;
            }
            j += 1;
        }
        true
    }

    fn skip_continuation(&mut self) {
        self.pos += 3;
        while self.pos < self.bytes.len() && self.cur() != b'\n' {
            self.pos += 1;
        }
        if self.pos < self.bytes.len() && self.cur() == b'\n' {
            self.pos += 1;
        }
    }

    fn fetch_whitespace(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.bytes.len() && is_blank(self.cur()) {
            self.pos += 1;
        }
        Token::new(TokenKind::Space, Span::new(start, self.pos))
    }

    fn fetch_identifier(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.bytes.len() && is_ident_continue(self.cur()) {
            self.pos += 1;
        }
        let text = &self.src[start..self.pos];
        let kind = TokenKind::reserved(text).unwrap_or_else(|| TokenKind::Ident(text.to_string()));
        Token::new(kind, Span::new(start, self.pos))
    }

    fn fetch_string(&mut self) -> Result<Token> {
        let start = self.pos;
        // pos is at the opening quote.
        let mut i = self.pos + 1;
        let mut value = String::new();
        loop {
            if i >= self.bytes.len() || self.byte_at(i) == b'\n' {
                return Err(self
                    .err(
                        Span::new(start, i.min(self.bytes.len())),
                        "unterminated string literal",
                        "string is not closed before end of line",
                    )
                    .with_help(
                        "close the string with a single quote `'`; use `''` for a literal quote",
                    ));
            }
            if self.byte_at(i) == b'\'' {
                if self.byte_at(i + 1) == b'\'' {
                    // Escaped quote.
                    value.push('\'');
                    i += 2;
                    continue;
                }
                // Closing quote.
                i += 1;
                break;
            }
            // Copy one UTF-8 char (advance by char boundary).
            let ch_len = utf8_len(self.byte_at(i));
            value.push_str(&self.src[i..i + ch_len]);
            i += ch_len;
        }
        self.pos = i;
        Ok(Token::new(TokenKind::Str(value), Span::new(start, i)))
    }

    fn fetch_blob(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.bytes.len() {
            let b = self.cur();
            if b == b'\n' || is_blank(b) || b == b'%' || b == b',' || b == b';' {
                break;
            }
            self.pos += 1;
        }
        let text = self.src[start..self.pos].to_string();
        Token::new(TokenKind::Str(text), Span::new(start, self.pos))
    }

    fn fetch_number(&mut self) -> Result<Token> {
        let start = self.pos;
        // Hex literal (Rust-port addition; FreeMat's scanner lacked it).
        if self.cur() == b'0' && (self.ahead(1) == b'x' || self.ahead(1) == b'X') {
            let mut i = self.pos + 2;
            while i < self.bytes.len() && self.byte_at(i).is_ascii_hexdigit() {
                i += 1;
            }
            if i == self.pos + 2 {
                return Err(self.err(
                    Span::new(start, i),
                    "malformed hexadecimal literal",
                    "expected hex digits after `0x`",
                ));
            }
            let text = self.src[start..i].to_string();
            self.pos = i;
            return Ok(Token::new(TokenKind::Real(text), Span::new(start, i)));
        }

        let mut len = self.digits(0);
        let mut look = len;
        // Fractional part.
        if self.ahead(look) == b'.' {
            look += 1;
            len = self.digits(look);
            look += len;
        }
        // Exponent.
        if self.ahead(look) == b'e' || self.ahead(look) == b'E' {
            look += 1;
            if self.ahead(look) == b'+' || self.ahead(look) == b'-' {
                look += 1;
            }
            len = self.digits(look);
            look += len;
        }
        // Back off if we swept a `.` that actually begins `...`, `.*`, `./`,
        // `.\`, `.^`, or `.'` — that `.` is the start of an operator, not a
        // fractional point, so it (and any suffix) cannot belong to the number.
        if look > 0 && self.ahead(look - 1) == b'.' {
            let nxt = self.ahead(look);
            if (nxt == b'.' && self.ahead(look + 1) == b'.')
                || matches!(nxt, b'*' | b'/' | b'\\' | b'^' | b'\'')
            {
                look -= 1;
                let text = self.src[start..start + look].to_string();
                self.pos = start + look;
                return Ok(Token::new(
                    TokenKind::Real(text),
                    Span::new(start, self.pos),
                ));
            }
        }
        // `num_len` is the digit-only span; the `text` we hand the parser must
        // exclude the `f`/`d`/`i`/`j` suffix so it parses as a plain number.
        let num_len = look;
        let mut single = false;
        if self.ahead(look) == b'f' || self.ahead(look) == b'F' {
            single = true;
            look += 1;
        } else if self.ahead(look) == b'd' || self.ahead(look) == b'D' {
            look += 1;
        }
        let mut imaginary = false;
        if matches!(self.ahead(look), b'i' | b'I' | b'j' | b'J') {
            imaginary = true;
            look += 1;
        }
        let text = self.src[start..start + num_len].to_string();
        self.pos = start + look;
        let kind = match (imaginary, single) {
            (false, false) => TokenKind::Real(text),
            (false, true) => TokenKind::RealF(text),
            (true, false) => TokenKind::Imag(text),
            (true, true) => TokenKind::ImagF(text),
        };
        Ok(Token::new(kind, Span::new(start, self.pos)))
    }

    /// Count consecutive digits starting `offset` bytes ahead of `pos`.
    fn digits(&self, offset: usize) -> usize {
        let mut n = 0;
        while self.ahead(offset + n).is_ascii_digit() {
            n += 1;
        }
        n
    }

    fn fetch_other(&mut self) -> Result<Token> {
        let start = self.pos;
        // Two-char `.`-prefixed operators.
        if self.cur() == b'.'
            && let Some(kind) = match self.ahead(1) {
                b'*' => Some(TokenKind::DotTimes),
                b'/' => Some(TokenKind::DotRDivide),
                b'\\' => Some(TokenKind::DotLDivide),
                b'^' => Some(TokenKind::DotPower),
                b'\'' => Some(TokenKind::DotTranspose),
                _ => None,
            }
        {
            self.pos += 2;
            return Ok(Token::new(kind, Span::new(start, self.pos)));
        }
        // Two-char comparison / logical operators.
        if let Some(kind) = match (self.cur(), self.ahead(1)) {
            (b'<', b'=') => Some(TokenKind::Le),
            (b'>', b'=') => Some(TokenKind::Ge),
            (b'=', b'=') => Some(TokenKind::Eq),
            (b'~', b'=') => Some(TokenKind::Ne),
            (b'&', b'&') => Some(TokenKind::AndAnd),
            (b'|', b'|') => Some(TokenKind::OrOr),
            _ => None,
        } {
            self.pos += 2;
            return Ok(Token::new(kind, Span::new(start, self.pos)));
        }
        // Single-char tokens.
        let c = self.cur();
        let kind = match c {
            b'+' => TokenKind::Plus,
            b'-' => TokenKind::Minus,
            b'*' => TokenKind::Times,
            b'/' => TokenKind::RDivide,
            b'\\' => TokenKind::LDivide,
            b'^' => TokenKind::Power,
            b'\'' => TokenKind::Transpose,
            b'<' => TokenKind::Lt,
            b'>' => TokenKind::Gt,
            b'&' => TokenKind::Amp,
            b'|' => TokenKind::Pipe,
            b'~' => TokenKind::Not,
            b':' => TokenKind::Colon,
            b'@' => TokenKind::At,
            b'=' => TokenKind::Assign,
            b',' => TokenKind::Comma,
            b';' => TokenKind::Semicolon,
            b'.' => TokenKind::Dot,
            b'(' => TokenKind::LParen,
            b')' => TokenKind::RParen,
            b'[' => {
                self.bracket_depth += 1;
                TokenKind::LBracket
            }
            b']' => {
                self.bracket_depth = (self.bracket_depth - 1).max(0);
                TokenKind::RBracket
            }
            b'{' => {
                self.bracket_depth += 1;
                TokenKind::LBrace
            }
            b'}' => {
                self.bracket_depth = (self.bracket_depth - 1).max(0);
                TokenKind::RBrace
            }
            _ => {
                let ch_len = utf8_len(c);
                let span = Span::new(start, start + ch_len);
                return Err(self.err(span, "unexpected character", "not valid here"));
            }
        };
        self.pos += 1;
        Ok(Token::new(kind, Span::new(start, self.pos)))
    }
}

/// Byte length of the UTF-8 character whose lead byte is `b`.
fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else {
        4
    }
}

/// Tokenize `src` fully, returning every token including `Space`/`Newline` and a
/// final `Eof`. Used by tests and as a convenience for non-incremental callers.
pub fn tokenize(src: &str) -> Result<Vec<Token>> {
    let mut lexer = Lexer::new(src);
    let mut out = Vec::new();
    loop {
        let tok = lexer.next_token()?;
        let is_eof = tok.kind == TokenKind::Eof;
        out.push(tok);
        if is_eof {
            break;
        }
    }
    Ok(out)
}
