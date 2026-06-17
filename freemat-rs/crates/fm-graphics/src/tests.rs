//! Unit tests for scene construction, linespec parsing, and JSON serialization.

use crate::{Axes, Figure, LineSeries, Scale, Scene, Series, default_color, parse_linespec};

#[test]
fn line_series_serializes_to_plotly_shaped_json() {
    let mut scene = Scene::new();
    let fig = scene.figure_mut_or_insert(1);
    fig.current_axes_mut().series.push(Series::Line(LineSeries {
        x: vec![0.0, 1.0, 2.0],
        y: vec![0.0, 1.0, 4.0],
        line_style: "-".into(),
        color: "rgb(0,0,255)".into(),
        ..Default::default()
    }));

    let msg = scene.to_message().unwrap();
    let v: serde_json::Value = serde_json::from_str(&msg).unwrap();
    assert_eq!(v["type"], "scene");
    let series = &v["scene"]["figures"][0]["axes"][0]["series"][0];
    assert_eq!(series["kind"], "line");
    assert_eq!(series["x"], serde_json::json!([0.0, 1.0, 2.0]));
    assert_eq!(series["y"], serde_json::json!([0.0, 1.0, 4.0]));
    assert_eq!(series["line_style"], "-");
    assert_eq!(series["color"], "rgb(0,0,255)");
}

#[test]
fn scene_roundtrips_through_json() {
    let mut scene = Scene::new();
    let fig = scene.figure_mut_or_insert(3);
    let ax = fig.current_axes_mut();
    ax.title = "demo".into();
    ax.grid = true;
    ax.yscale = Scale::Log;
    ax.series.push(Series::Line(LineSeries {
        x: vec![1.0, 2.0],
        y: vec![10.0, 100.0],
        ..Default::default()
    }));

    let json = serde_json::to_string(&scene).unwrap();
    let back: Scene = serde_json::from_str(&json).unwrap();
    assert_eq!(scene, back);
}

#[test]
fn empty_axes_omits_defaulted_fields() {
    let scene = Scene {
        figures: vec![Figure::new(7)],
    };
    let json = serde_json::to_string(&scene).unwrap();
    // Default linear scale / empty title / no grid should be skipped.
    assert!(!json.contains("title"));
    assert!(!json.contains("grid"));
    assert!(!json.contains("xscale"));
    assert_eq!(Axes::new().series.len(), 0);
}

#[test]
fn figure_lookup_and_insert() {
    let mut scene = Scene::new();
    scene.figure_mut_or_insert(2);
    scene.figure_mut_or_insert(5);
    scene.figure_mut_or_insert(2); // existing
    assert_eq!(scene.figures.len(), 2);
    assert!(scene.figure(5).is_some());
    assert!(scene.figure(9).is_none());
}

#[test]
fn linespec_parses_color_style_marker() {
    let s = parse_linespec("r--o");
    assert!(s.valid);
    assert_eq!(s.color, "rgb(255,0,0)");
    assert_eq!(s.line_style, "--");
    assert_eq!(s.marker, "o");

    let s = parse_linespec(":g");
    assert!(s.valid);
    assert_eq!(s.line_style, ":");
    assert_eq!(s.color, "rgb(0,128,0)");

    // A property name like "Color" is not a linespec.
    assert!(!parse_linespec("Color").valid);
}

#[test]
fn dash_dot_distinct_from_dash() {
    assert_eq!(parse_linespec("-.").line_style, "-.");
    assert_eq!(parse_linespec("-").line_style, "-");
    assert_eq!(parse_linespec("--").line_style, "--");
}

#[test]
fn color_order_cycles() {
    assert_eq!(default_color(0), "rgb(0,0,255)");
    assert_eq!(default_color(7), default_color(0));
}
