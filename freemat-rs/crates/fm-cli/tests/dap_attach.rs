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
