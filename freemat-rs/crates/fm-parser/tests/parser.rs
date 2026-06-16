//! Parser tests: parse-trees for representative snippets covering precedence,
//! matrix/cell literals, ranges, function & anonymous-function defs, every
//! control-flow construct, and assignments.

use fm_parser::ast::*;
use fm_parser::{parse_expression, parse_program, parse_statements};

/// Parse a single expression, panicking on error.
fn expr(src: &str) -> Expr {
    parse_expression(src).unwrap_or_else(|e| panic!("parse `{src}` failed: {e}"))
}

/// Parse to a script and return its statements.
fn script(src: &str) -> Vec<Stmt> {
    match parse_program(src).unwrap_or_else(|e| panic!("parse `{src}` failed: {e}")) {
        Program::Script(s) => s,
        Program::Functions(_) => panic!("expected a script"),
    }
}

/// Convenience: the kind of the only statement in a script.
fn only_stmt(src: &str) -> StmtKind {
    let mut s = script(src);
    assert_eq!(s.len(), 1, "expected exactly one statement in `{src}`");
    s.remove(0).kind
}

// --- precedence ------------------------------------------------------------

#[test]
fn precedence_mul_over_add() {
    // 1 + 2 * 3 == 1 + (2 * 3)
    let e = expr("1 + 2 * 3");
    let ExprKind::Binary { op, lhs, rhs } = e.kind else {
        panic!("not binary");
    };
    assert_eq!(op, BinaryOp::Add);
    assert!(matches!(lhs.kind, ExprKind::Real { .. }));
    assert!(matches!(
        rhs.kind,
        ExprKind::Binary {
            op: BinaryOp::Mul,
            ..
        }
    ));
}

#[test]
fn power_is_right_associative() {
    // 2 ^ 3 ^ 2 == 2 ^ (3 ^ 2)
    let e = expr("2 ^ 3 ^ 2");
    let ExprKind::Binary { op, lhs, rhs } = e.kind else {
        panic!("not binary");
    };
    assert_eq!(op, BinaryOp::Pow);
    assert!(matches!(lhs.kind, ExprKind::Real { .. }));
    assert!(matches!(
        rhs.kind,
        ExprKind::Binary {
            op: BinaryOp::Pow,
            ..
        }
    ));
}

#[test]
fn unary_minus_binds_looser_than_power() {
    // -2^2 == -(2^2)
    let e = expr("-2^2");
    let ExprKind::Unary { op, operand } = e.kind else {
        panic!("not unary");
    };
    assert_eq!(op, UnaryOp::Minus);
    assert!(matches!(
        operand.kind,
        ExprKind::Binary {
            op: BinaryOp::Pow,
            ..
        }
    ));
}

#[test]
fn comparison_below_arithmetic() {
    // a + b < c == (a + b) < c
    let e = expr("a + b < c");
    let ExprKind::Binary { op, lhs, .. } = e.kind else {
        panic!("not binary");
    };
    assert_eq!(op, BinaryOp::Lt);
    assert!(matches!(
        lhs.kind,
        ExprKind::Binary {
            op: BinaryOp::Add,
            ..
        }
    ));
}

// --- ranges ----------------------------------------------------------------

#[test]
fn range_two_part() {
    let e = expr("1:10");
    assert!(matches!(e.kind, ExprKind::Range { step: None, .. }));
}

#[test]
fn range_three_part() {
    let e = expr("1:2:10");
    let ExprKind::Range { step, .. } = e.kind else {
        panic!("not a range");
    };
    assert!(step.is_some());
}

// --- matrix / cell literals -----------------------------------------------

#[test]
fn matrix_rows_and_cols() {
    let e = expr("[1 2 3; 4 5 6]");
    let ExprKind::Matrix(rows) = e.kind else {
        panic!("not a matrix");
    };
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].len(), 3);
    assert_eq!(rows[1].len(), 3);
}

#[test]
fn matrix_unary_disambiguation() {
    // `[1 -2]` is two elements; `[1 - 2]` is one element (subtraction).
    let two = expr("[1 -2]");
    let ExprKind::Matrix(rows) = two.kind else {
        panic!()
    };
    assert_eq!(rows[0].len(), 2);

    let one = expr("[1 - 2]");
    let ExprKind::Matrix(rows) = one.kind else {
        panic!()
    };
    assert_eq!(rows[0].len(), 1);
    assert!(matches!(
        rows[0][0].kind,
        ExprKind::Binary {
            op: BinaryOp::Sub,
            ..
        }
    ));
}

#[test]
fn cell_literal() {
    let e = expr("{1, 'a'; 2, 'b'}");
    let ExprKind::Cell(rows) = e.kind else {
        panic!("not a cell");
    };
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].len(), 2);
}

// --- indexing & field access ----------------------------------------------

#[test]
fn index_field_brace_chain() {
    // a.b(3).c{1}
    let e = expr("a.b(3).c{1}");
    assert!(matches!(e.kind, ExprKind::CellIndex { .. }));
}

#[test]
fn magic_colon_index() {
    let e = expr("A(:)");
    let ExprKind::Index { args, .. } = e.kind else {
        panic!("not indexed");
    };
    assert_eq!(args.len(), 1);
    assert!(matches!(args[0].kind, ExprKind::Colon));
}

#[test]
fn end_in_index_context() {
    let e = expr("v(end)");
    let ExprKind::Index { args, .. } = e.kind else {
        panic!("not indexed");
    };
    assert!(matches!(args[0].kind, ExprKind::End));
}

#[test]
fn dynamic_field_access() {
    let e = expr("s.(name)");
    assert!(matches!(e.kind, ExprKind::DynField { .. }));
}

// --- transpose -------------------------------------------------------------

#[test]
fn transpose_postfix() {
    let e = expr("a'");
    assert!(matches!(
        e.kind,
        ExprKind::Transpose {
            conjugate: true,
            ..
        }
    ));
    let e = expr("a.'");
    assert!(matches!(
        e.kind,
        ExprKind::Transpose {
            conjugate: false,
            ..
        }
    ));
}

// --- anonymous functions & handles ----------------------------------------

#[test]
fn anonymous_function() {
    let e = expr("@(x, y) x + y");
    let ExprKind::AnonFunc { params, body } = e.kind else {
        panic!("not anon");
    };
    assert_eq!(params, vec!["x".to_string(), "y".to_string()]);
    assert!(matches!(
        body.kind,
        ExprKind::Binary {
            op: BinaryOp::Add,
            ..
        }
    ));
}

#[test]
fn function_handle() {
    let e = expr("@sin");
    assert!(matches!(e.kind, ExprKind::FuncHandle(ref n) if n == "sin"));
}

// --- assignments -----------------------------------------------------------

#[test]
fn simple_assignment() {
    let k = only_stmt("x = 5;");
    let StmtKind::Assign { lhs, .. } = k else {
        panic!("not assign");
    };
    assert!(matches!(lhs.kind, ExprKind::Ident(ref n) if n == "x"));
}

#[test]
fn indexed_assignment() {
    let k = only_stmt("a(2) = 7;");
    let StmtKind::Assign { lhs, .. } = k else {
        panic!("not assign");
    };
    assert!(matches!(lhs.kind, ExprKind::Index { .. }));
}

#[test]
fn multi_output_assignment() {
    let k = only_stmt("[a, b] = size(x);");
    let StmtKind::MultiAssign { lhs, .. } = k else {
        panic!("not multi-assign");
    };
    assert_eq!(lhs.len(), 2);
}

#[test]
fn quiet_vs_display_terminator() {
    let s = script("a = 1; b = 2\n");
    assert_eq!(s[0].terminator, Terminator::Quiet);
    assert_eq!(s[1].terminator, Terminator::Display);
}

// --- command syntax --------------------------------------------------------

#[test]
fn command_syntax() {
    let k = only_stmt("hold on");
    let StmtKind::Command { name, args } = k else {
        panic!("not a command: {k:?}");
    };
    assert_eq!(name, "hold");
    assert_eq!(args, vec!["on".to_string()]);
}

#[test]
fn command_syntax_multiple_args() {
    let k = only_stmt("format long g");
    let StmtKind::Command { name, args } = k else {
        panic!("not a command: {k:?}");
    };
    assert_eq!(name, "format");
    assert_eq!(args, vec!["long".to_string(), "g".to_string()]);
}

#[test]
fn function_call_is_not_command() {
    // `disp(3)` is a call, not command syntax.
    let k = only_stmt("disp(3)");
    assert!(matches!(k, StmtKind::Expr(_)), "got {k:?}");
}

// --- control flow ----------------------------------------------------------

#[test]
fn for_loop() {
    let k = only_stmt("for i = 1:10\n s = s + i;\nend");
    let StmtKind::For { var, body, .. } = k else {
        panic!("not for");
    };
    assert_eq!(var, "i");
    assert_eq!(body.len(), 1);
}

#[test]
fn while_loop() {
    let k = only_stmt("while x > 0\n x = x - 1;\nend");
    assert!(matches!(k, StmtKind::While { .. }));
}

#[test]
fn if_elseif_else() {
    let k = only_stmt("if a\n x=1;\nelseif b\n x=2;\nelse\n x=3;\nend");
    let StmtKind::If {
        elseifs, else_body, ..
    } = k
    else {
        panic!("not if");
    };
    assert_eq!(elseifs.len(), 1);
    assert!(else_body.is_some());
}

#[test]
fn switch_case_otherwise() {
    let k = only_stmt("switch x\ncase 1\n y=1;\ncase 2\n y=2;\notherwise\n y=0;\nend");
    let StmtKind::Switch {
        cases, otherwise, ..
    } = k
    else {
        panic!("not switch");
    };
    assert_eq!(cases.len(), 2);
    assert!(otherwise.is_some());
}

#[test]
fn try_catch() {
    let k = only_stmt("try\n risky();\ncatch\n recover();\nend");
    let StmtKind::Try { catch, .. } = k else {
        panic!("not try");
    };
    assert!(catch.is_some());
}

#[test]
fn keyword_statements() {
    assert!(matches!(
        only_stmt("break"),
        StmtKind::Keyword(KeywordStmt::Break)
    ));
    assert!(matches!(
        only_stmt("continue"),
        StmtKind::Keyword(KeywordStmt::Continue)
    ));
    assert!(matches!(
        only_stmt("return"),
        StmtKind::Keyword(KeywordStmt::Return)
    ));
}

#[test]
fn global_persistent_declarations() {
    assert!(matches!(only_stmt("global a b c"), StmtKind::Global(ref v) if v.len() == 3));
    assert!(matches!(only_stmt("persistent p q"), StmtKind::Persistent(ref v) if v.len() == 2));
}

// --- function definitions --------------------------------------------------

#[test]
fn function_with_outputs_and_inputs() {
    let prog = parse_program("function [a, b] = f(x, y)\n a = x;\n b = y;\nend").unwrap();
    let Program::Functions(funcs) = prog else {
        panic!("not functions");
    };
    assert_eq!(funcs.len(), 1);
    let f = &funcs[0];
    assert_eq!(f.name, "f");
    assert_eq!(f.outputs, vec!["a".to_string(), "b".to_string()]);
    assert_eq!(f.inputs.len(), 2);
    assert_eq!(f.body.len(), 2);
}

#[test]
fn function_single_output_shorthand() {
    let prog = parse_program("function y = sq(x)\n y = x*x;\nend").unwrap();
    let Program::Functions(funcs) = prog else {
        panic!()
    };
    assert_eq!(funcs[0].outputs, vec!["y".to_string()]);
}

#[test]
fn function_no_output() {
    let prog = parse_program("function greet(name)\n disp(name);\nend").unwrap();
    let Program::Functions(funcs) = prog else {
        panic!()
    };
    assert!(funcs[0].outputs.is_empty());
    assert_eq!(funcs[0].name, "greet");
}

#[test]
fn function_byref_param() {
    let prog = parse_program("function f(&x)\n x = 1;\nend").unwrap();
    let Program::Functions(funcs) = prog else {
        panic!()
    };
    assert!(funcs[0].inputs[0].by_ref);
}

#[test]
fn multiple_functions_in_file() {
    let src = "function main()\n helper();\nend\nfunction helper()\n disp(1);\nend";
    let prog = parse_program(src).unwrap();
    let Program::Functions(funcs) = prog else {
        panic!()
    };
    assert_eq!(funcs.len(), 2);
}

// --- real toolbox snippet --------------------------------------------------

#[test]
fn linspace_from_toolbox() {
    // Adapted from FreeMat/toolbox/array/linspace.m.
    let src = "function y = linspace(a, b, len)\n\
               if (nargin < 3)\n len = 100;\n end\n\
               if (a == b)\n y = a*ones(1, len);\n\
               else\n y = a + ((0:(len-1))*(b-a)/(len-1));\n end\nend";
    let prog = parse_program(src).unwrap_or_else(|e| panic!("linspace failed: {e}"));
    let Program::Functions(funcs) = prog else {
        panic!("not functions");
    };
    assert_eq!(funcs[0].name, "linspace");
    assert_eq!(funcs[0].inputs.len(), 3);
}

#[test]
fn statements_helper_parses_repl_line() {
    let stmts = parse_statements("a = 1, b = 2; c = a + b").unwrap();
    assert_eq!(stmts.len(), 3);
    assert_eq!(stmts[0].terminator, Terminator::Display);
    assert_eq!(stmts[1].terminator, Terminator::Quiet);
}

#[test]
fn nested_function() {
    let src = "function outer()\n x = 1;\n function inner()\n y = 2;\n end\nend";
    let prog = parse_program(src).unwrap_or_else(|e| panic!("nested failed: {e}"));
    let Program::Functions(funcs) = prog else {
        panic!()
    };
    assert_eq!(funcs[0].nested.len(), 1);
    assert_eq!(funcs[0].nested[0].name, "inner");
}
