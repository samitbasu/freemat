//! Recursive-descent parser.
//!
//! Reproduces FreeMat's grammar and operator precedence (`Parser.cpp`) in
//! idiomatic Rust, producing the span-carrying [`crate::ast`] types.
//!
//! Whitespace is significant only inside `[ ]` / `{ }` literals (where a space
//! separates columns). Everywhere else `Space` tokens are skipped. We model
//! FreeMat's `m_ignorews` flag stack with a `ws_significant` counter and a
//! one-token lookahead buffer.
//!
//! Statement parsing uses bounded backtracking (snapshotting the lexer, which is
//! `Clone`) for the cases FreeMat parses tentatively: assignment, multi-output
//! assignment `[a,b]=...`, and command syntax `hold on`.

use crate::ast::*;
use crate::error::{ParseError, Result};
use crate::lexer::Lexer;
use crate::span::Span;
use crate::token::{Token, TokenKind};

/// Operator precedence levels, matching FreeMat's `precedence()`.
fn precedence(kind: &TokenKind) -> u8 {
    match kind {
        TokenKind::OrOr => 1,
        TokenKind::AndAnd => 2,
        TokenKind::Pipe => 3,
        TokenKind::Amp => 4,
        TokenKind::Lt
        | TokenKind::Gt
        | TokenKind::Le
        | TokenKind::Ge
        | TokenKind::Eq
        | TokenKind::Ne => 5,
        TokenKind::Colon => 6,
        TokenKind::Plus | TokenKind::Minus => 7,
        TokenKind::Times
        | TokenKind::RDivide
        | TokenKind::LDivide
        | TokenKind::DotTimes
        | TokenKind::DotRDivide
        | TokenKind::DotLDivide => 8,
        TokenKind::Power | TokenKind::DotPower => 10,
        _ => 1,
    }
}

/// Only `^` / `.^` are right-associative.
fn right_associative(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::Power | TokenKind::DotPower)
}

fn binary_op(kind: &TokenKind) -> Option<BinaryOp> {
    Some(match kind {
        TokenKind::Plus => BinaryOp::Add,
        TokenKind::Minus => BinaryOp::Sub,
        TokenKind::Times => BinaryOp::Mul,
        TokenKind::RDivide => BinaryOp::RDiv,
        TokenKind::LDivide => BinaryOp::LDiv,
        TokenKind::Power => BinaryOp::Pow,
        TokenKind::DotTimes => BinaryOp::ElMul,
        TokenKind::DotRDivide => BinaryOp::ElRDiv,
        TokenKind::DotLDivide => BinaryOp::ElLDiv,
        TokenKind::DotPower => BinaryOp::ElPow,
        TokenKind::Lt => BinaryOp::Lt,
        TokenKind::Gt => BinaryOp::Gt,
        TokenKind::Le => BinaryOp::Le,
        TokenKind::Ge => BinaryOp::Ge,
        TokenKind::Eq => BinaryOp::Eq,
        TokenKind::Ne => BinaryOp::Ne,
        TokenKind::Amp => BinaryOp::And,
        TokenKind::Pipe => BinaryOp::Or,
        TokenKind::AndAnd => BinaryOp::ShortAnd,
        TokenKind::OrOr => BinaryOp::ShortOr,
        _ => return None,
    })
}

/// The recursive-descent parser.
pub struct Parser<'src> {
    lexer: Lexer<'src>,
    /// One-token lookahead buffer (the most recently lexed token).
    lookahead: Token,
    src: &'src str,
    /// > 0 when whitespace is a significant token (inside matrix/cell literals).
    ws_significant: u32,
    /// The deepest-reaching error seen so far (FreeMat's `lastpos`/`lasterr`).
    /// Tentative parses backtrack and lose their error; this remembers the one
    /// that consumed the most input so the user sees the most relevant message.
    furthest: Option<ParseError>,
}

/// The result of parsing a call's argument list: positional arguments paired
/// with IDL-style keyword arguments (`(name, optional value)`).
type CallArgs = (Vec<Expr>, Vec<(String, Option<Expr>)>);

/// A snapshot used to backtrack a tentative parse.
struct Save<'src> {
    lexer: Lexer<'src>,
    lookahead: Token,
}

impl<'src> Parser<'src> {
    /// Construct a parser over `src`.
    pub fn new(src: &'src str) -> Result<Self> {
        let mut lexer = Lexer::new(src);
        // Skip a leading insignificant `Space` (a file that starts with indented
        // content — e.g. an indented comment before `function` — must classify
        // the same as one that doesn't). Outside brackets ws is insignificant,
        // mirroring `refill`'s whitespace handling.
        let mut lookahead = lexer.next_token()?;
        while lookahead.kind == TokenKind::Space {
            lookahead = lexer.next_token()?;
        }
        Ok(Self {
            lexer,
            lookahead,
            src,
            ws_significant: 0,
            furthest: None,
        })
    }

    /// Record `e` if it reaches further than any previously seen error, then
    /// return it unchanged.
    fn record(&mut self, e: ParseError) -> ParseError {
        let deeper = self
            .furthest
            .as_ref()
            .is_none_or(|f| e.span().start >= f.span().start);
        if deeper {
            self.furthest = Some(e.clone_shallow());
        }
        e
    }

    /// On a tentative-parse fallback failure, prefer the deepest error seen.
    fn best(&self, fallback: ParseError) -> ParseError {
        match &self.furthest {
            Some(f) if f.span().start > fallback.span().start => f.clone_shallow(),
            _ => fallback,
        }
    }

    // --- token plumbing -----------------------------------------------------

    fn save(&self) -> Save<'src> {
        Save {
            lexer: self.lexer.clone(),
            lookahead: self.lookahead.clone(),
        }
    }

    fn restore(&mut self, s: Save<'src>) {
        self.lexer = s.lexer;
        self.lookahead = s.lookahead;
    }

    fn set_blob_mode(&mut self, on: bool) {
        self.lexer.set_blob_mode(on);
    }

    /// Pull the next token from the lexer, recording any lexical error so a
    /// later backtrack does not bury it.
    fn lex_next(&mut self) -> Result<Token> {
        self.lexer.next_token().map_err(|e| self.record(e))
    }

    /// Refill the lookahead from the lexer, honoring whitespace significance.
    fn refill(&mut self) -> Result<()> {
        loop {
            self.lookahead = self.lex_next()?;
            if self.ws_significant == 0 && self.lookahead.kind == TokenKind::Space {
                continue;
            }
            return Ok(());
        }
    }

    /// The current lookahead token's kind.
    fn peek(&self) -> &TokenKind {
        &self.lookahead.kind
    }

    /// The current lookahead token's span.
    fn peek_span(&self) -> Span {
        self.lookahead.span
    }

    fn at(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(kind)
    }

    /// Consume and return the current token, refilling the lookahead.
    fn bump(&mut self) -> Result<Token> {
        let tok = self.lookahead.clone();
        self.refill()?;
        Ok(tok)
    }

    fn eat(&mut self, kind: &TokenKind) -> Result<bool> {
        if self.at(kind) {
            self.bump()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn expect(&mut self, kind: &TokenKind, because: &str) -> Result<Token> {
        if self.at(kind) {
            self.bump()
        } else {
            Err(self.error_here(&format!("expected {} {}", kind.describe(), because)))
        }
    }

    fn error_here(&mut self, message: &str) -> ParseError {
        let label = format!("found {} here", self.peek().describe());
        let e = ParseError::new(self.src.to_string(), self.peek_span(), message, label);
        self.record(e)
    }

    fn error_at(&mut self, span: Span, message: &str, label: &str) -> ParseError {
        let e = ParseError::new(self.src.to_string(), span, message, label);
        self.record(e)
    }

    /// Run `f` with whitespace significant (matrix/cell context).
    fn with_ws<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        self.ws_significant += 1;
        let r = f(self);
        self.ws_significant -= 1;
        // Re-skip a leading space the new flag now ignores.
        if self.ws_significant == 0 && self.lookahead.kind == TokenKind::Space {
            self.refill()?;
        }
        r
    }

    /// Skip statement separators (`;`, `,`, newlines) when starting a block.
    fn skip_separators(&mut self) -> Result<()> {
        while matches!(
            self.peek(),
            TokenKind::Semicolon | TokenKind::Comma | TokenKind::Newline
        ) {
            self.bump()?;
        }
        Ok(())
    }

    // --- entry points -------------------------------------------------------

    /// Parse a whole source unit (script or function file).
    pub fn parse_program(&mut self) -> Result<Program> {
        self.skip_separators()?;
        if self.at(&TokenKind::Function) {
            let mut funcs = Vec::new();
            while self.at(&TokenKind::Function) {
                funcs.push(self.function_definition()?);
                self.skip_separators()?;
            }
            self.expect_eof()?;
            Ok(Program::Functions(funcs))
        } else {
            let stmts = self.statement_list()?;
            self.expect_eof()?;
            Ok(Program::Script(stmts))
        }
    }

    /// Parse a statement list (used for scripts and REPL input).
    pub fn parse_statements(&mut self) -> Result<Vec<Stmt>> {
        let stmts = self.statement_list()?;
        self.expect_eof()?;
        Ok(stmts)
    }

    /// Parse a single expression and require it consumes all input.
    pub fn parse_expression(&mut self) -> Result<Expr> {
        let e = self.expression()?;
        self.expect_eof()?;
        Ok(e)
    }

    fn expect_eof(&mut self) -> Result<()> {
        if self.at(&TokenKind::Eof) {
            Ok(())
        } else {
            Err(self.error_here("unexpected trailing input"))
        }
    }

    // --- statements ---------------------------------------------------------

    fn statement_list(&mut self) -> Result<Vec<Stmt>> {
        let mut out = Vec::new();
        loop {
            self.skip_separators()?;
            if self.block_terminator() {
                break;
            }
            let (kind, span) = self.statement()?;
            let terminator = self.statement_terminator()?;
            out.push(Stmt {
                kind,
                terminator,
                span,
            });
        }
        Ok(out)
    }

    /// Whether the current token ends an enclosing block / the input.
    fn block_terminator(&self) -> bool {
        matches!(
            self.peek(),
            TokenKind::Eof
                | TokenKind::End
                | TokenKind::Else
                | TokenKind::ElseIf
                | TokenKind::Case
                | TokenKind::Otherwise
                | TokenKind::Catch
        )
    }

    /// Consume the terminator after a statement, returning its display mode.
    fn statement_terminator(&mut self) -> Result<Terminator> {
        match self.peek() {
            TokenKind::Semicolon => {
                self.bump()?;
                Ok(Terminator::Quiet)
            }
            TokenKind::Comma | TokenKind::Newline => {
                self.bump()?;
                Ok(Terminator::Display)
            }
            // End of input / enclosing block: an implicit display terminator.
            _ if self.block_terminator() => Ok(Terminator::Display),
            other => {
                let msg = format!(
                    "expected `;`, `,`, or newline after statement, found {}",
                    other.describe()
                );
                let here = self.error_here(&msg);
                // A deeper error from a backtracked tentative parse (e.g. a
                // malformed assignment RHS) is more relevant than this surface
                // "expected terminator" complaint.
                Err(self.best(here))
            }
        }
    }

    fn statement(&mut self) -> Result<(StmtKind, Span)> {
        let start = self.peek_span();
        match self.peek() {
            TokenKind::For => self.for_statement(),
            TokenKind::While => self.while_statement(),
            TokenKind::If => self.if_statement(),
            TokenKind::Switch => self.switch_statement(),
            TokenKind::Try => self.try_statement(),
            TokenKind::Global => self.declaration(true),
            TokenKind::Persistent => self.declaration(false),
            TokenKind::Function => {
                let f = self.function_definition()?;
                let span = f.span;
                Ok((StmtKind::FunctionDef(Box::new(f)), span))
            }
            TokenKind::Break => self.keyword_stmt(KeywordStmt::Break, start),
            TokenKind::Continue => self.keyword_stmt(KeywordStmt::Continue, start),
            TokenKind::Return => self.return_stmt(start),
            TokenKind::Retall => self.keyword_stmt(KeywordStmt::Retall, start),
            TokenKind::Keyboard => self.keyword_stmt(KeywordStmt::Keyboard, start),
            TokenKind::Quit => self.keyword_stmt(KeywordStmt::Quit, start),
            TokenKind::DbUp => self.keyword_stmt(KeywordStmt::DbUp, start),
            TokenKind::DbDown => self.keyword_stmt(KeywordStmt::DbDown, start),
            TokenKind::Ident(_) => self.ident_statement(),
            TokenKind::LBracket => self.bracket_statement(),
            _ => {
                let e = self.expression()?;
                let span = e.span;
                Ok((StmtKind::Expr(e), span))
            }
        }
    }

    fn keyword_stmt(&mut self, kw: KeywordStmt, start: Span) -> Result<(StmtKind, Span)> {
        self.bump()?;
        Ok((StmtKind::Keyword(kw), start))
    }

    /// `return` (FreeMat allows a trailing expression, e.g. `return (p==q);`).
    ///
    /// FreeMat parses `return` as a singleton statement and `return` exits the
    /// function immediately, so any trailing expression on the same line is dead
    /// code. We accept and discard it so files like `tests/io/test_file1.m`
    /// (which end in `return (p==q);`) load. If no expression follows, this is a
    /// plain `return`.
    fn return_stmt(&mut self, start: Span) -> Result<(StmtKind, Span)> {
        self.bump()?;
        // A terminator / end-of-block means a bare `return`.
        if !matches!(
            self.peek(),
            TokenKind::Semicolon | TokenKind::Comma | TokenKind::Newline
        ) && !self.block_terminator()
        {
            // Parse and discard the trailing expression (dead code after return).
            let _ = self.expression()?;
        }
        Ok((StmtKind::Keyword(KeywordStmt::Return), start))
    }

    fn declaration(&mut self, global: bool) -> Result<(StmtKind, Span)> {
        let start = self.peek_span();
        self.bump()?;
        let mut names = Vec::new();
        let mut span = start;
        while let TokenKind::Ident(_) = self.peek() {
            let tok = self.bump()?;
            span = span.merge(tok.span);
            if let TokenKind::Ident(n) = tok.kind {
                names.push(n);
            }
        }
        let kind = if global {
            StmtKind::Global(names)
        } else {
            StmtKind::Persistent(names)
        };
        Ok((kind, span))
    }

    /// A statement starting with an identifier: assignment, command syntax, or a
    /// bare expression — disambiguated with bounded backtracking.
    fn ident_statement(&mut self) -> Result<(StmtKind, Span)> {
        // 1. Try assignment: `lhs = rhs`.
        let snap = self.save();
        if let Ok(res) = self.try_assignment() {
            return Ok(res);
        }
        self.restore(snap);

        // 2. Try command syntax: `name arg1 arg2`.
        let snap = self.save();
        if let Ok(res) = self.try_command() {
            return Ok(res);
        }
        self.restore(snap);

        // 3. Fall back to a bare expression statement. If even that fails,
        // report the deepest-reaching error seen across the tentative attempts
        // (FreeMat's `lastpos`/`lasterr` behaviour) rather than the shallow one.
        let e = self.expression().map_err(|fallback| self.best(fallback))?;
        let span = e.span;
        Ok((StmtKind::Expr(e), span))
    }

    fn try_assignment(&mut self) -> Result<(StmtKind, Span)> {
        let lhs = self.postfix_expr()?;
        if !self.at(&TokenKind::Assign) {
            return Err(self.error_here("not an assignment"));
        }
        self.bump()?;
        let rhs = self.expression()?;
        let span = lhs.span.merge(rhs.span);
        Ok((StmtKind::Assign { lhs, rhs }, span))
    }

    /// Try to parse command syntax: an identifier, then a significant space,
    /// then verbatim word/string arguments. Mirrors FreeMat's `specialFunctionCall`.
    fn try_command(&mut self) -> Result<(StmtKind, Span)> {
        let TokenKind::Ident(name) = self.peek().clone() else {
            return Err(self.error_here("not a command"));
        };
        let start = self.peek_span();
        // The lookahead currently holds the command-name identifier (lexed with
        // whitespace insignificant, which is fine — it is not a space). Switch
        // whitespace on and consume it raw so the following `Space` survives.
        self.ws_significant += 1;
        let name_tok = self.bump_keep_ws()?; // consume the identifier
        debug_assert!(matches!(name_tok.kind, TokenKind::Ident(_)));
        if self.lookahead.kind != TokenKind::Space {
            self.ws_significant -= 1;
            return Err(self.error_here("not a command (no space after name)"));
        }
        // Lookahead-check the token after the space in *non-blob* mode on a
        // throwaway lexer clone (FreeMat's `t_lex`): a `;`/newline/`(`/`,`/`=`,
        // EOF, or a binary operator means this is an ordinary expression, not a
        // command.
        {
            let mut probe = self.lexer.clone();
            // current lookahead is the space; the probe is positioned after it.
            let after_space = probe.next_token()?;
            if matches!(
                after_space.kind,
                TokenKind::Semicolon
                    | TokenKind::Newline
                    | TokenKind::LParen
                    | TokenKind::Comma
                    | TokenKind::Assign
                    | TokenKind::Eof
            ) || after_space.kind.is_binary_operator()
            {
                self.ws_significant -= 1;
                return Err(self.error_here("not a command"));
            }
        }
        // Enable blob mode, then consume the space so the first argument is
        // lexed verbatim.
        self.set_blob_mode(true);
        self.bump_keep_ws()?; // consume the space (next token lexed as a blob)
        let mut args = Vec::new();
        let mut span = start;
        while !matches!(
            self.lookahead.kind,
            TokenKind::Semicolon | TokenKind::Newline | TokenKind::Comma | TokenKind::Eof
        ) {
            match &self.lookahead.kind {
                TokenKind::Str(s) => {
                    args.push(s.clone());
                    span = span.merge(self.lookahead.span);
                    self.bump_keep_ws()?;
                }
                TokenKind::Space => {
                    self.bump_keep_ws()?;
                }
                _ => {
                    self.set_blob_mode(false);
                    self.ws_significant -= 1;
                    return Err(self.error_here("not a command"));
                }
            }
        }
        self.set_blob_mode(false);
        self.ws_significant -= 1;
        // Re-skip insignificant whitespace now.
        if self.lookahead.kind == TokenKind::Space {
            self.refill()?;
        }
        Ok((StmtKind::Command { name, args }, span))
    }

    /// Bump while keeping whitespace significant.
    fn bump_keep_ws(&mut self) -> Result<Token> {
        let tok = self.lookahead.clone();
        self.lookahead = self.lex_next()?;
        Ok(tok)
    }

    /// A statement starting with `[`: try `[a,b]=rhs`, else a bare expression.
    fn bracket_statement(&mut self) -> Result<(StmtKind, Span)> {
        let snap = self.save();
        if let Ok(res) = self.try_multi_assign() {
            return Ok(res);
        }
        self.restore(snap);
        let e = self.expression().map_err(|fallback| self.best(fallback))?;
        let span = e.span;
        Ok((StmtKind::Expr(e), span))
    }

    fn try_multi_assign(&mut self) -> Result<(StmtKind, Span)> {
        let start = self.peek_span();
        self.expect(&TokenKind::LBracket, "to begin a multi-assignment")?;
        let mut lhs = Vec::new();
        while !self.at(&TokenKind::RBracket) {
            lhs.push(self.postfix_expr()?);
            // Targets are separated by commas (or whitespace, which is skipped
            // here since this is outside a matrix literal).
            self.eat(&TokenKind::Comma)?;
        }
        self.expect(&TokenKind::RBracket, "to close the assignment targets")?;
        self.expect(&TokenKind::Assign, "in a multi-assignment")?;
        let rhs = self.expression()?;
        let span = start.merge(rhs.span);
        Ok((StmtKind::MultiAssign { lhs, rhs }, span))
    }

    // --- control flow -------------------------------------------------------

    fn for_statement(&mut self) -> Result<(StmtKind, Span)> {
        let start = self.peek_span();
        self.bump()?; // for
        let paren = self.eat(&TokenKind::LParen)?;
        let var_tok = self.expect_ident("for loop variable")?;
        let var = ident_text(&var_tok);
        self.expect(&TokenKind::Assign, "in a for loop")?;
        let range = self.expression()?;
        if paren {
            self.expect(&TokenKind::RParen, "to close the for header")?;
        }
        self.skip_separators()?;
        let body = self.statement_list()?;
        let end = self.expect(&TokenKind::End, "to close the for loop")?;
        Ok((StmtKind::For { var, range, body }, start.merge(end.span)))
    }

    fn while_statement(&mut self) -> Result<(StmtKind, Span)> {
        let start = self.peek_span();
        self.bump()?;
        let cond = self.expression()?;
        self.skip_separators()?;
        let body = self.statement_list()?;
        let end = self.expect(&TokenKind::End, "to close the while loop")?;
        Ok((StmtKind::While { cond, body }, start.merge(end.span)))
    }

    fn if_statement(&mut self) -> Result<(StmtKind, Span)> {
        let start = self.peek_span();
        self.bump()?;
        let cond = self.expression()?;
        self.skip_separators()?;
        let body = self.statement_list()?;
        let mut elseifs = Vec::new();
        while self.at(&TokenKind::ElseIf) {
            let es = self.peek_span();
            self.bump()?;
            let c = self.expression()?;
            self.skip_separators()?;
            let b = self.statement_list()?;
            let span = es.merge(c.span);
            elseifs.push(ElseIf {
                cond: c,
                body: b,
                span,
            });
        }
        let mut else_body = None;
        if self.at(&TokenKind::Else) {
            self.bump()?;
            self.skip_separators()?;
            else_body = Some(self.statement_list()?);
        }
        let end = self.expect(&TokenKind::End, "to close the if statement")?;
        Ok((
            StmtKind::If {
                cond,
                body,
                elseifs,
                else_body,
            },
            start.merge(end.span),
        ))
    }

    fn switch_statement(&mut self) -> Result<(StmtKind, Span)> {
        let start = self.peek_span();
        self.bump()?;
        let scrutinee = self.expression()?;
        self.skip_separators()?;
        let mut cases = Vec::new();
        while self.at(&TokenKind::Case) {
            let cs = self.peek_span();
            self.bump()?;
            let label = self.expression()?;
            self.skip_separators()?;
            let body = self.statement_list()?;
            let span = cs.merge(label.span);
            cases.push(Case { label, body, span });
        }
        let mut otherwise = None;
        if self.at(&TokenKind::Otherwise) {
            self.bump()?;
            self.skip_separators()?;
            otherwise = Some(self.statement_list()?);
        }
        let end = self.expect(&TokenKind::End, "to close the switch statement")?;
        Ok((
            StmtKind::Switch {
                scrutinee,
                cases,
                otherwise,
            },
            start.merge(end.span),
        ))
    }

    fn try_statement(&mut self) -> Result<(StmtKind, Span)> {
        let start = self.peek_span();
        self.bump()?;
        self.skip_separators()?;
        let body = self.statement_list()?;
        let mut catch = None;
        if self.at(&TokenKind::Catch) {
            self.bump()?;
            // An optional catch identifier (`catch err`) on the same line: skip
            // it as part of the handler body (it parses as an expr/assignment).
            self.skip_separators()?;
            catch = Some(self.statement_list()?);
        }
        let end = self.expect(&TokenKind::End, "to close the try block")?;
        Ok((StmtKind::Try { body, catch }, start.merge(end.span)))
    }

    // --- functions ----------------------------------------------------------

    fn function_definition(&mut self) -> Result<FunctionDef> {
        let start = self.peek_span();
        self.bump()?; // function
        let mut outputs = Vec::new();
        let name;
        if self.at(&TokenKind::LBracket) {
            self.bump()?;
            while !self.at(&TokenKind::RBracket) {
                let t = self.expect_ident("an output parameter")?;
                outputs.push(ident_text(&t));
                // Outputs are comma- or whitespace-separated (whitespace is
                // skipped here, outside a matrix literal).
                self.eat(&TokenKind::Comma)?;
            }
            self.expect(&TokenKind::RBracket, "to close the output list")?;
            self.expect(&TokenKind::Assign, "before the function name")?;
            let nt = self.expect_ident("the function name")?;
            name = ident_text(&nt);
        } else {
            let first = self.expect_ident("the function name or single output")?;
            if self.at(&TokenKind::Assign) {
                self.bump()?;
                outputs.push(ident_text(&first));
                let nt = self.expect_ident("the function name")?;
                name = ident_text(&nt);
            } else {
                name = ident_text(&first);
            }
        }
        // Parameters.
        let mut inputs = Vec::new();
        if self.eat(&TokenKind::LParen)? {
            while !self.at(&TokenKind::RParen) {
                let pspan = self.peek_span();
                let by_ref = self.eat(&TokenKind::Amp)?;
                let pt = self.expect_ident("a function parameter")?;
                inputs.push(Param {
                    name: ident_text(&pt),
                    by_ref,
                    span: pspan.merge(pt.span),
                });
                if !self.at(&TokenKind::RParen) {
                    self.expect(&TokenKind::Comma, "between parameters")?;
                }
            }
            self.expect(&TokenKind::RParen, "to close the parameter list")?;
        }
        self.skip_separators()?;
        let (body, nested) = self.function_body()?;
        // An `end` is optional in FreeMat unless nested functions exist; we
        // accept and consume it when present.
        let mut span = start;
        if self.at(&TokenKind::End) {
            let e = self.bump()?;
            span = span.merge(e.span);
        } else if let Some(last) = body.last() {
            span = span.merge(last.span);
        }
        Ok(FunctionDef {
            name,
            outputs,
            inputs,
            body,
            nested,
            span,
        })
    }

    /// Parse a function body: statements up to the next `function` keyword,
    /// `end`, or EOF.
    ///
    /// FreeMat function files use **flat subfunctions**: every `function` in a
    /// file is a file-level sibling (none are `end`-terminated). A `function`
    /// keyword therefore terminates the current body — it is *not* consumed as a
    /// lexically nested function (doing so would wrongly nest the file's third
    /// subfunction inside the second, and so on). The top-level
    /// [`parse_program`](Self::parse_program) loop collects the siblings.
    fn function_body(&mut self) -> Result<(Vec<Stmt>, Vec<FunctionDef>)> {
        let mut body = Vec::new();
        let nested = Vec::new();
        loop {
            self.skip_separators()?;
            if self.at(&TokenKind::Eof) || self.at(&TokenKind::End) || self.at(&TokenKind::Function)
            {
                break;
            }
            let (kind, span) = self.statement()?;
            let terminator = self.statement_terminator()?;
            body.push(Stmt {
                kind,
                terminator,
                span,
            });
        }
        Ok((body, nested))
    }

    // --- expressions --------------------------------------------------------

    /// Parse a full expression (precedence 0).
    pub(crate) fn expression(&mut self) -> Result<Expr> {
        self.expr_bp(0)
    }

    /// Precedence-climbing expression parser. `min_bp` is the minimum binding
    /// power a binary operator must have to be consumed at this level.
    fn expr_bp(&mut self, min_bp: u8) -> Result<Expr> {
        let mut lhs = self.unary_expr()?;
        loop {
            let kind = self.peek().clone();
            if !kind.is_binary_operator() {
                break;
            }
            let prec = precedence(&kind);
            if prec < min_bp {
                break;
            }
            self.bump()?;
            let next_min = if right_associative(&kind) {
                prec
            } else {
                prec + 1
            };
            // `:` is handled as a range node, with optional step.
            if kind == TokenKind::Colon {
                let mid = self.expr_bp(next_min)?;
                if self.at(&TokenKind::Colon) {
                    self.bump()?;
                    let stop = self.expr_bp(next_min)?;
                    let span = lhs.span.merge(stop.span);
                    lhs = Expr {
                        kind: ExprKind::Range {
                            start: Box::new(lhs),
                            step: Some(Box::new(mid)),
                            stop: Box::new(stop),
                        },
                        span,
                    };
                } else {
                    let span = lhs.span.merge(mid.span);
                    lhs = Expr {
                        kind: ExprKind::Range {
                            start: Box::new(lhs),
                            step: None,
                            stop: Box::new(mid),
                        },
                        span,
                    };
                }
                continue;
            }
            let rhs = self.expr_bp(next_min)?;
            let span = lhs.span.merge(rhs.span);
            let op = binary_op(&kind).expect("checked is_binary_operator");
            lhs = Expr {
                kind: ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            };
        }
        Ok(lhs)
    }

    fn unary_expr(&mut self) -> Result<Expr> {
        let kind = self.peek().clone();
        let op = match kind {
            TokenKind::Plus => Some(UnaryOp::Plus),
            TokenKind::Minus => Some(UnaryOp::Minus),
            TokenKind::Not => Some(UnaryOp::Not),
            _ => None,
        };
        if let Some(op) = op {
            let start = self.peek_span();
            self.bump()?;
            // Unary precedence is 9 in FreeMat (binds tighter than `*`, looser
            // than `^`). exp(prec) where prec(unary) = 9.
            let operand = self.expr_bp(9)?;
            let span = start.merge(operand.span);
            return Ok(Expr {
                kind: ExprKind::Unary {
                    op,
                    operand: Box::new(operand),
                },
                span,
            });
        }
        self.postfix_expr()
    }

    /// A primary expression followed by any postfix chain (indexing / field /
    /// transpose).
    fn postfix_expr(&mut self) -> Result<Expr> {
        let mut e = self.primary_expr()?;
        loop {
            match self.peek() {
                TokenKind::LParen => {
                    self.bump()?;
                    let (args, keywords) = self.call_args(&TokenKind::RParen)?;
                    let end = self.expect(&TokenKind::RParen, "to close indexing")?;
                    let span = e.span.merge(end.span);
                    // Keyword arguments (`/name` / `/name=expr`) are an IDL-style
                    // call form; produce them only when present so ordinary calls
                    // and array indexing stay on the plain `Index` node.
                    e = if keywords.is_empty() {
                        Expr {
                            kind: ExprKind::Index {
                                base: Box::new(e),
                                args,
                            },
                            span,
                        }
                    } else {
                        Expr {
                            kind: ExprKind::KeywordCall {
                                base: Box::new(e),
                                args,
                                keywords,
                            },
                            span,
                        }
                    };
                }
                TokenKind::LBrace => {
                    self.bump()?;
                    let args = self.index_args(&TokenKind::RBrace)?;
                    let end = self.expect(&TokenKind::RBrace, "to close cell indexing")?;
                    let span = e.span.merge(end.span);
                    e = Expr {
                        kind: ExprKind::CellIndex {
                            base: Box::new(e),
                            args,
                        },
                        span,
                    };
                }
                TokenKind::Dot => {
                    self.bump()?;
                    if self.eat(&TokenKind::LParen)? {
                        let name = self.expression()?;
                        let end = self.expect(&TokenKind::RParen, "to close dynamic field")?;
                        let span = e.span.merge(end.span);
                        e = Expr {
                            kind: ExprKind::DynField {
                                base: Box::new(e),
                                name: Box::new(name),
                            },
                            span,
                        };
                    } else {
                        let t = self.expect_ident("a field name")?;
                        let span = e.span.merge(t.span);
                        e = Expr {
                            kind: ExprKind::Field {
                                base: Box::new(e),
                                name: ident_text(&t),
                            },
                            span,
                        };
                    }
                }
                TokenKind::Transpose | TokenKind::DotTranspose => {
                    let conjugate = matches!(self.peek(), TokenKind::Transpose);
                    let t = self.bump()?;
                    let span = e.span.merge(t.span);
                    e = Expr {
                        kind: ExprKind::Transpose {
                            operand: Box::new(e),
                            conjugate,
                        },
                        span,
                    };
                }
                _ => break,
            }
        }
        Ok(e)
    }

    /// Parse a call's argument list, splitting positional arguments from
    /// IDL-style keyword arguments. A keyword argument is recognized **only** at
    /// the start of an argument (just after `(` or a `,`) as a `/` immediately
    /// followed by an identifier: `/name` (bare → binds `logical(1)`) or
    /// `/name = expr`. Everywhere else `/` is the ordinary division operator, so
    /// this peek-and-commit is the sole place `/` gets special treatment.
    fn call_args(&mut self, close: &TokenKind) -> Result<CallArgs> {
        let mut args = Vec::new();
        let mut keywords = Vec::new();
        while !self.at(close) {
            if self.at(&TokenKind::RDivide) {
                // Peek past `/` for an identifier without consuming on failure.
                let snap = self.save();
                self.bump()?;
                if let TokenKind::Ident(name) = self.peek().clone() {
                    self.bump()?;
                    let value = if self.eat(&TokenKind::Assign)? {
                        Some(self.expression()?)
                    } else {
                        None
                    };
                    keywords.push((name, value));
                    if !self.at(close) {
                        self.expect(&TokenKind::Comma, "between call arguments")?;
                    }
                    continue;
                }
                // Not `/ident` — `/` was not a keyword marker. Back up and parse
                // as an ordinary expression (where `/` is division).
                self.restore(snap);
            }
            self.push_index_arg(&mut args, close)?;
            if !self.at(close) {
                self.expect(&TokenKind::Comma, "between call arguments")?;
            }
        }
        Ok((args, keywords))
    }

    /// Parse one positional argument (handling the bare `:` magic-colon) and push
    /// it onto `args`. Shared by [`call_args`] and [`index_args`].
    fn push_index_arg(&mut self, args: &mut Vec<Expr>, close: &TokenKind) -> Result<()> {
        if self.at(&TokenKind::Colon) {
            // A bare `:` is the magic colon only if it stands alone as an
            // argument (next is `,` or the closer).
            let snap = self.save();
            let cspan = self.peek_span();
            self.bump()?;
            if self.at(&TokenKind::Comma) || self.at(close) {
                args.push(Expr {
                    kind: ExprKind::Colon,
                    span: cspan,
                });
            } else {
                self.restore(snap);
                args.push(self.expression()?);
            }
        } else {
            args.push(self.expression()?);
        }
        Ok(())
    }

    /// Parse comma-separated index arguments, allowing the bare `:` magic-colon.
    fn index_args(&mut self, close: &TokenKind) -> Result<Vec<Expr>> {
        let mut args = Vec::new();
        while !self.at(close) {
            self.push_index_arg(&mut args, close)?;
            if !self.at(close) {
                self.expect(&TokenKind::Comma, "between index arguments")?;
            }
        }
        Ok(args)
    }

    fn primary_expr(&mut self) -> Result<Expr> {
        let span = self.peek_span();
        match self.peek().clone() {
            TokenKind::Real(text) => {
                self.bump()?;
                Ok(Expr {
                    kind: ExprKind::Real {
                        text,
                        single: false,
                    },
                    span,
                })
            }
            TokenKind::RealF(text) => {
                self.bump()?;
                Ok(Expr {
                    kind: ExprKind::Real { text, single: true },
                    span,
                })
            }
            TokenKind::Imag(text) => {
                self.bump()?;
                Ok(Expr {
                    kind: ExprKind::Imag {
                        text,
                        single: false,
                    },
                    span,
                })
            }
            TokenKind::ImagF(text) => {
                self.bump()?;
                Ok(Expr {
                    kind: ExprKind::Imag { text, single: true },
                    span,
                })
            }
            TokenKind::Str(s) => {
                self.bump()?;
                Ok(Expr {
                    kind: ExprKind::Str(s),
                    span,
                })
            }
            TokenKind::Ident(name) => {
                self.bump()?;
                Ok(Expr {
                    kind: ExprKind::Ident(name),
                    span,
                })
            }
            TokenKind::End => {
                self.bump()?;
                Ok(Expr {
                    kind: ExprKind::End,
                    span,
                })
            }
            TokenKind::LParen => {
                self.bump()?;
                let e = self.expression()?;
                self.expect(&TokenKind::RParen, "to close the parenthesis")?;
                Ok(e)
            }
            TokenKind::LBracket => self.matrix_literal(),
            TokenKind::LBrace => self.cell_literal(),
            TokenKind::At => self.handle_or_anon(),
            other => Err(self.error_at(
                span,
                "expected an expression",
                &format!("{} cannot start an expression", other.describe()),
            )),
        }
    }

    fn handle_or_anon(&mut self) -> Result<Expr> {
        let start = self.peek_span();
        self.bump()?; // @
        if self.at(&TokenKind::LParen) {
            self.bump()?;
            let mut params = Vec::new();
            while !self.at(&TokenKind::RParen) {
                let t = self.expect_ident("an anonymous-function parameter")?;
                params.push(ident_text(&t));
                if !self.at(&TokenKind::RParen) {
                    self.expect(&TokenKind::Comma, "between parameters")?;
                }
            }
            self.expect(&TokenKind::RParen, "to close the parameter list")?;
            let body = self.expression()?;
            let span = start.merge(body.span);
            Ok(Expr {
                kind: ExprKind::AnonFunc {
                    params,
                    body: Box::new(body),
                },
                span,
            })
        } else {
            let t = self.expect_ident("a function name after `@`")?;
            let span = start.merge(t.span);
            Ok(Expr {
                kind: ExprKind::FuncHandle(ident_text(&t)),
                span,
            })
        }
    }

    fn matrix_literal(&mut self) -> Result<Expr> {
        let start = self.peek_span();
        self.bump()?; // [
        let rows = self.with_ws(|p| p.matrix_rows(&TokenKind::RBracket))?;
        let end = self.expect(&TokenKind::RBracket, "to close the matrix")?;
        Ok(Expr {
            kind: ExprKind::Matrix(rows),
            span: start.merge(end.span),
        })
    }

    fn cell_literal(&mut self) -> Result<Expr> {
        let start = self.peek_span();
        self.bump()?; // {
        let rows = self.with_ws(|p| p.matrix_rows(&TokenKind::RBrace))?;
        let end = self.expect(&TokenKind::RBrace, "to close the cell array")?;
        Ok(Expr {
            kind: ExprKind::Cell(rows),
            span: start.merge(end.span),
        })
    }

    /// Parse matrix/cell rows. Whitespace is significant here: a space (not
    /// followed by a binary continuation) separates columns; `;` / newline
    /// separates rows.
    fn matrix_rows(&mut self, close: &TokenKind) -> Result<Vec<Vec<Expr>>> {
        let mut rows = Vec::new();
        // Skip leading whitespace.
        while self.at(&TokenKind::Space) {
            self.bump()?;
        }
        while !self.at(close) {
            let mut row = Vec::new();
            while !self.at(&TokenKind::Semicolon)
                && !self.at(&TokenKind::Newline)
                && !self.at(close)
            {
                row.push(self.matrix_element()?);
                if self.at(&TokenKind::Comma) {
                    self.bump()?;
                    while self.at(&TokenKind::Space) {
                        self.bump()?;
                    }
                } else if self.at(&TokenKind::Space) {
                    self.bump()?;
                }
            }
            if self.at(&TokenKind::Semicolon) || self.at(&TokenKind::Newline) {
                self.bump()?;
            }
            while self.at(&TokenKind::Space) {
                self.bump()?;
            }
            // Skip blank rows produced by trailing separators.
            if !row.is_empty() {
                rows.push(row);
            }
        }
        Ok(rows)
    }

    /// Parse a single matrix element. Inside a matrix, whitespace is significant,
    /// so a binary operator surrounded by spaces still binds, but a leading sign
    /// with no following space starts a new element — handled by stopping at a
    /// `Space`.
    fn matrix_element(&mut self) -> Result<Expr> {
        self.matrix_expr_bp(0)
    }

    /// A precedence-climbing parser used inside matrices, where a `Space` token
    /// terminates the current element unless it sits between an operator and its
    /// operand.
    fn matrix_expr_bp(&mut self, min_bp: u8) -> Result<Expr> {
        let mut lhs = self.matrix_unary()?;
        loop {
            // A space here ends the element unless the next non-space token is a
            // binary operator immediately followed (no space) by its operand
            // — but FreeMat treats `a - b` (spaces both sides) as subtraction
            // and `a -b` as two elements. We honour: if there is a space and the
            // following operator is a binary op with a space after it, continue;
            // otherwise stop.
            if self.at(&TokenKind::Space) {
                let snap = self.save();
                self.bump()?; // the space
                let kind = self.peek().clone();
                if kind.is_binary_operator() {
                    // peek whether a space follows the operator.
                    let after = self.save();
                    self.bump()?; // operator
                    let space_after = self.at(&TokenKind::Space);
                    self.restore(after);
                    if space_after || kind == TokenKind::Colon {
                        // Treat as a normal binary operator: fall through with
                        // the space consumed.
                        let prec = precedence(&kind);
                        if prec < min_bp {
                            self.restore(snap);
                            break;
                        }
                        lhs = self.matrix_consume_binop(lhs, &kind)?;
                        continue;
                    }
                }
                // Otherwise the space ends this element.
                self.restore(snap);
                break;
            }
            let kind = self.peek().clone();
            if !kind.is_binary_operator() {
                break;
            }
            let prec = precedence(&kind);
            if prec < min_bp {
                break;
            }
            lhs = self.matrix_consume_binop(lhs, &kind)?;
        }
        Ok(lhs)
    }

    fn matrix_consume_binop(&mut self, lhs: Expr, kind: &TokenKind) -> Result<Expr> {
        let prec = precedence(kind);
        self.bump()?; // operator
        while self.at(&TokenKind::Space) {
            self.bump()?;
        }
        let next_min = if right_associative(kind) {
            prec
        } else {
            prec + 1
        };
        if *kind == TokenKind::Colon {
            let mid = self.matrix_expr_bp(next_min)?;
            // optional second colon
            let mut step = None;
            let mut stop = mid;
            if self.at(&TokenKind::Colon) {
                self.bump()?;
                while self.at(&TokenKind::Space) {
                    self.bump()?;
                }
                step = Some(Box::new(stop));
                stop = self.matrix_expr_bp(next_min)?;
            }
            let span = lhs.span.merge(stop.span);
            return Ok(Expr {
                kind: ExprKind::Range {
                    start: Box::new(lhs),
                    step,
                    stop: Box::new(stop),
                },
                span,
            });
        }
        let rhs = self.matrix_expr_bp(next_min)?;
        let span = lhs.span.merge(rhs.span);
        let op = binary_op(kind).expect("binary op");
        Ok(Expr {
            kind: ExprKind::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
            span,
        })
    }

    fn matrix_unary(&mut self) -> Result<Expr> {
        let kind = self.peek().clone();
        let op = match kind {
            TokenKind::Plus => Some(UnaryOp::Plus),
            TokenKind::Minus => Some(UnaryOp::Minus),
            TokenKind::Not => Some(UnaryOp::Not),
            _ => None,
        };
        if let Some(op) = op {
            let start = self.peek_span();
            self.bump()?;
            let operand = self.matrix_expr_bp(9)?;
            let span = start.merge(operand.span);
            return Ok(Expr {
                kind: ExprKind::Unary {
                    op,
                    operand: Box::new(operand),
                },
                span,
            });
        }
        self.postfix_expr()
    }

    // --- small helpers ------------------------------------------------------

    fn expect_ident(&mut self, because: &str) -> Result<Token> {
        if let TokenKind::Ident(_) = self.peek() {
            self.bump()
        } else {
            Err(self.error_here(&format!("expected {because}")))
        }
    }
}

fn ident_text(tok: &Token) -> String {
    match &tok.kind {
        TokenKind::Ident(s) => s.clone(),
        _ => String::new(),
    }
}
