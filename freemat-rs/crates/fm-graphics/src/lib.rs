//! `fm-graphics` — FreeMat-rs handle-graphics scene model and JSON wire protocol.
//!
//! A *semantic*, renderer-agnostic scene graph: a [`Scene`] holds [`Figure`]s,
//! each holding [`Axes`], each holding [`Series`] (line / surface / image). It is
//! [`serde::Serialize`] to a tagged JSON message the browser frontend maps onto
//! Plotly traces + layout. Nothing here knows about pixels, websockets, or the
//! interpreter.
//!
//! The interpreter pushes updates through a [`GraphicsSink`] (a trait the
//! webserver implements). When no sink is installed, graphics builtins still
//! maintain the retained scene in interpreter state — they just don't broadcast
//! — so library and conformance tests need no server.

mod linespec;
mod scene;

pub use linespec::{LineSpec, parse as parse_linespec};
pub use scene::{
    Axes, AxisLimits, ContourSeries, Figure, ImageSeries, Legend, LineSeries, Scale, Scene, Series,
    SurfaceSeries, WireMessage, default_color,
};

/// A consumer the interpreter pushes scene snapshots to (e.g. the webserver,
/// which broadcasts them to connected browsers). Kept dyn-compatible so the
/// interpreter can hold an `Option<Box<dyn GraphicsSink>>` without depending on
/// `axum`/`tokio`.
pub trait GraphicsSink: Send + Sync {
    /// Publish the current scene. Called on `drawnow` (and after a
    /// non-suppressed plotting command). Implementations should be cheap /
    /// non-blocking (e.g. broadcast over a channel).
    fn publish(&self, scene: &Scene);
}

#[cfg(test)]
mod tests;
