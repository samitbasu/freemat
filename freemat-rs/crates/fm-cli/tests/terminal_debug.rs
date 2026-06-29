//! Terminal (`K>>`) debugging: the `dbstop`/`dbcont`/`dbstep`/`dbquit` +
//! `dbstack`/`dbup`/`dbdown` builtins, driven over the engine's raw
//! [`EvalReply`] protocol exactly as the interactive `main.rs` driver does.
//!
//! These exercise terminal debugging **without** a DAP client attached: a
//! `dbstop` breakpoint pauses the run, the engine streams a `Stopped` reply, and
//! nested evals run in the paused frame until a resume command. No IDE needed.

use std::sync::mpsc::Receiver;

use fm_cli::{Engine, EvalReply, StopInfo};

/// Block for the next reply and require it to be a stop; return the stop info.
fn expect_stopped(rx: &Receiver<EvalReply>) -> StopInfo {
    match rx.recv().expect("engine replied") {
        EvalReply::Stopped(info) => info,
        EvalReply::Done(o) => panic!("expected a stop, got Done: {:?}", o.output),
        EvalReply::Resumed => panic!("expected a stop, got Resumed"),
    }
}

/// Block for the next reply and require it to be the run's completion.
fn expect_done(rx: &Receiver<EvalReply>) -> Option<String> {
    match rx.recv().expect("engine replied") {
        EvalReply::Done(o) => o.error.map(|e| e.message),
        EvalReply::Stopped(_) => panic!("expected Done, got another stop"),
        EvalReply::Resumed => panic!("expected Done, got Resumed"),
    }
}

#[test]
fn dbstop_pauses_a_top_level_run_and_dbcont_resumes() {
    let engine = Engine::spawn(|_| {});

    // Break at line 2 of the next program.
    let set = engine.eval("dbstop(2)");
    assert!(set.error.is_none(), "dbstop failed: {:?}", set.error);

    // Run a three-line program; it pauses *before* line 2 runs.
    let rx = engine.send_eval_raw("a = 1;\nb = 2;\nc = 3;\n");
    let info = expect_stopped(&rx);
    assert_eq!(info.reason, "breakpoint");
    assert_eq!(info.line, 2, "stopped before line 2");
    assert_eq!(info.function, "", "top-level program");

    // The paused frame is live: line 1 ran (a = 1), line 2 has not (b unset).
    let a = engine.eval("a");
    assert!(
        a.output.contains('1'),
        "a should be visible: {:?}",
        a.output
    );
    let b = engine.eval("b");
    assert!(b.error.is_some(), "b must be undefined before line 2");

    // dbcont resumes; the run finishes line 3.
    let cont = engine.eval("dbcont");
    assert!(cont.error.is_none(), "dbcont errored: {:?}", cont.error);
    assert!(expect_done(&rx).is_none(), "run should finish cleanly");

    let c = engine.eval("c");
    assert!(
        c.output.contains('3'),
        "line 3 should have run: {:?}",
        c.output
    );
}

#[test]
fn dbstop_in_a_function_supports_dbstack_and_dbup() {
    let engine = Engine::spawn(|_| {});
    engine.eval("function y = inc(x)\ny = x + 1;\ny = y + 1;\nend\n");

    let set = engine.eval("dbstop('inc', 2)");
    assert!(set.error.is_none(), "dbstop failed: {:?}", set.error);

    let rx = engine.send_eval_raw("r = inc(5);\n");
    let info = expect_stopped(&rx);
    assert_eq!(info.function, "inc");
    assert_eq!(info.line, 2);

    // dbstack shows the inc frame (marked active) above base.
    let st = engine.eval("dbstack");
    assert!(st.output.contains("inc"), "dbstack: {:?}", st.output);
    assert!(st.output.contains("base"), "dbstack: {:?}", st.output);

    // The argument is bound in the paused frame.
    let x = engine.eval("x");
    assert!(x.output.contains('5'), "x bound to 5: {:?}", x.output);

    // dbup/dbdown move the inspected frame.
    let up = engine.eval("dbup");
    assert!(up.output.contains("base"), "dbup → base: {:?}", up.output);
    let down = engine.eval("dbdown");
    assert!(
        down.output.contains("inc"),
        "dbdown → inc: {:?}",
        down.output
    );

    // Resume; inc returns x+2 = 7.
    assert!(engine.eval("dbcont").error.is_none());
    assert!(expect_done(&rx).is_none());
    assert!(engine.eval("r").output.contains('7'), "inc(5) should be 7");
}

#[test]
fn dbstep_advances_one_statement_at_a_time() {
    let engine = Engine::spawn(|_| {});
    assert!(engine.eval("dbstop(1)").error.is_none());

    let rx = engine.send_eval_raw("p = 10;\nq = 20;\ns = p + q;\n");
    assert_eq!(expect_stopped(&rx).line, 1);

    // dbstep resumes and stops again at the next statement (line 2).
    assert!(engine.eval("dbstep").error.is_none());
    let stepped = expect_stopped(&rx);
    assert_eq!(stepped.line, 2, "stepped to line 2");
    assert_eq!(stepped.reason, "step");

    // Clear the line-1 breakpoint while paused, then run to completion. (A bare
    // top-level breakpoint persists and would otherwise re-fire on the next
    // single-line eval `s`, which is also line 1.)
    assert!(engine.eval("dbclear('all')").error.is_none());
    assert!(engine.eval("dbcont").error.is_none());
    assert!(expect_done(&rx).is_none());
    assert!(engine.eval("s").output.contains("30"));
}

#[test]
fn dbquit_aborts_the_paused_run() {
    let engine = Engine::spawn(|_| {});
    assert!(engine.eval("dbstop(2)").error.is_none());

    let rx = engine.send_eval_raw("u = 1;\nv = 2;\nw = 3;\n");
    expect_stopped(&rx);

    // dbquit aborts the run back to the top level.
    assert!(engine.eval("dbquit").error.is_none());
    let _ = expect_done(&rx);

    // Line 1 ran, but line 3 (after the breakpoint) never did.
    assert!(engine.eval("u").output.contains('1'));
    assert!(
        engine.eval("w").error.is_some(),
        "w must be undefined after dbquit aborted the run"
    );
}

#[test]
fn dbcont_outside_debug_mode_is_an_error() {
    let engine = Engine::spawn(|_| {});
    let out = engine.eval("dbcont");
    assert!(
        out.error.is_some(),
        "dbcont with no paused run should error"
    );
}

#[test]
fn dbstatus_lists_breakpoints() {
    let engine = Engine::spawn(|_| {});
    engine.eval("dbstop('foo', 4)");
    engine.eval("dbstop(7)");
    let status = engine.eval("dbstatus");
    assert!(status.output.contains("foo"), "status: {:?}", status.output);
    assert!(status.output.contains('4'), "status: {:?}", status.output);
    assert!(status.output.contains('7'), "status: {:?}", status.output);

    // dbclear removes them.
    engine.eval("dbclear('all')");
    let after = engine.eval("dbstatus");
    assert!(
        after.output.contains("No breakpoints"),
        "after dbclear: {:?}",
        after.output
    );
}
