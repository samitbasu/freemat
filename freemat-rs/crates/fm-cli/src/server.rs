//! The embedded graphics webserver: an `axum` app on a background `tokio`
//! runtime serving `web/index.html` (Plotly + a small websocket client) and a
//! websocket that streams the [`Scene`](fm_graphics::Scene) as JSON.
//!
//! # How the REPL and server coexist
//! The REPL is a blocking `rustyline` loop on the main thread. The server runs
//! on its own multi-threaded `tokio` runtime in a **background OS thread**, so
//! the two never block each other. They communicate through:
//! - a [`tokio::sync::broadcast`] channel of serialized scene JSON, and
//! - a shared `Mutex<Scene>` holding the latest snapshot (sent to each new
//!   browser tab on connect, so a freshly-opened tab shows existing figures).
//!
//! The interpreter pushes updates via [`ServerHandle`] (which implements
//! [`fm_graphics::GraphicsSink`]): on `drawnow` / the implicit post-command
//! draw, it stores the snapshot and broadcasts the JSON; connected clients
//! receive it and Plotly redraws in place.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender as StdSender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use fm_graphics::{GraphicsSink, Scene, capture_message};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::broadcast;

/// In-flight `print`/`capture` requests, keyed by `reqid`. The blocking
/// `render_image` call inserts a std mpsc Sender here and waits; the websocket
/// `recv_task` looks it up when the matching `image` reply arrives and forwards
/// the result.
type PendingMap = Arc<Mutex<HashMap<u64, StdSender<Result<Vec<u8>, String>>>>>;

/// The browser's reply to a `capture` request.
#[derive(Deserialize)]
struct ImageReply {
    /// Frame discriminator; we only act on `"image"`.
    #[serde(rename = "type")]
    kind: String,
    /// The correlation id from the originating `capture` frame.
    reqid: u64,
    /// Base64 image data (optionally a `data:...;base64,` data URL) on success.
    #[serde(default)]
    data: Option<String>,
    /// Error string on failure.
    #[serde(default)]
    error: Option<String>,
}

/// The static frontend, embedded so the binary is self-contained.
const INDEX_HTML: &str = include_str!("../../../web/index.html");

/// Server state shared by the websocket handler.
#[derive(Clone)]
struct AppState {
    /// Broadcasts serialized scene JSON to all connected clients.
    tx: broadcast::Sender<String>,
    /// The latest scene snapshot (sent to each new connection).
    latest: Arc<Mutex<Scene>>,
    /// In-flight `print`/`capture` requests, keyed by `reqid`.
    pending: PendingMap,
}

/// A handle the interpreter uses to publish scenes; implements
/// [`GraphicsSink`]. Cheap to clone; publishing is non-blocking.
#[derive(Clone)]
pub struct ServerHandle {
    tx: broadcast::Sender<String>,
    latest: Arc<Mutex<Scene>>,
    /// In-flight `print`/`capture` requests, keyed by `reqid`.
    pending: PendingMap,
    /// Monotonic source of `reqid`s for capture round-trips.
    next_reqid: Arc<AtomicU64>,
    /// The bound address (so the REPL can print / open the URL).
    pub addr: SocketAddr,
}

impl ServerHandle {
    /// The `http://host:port` URL the browser should open.
    #[must_use]
    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }
}

impl GraphicsSink for ServerHandle {
    fn publish(&self, scene: &Scene) {
        // Store the snapshot (for new connections) and broadcast the JSON.
        if let Ok(mut guard) = self.latest.lock() {
            *guard = scene.clone();
        }
        if let Ok(json) = scene.to_message() {
            // Ignore "no subscribers" — the scene is retained in `latest`.
            let _ = self.tx.send(json);
        }
    }

    /// The running server's base URL, so `help`/`helpwin` can form the
    /// `{base}/help/<name>` page URL (help-system P5).
    fn base_url(&self) -> Option<String> {
        Some(self.url())
    }

    /// Render a figure to image bytes via a websocket round-trip to the browser
    /// (which calls `Plotly.toImage`). Runs on the synchronous interpreter
    /// thread and blocks (via `recv_timeout`) until the browser replies.
    fn render_image(&self, figure: u64, format: &str) -> Option<Result<Vec<u8>, String>> {
        // No browser tab is subscribed → nothing can render the figure.
        if self.tx.receiver_count() == 0 {
            return Some(Err(
                "no browser is connected — open the graphics URL to use print".into(),
            ));
        }

        let reqid = self.next_reqid.fetch_add(1, Ordering::Relaxed);
        let (reply_tx, reply_rx) = std::sync::mpsc::channel::<Result<Vec<u8>, String>>();
        if let Ok(mut pend) = self.pending.lock() {
            pend.insert(reqid, reply_tx);
        }

        // Broadcast the capture request to all connected browsers.
        match capture_message(reqid, figure, format) {
            Ok(json) => {
                let _ = self.tx.send(json);
            }
            Err(e) => {
                if let Ok(mut pend) = self.pending.lock() {
                    pend.remove(&reqid);
                }
                return Some(Err(format!("failed to build capture request: {e}")));
            }
        }

        // Block waiting for the browser's reply (or time out).
        let result = match reply_rx.recv_timeout(Duration::from_secs(15)) {
            Ok(r) => r,
            Err(_) => Err("timed out waiting for the browser to render the figure".into()),
        };
        if let Ok(mut pend) = self.pending.lock() {
            pend.remove(&reqid);
        }
        Some(result)
    }
}

/// Start the graphics webserver on `127.0.0.1:<port>` (port `0` = OS-chosen).
///
/// Spawns a background thread running a multi-threaded `tokio` runtime, binds
/// the listener synchronously (so the returned handle has the real port), and
/// returns a [`ServerHandle`] to install as the interpreter's graphics sink.
///
/// # Errors
/// Returns an error string if the runtime can't start or the port can't bind.
pub fn start(port: u16) -> Result<ServerHandle, String> {
    let (tx, _rx) = broadcast::channel::<String>(64);
    let latest = Arc::new(Mutex::new(Scene::new()));
    let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
    let next_reqid = Arc::new(AtomicU64::new(1));
    let state = AppState {
        tx: tx.clone(),
        latest: latest.clone(),
        pending: pending.clone(),
    };

    // Bind synchronously with a std listener so we know the real address before
    // returning (important when port == 0), without needing an async runtime
    // here — `start` can be called from inside another tokio runtime (tests).
    let std_listener = std::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port)))
        .map_err(|e| format!("bind failed: {e}"))?;
    std_listener
        .set_nonblocking(true)
        .map_err(|e| e.to_string())?;
    let addr = std_listener.local_addr().map_err(|e| e.to_string())?;

    // Run the server on its own background runtime/thread.
    std::thread::Builder::new()
        .name("fm-graphics-server".into())
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("graphics server runtime failed: {e}");
                    return;
                }
            };
            rt.block_on(async move {
                let app = crate::help::add_routes(
                    Router::new()
                        .route("/", get(|| async { Html(INDEX_HTML) }))
                        .route("/ws", get(ws_handler)),
                )
                .with_state(state);
                // Adopt the std listener into this runtime.
                let listener = match tokio::net::TcpListener::from_std(std_listener) {
                    Ok(l) => l,
                    Err(e) => {
                        eprintln!("graphics listener error: {e}");
                        return;
                    }
                };
                if let Err(e) = axum::serve(listener, app).await {
                    eprintln!("graphics server error: {e}");
                }
            });
        })
        .map_err(|e| e.to_string())?;

    Ok(ServerHandle {
        tx,
        latest,
        pending,
        next_reqid,
        addr,
    })
}

/// Upgrade an HTTP request to a websocket connection.
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Per-connection loop: send the current scene immediately, then forward every
/// broadcast update until the client disconnects.
async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();

    // 1. Send the current scene so a freshly-opened tab shows existing figures.
    //    Clone out under the lock, then drop the guard before any `.await`.
    let snapshot = state.latest.lock().ok().map(|g| g.clone());
    if let Some(scene) = snapshot
        && let Ok(json) = scene.to_message()
        && sender.send(Message::Text(json.into())).await.is_err()
    {
        return;
    }

    // 2. Pump broadcast updates to this client, while draining inbound frames
    //    (so close/ping are handled). Either task ending closes the connection.
    let send_task = tokio::spawn(async move {
        while let Ok(json) = rx.recv().await {
            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });
    let pending = state.pending.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Close(_) => break,
                Message::Text(t) => handle_inbound(&pending, &t),
                _ => {}
            }
        }
    });
    // When either side finishes, abort the other.
    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
    }
}

/// Parse one inbound text frame; if it is an `image` reply to a pending
/// `capture` request, decode it and forward the result to the waiting
/// `render_image` caller. Unknown frames are ignored.
fn handle_inbound(pending: &PendingMap, text: &str) {
    let Ok(reply) = serde_json::from_str::<ImageReply>(text) else {
        return;
    };
    if reply.kind != "image" {
        return;
    }
    let Some(sender) = pending.lock().ok().and_then(|mut p| p.remove(&reply.reqid)) else {
        return; // No waiter (already timed out / removed); drop the reply.
    };
    let result = if let Some(err) = reply.error {
        Err(err)
    } else if let Some(data) = reply.data {
        // Strip an optional `data:...;base64,` data-URL prefix before decoding.
        let b64 = match data.find("base64,") {
            Some(i) => &data[i + "base64,".len()..],
            None => data.as_str(),
        };
        BASE64
            .decode(b64.as_bytes())
            .map_err(|e| format!("failed to decode image data: {e}"))
    } else {
        Err("browser returned an image reply with neither data nor error".into())
    };
    let _ = sender.send(result);
}
