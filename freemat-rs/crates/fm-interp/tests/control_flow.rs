//! Control flow: for/while/if/switch/try-catch/break/continue/return.

mod common;
use common::run_f64;

#[test]
fn if_elseif_else() {
    assert_eq!(
        run_f64("x = 5; if x > 3, y = 1; else, y = 0; end", "y"),
        1.0
    );
    assert_eq!(
        run_f64("x = 1; if x > 3, y = 1; else, y = 0; end", "y"),
        0.0
    );
    assert_eq!(
        run_f64(
            "x = 2; if x==1, y=10; elseif x==2, y=20; else, y=30; end",
            "y"
        ),
        20.0
    );
}

#[test]
fn for_loop_sum() {
    assert_eq!(run_f64("s = 0; for i = 1:5, s = s + i; end", "s"), 15.0);
}

#[test]
fn for_over_matrix_columns() {
    // for c = [1 2;3 4] iterates over columns [1;3] then [2;4]; sum each → 4,6
    assert_eq!(
        run_f64("t = 0; for c = [1 2;3 4], t = t + sum(c); end", "t"),
        10.0
    );
}

#[test]
fn while_loop() {
    assert_eq!(run_f64("i = 0; while i < 10, i = i + 1; end", "i"), 10.0);
}

#[test]
fn break_in_loop() {
    assert_eq!(
        run_f64(
            "s = 0; for i = 1:100, if i > 3, break; end, s = s + i; end",
            "s"
        ),
        6.0 // 1+2+3
    );
}

#[test]
fn continue_in_loop() {
    // Sum odd numbers 1..6 by skipping evens.
    assert_eq!(
        run_f64(
            "s = 0; for i = 1:6, if mod(i,2)==0, continue; end, s = s + i; end",
            "s"
        ),
        9.0 // 1+3+5
    );
}

#[test]
fn switch_numeric() {
    assert_eq!(
        run_f64(
            "x = 2; switch x, case 1, y=10; case 2, y=20; otherwise, y=0; end",
            "y"
        ),
        20.0
    );
}

#[test]
fn switch_string_and_cell() {
    assert_eq!(
        run_f64(
            "s = 'b'; switch s, case 'a', y=1; case {'b','c'}, y=2; otherwise, y=3; end",
            "y"
        ),
        2.0
    );
}

#[test]
fn switch_otherwise() {
    assert_eq!(
        run_f64("x = 9; switch x, case 1, y=1; otherwise, y=99; end", "y"),
        99.0
    );
}

#[test]
fn try_catch_recovers() {
    // The error in the try body is caught; the handler runs.
    assert_eq!(
        run_f64("y = 0; try, error('boom'); y = 1; catch, y = 2; end", "y"),
        2.0
    );
}

#[test]
fn try_no_error() {
    assert_eq!(run_f64("try, y = 5; catch, y = -1; end", "y"), 5.0);
}

#[test]
fn nested_loops_with_break() {
    // Break only exits the inner loop: inner runs j=1 then breaks, over 3 outer
    // iterations → c == 3.
    let src = "\
c = 0;
for i = 1:3
  for j = 1:3
    if j == 2
      break;
    end
    c = c + 1;
  end
end";
    assert_eq!(run_f64(src, "c"), 3.0);
}
