//! Error-path tests: a malformed snippet must produce a diagnostic whose label
//! points at the correct byte span, and must render as an annotated message.

use fm_parser::parse_program;

/// Parse `src`, expecting failure, and return the error.
fn err(src: &str) -> fm_parser::ParseError {
    parse_program(src).expect_err("expected a parse error")
}

/// The 0-based byte offset the diagnostic points at.
fn at(src: &str) -> usize {
    err(src).span().start
}

#[test]
fn unterminated_string_points_at_quote() {
    let src = "x = 'oops";
    let e = err(src);
    // The label should sit on the string literal starting at the quote (index 4).
    assert_eq!(e.span().start, 4);
    assert!(e.to_string().contains("unterminated string"));
}

#[test]
fn unexpected_close_bracket() {
    // `a(]` — the `]` cannot start an index argument.
    let src = "a(]";
    assert_eq!(at(src), 2, "should point at the `]`");
}

#[test]
fn missing_close_bracket_points_at_eof() {
    let src = "[1 2 3";
    let e = err(src);
    // End-of-input is at offset 6.
    assert_eq!(e.span().start, 6);
}

#[test]
fn dangling_binary_operator() {
    // `1 +` with nothing after — error at EOF (offset 3).
    let src = "1 +";
    assert_eq!(at(src), 3);
}

#[test]
fn deep_error_in_assignment_rhs_is_reported() {
    // The deepest error (inside the parenthesised RHS) should win over the
    // shallow "expected terminator" complaint from backtracking.
    let src = "x = (1 + ";
    let e = err(src);
    assert_eq!(e.span().start, 9, "should point near the unfinished RHS");
    assert!(e.to_string().contains("expression"), "got: {e}");
}

#[test]
fn empty_index_argument() {
    // `foo(,)` — a comma with no preceding expression.
    let src = "foo(,)";
    assert_eq!(at(src), 4);
}

#[test]
fn missing_function_end_keyword_ok_without_nested() {
    // FreeMat allows omitting `end` for a single non-nested function. This is a
    // documented MATLAB-compatible choice (see PROGRESS.md), so it must parse.
    assert!(parse_program("function y = f(x)\n y = x + 1;").is_ok());
}

#[test]
fn malformed_block_comment_is_unterminated() {
    let src = "%{\nstuff that never closes\n";
    let e = err(src);
    assert!(e.to_string().contains("block comment"), "got: {e}");
}

#[test]
fn diagnostic_renders_annotated_snippet() {
    // With miette's `fancy` feature (enabled for the test target) the report
    // renders an annotated, source-highlighted message.
    let e = err("x = (1 +\n");
    let report = miette::Report::new(e);
    let rendered = format!("{report:?}");
    // The rendered output should reproduce a line of the offending source and a
    // caret/label pointing at the problem.
    assert!(
        rendered.contains("expected an expression"),
        "rendered report missing message:\n{rendered}"
    );
}
