//! P4 proof: the browser help routes are served by the *same* embedded server
//! as `/` and `/ws`. Starts the server (`fm_cli::start`), then issues plain
//! HTTP/1.0 GETs (no extra HTTP-client dependency) against `/help`,
//! `/help/cos`, `/help/pi`, and `/help/search?q=cos`, asserting:
//! - all return `200 OK`,
//! - the `cos` page renders its uppercased title and Usage section,
//! - the `pi` page renders its captured transcript (input echo `-->`),
//! - search returns the `cos` topic.
//!
//! The full migrated corpus (help-system P7) is linked into `fm-cli` via
//! `fm-builtins`, so the global registry the routes read includes these topics.

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Minimal HTTP/1.0 GET: returns `(status_line, body)`. HTTP/1.0 + `Connection:
/// close` means the server closes the socket at end-of-body, so reading to EOF
/// gives the whole response without parsing `Content-Length`.
async fn http_get(addr: std::net::SocketAddr, path: &str) -> (String, String) {
    let mut stream = TcpStream::connect(addr).await.expect("connect");
    let req = format!("GET {path} HTTP/1.0\r\nHost: {addr}\r\n\r\n");
    stream.write_all(req.as_bytes()).await.expect("write");
    let mut raw = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut raw))
        .await
        .expect("no timeout")
        .expect("read");
    let text = String::from_utf8_lossy(&raw).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((text.as_str(), ""));
    let status = head.lines().next().unwrap_or("").to_string();
    (status, body.to_string())
}

#[tokio::test]
async fn help_routes_serve_index_topic_and_search() {
    let handle = fm_cli::start(0).expect("server starts");
    let addr = handle.addr;

    // /help — the index, 200, lists the cos topic and the section.
    let (status, body) = http_get(addr, "/help").await;
    assert!(status.contains("200"), "GET /help status: {status}");
    assert!(
        body.contains("Mathematical Functions"),
        "index lists the section: {body}"
    );
    assert!(
        body.contains("/help/cos"),
        "index links the cos topic: {body}"
    );
    assert!(
        body.contains("fm-search-input"),
        "index has a search box: {body}"
    );

    // /help/cos — the topic page, 200, with the uppercased title + Usage
    // section. (`cos`'s migrated example is a figure plot, downgraded to
    // display-only, so it has no captured transcript — the transcript-rendering
    // assertions below use `pi`, which keeps a live `fm-exec` fragment.)
    let (status, body) = http_get(addr, "/help/cos").await;
    assert!(status.contains("200"), "GET /help/cos status: {status}");
    assert!(body.contains("COS"), "uppercased title: {body}");
    assert!(body.contains("<h2>Usage</h2>"), "Usage section: {body}");

    // /help/pi — a topic with a kept-live fragment: its captured transcript is
    // rendered (input echo `-->`, HTML-escaped, in an `fm-transcript` block) and
    // the executable source is highlighted.
    let (status, body) = http_get(addr, "/help/pi").await;
    assert!(status.contains("200"), "GET /help/pi status: {status}");
    assert!(
        body.contains("fm-transcript"),
        "captured transcript block present: {body}"
    );
    // The transcript is HTML-escaped: `-->` becomes `--&gt;`.
    assert!(
        body.contains("--&gt;"),
        "transcript input echo present: {body}"
    );
    // Executed blocks render as the transcript only (no duplicated source
    // fence); a "Copy" button carries the clean pasteable script.
    assert!(
        !body.contains("class=\"language-fm-exec\""),
        "exec source fence should be suppressed: {body}"
    );
    assert!(
        body.contains("class=\"fm-copy\"") && body.contains("data-copy="),
        "copy button present: {body}"
    );

    // /help/search?q=cos — JSON array including cos.
    let (status, body) = http_get(addr, "/help/search?q=cos").await;
    assert!(status.contains("200"), "GET /help/search status: {status}");
    let hits: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
    let arr = hits.as_array().expect("array");
    assert!(
        arr.iter().any(|h| h["name"] == "cos"),
        "search returns cos: {body}"
    );

    // A missing topic returns 404.
    let (status, _body) = http_get(addr, "/help/definitelynotatopic").await;
    assert!(status.contains("404"), "missing topic is 404: {status}");
}
