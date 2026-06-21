//! Browser help routes (§6.5 of `docs/HELP_SYSTEM.md`, phase P4).
//!
//! These mount onto the *existing* embedded graphics [`crate::server`] Router
//! (no second server): `GET /help` (index), `GET /help/{name}` (one topic), and
//! `GET /help/search?q=…` (JSON). They render with the pure
//! [`fm_doc::render::html`] functions and wrap the returned body fragments in a
//! shared HTML shell that pulls KaTeX + highlight.js + Plotly from CDNs (locked
//! decision §0 #5), mirroring `web/index.html`'s Plotly CDN usage and its
//! "vendor for offline" note.

use axum::Router;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Json};
use axum::routing::get;
use fm_doc::render::html;
use fm_doc::{Registry, Resolution};
use serde::{Deserialize, Serialize};

/// Add the `/help` routes to an existing Router. Called by
/// [`crate::server::start`] so the help pages share the one server with `/`
/// (graphics) and `/ws` (the websocket).
pub(crate) fn add_routes<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router
        .route("/help", get(index_page))
        .route("/help/search", get(search))
        .route("/help/{name}", get(topic_page))
}

/// `GET /help` — the index: sections → topics, plus a search box.
async fn index_page() -> Html<String> {
    let registry = Registry::global();
    let body = html::render_index(registry);
    Html(shell("FreeMat Help", &body))
}

/// `GET /help/{name}` — one topic, resolved per §6.1. `Exact` → the page;
/// `Suggestions` → a "did you mean" page; `None` → a 404 page.
async fn topic_page(Path(name): Path<String>) -> impl IntoResponse {
    let registry = Registry::global();
    match registry.resolve(&name) {
        Resolution::Exact(entry) => {
            let body = html::render_page(entry, registry);
            let title = format!("{} — FreeMat Help", entry.name);
            (StatusCode::OK, Html(shell(&title, &body)))
        }
        Resolution::Suggestions(candidates) => {
            let body = did_you_mean_body(&name, &candidates);
            (
                StatusCode::NOT_FOUND,
                Html(shell("Not found — FreeMat Help", &body)),
            )
        }
        Resolution::None => {
            let body = not_found_body(&name);
            (
                StatusCode::NOT_FOUND,
                Html(shell("Not found — FreeMat Help", &body)),
            )
        }
    }
}

/// Query for `GET /help/search`.
#[derive(Debug, Deserialize)]
struct SearchQuery {
    /// The search string (`?q=…`). Defaults to empty.
    #[serde(default)]
    q: String,
}

/// One search hit returned as JSON.
#[derive(Debug, Serialize)]
struct SearchHit {
    name: &'static str,
    summary: &'static str,
}

/// `GET /help/search?q=…` — JSON array of `{name, summary}`: the resolver's
/// exact/alias hit (if any) first, then substring matches on name/summary.
async fn search(Query(query): Query<SearchQuery>) -> Json<Vec<SearchHit>> {
    let registry = Registry::global();
    let q = query.q.trim();
    let mut hits: Vec<SearchHit> = Vec::new();
    let mut seen: Vec<&str> = Vec::new();

    if !q.is_empty() {
        // The exact/alias/case-insensitive resolution hit ranks first.
        if let Resolution::Exact(entry) = registry.resolve(q) {
            hits.push(SearchHit {
                name: entry.name,
                summary: entry.summary,
            });
            seen.push(entry.name);
        }
        let ql = q.to_lowercase();
        for e in registry.iter() {
            if seen.contains(&e.name) {
                continue;
            }
            if e.name.to_lowercase().contains(&ql) || e.summary.to_lowercase().contains(&ql) {
                hits.push(SearchHit {
                    name: e.name,
                    summary: e.summary,
                });
                seen.push(e.name);
            }
        }
    }

    Json(hits)
}

/// Body fragment for the "did you mean" page.
fn did_you_mean_body(query: &str, candidates: &[&str]) -> String {
    let mut s = String::from("<article class=\"fm-help-page\">\n<h1>Not found</h1>\n");
    s.push_str("<p>No help topic named <code>");
    push_escaped(&mut s, query);
    s.push_str("</code>. Did you mean:</p>\n<ul class=\"fm-topic-list\">\n");
    for c in candidates {
        s.push_str("  <li><a href=\"/help/");
        push_escaped(&mut s, c);
        s.push_str("\">");
        push_escaped(&mut s, c);
        s.push_str("</a></li>\n");
    }
    s.push_str("</ul>\n<p><a href=\"/help\">Back to the index</a></p>\n</article>\n");
    s
}

/// Body fragment for the 404 page (no suggestions).
fn not_found_body(query: &str) -> String {
    let mut s = String::from("<article class=\"fm-help-page\">\n<h1>Not found</h1>\n");
    s.push_str("<p>No help topic named <code>");
    push_escaped(&mut s, query);
    s.push_str("</code>.</p>\n<p><a href=\"/help\">Back to the index</a></p>\n</article>\n");
    s
}

/// Append `text` to `out` with HTML text-content escaping.
fn push_escaped(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
}

/// Wrap a body fragment in the shared HTML shell: a `<head>` pulling KaTeX (+
/// auto-render), highlight.js (+ a CSS theme), and Plotly from CDNs, an inline
/// `<style>`, and the figure/search/KaTeX client JS in the footer.
///
/// The CDN includes need internet access; to run offline, vendor the listed
/// assets next to `web/` and repoint these `src`/`href` URLs — the same escape
/// hatch the Plotly include in `web/index.html` documents.
fn shell(title: &str, body: &str) -> String {
    let mut esc_title = String::new();
    push_escaped(&mut esc_title, title);
    format!(
        r#"<!doctype html>
<!-- FreeMat-rs help page. Pulls KaTeX (math), highlight.js (code), and Plotly
     (figures) from CDNs. NOTE: these CDN includes need internet access. To run
     offline, vendor the .js/.css/fonts next to web/ and repoint the src/href
     URLs below — the same vendoring escape hatch web/index.html documents for
     its Plotly include. -->
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title}</title>
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css" />
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/gh/highlightjs/cdn-release@11.9.0/build/styles/github.min.css" />
  <script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.js"></script>
  <script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/contrib/auto-render.min.js"></script>
  <script src="https://cdn.jsdelivr.net/gh/highlightjs/cdn-release@11.9.0/build/highlight.min.js"></script>
  <script src="https://cdn.plot.ly/plotly-2.35.2.min.js" charset="utf-8"></script>
  <style>
    body {{ font-family: system-ui, sans-serif; margin: 0; background: #fafafa; color: #1b1b1b; }}
    header {{ padding: 8px 16px; background: #2b3a55; color: #fff; font-size: 14px; }}
    header a {{ color: #cfe0ff; text-decoration: none; }}
    main {{ max-width: 880px; margin: 0 auto; padding: 16px 24px 48px; }}
    h1.fm-title {{ font-size: 22px; }}
    .fm-summary {{ font-weight: 400; color: #555; }}
    pre {{ background: #f4f4f4; border: 1px solid #e2e2e2; border-radius: 4px;
          padding: 10px 12px; overflow-x: auto; }}
    pre.fm-transcript {{ background: #1e1e1e; margin: 0; }}
    pre.fm-transcript code {{ color: #e8e8e8; }}
    .fm-example {{ position: relative; margin: 12px 0; }}
    .fm-copy {{ position: absolute; top: 6px; right: 6px; z-index: 1;
                font: 12px ui-monospace, monospace; color: #ddd;
                background: #333; border: 1px solid #555; border-radius: 4px;
                padding: 2px 8px; cursor: pointer; opacity: 0.85; }}
    .fm-copy:hover {{ opacity: 1; background: #444; }}
    .fm-copy.fm-copied {{ color: #9f9; border-color: #6a6; }}
    code {{ font-family: ui-monospace, "SFMono-Regular", Menlo, monospace; }}
    p code, li code {{ background: #eee; padding: 0 3px; border-radius: 3px; }}
    table {{ border-collapse: collapse; }}
    th, td {{ border: 1px solid #ddd; padding: 4px 8px; }}
    .fm-figure {{ width: 100%; height: 420px; margin: 12px 0; border: 1px solid #ddd;
                 background: #fff; border-radius: 4px; }}
    .fm-search input {{ width: 100%; padding: 8px; font-size: 15px; box-sizing: border-box; }}
    .fm-search-results {{ list-style: none; padding: 0; margin: 6px 0; }}
    .fm-search-results li {{ padding: 2px 0; }}
    .fm-section-summary {{ color: #555; }}
    .fm-topic-list {{ line-height: 1.6; }}
  </style>
</head>
<body>
  <header><a href="/help">FreeMat Help</a> · <a href="/">graphics</a></header>
  <main>
{body}
  </main>
  <script>
    "use strict";
    // Typeset math once KaTeX + auto-render have loaded.
    window.addEventListener("DOMContentLoaded", function () {{
      if (window.renderMathInElement) {{
        renderMathInElement(document.body, {{
          delimiters: [
            {{ left: "$$", right: "$$", display: true }},
            {{ left: "$", right: "$", display: false }},
          ],
          throwOnError: false,
        }});
      }}
      // Highlight code blocks (highlight.js loaded synchronously).
      if (window.hljs) {{
        document.querySelectorAll("pre code").forEach(function (el) {{
          // Don't re-highlight the captured transcript (it's plain REPL text).
          if (!el.closest(".fm-transcript")) hljs.highlightElement(el);
        }});
      }}
      // "Copy" buttons on examples: copy the clean script (input only) so the
      // user can paste it straight into an `fm` session.
      document.querySelectorAll(".fm-copy").forEach(function (btn) {{
        btn.addEventListener("click", function () {{
          var script = btn.getAttribute("data-copy") || "";
          var done = function () {{
            var old = btn.textContent;
            btn.textContent = "Copied";
            btn.classList.add("fm-copied");
            setTimeout(function () {{
              btn.textContent = old;
              btn.classList.remove("fm-copied");
            }}, 1200);
          }};
          if (navigator.clipboard && navigator.clipboard.writeText) {{
            navigator.clipboard.writeText(script).then(done, function () {{ fallback(); }});
          }} else {{ fallback(); }}
          function fallback() {{
            var ta = document.createElement("textarea");
            ta.value = script; ta.style.position = "fixed"; ta.style.opacity = "0";
            document.body.appendChild(ta); ta.focus(); ta.select();
            try {{ document.execCommand("copy"); done(); }} catch (e) {{}}
            document.body.removeChild(ta);
          }}
        }});
      }});
      // Render captured figures with Plotly (mirrors web/index.html).
      document.querySelectorAll(".fm-figure").forEach(function (el) {{
        var raw = el.getAttribute("data-plotly");
        if (!raw) return;
        try {{
          var msg = JSON.parse(raw);
          // The captured payload is the scene *envelope* {{type:"scene",scene:{{…}}}}
          // (Scene::to_message), exactly like the websocket sends; unwrap it the
          // same way web/index.html does (renderScene(msg.scene)).
          var scene = msg.scene || msg;
          var fig = (scene.figures && scene.figures[0]) || null;
          var data = [];
          var layout = {{ margin: {{ l: 50, r: 20, t: 30, b: 40 }} }};
          var axes = (fig && fig.axes && fig.axes.length) ? fig.axes : [{{ series: [] }}];
          axes.forEach(function (ax) {{
            // 3-D series (surface/line3d) bind to a Plotly `scene`; without a
            // `layout.scene` object Plotly squashes the z-axis. Give it a "cube"
            // aspect ("data" under `axis equal`) and the requested camera view.
            var is3d = (ax.series || []).some(function (s) {{
              return s.kind === "surface" || s.kind === "line3d";
            }});
            if (is3d) {{
              var scene = {{ aspectmode: ax.equal ? "data" : "cube" }};
              if ((ax.xlabel || "").length) scene.xaxis = {{ title: ax.xlabel }};
              if ((ax.ylabel || "").length) scene.yaxis = {{ title: ax.ylabel }};
              if ((ax.zlabel || "").length) scene.zaxis = {{ title: ax.zlabel }};
              if (ax.view && ax.view.length >= 2) {{
                var az = ax.view[0] * Math.PI / 180, el = ax.view[1] * Math.PI / 180, r = 1.8;
                scene.camera = {{ eye: {{
                  x: r * Math.cos(el) * Math.sin(az),
                  y: -r * Math.cos(el) * Math.cos(az),
                  z: r * Math.sin(el),
                }} }};
              }}
              layout.scene = scene;
            }}
            var showscale = !!ax.colorbar;
            var cscale = function (cm) {{
              if (!cm) return "Viridis";
              if (typeof cm === "string" && cm.charAt(0) === "[") {{
                try {{ return JSON.parse(cm); }} catch (e) {{ return "Viridis"; }}
              }}
              return cm;
            }};
            (ax.series || []).forEach(function (s) {{
              if (s.kind === "line") {{
                var lmode = (s.line_style && s.line_style.length) ? "lines" : "markers";
                if (s.marker && s.marker.length && s.line_style && s.line_style.length) lmode = "lines+markers";
                data.push({{ type: "scatter", mode: lmode, x: s.x, y: s.y,
                             name: s.name || "", line: s.color ? {{ color: s.color }} : {{}},
                             marker: s.color ? {{ color: s.color }} : {{}} }});
              }} else if (s.kind === "surface") {{
                data.push({{ type: "surface", z: s.z, x: s.x, y: s.y,
                             colorscale: cscale(s.colormap) }});
              }} else if (s.kind === "image") {{
                data.push({{ type: "heatmap", z: s.data, colorscale: cscale(s.colormap), showscale: showscale }});
              }} else if (s.kind === "pcolor") {{
                data.push({{ type: "heatmap", z: s.data, x: s.x, y: s.y,
                             colorscale: cscale(s.colormap), showscale: showscale }});
              }} else if (s.kind === "contour") {{
                var ct = {{ type: "contour", z: s.z, x: s.x, y: s.y,
                            colorscale: cscale(s.colormap), showscale: showscale,
                            contours: {{ coloring: s.filled ? "fill" : "lines" }} }};
                data.push(ct);
              }} else if (s.kind === "bar") {{
                var b = {{ type: "bar", name: s.name || "" }};
                if (s.horizontal) {{ b.orientation = "h"; b.y = s.x; b.x = s.y; }}
                else {{ b.x = s.x; b.y = s.y; }}
                if (s.color) b.marker = {{ color: s.color }};
                data.push(b);
              }} else if (s.kind === "stem") {{
                var xs = [], ys = [];
                for (var k = 0; k < s.x.length; k++) {{ xs.push(s.x[k], s.x[k], null); ys.push(0, s.y[k], null); }}
                data.push({{ type: "scatter", mode: "lines", x: xs, y: ys, connectgaps: false,
                             line: s.color ? {{ color: s.color }} : {{}}, showlegend: false }});
                data.push({{ type: "scatter", mode: "markers", x: s.x, y: s.y,
                             marker: {{ symbol: "circle", color: s.color || undefined }}, name: s.name || "" }});
              }} else if (s.kind === "stairs") {{
                data.push({{ type: "scatter", mode: "lines", x: s.x, y: s.y,
                             line: {{ shape: "hv", color: s.color || undefined }}, name: s.name || "" }});
              }} else if (s.kind === "errorbar") {{
                data.push({{ type: "scatter", mode: "lines+markers", x: s.x, y: s.y,
                             error_y: {{ type: "data", array: s.e, visible: true }},
                             line: s.color ? {{ color: s.color }} : {{}}, name: s.name || "" }});
              }} else if (s.kind === "line3d") {{
                var l3mode = (s.line_style && s.line_style.length) ? "lines" : "markers";
                data.push({{ type: "scatter3d", mode: l3mode, x: s.x, y: s.y, z: s.z,
                             line: s.color ? {{ color: s.color }} : {{}},
                             marker: s.color ? {{ color: s.color }} : {{}}, name: s.name || "" }});
              }}
            }});
          }});
          Plotly.newPlot(el, data, layout, {{ responsive: true }});
        }} catch (e) {{ console.error("bad figure JSON", e); }}
      }});
    }});

    // Live search: GET /help/search?q=… and list the hits.
    (function () {{
      var input = document.getElementById("fm-search-input");
      var results = document.getElementById("fm-search-results");
      if (!input || !results) return;
      input.addEventListener("input", function () {{
        var q = input.value.trim();
        if (!q) {{ results.innerHTML = ""; return; }}
        fetch("/help/search?q=" + encodeURIComponent(q))
          .then(function (r) {{ return r.json(); }})
          .then(function (hits) {{
            results.innerHTML = "";
            hits.forEach(function (h) {{
              var li = document.createElement("li");
              var a = document.createElement("a");
              a.href = "/help/" + h.name;
              a.textContent = h.name;
              li.appendChild(a);
              li.appendChild(document.createTextNode(" — " + h.summary));
              results.appendChild(li);
            }});
          }})
          .catch(function (e) {{ console.error("search failed", e); }});
      }});
    }})();
  </script>
</body>
</html>
"#
    )
}
