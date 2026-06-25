//! A minimal DAP client used by the integration tests, plus a harness that
//! spins up a real `fm-dap` server on a TCP loopback and connects to it.
//!
//! The server runs the interpreter (which is `!Send` because the debug hook is
//! `Rc`-backed) entirely on its own thread; the test thread only ever touches a
//! `TcpStream`, so there are no cross-thread interpreter borrows.
#![allow(dead_code, reason = "shared client helpers used across several tests")]

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::JoinHandle;

use serde_json::{Value, json};

/// A connected DAP client speaking the `Content-Length`-framed wire protocol.
pub struct DapClient {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
    seq: i64,
    /// The server thread, joined on drop so panics surface.
    server: Option<JoinHandle<()>>,
}

impl DapClient {
    /// Start a server on an ephemeral port and connect a client to it.
    pub fn spawn() -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind loopback");
        let addr = listener.local_addr().expect("local addr");

        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("server accept");
            let reader = BufReader::new(stream.try_clone().expect("clone stream"));
            fm_dap::serve(Box::new(reader), Box::new(stream)).expect("serve session");
        });

        let stream = TcpStream::connect(addr).expect("client connect");
        let reader = BufReader::new(stream.try_clone().expect("clone client stream"));
        DapClient {
            reader,
            writer: stream,
            seq: 0,
            server: Some(server),
        }
    }

    /// Send a request and return the seq it was sent with.
    pub fn send(&mut self, command: &str, arguments: Value) -> i64 {
        self.seq += 1;
        let msg = json!({
            "seq": self.seq,
            "type": "request",
            "command": command,
            "arguments": arguments,
        });
        write_frame(&mut self.writer, &msg);
        self.seq
    }

    /// Read the next framed message from the server.
    pub fn recv(&mut self) -> Value {
        read_frame(&mut self.reader).expect("server closed unexpectedly")
    }

    /// Read messages until the `response` for `request_seq` arrives, returning
    /// it. Events seen along the way are discarded (use [`Self::wait_event`] when
    /// you care about them).
    pub fn wait_response(&mut self, request_seq: i64) -> Value {
        loop {
            let msg = self.recv();
            if msg["type"] == "response" && msg["request_seq"] == json!(request_seq) {
                return msg;
            }
        }
    }

    /// Send a request and return its response (discarding interleaved events).
    pub fn request(&mut self, command: &str, arguments: Value) -> Value {
        let seq = self.send(command, arguments);
        self.wait_response(seq)
    }

    /// Read messages until an event named `event` arrives, returning it.
    pub fn wait_event(&mut self, event: &str) -> Value {
        loop {
            let msg = self.recv();
            if msg["type"] == "event" && msg["event"] == json!(event) {
                return msg;
            }
        }
    }

    /// Drive the standard startup handshake: initialize, set line breakpoints on
    /// the inline program `source`, launch, configurationDone. Returns once the
    /// program is running.
    pub fn launch(&mut self, source: &str, breakpoints: &[i64], stop_on_entry: bool) {
        let init = self.request("initialize", json!({ "adapterID": "fm-dap" }));
        assert_eq!(init["success"], json!(true), "initialize failed");
        // The server announces `initialized` after responding to initialize.
        self.wait_event("initialized");

        self.request(
            "launch",
            json!({ "source": source, "program": "test.m", "stopOnEntry": stop_on_entry }),
        );

        let bps: Vec<Value> = breakpoints.iter().map(|l| json!({ "line": l })).collect();
        let resp = self.request(
            "setBreakpoints",
            json!({ "source": { "path": "test.m" }, "breakpoints": bps }),
        );
        assert_eq!(resp["success"], json!(true), "setBreakpoints failed");

        self.request("configurationDone", json!({}));
    }

    /// Convenience: read the locals of frame `frame_id` as a name→value map.
    pub fn locals(&mut self, frame_id: i64) -> std::collections::HashMap<String, String> {
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

impl Drop for DapClient {
    fn drop(&mut self) {
        // Best-effort clean shutdown so the server thread returns and we can
        // observe any panic it raised.
        let _ = self.send("disconnect", json!({}));
        let _ = self.writer.flush();
        if let Some(server) = self.server.take() {
            let _ = server.join();
        }
    }
}

fn write_frame(stream: &mut TcpStream, msg: &Value) {
    let body = serde_json::to_vec(msg).unwrap();
    write!(stream, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
    stream.write_all(&body).unwrap();
    stream.flush().unwrap();
}

fn read_frame<R: BufRead>(reader: &mut R) -> Option<Value> {
    let mut content_length: Option<usize> = None;
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
    let len = content_length?;
    let mut buf = vec![0u8; len];
    std::io::Read::read_exact(reader, &mut buf).ok()?;
    serde_json::from_slice(&buf).ok()
}
