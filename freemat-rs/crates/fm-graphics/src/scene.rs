//! The retained scene-graph: a semantic model of FreeMat's handle hierarchy.
//!
//! This mirrors `Figure → Axes → Series` at a **semantic** level (not pixel
//! primitives): a line series carries `x`/`y`/style/color/marker/legend; a
//! surface carries `Z` + colormap; axes carry limits/scale/labels/title/grid.
//! The model is renderer-agnostic — the frontend (`web/index.html`) maps it onto
//! Plotly traces + layout. Everything is [`serde::Serialize`] so the whole scene
//! can be streamed over the websocket as JSON.

use serde::{Deserialize, Serialize};

/// The whole graphics state: every open figure, keyed by stable id.
///
/// This is the wire payload sent to a freshly-connected browser tab (so it shows
/// existing figures) and re-sent on every update.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Scene {
    /// Open figures, in creation order.
    pub figures: Vec<Figure>,
}

impl Scene {
    /// An empty scene.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Serialize the scene as a wire message (`{"type":"scene", ...}`).
    ///
    /// # Errors
    /// Propagates any `serde_json` serialization error (should not happen for
    /// this plain-data model).
    pub fn to_message(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&WireMessage::Scene { scene: self })
    }

    /// Look up a figure by id.
    #[must_use]
    pub fn figure(&self, id: u64) -> Option<&Figure> {
        self.figures.iter().find(|f| f.id == id)
    }

    /// Look up a figure by id (mutable), inserting it if missing.
    pub fn figure_mut_or_insert(&mut self, id: u64) -> &mut Figure {
        if let Some(pos) = self.figures.iter().position(|f| f.id == id) {
            &mut self.figures[pos]
        } else {
            self.figures.push(Figure::new(id));
            self.figures.last_mut().unwrap()
        }
    }
}

/// The websocket wire message envelope. Tagged so the frontend can dispatch.
/// Serialize-only (it borrows the scene); the frontend parses JSON in JS.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WireMessage<'a> {
    /// A full scene snapshot (sent on connect and on every update).
    Scene {
        /// The scene to render.
        scene: &'a Scene,
    },
    /// A request that the browser render a figure to image bytes and reply with
    /// an `{"type":"image",...}` frame carrying the same `reqid` (the `print`
    /// round-trip). Owned, so it is `'static`-friendly for the server's blocking
    /// request path.
    Capture {
        /// Correlation id matching the eventual `image` reply.
        reqid: u64,
        /// The figure id to render.
        figure: u64,
        /// Image format (`png`/`jpeg`/`webp`/`svg`).
        format: String,
    },
}

/// Serialize a `capture` request frame (`{"type":"capture",reqid,figure,format}`).
///
/// # Errors
/// Propagates any `serde_json` serialization error (should not happen).
pub fn capture_message(reqid: u64, figure: u64, format: &str) -> Result<String, serde_json::Error> {
    serde_json::to_string(&WireMessage::Capture {
        reqid,
        figure,
        format: format.to_string(),
    })
}

/// A figure window: an ordered stack of axes plus a title/visibility.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Figure {
    /// Stable figure id (FreeMat figure number; 1-based).
    pub id: u64,
    /// The axes contained in this figure (Stage 7.5: N axes for `subplot`).
    pub axes: Vec<Axes>,
    /// Index of the current axes within `axes` (`gca` target). Defaults to the
    /// last axes; `subplot`/`axes` switch it.
    #[serde(default)]
    pub current_axes: usize,
    /// The figure's current colormap, already resolved to a Plotly colorscale
    /// (a named scale like `"Hot"` / `"Jet"`, or an explicit RGB scale
    /// serialized as a JSON array string `[[t,"rgb(r,g,b)"],…]`). Set by
    /// `colormap(...)`; color-mapped series (`image`/`surf`/`pcolor`/…) stamp it
    /// onto themselves so the renderer uses it. `None` = renderer default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colormap: Option<String>,
    /// The figure's pixel size `[width, height]` (`sizefig`); `None` = renderer
    /// default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<[f64; 2]>,
}

impl Figure {
    /// A new empty figure with a single default full-frame axes.
    #[must_use]
    pub fn new(id: u64) -> Self {
        Figure {
            id,
            axes: vec![Axes::new()],
            current_axes: 0,
            colormap: None,
            size: None,
        }
    }

    /// The current axes (the `subplot`/`axes`-selected one), creating one if
    /// the figure somehow has none.
    pub fn current_axes_mut(&mut self) -> &mut Axes {
        if self.axes.is_empty() {
            self.axes.push(Axes::new());
            self.current_axes = 0;
        }
        let idx = self.current_axes.min(self.axes.len() - 1);
        self.current_axes = idx;
        &mut self.axes[idx]
    }
}

/// A coordinate-system: data limits, scales, labels, title, grid, and the
/// series (lines / surfaces / images) drawn into it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Axes {
    /// Stable handle id assigned by the interpreter's registry (`gca`, `set`,
    /// `get` all key off this). `0` for an axes not yet registered.
    #[serde(default)]
    pub handle: u64,
    /// Normalized position rectangle `[left, bottom, width, height]` in figure
    /// coordinates (0..1). Drives the Plotly subplot domain. Default is the
    /// full frame, matching a single-axes figure.
    #[serde(default = "full_position")]
    pub position: [f64; 4],
    /// The data series drawn in this axes, in z-order.
    pub series: Vec<Series>,
    /// Axes title (empty = none).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    /// X-axis label.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub xlabel: String,
    /// Y-axis label.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ylabel: String,
    /// Z-axis label (3-D).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub zlabel: String,
    /// Explicit `[xmin, xmax, ymin, ymax]` limits (`axis([...])`); `None` = auto.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<AxisLimits>,
    /// Explicit color-axis limits `[cmin, cmax]` (`clim([...])`); `None` = auto.
    /// Renderers map this to `zmin`/`zmax` on the color-mapped trace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clim: Option<[f64; 2]>,
    /// X-axis scale.
    #[serde(default, skip_serializing_if = "Scale::is_linear")]
    pub xscale: Scale,
    /// Y-axis scale.
    #[serde(default, skip_serializing_if = "Scale::is_linear")]
    pub yscale: Scale,
    /// Whether grid lines are shown.
    #[serde(default, skip_serializing_if = "is_false")]
    pub grid: bool,
    /// Whether to show the legend, and (optionally) explicit entry names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legend: Option<Legend>,
    /// Whether the axes is in `hold on` mode (new series append vs replace).
    #[serde(default, skip_serializing_if = "is_false")]
    pub hold: bool,
    /// `axis equal` — equal data-unit aspect ratio.
    #[serde(default, skip_serializing_if = "is_false")]
    pub equal: bool,
    /// Requested 3-D viewing angles `[azimuth, elevation]` in degrees, as set by
    /// `view(az, el)` / `view(3)` / `view(2)`. `None` = renderer default. The
    /// renderers map this to the Plotly `scene.camera.eye`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<[f64; 2]>,
    /// `colorbar` — show the colorscale for the color-mapped series in this axes.
    /// The renderers turn this into `showscale:true` on the color-mapped trace.
    #[serde(default, skip_serializing_if = "is_false")]
    pub colorbar: bool,
    /// Free-floating text annotations placed at data coordinates (`text(x,y,str)`
    /// / `text(x,y,z,str)`). Not a series — the renderers emit each as a Plotly
    /// `layout.annotations` entry (`showarrow:false`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub texts: Vec<TextLabel>,
}

impl Default for Axes {
    fn default() -> Self {
        Axes {
            handle: 0,
            position: full_position(),
            series: Vec::new(),
            title: String::new(),
            xlabel: String::new(),
            ylabel: String::new(),
            zlabel: String::new(),
            limits: None,
            clim: None,
            xscale: Scale::Linear,
            yscale: Scale::Linear,
            grid: false,
            legend: None,
            hold: false,
            equal: false,
            view: None,
            colorbar: false,
            texts: Vec::new(),
        }
    }
}

impl Axes {
    /// A fresh empty full-frame axes.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A fresh axes occupying the given normalized `[left, bottom, w, h]`.
    #[must_use]
    pub fn with_position(position: [f64; 4]) -> Self {
        Axes {
            position,
            ..Self::default()
        }
    }
}

/// The default full-frame axes position `[left, bottom, width, height]`.
#[must_use]
fn full_position() -> [f64; 4] {
    [0.0, 0.0, 1.0, 1.0]
}

/// Explicit axis limits set via `axis([xmin xmax ymin ymax])`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct AxisLimits {
    /// Minimum x.
    pub xmin: f64,
    /// Maximum x.
    pub xmax: f64,
    /// Minimum y.
    pub ymin: f64,
    /// Maximum y.
    pub ymax: f64,
    /// Minimum z (3-D). `None` = auto / not set (kept out of serialized JSON so
    /// existing 2-D figure transcripts are byte-for-byte unchanged).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zmin: Option<f64>,
    /// Maximum z (3-D). `None` = auto / not set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zmax: Option<f64>,
}

/// An axis scale.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Scale {
    /// Linear scale (the default).
    #[default]
    Linear,
    /// Logarithmic scale.
    Log,
}

impl Scale {
    /// True when linear (used to skip serializing the common case).
    #[must_use]
    pub fn is_linear(&self) -> bool {
        matches!(self, Scale::Linear)
    }
}

/// Legend configuration for an axes.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Legend {
    /// Whether the legend is visible.
    pub visible: bool,
    /// Explicit entry names (override per-series `name`), if given.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<String>,
}

/// A free-floating text annotation at data coordinates (`text(x,y,str)` /
/// `text(x,y,z,str)`). The renderers emit it as a Plotly `layout.annotations`
/// entry with `showarrow:false`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TextLabel {
    /// X data coordinate.
    pub x: f64,
    /// Y data coordinate.
    pub y: f64,
    /// Optional Z data coordinate (3-D `text`); `None` for a 2-D annotation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub z: Option<f64>,
    /// The annotation string.
    pub str: String,
}

/// A drawable data series. The variant maps onto a Plotly trace kind.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Series {
    /// A 2-D line / scatter series (`plot`, `line`).
    Line(LineSeries),
    /// A 3-D surface (`surf`, `mesh`).
    Surface(SurfaceSeries),
    /// A 2-D image / heatmap (`image`, `imagesc`).
    Image(ImageSeries),
    /// A 2-D pseudocolor plot (`pcolor`): flat-shaded cells over an x/y grid,
    /// rendered as a heatmap honoring the current colormap.
    Pcolor(PcolorSeries),
    /// A 2-D contour plot (`contour`, `contourf`).
    Contour(ContourSeries),
    /// A bar chart (`bar`, `barh`).
    Bar(BarSeries),
    /// A stem plot (`stem`).
    Stem(StemSeries),
    /// A staircase step plot (`stairs`).
    Stairs(StairsSeries),
    /// A line with symmetric vertical error bars (`errorbar`).
    Errorbar(ErrorbarSeries),
    /// A 3-D line plot (`plot3`).
    Line3d(Line3dSeries),
    /// A filled area under a line (`area`) → scatter with `fill:"tozeroy"`.
    Area(AreaSeries),
    /// A filled polygon (`fill`) → scatter with `fill:"toself"`.
    Fill(FillSeries),
    /// A pie chart (`pie`) → Plotly `pie` trace (a non-axes, full-cell trace).
    Pie(PieSeries),
    /// A polar line plot (`polar`) → Plotly `scatterpolar` on a polar subplot.
    Polar(PolarSeries),
    /// A 2-D vector field (`quiver`) → null-gapped scatter line segments.
    Quiver(QuiverSeries),
}

/// A 2-D line series: x/y data plus style/color/marker/legend.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LineSeries {
    /// X data (column-major flattened vector).
    pub x: Vec<f64>,
    /// Y data.
    pub y: Vec<f64>,
    /// Line style (`-`, `--`, `:`, `-.`, or empty = no line).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub line_style: String,
    /// Marker symbol (`o`, `+`, `*`, `.`, `x`, `s`, `d`, ... or empty = none).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub marker: String,
    /// CSS / `rgb(r,g,b)` color string; empty = let the frontend cycle.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub color: String,
    /// Legend display name (empty = auto / none).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
}

/// A 3-D surface: a Z grid plus optional explicit x/y vectors and a colormap.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SurfaceSeries {
    /// Z values as rows (`z[row][col]`).
    pub z: Vec<Vec<f64>>,
    /// Optional x coordinates (length = number of columns).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub x: Vec<f64>,
    /// Optional y coordinates (length = number of rows).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub y: Vec<f64>,
    /// Optional full 2-D X grid (`xmat[row][col]`) for parametric surfaces such
    /// as `tubeplot`, where each vertex has independent x/y/z coordinates and a
    /// 1-D `x`/`y` vector cannot describe the mesh. Empty = use `x`/`y` instead.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub xmat: Vec<Vec<f64>>,
    /// Optional full 2-D Y grid (`ymat[row][col]`) for parametric surfaces.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ymat: Vec<Vec<f64>>,
    /// Colormap name (Plotly colorscale, e.g. `Viridis`, `Jet`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub colormap: String,
    /// `true` = wireframe (`mesh`), `false` = filled (`surf`).
    #[serde(default, skip_serializing_if = "is_false")]
    pub wireframe: bool,
}

/// A 2-D contour plot: a Z grid, optional x/y vectors, optional explicit levels.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ContourSeries {
    /// Z values as rows (`z[row][col]`).
    pub z: Vec<Vec<f64>>,
    /// Optional x coordinates (length = number of columns).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub x: Vec<f64>,
    /// Optional y coordinates (length = number of rows).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub y: Vec<f64>,
    /// Explicit contour levels (empty = let the frontend auto-pick).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub levels: Vec<f64>,
    /// Colormap name (Plotly colorscale).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub colormap: String,
    /// `true` = filled contour (`contourf`) → Plotly `contours.coloring:"fill"`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub filled: bool,
    /// `true` = draw inline level labels on the contour lines (`clabel`).
    #[serde(default, skip_serializing_if = "is_false")]
    pub labels: bool,
}

/// A 2-D pseudocolor plot (`pcolor`): a value grid plus optional x/y vectors,
/// rendered as a flat-shaded heatmap honoring the current colormap.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PcolorSeries {
    /// Cell values as rows (`data[row][col]`).
    pub data: Vec<Vec<f64>>,
    /// Optional x coordinates (length = number of columns).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub x: Vec<f64>,
    /// Optional y coordinates (length = number of rows).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub y: Vec<f64>,
    /// Colormap name (Plotly colorscale).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub colormap: String,
}

/// A 2-D image / heatmap: a value grid.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ImageSeries {
    /// Pixel values as rows (`data[row][col]`).
    pub data: Vec<Vec<f64>>,
    /// Colormap name.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub colormap: String,
}

/// A bar chart: x positions and bar heights, with an orientation.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BarSeries {
    /// Bar positions (category centers).
    pub x: Vec<f64>,
    /// Bar heights.
    pub y: Vec<f64>,
    /// `true` = horizontal bars (`barh`), `false` = vertical (`bar`).
    #[serde(default, skip_serializing_if = "is_false")]
    pub horizontal: bool,
    /// CSS / `rgb(r,g,b)` color string; empty = let the frontend cycle.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub color: String,
    /// Legend display name (empty = auto / none).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
}

/// A stem plot: markers atop vertical stems rising from the baseline.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StemSeries {
    /// X data.
    pub x: Vec<f64>,
    /// Y data (stem heights).
    pub y: Vec<f64>,
    /// CSS / `rgb(r,g,b)` color string; empty = let the frontend cycle.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub color: String,
    /// Marker symbol (default `o`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub marker: String,
    /// Legend display name.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
}

/// A staircase step plot (`stairs`): a piecewise-constant line.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StairsSeries {
    /// X data.
    pub x: Vec<f64>,
    /// Y data.
    pub y: Vec<f64>,
    /// CSS / `rgb(r,g,b)` color string; empty = let the frontend cycle.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub color: String,
    /// Legend display name.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
}

/// A line with symmetric vertical error bars (`errorbar`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ErrorbarSeries {
    /// X data.
    pub x: Vec<f64>,
    /// Y data.
    pub y: Vec<f64>,
    /// Symmetric error magnitudes (one per point).
    pub e: Vec<f64>,
    /// CSS / `rgb(r,g,b)` color string; empty = let the frontend cycle.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub color: String,
    /// Legend display name.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
}

/// A 3-D line plot (`plot3`): x/y/z polyline.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Line3dSeries {
    /// X data.
    pub x: Vec<f64>,
    /// Y data.
    pub y: Vec<f64>,
    /// Z data.
    pub z: Vec<f64>,
    /// Line style (`-`, `--`, `:`, `-.`, or empty = no line).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub line_style: String,
    /// Marker symbol (or empty = none).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub marker: String,
    /// CSS / `rgb(r,g,b)` color string; empty = let the frontend cycle.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub color: String,
    /// Legend display name.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
}

/// A filled area under a line (`area`): x/y data, rendered as a scatter trace
/// with `fill:"tozeroy"`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AreaSeries {
    /// X data.
    pub x: Vec<f64>,
    /// Y data.
    pub y: Vec<f64>,
    /// CSS / `rgb(r,g,b)` color string; empty = let the frontend cycle.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub color: String,
    /// Legend display name.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
}

/// A filled polygon (`fill`): x/y vertices plus a fill color, rendered as a
/// scatter trace with `fill:"toself"`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FillSeries {
    /// X vertices of the polygon.
    pub x: Vec<f64>,
    /// Y vertices of the polygon.
    pub y: Vec<f64>,
    /// Fill color as a CSS / `rgb(r,g,b)` string.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub color: String,
    /// Legend display name.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
}

/// A pie chart (`pie`): values plus optional slice labels. Rendered as a Plotly
/// `pie` trace that takes the whole figure cell (no xaxis/yaxis binding).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PieSeries {
    /// Slice values.
    pub values: Vec<f64>,
    /// Optional slice labels (length should match `values`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
}

/// A polar line plot (`polar`): angle (radians) + radius, rendered as a Plotly
/// `scatterpolar` trace on a polar subplot. The renderers convert `theta` to
/// degrees for Plotly.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PolarSeries {
    /// Angle data in radians.
    pub theta: Vec<f64>,
    /// Radius data.
    pub r: Vec<f64>,
    /// CSS / `rgb(r,g,b)` color string; empty = let the frontend cycle.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub color: String,
    /// Legend display name.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
}

/// A 2-D vector field (`quiver`): each arrow has a base `(x,y)` and a component
/// `(u,v)`. Plotly has no native quiver, so the renderers draw the shafts as a
/// single null-gapped scatter line plus a small markers trace at the arrow tips
/// (a simplified arrowhead).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct QuiverSeries {
    /// Arrow base x-coordinates.
    pub x: Vec<f64>,
    /// Arrow base y-coordinates.
    pub y: Vec<f64>,
    /// Arrow x-components.
    pub u: Vec<f64>,
    /// Arrow y-components.
    pub v: Vec<f64>,
    /// CSS / `rgb(r,g,b)` color string; empty = let the frontend cycle.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub color: String,
    /// Legend display name.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
}

/// Helper for `skip_serializing_if` on `bool` fields.
#[allow(clippy::trivially_copy_pass_by_ref)] // serde's predicate takes &bool.
fn is_false(b: &bool) -> bool {
    !*b
}

/// FreeMat's default axes color order (`HandleAxis.cpp`), as `rgb(...)` strings.
/// Series with no explicit color cycle through these in order.
#[must_use]
pub fn default_color(index: usize) -> String {
    const ORDER: [(u8, u8, u8); 7] = [
        (0, 0, 255),   // blue
        (0, 128, 0),   // green
        (255, 0, 0),   // red
        (0, 191, 191), // cyan
        (191, 0, 191), // magenta
        (191, 191, 0), // yellow
        (64, 64, 64),  // dark gray
    ];
    let (r, g, b) = ORDER[index % ORDER.len()];
    format!("rgb({r},{g},{b})")
}
