//! Lexer tokenization tests, including the transpose-vs-string disambiguation
//! and command-syntax (blob) handling.

use fm_parser::token::TokenKind;
use fm_parser::{Parser, tokenize};

/// Collect the significant token kinds (dropping `Space`/`Newline`/`Eof`).
fn kinds(src: &str) -> Vec<TokenKind> {
    tokenize(src)
        .unwrap()
        .into_iter()
        .map(|t| t.kind)
        .filter(|k| !matches!(k, TokenKind::Space | TokenKind::Newline | TokenKind::Eof))
        .collect()
}

#[test]
fn numbers_int_float_scientific() {
    assert_eq!(kinds("42"), vec![TokenKind::Real("42".into())]);
    assert_eq!(kinds("3.14"), vec![TokenKind::Real("3.14".into())]);
    assert_eq!(kinds(".5"), vec![TokenKind::Real(".5".into())]);
    assert_eq!(kinds("1."), vec![TokenKind::Real("1.".into())]);
    assert_eq!(kinds("1e-5"), vec![TokenKind::Real("1e-5".into())]);
    assert_eq!(kinds("6.02E23"), vec![TokenKind::Real("6.02E23".into())]);
}

#[test]
fn numbers_single_and_imaginary() {
    assert_eq!(kinds("2.0f"), vec![TokenKind::RealF("2.0".into())]);
    assert_eq!(kinds("3i"), vec![TokenKind::Imag("3".into())]);
    assert_eq!(kinds("4j"), vec![TokenKind::Imag("4".into())]);
    assert_eq!(kinds("2.5fi"), vec![TokenKind::ImagF("2.5".into())]);
}

#[test]
fn hex_literal_extension() {
    // FreeMat's scanner lacked hex; the Rust port accepts `0x...`.
    assert_eq!(kinds("0xFF"), vec![TokenKind::Real("0xFF".into())]);
    assert_eq!(kinds("0x1a2b"), vec![TokenKind::Real("0x1a2b".into())]);
}

#[test]
fn number_does_not_swallow_dot_operators() {
    // `1.^2` must lex as `1`, `.^`, `2` — the number backs off the `.`.
    assert_eq!(
        kinds("1.^2"),
        vec![
            TokenKind::Real("1".into()),
            TokenKind::DotPower,
            TokenKind::Real("2".into()),
        ]
    );
    // `2.*x` -> 2 .* x
    assert_eq!(
        kinds("2.*x"),
        vec![
            TokenKind::Real("2".into()),
            TokenKind::DotTimes,
            TokenKind::Ident("x".into()),
        ]
    );
}

#[test]
fn string_with_escaped_quote() {
    assert_eq!(kinds("'it''s'"), vec![TokenKind::Str("it's".into())]);
    assert_eq!(
        kinds("'hello world'"),
        vec![TokenKind::Str("hello world".into())]
    );
}

#[test]
fn transpose_vs_string_disambiguation() {
    // After an identifier, `'` is transpose.
    assert_eq!(
        kinds("a'"),
        vec![TokenKind::Ident("a".into()), TokenKind::Transpose]
    );
    // After `)`, `]`, `}` and a number, `'` is transpose.
    assert_eq!(
        kinds("(a)'"),
        vec![
            TokenKind::LParen,
            TokenKind::Ident("a".into()),
            TokenKind::RParen,
            TokenKind::Transpose,
        ]
    );
    assert_eq!(
        kinds("3'"),
        vec![TokenKind::Real("3".into()), TokenKind::Transpose]
    );
    // At the start, after a space, or after an operator, `'` starts a string.
    assert_eq!(kinds("'hi'"), vec![TokenKind::Str("hi".into())]);
    assert_eq!(
        kinds("x = 'hi'"),
        vec![
            TokenKind::Ident("x".into()),
            TokenKind::Assign,
            TokenKind::Str("hi".into()),
        ]
    );
    // `a 'hi'` — space before quote means string.
    assert_eq!(
        kinds("a 'hi'"),
        vec![TokenKind::Ident("a".into()), TokenKind::Str("hi".into())]
    );
}

#[test]
fn multichar_operators() {
    assert_eq!(
        kinds("a<=b>=c==d~=e&&f||g"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Le,
            TokenKind::Ident("b".into()),
            TokenKind::Ge,
            TokenKind::Ident("c".into()),
            TokenKind::Eq,
            TokenKind::Ident("d".into()),
            TokenKind::Ne,
            TokenKind::Ident("e".into()),
            TokenKind::AndAnd,
            TokenKind::Ident("f".into()),
            TokenKind::OrOr,
            TokenKind::Ident("g".into()),
        ]
    );
}

#[test]
fn elementwise_operators() {
    assert_eq!(
        kinds("a.*b./c.\\d.^e.'"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::DotTimes,
            TokenKind::Ident("b".into()),
            TokenKind::DotRDivide,
            TokenKind::Ident("c".into()),
            TokenKind::DotLDivide,
            TokenKind::Ident("d".into()),
            TokenKind::DotPower,
            TokenKind::Ident("e".into()),
            TokenKind::DotTranspose,
        ]
    );
}

#[test]
fn line_comment_skipped() {
    assert_eq!(
        kinds("a % this is a comment\nb"),
        vec![TokenKind::Ident("a".into()), TokenKind::Ident("b".into())]
    );
}

#[test]
fn block_comment_skipped() {
    let src = "a\n%{\nignored line one\nignored line two\n%}\nb";
    assert_eq!(
        kinds(src),
        vec![TokenKind::Ident("a".into()), TokenKind::Ident("b".into())]
    );
}

#[test]
fn line_continuation_joins_lines() {
    // `...` swallows the rest of the line and the newline.
    assert_eq!(
        kinds("1 + ...\n2"),
        vec![
            TokenKind::Real("1".into()),
            TokenKind::Plus,
            TokenKind::Real("2".into()),
        ]
    );
}

#[test]
fn reserved_words() {
    assert_eq!(kinds("if"), vec![TokenKind::If]);
    assert_eq!(kinds("for"), vec![TokenKind::For]);
    assert_eq!(kinds("function"), vec![TokenKind::Function]);
    assert_eq!(kinds("end"), vec![TokenKind::End]);
    // An identifier that merely contains a keyword is not reserved.
    assert_eq!(kinds("endgame"), vec![TokenKind::Ident("endgame".into())]);
}

#[test]
fn spans_track_byte_offsets() {
    let toks = tokenize("ab + cd").unwrap();
    // ab @ 0..2, space, + @ 3..4, space, cd @ 5..7, eof.
    assert_eq!(toks[0].span.start, 0);
    assert_eq!(toks[0].span.end, 2);
    assert_eq!(toks[2].span.start, 3);
    assert_eq!(toks[2].span.end, 4);
    assert_eq!(toks[4].span.start, 5);
    assert_eq!(toks[4].span.end, 7);
}

#[test]
fn blob_mode_lexes_command_arguments_verbatim() {
    // Drive blob mode the way the parser does: consume `format`, a space, then
    // turn blob mode on and read the argument as a single string token.
    let mut lexer = fm_parser::lexer::Lexer::new("format long");
    let name = lexer.next_token().unwrap();
    assert_eq!(name.kind, TokenKind::Ident("format".into()));
    let space = lexer.next_token().unwrap();
    assert_eq!(space.kind, TokenKind::Space);
    lexer.set_blob_mode(true);
    let arg = lexer.next_token().unwrap();
    assert_eq!(arg.kind, TokenKind::Str("long".into()));
}

#[test]
fn parser_consumes_full_input() {
    // A sanity check that `Parser::new` succeeds and a trivial program parses.
    assert!(Parser::new("x = 1;").is_ok());
}
