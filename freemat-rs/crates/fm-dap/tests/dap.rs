//! End-to-end DAP server tests. Each test drives a real `fm-dap` session over a
//! TCP loopback via the [`common::DapClient`] harness, exercising the protocol
//! exactly as an editor would.

mod common;

use common::DapClient;
use serde_json::json;

/// Parse a DAP variable/`evaluate` value string (e.g. `"3"`, `"3.0000"`) as a
/// number, so assertions don't hinge on the formatter's exact padding.
fn num(s: &str) -> f64 {
    s.trim()
        .parse()
        .unwrap_or_else(|_| panic!("not numeric: {s:?}"))
}

const STRAIGHT_LINE: &str = "x = 1;\ny = 2;\nz = x + y;\nw = z * 2;\n";

#[test]
fn breakpoint_stops_at_the_right_line_with_correct_variables() {
    let mut c = DapClient::spawn();
    c.launch(STRAIGHT_LINE, &[3], false);

    // The run pauses before line 3 (`z = x + y;`).
    let stopped = c.wait_event("stopped");
    assert_eq!(stopped["body"]["reason"], json!("breakpoint"));

    let stack = c.request("stackTrace", json!({ "threadId": 1 }));
    let frames = stack["body"]["stackFrames"].as_array().unwrap();
    assert_eq!(frames.len(), 1, "single top-level frame");
    assert_eq!(frames[0]["line"], json!(3), "stopped on line 3");

    // x and y already ran; z has not been assigned yet.
    let frame_id = frames[0]["id"].as_i64().unwrap();
    let locals = c.locals(frame_id);
    assert_eq!(num(&locals["x"]), 1.0);
    assert_eq!(num(&locals["y"]), 2.0);
    assert!(
        !locals.contains_key("z"),
        "z not assigned yet, got {locals:?}"
    );
}

#[test]
fn evaluate_runs_in_the_stopped_frame() {
    let mut c = DapClient::spawn();
    c.launch(STRAIGHT_LINE, &[3], false);
    c.wait_event("stopped");

    let resp = c.request("evaluate", json!({ "expression": "x + y" }));
    assert_eq!(resp["success"], json!(true));
    assert_eq!(num(resp["body"]["result"].as_str().unwrap()), 3.0);

    // An undefined name reports an error rather than a wrong value.
    let bad = c.request("evaluate", json!({ "expression": "nope_undefined" }));
    assert_eq!(bad["success"], json!(false));
}

#[test]
fn stepping_advances_one_line_and_reveals_new_locals() {
    let mut c = DapClient::spawn();
    c.launch(STRAIGHT_LINE, &[3], false);
    c.wait_event("stopped"); // at line 3

    // Step over `z = x + y;` → pause before line 4.
    c.request("next", json!({ "threadId": 1 }));
    let stopped = c.wait_event("stopped");
    assert_eq!(stopped["body"]["reason"], json!("step"));

    let stack = c.request("stackTrace", json!({ "threadId": 1 }));
    let frame = &stack["body"]["stackFrames"][0];
    assert_eq!(frame["line"], json!(4), "advanced to line 4");

    // z is now defined and equals 3.
    let locals = c.locals(frame["id"].as_i64().unwrap());
    assert_eq!(num(&locals["z"]), 3.0);
}

#[test]
fn continue_runs_to_termination() {
    let mut c = DapClient::spawn();
    c.launch(STRAIGHT_LINE, &[3], false);
    c.wait_event("stopped");

    c.request("continue", json!({ "threadId": 1 }));
    // No further breakpoints → the program terminates.
    c.wait_event("terminated");
}

#[test]
fn stop_on_entry_pauses_before_the_first_statement() {
    let mut c = DapClient::spawn();
    c.launch(STRAIGHT_LINE, &[], true);

    let stopped = c.wait_event("stopped");
    assert_eq!(stopped["body"]["reason"], json!("step"));
    let stack = c.request("stackTrace", json!({ "threadId": 1 }));
    assert_eq!(stack["body"]["stackFrames"][0]["line"], json!(1));
}

#[test]
fn multiple_breakpoints_are_hit_in_order() {
    let mut c = DapClient::spawn();
    c.launch(STRAIGHT_LINE, &[2, 4], false);

    c.wait_event("stopped");
    let s1 = c.request("stackTrace", json!({ "threadId": 1 }));
    assert_eq!(s1["body"]["stackFrames"][0]["line"], json!(2));

    c.request("continue", json!({ "threadId": 1 }));
    c.wait_event("stopped");
    let s2 = c.request("stackTrace", json!({ "threadId": 1 }));
    assert_eq!(s2["body"]["stackFrames"][0]["line"], json!(4));

    c.request("continue", json!({ "threadId": 1 }));
    c.wait_event("terminated");
}

/// A program whose call stack grows: stepping into `sq` should expose a second
/// frame whose locals include the argument.
const WITH_CALL: &str = "a = 3;\nfunction r = sq(x)\n  r = x * x;\nend\nb = sq(a);\n";

#[test]
fn step_into_a_function_shows_a_nested_frame() {
    let mut c = DapClient::spawn();
    // Break on the call site (line 5: `b = sq(a);`).
    c.launch(WITH_CALL, &[5], false);
    c.wait_event("stopped");

    // Step in → we should land inside `sq` at line 3 (`r = x * x;`).
    c.request("stepIn", json!({ "threadId": 1 }));
    c.wait_event("stopped");

    let stack = c.request("stackTrace", json!({ "threadId": 1 }));
    let frames = stack["body"]["stackFrames"].as_array().unwrap();
    assert_eq!(frames.len(), 2, "main + sq, got {frames:#?}");
    // Innermost frame first.
    assert_eq!(frames[0]["name"], json!("sq"));
    assert_eq!(frames[0]["line"], json!(3));

    // The callee's argument is bound in its own frame.
    let callee_locals = c.locals(frames[0]["id"].as_i64().unwrap());
    assert_eq!(num(&callee_locals["x"]), 3.0);

    // The caller frame still shows `a`.
    let caller_locals = c.locals(frames[1]["id"].as_i64().unwrap());
    assert_eq!(num(&caller_locals["a"]), 3.0);
}

#[test]
fn initialize_advertises_configuration_done_support() {
    let mut c = DapClient::spawn();
    let init = c.request("initialize", json!({ "adapterID": "fm-dap" }));
    assert_eq!(init["success"], json!(true));
    assert_eq!(
        init["body"]["supportsConfigurationDoneRequest"],
        json!(true)
    );
}
