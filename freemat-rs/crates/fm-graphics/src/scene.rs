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
}

impl Figure {
    /// A new empty figure with a single default full-frame axes.
    #[must_use]
    pub fn new(id: u64) -> Self {
        Figure {
            id,
            axes: vec![Axes::new()],
            current_axes: 0,
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
            xscale: Scale::Linear,
            yscale: Scale::Linear,
            grid: false,
            legend: None,
            hold: false,
            equal: false,
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
    /// A 2-D contour plot (`contour`).
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
