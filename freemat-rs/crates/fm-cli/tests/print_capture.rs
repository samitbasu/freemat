//! Proof of the `print` round-trip: the synchronous interpreter asks the
//! graphics server to render a figure to an image; the server broadcasts a
//! `capture` request over the websocket; the (browser stand-in) client replies
//! with `{"type":"image",...}`; and `render_image` / the `print` builtin returns
//! exactly those bytes. Also proves the no-browser failure path returns a clean
//! error instead of hanging.

use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use fm_graphics::GraphicsSink;
use fm_interp::Interpreter;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

/// Read the next websocket text frame and parse it as JSON, with a timeout so a
/// hang fails the test instead of blocking forever.
async fn next_json<S>(ws: &mut S) -> serde_json::Value
where
    S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    let msg = tokio::time::timeout(Duration::from_secs(5), ws.next())
        .await
        .expect("no timeout")
        .expect("stream not closed")
        .expect("frame ok");
    match msg {
        Message::Text(t) => serde_json::from_str(&t).expect("valid JSON"),
        other => panic!("unexpected ws frame: {other:?}"),
    }
}

#[tokio::test]
async fn render_image_round_trips_through_the_browser() {
    let handle = fm_cli::start(0).expect("server starts");
    let ws_url = format!("ws://{}/ws", handle.addr);

    // Build a figure through the interpreter, with the server as the sink.
    let mut interp = Interpreter::new();
    fm_builtins::register_standard_library(&mut interp);
    interp.set_graphics_sink(Box::new(handle.clone()));

    // Connect the browser stand-in and drain the initial snapshot.
    let (mut ws, _resp) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .expect("websocket connects");
    let _empty = next_json(&mut ws).await;

    interp.run("x = 0:.1:10;").expect("range ok");
    interp.run("plot(x, sin(x));").expect("plot ok");

    // Drain the scene update so the next frame we wait on is the capture request.
    let scene = next_json(&mut ws).await;
    let fig = scene["scene"]["figures"][0]["id"].as_u64().expect("fig id");

    // Call render_image on a blocking thread (it uses recv_timeout).
    let h2 = handle.clone();
    let render = std::thread::spawn(move || h2.render_image(fig, "png"));

    // The server should broadcast a capture request to us.
    let cap = next_json(&mut ws).await;
    assert_eq!(cap["type"], "capture");
    assert_eq!(cap["figure"].as_u64(), Some(fig));
    assert_eq!(cap["format"], "png");
    let reqid = cap["reqid"].as_u64().expect("reqid");

    // Reply with some known bytes, base64-encoded as the browser would.
    let bytes = b"PNGDATA".to_vec();
    let data = BASE64.encode(&bytes);
    let reply = serde_json::json!({ "type": "image", "reqid": reqid, "data": data });
    ws.send(Message::Text(reply.to_string()))
        .await
        .expect("send reply");

    let got = render.join().expect("render thread");
    assert_eq!(got, Some(Ok(bytes)), "render_image returns the exact bytes");
}

#[tokio::test]
async fn print_builtin_writes_browser_image_to_file() {
    let handle = fm_cli::start(0).expect("server starts");
    let ws_url = format!("ws://{}/ws", handle.addr);

    let tmp = std::env::temp_dir().join(format!("fm-print-{}.png", std::process::id()));
    let tmp_path = tmp.to_string_lossy().to_string();
    let _ = std::fs::remove_file(&tmp);

    let (mut ws, _resp) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .expect("websocket connects");
    let _empty = next_json(&mut ws).await;

    // Run `print` on a blocking thread (the interpreter is synchronous).
    let h2 = handle.clone();
    let tmp_for_thread = tmp_path.clone();
    let printer = std::thread::spawn(move || {
        let mut interp = Interpreter::new();
        fm_builtins::register_standard_library(&mut interp);
        interp.set_graphics_sink(Box::new(h2));
        interp.run("x = 0:.1:10;").expect("range ok");
        interp.run("plot(x, sin(x));").expect("plot ok");
        interp.run(&format!("print('{tmp_for_thread}');"))
    });

    // Drain frames until we see the capture request (a scene frame precedes it).
    let reqid = loop {
        let msg = next_json(&mut ws).await;
        if msg["type"] == "capture" {
            assert_eq!(msg["format"], "png");
            break msg["reqid"].as_u64().expect("reqid");
        }
    };

    let bytes = b"PNGFILEBYTES".to_vec();
    let data = BASE64.encode(&bytes);
    let reply = serde_json::json!({ "type": "image", "reqid": reqid, "data": data });
    ws.send(Message::Text(reply.to_string()))
        .await
        .expect("send reply");

    printer.join().expect("print thread").expect("print ok");

    let written = std::fs::read(&tmp).expect("file written");
    assert_eq!(written, bytes, "print wrote the browser image bytes");
    let _ = std::fs::remove_file(&tmp);
}

#[tokio::test]
async fn render_image_errors_cleanly_with_no_browser() {
    // No client ever connects → render_image must return Some(Err(..)) promptly.
    let handle = fm_cli::start(0).expect("server starts");
    let h2 = handle.clone();
    let result = tokio::task::spawn_blocking(move || h2.render_image(1, "png"))
        .await
        .expect("blocking task");
    match result {
        Some(Err(msg)) => assert!(msg.contains("no browser"), "got: {msg}"),
        other => panic!("expected a no-browser error, got {other:?}"),
    }
}

#[tokio::test]
async fn print_builtin_errors_cleanly_with_no_browser() {
    let handle = fm_cli::start(0).expect("server starts");
    let h2 = handle.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut interp = Interpreter::new();
        fm_builtins::register_standard_library(&mut interp);
        interp.set_graphics_sink(Box::new(h2));
        interp.run("plot(1:3);").expect("plot ok");
        interp.run("print('nope.png');")
    })
    .await
    .expect("blocking task");
    assert!(
        result.is_err(),
        "print with no browser should error, not hang"
    );
}

#[tokio::test]
async fn print_builtin_errors_cleanly_with_no_sink() {
    // No graphics sink at all → a clean error, no panic.
    let result = tokio::task::spawn_blocking(|| {
        let mut interp = Interpreter::new();
        fm_builtins::register_standard_library(&mut interp);
        interp.run("plot(1:3);").expect("plot ok");
        interp.run("print('nope.png');")
    })
    .await
    .expect("blocking task");
    assert!(result.is_err(), "print with no sink should error");
}
