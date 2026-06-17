//! Milestone-2 automated proof: start the embedded graphics server, connect a
//! websocket client, run `x=0:.1:10; plot(x,sin(x))` through the interpreter
//! (with the server installed as the graphics sink), and assert the client
//! receives the expected scene JSON — a line trace with the right x/y.
//!
//! This stands in for the manual browser check: it exercises exactly the path a
//! real browser tab uses (the same `/ws` endpoint and the same scene message).

use std::time::Duration;

use fm_interp::Interpreter;
use futures_util::StreamExt;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn plot_streams_expected_scene_over_websocket() {
    // 1. Start the server on an OS-chosen port and install it as the sink.
    let handle = fm_cli::start(0).expect("server starts");
    let ws_url = format!("ws://{}/ws", handle.addr);

    let mut interp = Interpreter::new();
    fm_builtins::register_standard_library(&mut interp);
    interp.set_graphics_sink(Box::new(handle.clone()));

    // 2. Connect a websocket client (a stand-in for the browser tab).
    let (mut ws, _resp) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .expect("websocket connects");

    // The first message is the (empty) current-scene snapshot on connect.
    let first = next_scene(&mut ws).await;
    assert_eq!(first["type"], "scene");
    assert_eq!(
        first["scene"]["figures"].as_array().map(Vec::len),
        Some(0),
        "fresh connection sees an empty scene"
    );

    // 3. Run the Milestone-2 commands. The implicit draw flushes through the
    //    sink, which broadcasts to our connected client.
    interp.run("x = 0:.1:10;").expect("range ok");
    interp.run("plot(x, sin(x));").expect("plot ok");

    // 4. Receive the scene update and assert the line trace.
    let scene = loop {
        let msg = next_scene(&mut ws).await;
        let figs = msg["scene"]["figures"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        if !figs.is_empty() {
            break msg;
        }
    };

    let series = &scene["scene"]["figures"][0]["axes"][0]["series"][0];
    assert_eq!(series["kind"], "line");
    let x = series["x"].as_array().expect("x array");
    let y = series["y"].as_array().expect("y array");
    assert_eq!(x.len(), 101);
    assert_eq!(y.len(), 101);
    assert!((x[0].as_f64().unwrap() - 0.0).abs() < 1e-12);
    assert!((x[100].as_f64().unwrap() - 10.0).abs() < 1e-12);
    assert!((y[0].as_f64().unwrap() - 0.0_f64.sin()).abs() < 1e-12);
    // sin(1.0) at x = 1.0 (the 11th sample, index 10).
    assert!((y[10].as_f64().unwrap() - 1.0_f64.sin()).abs() < 1e-9);
    assert_eq!(series["color"], "rgb(0,0,255)");
    assert_eq!(series["line_style"], "-");

    // 5. A second plot updates live: drive it and assert the new y data.
    interp.run("plot(x, cos(x));").expect("second plot ok");
    let scene2 = next_scene(&mut ws).await;
    let series2 = &scene2["scene"]["figures"][0]["axes"][0]["series"][0];
    let y2 = series2["y"].as_array().expect("y2 array");
    assert!((y2[0].as_f64().unwrap() - 0.0_f64.cos()).abs() < 1e-12);
}

/// Read the next websocket text frame and parse it as JSON, with a timeout so a
/// hang fails the test instead of blocking forever.
async fn next_scene<S>(ws: &mut S) -> serde_json::Value
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
