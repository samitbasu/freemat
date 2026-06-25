//! Phase 2 (interpreter-as-service): the embedded DAP-over-TCP "attach" server.
//!
//! A DAP client attaches to a live engine, sets a breakpoint, and a *separate*
//! REPL `Eval` (via [`EngineClient`]) triggers a run that stops at it — proving
//! the debugger and the REPL share one interpreter session.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use fm_cli::Engine;
use serde_json::{Value, json};

/// A minimal DAP client over a `TcpStream`.
struct DapClient {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
    seq: i64,
}

impl DapClient {
    fn connect(port: u16) -> Self {
        let stream = TcpStream::connect(("127.0.0.1", port)).expect("connect to DAP port");
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let reader = BufReader::new(stream.try_clone().unwrap());
        DapClient {
            reader,
            writer: stream,
            seq: 0,
        }
    }

    fn send(&mut self, command: &str, arguments: Value) -> i64 {
        self.seq += 1;
        let msg = json!({
            "seq": self.seq, "type": "request", "command": command, "arguments": arguments,
        });
        let body = serde_json::to_vec(&msg).unwrap();
        write!(self.writer, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
        self.writer.write_all(&body).unwrap();
        self.writer.flush().unwrap();
        self.seq
    }

    fn recv(&mut self) -> Value {
        read_frame(&mut self.reader).expect("server closed / timed out")
    }

    /// Send a request and return its response (discarding interleaved events).
    fn request(&mut self, command: &str, arguments: Value) -> Value {
        let seq = self.send(command, arguments);
        loop {
            let msg = self.recv();
            if msg["type"] == "response" && msg["request_seq"] == json!(seq) {
                return msg;
            }
        }
    }

    fn wait_event(&mut self, event: &str) -> Value {
        loop {
            let msg = self.recv();
            if msg["type"] == "event" && msg["event"] == json!(event) {
                return msg;
            }
        }
    }

    /// The attach handshake: initialize, attach, set breakpoints, configurationDone.
    fn attach(&mut self, breakpoints: &[i64]) {
        let init = self.request("initialize", json!({ "adapterID": "fm-dap" }));
        assert_eq!(init["success"], json!(true));
        self.wait_event("initialized");
        self.request("attach", json!({}));
        let bps: Vec<Value> = breakpoints.iter().map(|l| json!({ "line": l })).collect();
        let resp = self.request(
            "setBreakpoints",
            json!({ "source": { "path": "session.m" }, "breakpoints": bps }),
        );
        assert_eq!(resp["success"], json!(true));
        self.request("configurationDone", json!({}));
    }

    fn locals(&mut self, frame_id: i64) -> std::collections::HashMap<String, String> {
        let scopes = self.request("scopes", json!({ "frameId": frame_id }));
        let var_ref = scopes["body"]["scopes"][0]["variablesReference"].clone();
        let vars = self.request("variables", json!({ "variablesReference": var_ref }));
        let mut out = std::collections::HashMap::new();
        for v in vars["body"]["variables"].as_array().unwrap() {
            out.insert(
                v["name"].as_str().unwrap().to_string(),
                v["value"].as_str().unwrap().to_string(),
            );
        }
        out
    }
}

fn read_frame<R: BufRead>(reader: &mut R) -> Option<Value> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(v) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(v.trim().parse().ok()?);
        }
    }
    let len: usize = content_length?;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).ok()?;
    serde_json::from_slice(&buf).ok()
}

fn num(s: &str) -> f64 {
    s.trim()
        .parse()
        .unwrap_or_else(|_| panic!("not numeric: {s:?}"))
}

const SESSION: &str = "a = 10;\nb = 20;\nc = a + b;\nd = c * 2;\n";

#[test]
fn debugger_attaches_to_the_live_repl_session() {
    let (engine, port) = Engine::spawn_with_dap(0, |_| {}).expect("spawn engine with DAP");

    let mut dap = DapClient::connect(port);
    // Break on line 3 (`c = a + b;`).
    dap.attach(&[3]);

    // Trigger a run from a *separate* client (the "REPL"), which will block at
    // the breakpoint until we continue.
    let client = engine.client();
    let runner = std::thread::spawn(move || client.eval(SESSION));

    // The debugger sees the stop.
    let stopped = dap.wait_event("stopped");
    assert_eq!(stopped["body"]["reason"], json!("breakpoint"));

    // The call stack and locals reflect the live run: a and b set, c not yet.
    let stack = dap.request("stackTrace", json!({ "threadId": 1 }));
    let frame = &stack["body"]["stackFrames"][0];
    assert_eq!(frame["line"], json!(3), "stopped on line 3");
    let locals = dap.locals(frame["id"].as_i64().unwrap());
    assert_eq!(num(&locals["a"]), 10.0);
    assert_eq!(num(&locals["b"]), 20.0);
    assert!(!locals.contains_key("c"), "c not assigned yet: {locals:?}");

    // Evaluate in the stopped frame.
    let ev = dap.request("evaluate", json!({ "expression": "a + b" }));
    assert_eq!(num(ev["body"]["result"].as_str().unwrap()), 30.0);

    // Resume; the run completes.
    dap.request("continue", json!({ "threadId": 1 }));
    let outcome = runner.join().expect("runner thread");
    assert!(
        outcome.error.is_none(),
        "run errored: {:?}",
        outcome.error.map(|e| e.message)
    );

    // The session persists: the same engine sees the variables the run created
    // (c = a + b = 30, d = c * 2 = 60).
    let check = engine.eval("d");
    assert!(
        check.output.contains("60"),
        "session state lost: {:?}",
        check.output
    );
}

#[test]
fn stepping_through_a_repl_run_advances_lines() {
    let (engine, port) = Engine::spawn_with_dap(0, |_| {}).expect("spawn engine with DAP");
    let mut dap = DapClient::connect(port);
    dap.attach(&[1]);

    let client = engine.client();
    let runner = std::thread::spawn(move || client.eval(SESSION));

    dap.wait_event("stopped"); // at line 1
    dap.request("next", json!({ "threadId": 1 }));
    let stepped = dap.wait_event("stopped");
    assert_eq!(stepped["body"]["reason"], json!("step"));
    let stack = dap.request("stackTrace", json!({ "threadId": 1 }));
    assert_eq!(stack["body"]["stackFrames"][0]["line"], json!(2));

    dap.request("continue", json!({ "threadId": 1 }));
    let outcome = runner.join().unwrap();
    assert!(outcome.error.is_none());
}

#[test]
fn runs_are_not_paused_before_a_debugger_connects() {
    // With DAP enabled but no client connected, breakpoints can't exist and runs
    // proceed normally (the hook is a no-op while no writer is attached).
    let (engine, _port) = Engine::spawn_with_dap(0, |_| {}).expect("spawn engine with DAP");
    let out = engine.eval("x = 7 * 6");
    assert!(out.error.is_none());
    assert!(
        out.output.contains("42"),
        "run did not complete: {:?}",
        out.output
    );
}

// ---- Phase 3: the nested debugger REPL --------------------------------------

#[test]
fn nested_repl_eval_mutates_the_paused_frame() {
    let (engine, port) = Engine::spawn_with_dap(0, |_| {}).expect("spawn engine with DAP");
    let mut dap = DapClient::connect(port);
    dap.attach(&[3]); // break before `c = a + b;`

    let client = engine.client();
    let runner = std::thread::spawn(move || client.eval(SESSION));
    dap.wait_event("stopped");

    // A second client runs a line *in the paused frame*: change `a`.
    let nested = engine.client().eval("a = 100;");
    assert!(
        nested.error.is_none(),
        "nested eval errored: {:?}",
        nested.error.map(|e| e.message)
    );

    // Resume: `c = a + b` now uses the mutated a (100 + 20 = 120), so d = 240.
    dap.request("continue", json!({ "threadId": 1 }));
    runner.join().unwrap();
    let check = engine.eval("d");
    assert!(
        check.output.contains("240"),
        "nested-frame mutation not reflected on resume: {:?}",
        check.output
    );
}

#[test]
fn recursive_breakpoint_pushes_and_pops_levels() {
    let (engine, port) = Engine::spawn_with_dap(0, |_| {}).expect("spawn engine with DAP");
    let mut dap = DapClient::connect(port);
    // Break on line 3. In SESSION that's `c = a + b;`; the nested command below
    // also puts its breakpoint-worthy statement on its own line 3.
    dap.attach(&[3]);

    let client = engine.client();
    let runner = std::thread::spawn(move || client.eval(SESSION));
    let s1 = dap.wait_event("stopped");
    assert_eq!(
        s1["body"]["fmPauseLevel"],
        json!(1),
        "first stop is level 1"
    );

    // A nested command whose own line 3 trips the breakpoint → a deeper stop. It
    // runs on its own thread because it blocks at that breakpoint.
    let nested_client = engine.client();
    let nested = std::thread::spawn(move || nested_client.eval("p = 40;\nr = 2;\ns = p + r;\n"));

    let s2 = dap.wait_event("stopped");
    assert_eq!(
        s2["body"]["fmPauseLevel"],
        json!(2),
        "nested stop is level 2"
    );
    assert_eq!(s2["body"]["reason"], json!("breakpoint"));

    // Pop level 2 (the nested command finishes), then level 1 (the program).
    dap.request("continue", json!({ "threadId": 1 }));
    nested.join().unwrap();
    dap.request("continue", json!({ "threadId": 1 }));
    runner.join().unwrap();

    // The nested command's work landed in the shared session (s = p + r = 42).
    assert!(
        engine.eval("s").output.contains("42"),
        "nested frame's assignments not persisted"
    );
}

#[test]
fn debug_console_repl_evaluate_mutates_the_frame() {
    let (engine, port) = Engine::spawn_with_dap(0, |_| {}).expect("spawn engine with DAP");
    let mut dap = DapClient::connect(port);
    dap.attach(&[3]);

    let client = engine.client();
    let runner = std::thread::spawn(move || client.eval(SESSION));
    dap.wait_event("stopped");

    // The IDE debug console: `evaluate` with context "repl" runs a *statement*
    // in the paused frame (not just an expression).
    let ev = dap.request(
        "evaluate",
        json!({ "expression": "a = 100;", "context": "repl" }),
    );
    assert_eq!(ev["success"], json!(true));

    dap.request("continue", json!({ "threadId": 1 }));
    runner.join().unwrap();
    assert!(
        engine.eval("d").output.contains("240"),
        "debug-console assignment not reflected on resume"
    );
}

// ---- Phase 4: pause + output streaming --------------------------------------

#[test]
fn pause_stops_a_running_program() {
    let (engine, port) = Engine::spawn_with_dap(0, |_| {}).expect("spawn engine with DAP");
    let mut dap = DapClient::connect(port);
    dap.attach(&[]); // no breakpoints — we'll pause asynchronously

    let client = engine.client();
    let runner = std::thread::spawn(move || client.eval("while true; x = 1; end"));

    // Let the loop get going (so run_eval has cleared any stale pause), then ask
    // the IDE-side `pause`.
    std::thread::sleep(Duration::from_millis(100));
    dap.send("pause", json!({ "threadId": 1 }));

    let stopped = dap.wait_event("stopped");
    assert_eq!(stopped["body"]["reason"], json!("pause"));

    // Disconnect to abort the otherwise-infinite run, then it unwinds cleanly.
    dap.request("disconnect", json!({}));
    runner.join().unwrap();
}

#[test]
fn program_output_streams_to_the_debug_console() {
    let (engine, port) = Engine::spawn_with_dap(0, |_| {}).expect("spawn engine with DAP");
    let mut dap = DapClient::connect(port);
    dap.attach(&[3]); // break after the two disp() calls

    let client = engine.client();
    let prog = "disp(111);\ndisp(222);\nx = 5;\n";
    let runner = std::thread::spawn(move || client.eval(prog));

    // Collect `output` events until the program stops at the breakpoint.
    let mut console = String::new();
    loop {
        let m = dap.recv();
        if m["type"] == "event" && m["event"] == json!("output") {
            console.push_str(m["body"]["output"].as_str().unwrap_or(""));
        } else if m["type"] == "event" && m["event"] == json!("stopped") {
            break;
        }
    }
    assert!(
        console.contains("111") && console.contains("222"),
        "program output did not reach the debug console: {console:?}"
    );

    dap.request("continue", json!({ "threadId": 1 }));
    let outcome = runner.join().unwrap();
    // The REPL reply still carries the full output too (one buffer, two sinks).
    assert!(
        outcome.output.contains("111") && outcome.output.contains("222"),
        "REPL lost output that was mirrored to the console: {:?}",
        outcome.output
    );
}
