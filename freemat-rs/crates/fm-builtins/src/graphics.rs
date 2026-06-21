//! Graphics builtins: figure/axes lifecycle, the handle property system
//! (`set`/`get`/`gcf`/`gca`/`gco`/`figure`/`axes`/`delete`/`cla`/`clf`/`close`/
//! `findobj`/`ishandle`), the plotting commands (`plot`/`line`/`surf`/`mesh`/
//! `image`/`contour`/`semilog*`), and the per-axes property setters
//! (`title`/`xlabel`/`hold`/`axis`/`grid`/`legend`) and grid layout (`subplot`).
//!
//! These build the semantic [`fm_graphics`] scene directly in interpreter state
//! (via the [`GraphicsState`](fm_interp::GraphicsState) handle registry) and
//! mark it dirty; the implicit draw at the end of a top-level command (or an
//! explicit `drawnow`) flushes it through the optional sink.

use fm_core::{Array, DataClass};
use fm_graphics::{
    AxisLimits, BarSeries, ContourSeries, ErrorbarSeries, ImageSeries, Legend, Line3dSeries,
    LineSeries, Scale, Series, StairsSeries, StemSeries, SurfaceSeries, default_color,
    parse_linespec,
};
use fm_interp::error::Flow;
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter, ObjKind, ObjLocation};

use crate::util::err;

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("figure", b_figure);
    table.add_builtin("axes", b_axes);
    table.add_builtin("subplot", b_subplot);
    table.add_builtin("plot", b_plot);
    table.add_builtin("line", b_line);
    table.add_builtin("contour", b_contour);
    table.add_builtin("title", b_title);
    table.add_builtin("xlabel", |i, a, _n| label(i, a, Axis::X));
    table.add_builtin("ylabel", |i, a, _n| label(i, a, Axis::Y));
    table.add_builtin("zlabel", |i, a, _n| label(i, a, Axis::Z));
    table.add_builtin("legend", b_legend);
    table.add_builtin("hold", b_hold);
    table.add_builtin("axis", b_axis);
    table.add_builtin("grid", b_grid);
    table.add_builtin("cla", b_cla);
    table.add_builtin("clf", b_clf);
    table.add_builtin("close", b_close);
    table.add_builtin("delete", b_delete);
    table.add_builtin("gcf", b_gcf);
    table.add_builtin("gca", b_gca);
    table.add_builtin("gco", b_gco);
    table.add_builtin("drawnow", b_drawnow);
    table.add_builtin("set", b_set);
    table.add_builtin("get", b_get);
    table.add_builtin("ishandle", b_ishandle);
    table.add_builtin("findobj", b_findobj);
    table.add_builtin("surf", |i, a, _n| surface(i, a, false));
    table.add_builtin("mesh", |i, a, _n| surface(i, a, true));
    table.add_builtin("image", b_image);
    table.add_builtin("imagesc", b_image);
    table.add_builtin("bar", |i, a, _n| bar(i, a, false));
    table.add_builtin("barh", |i, a, _n| bar(i, a, true));
    table.add_builtin("hist", b_hist);
    table.add_builtin("stem", b_stem);
    table.add_builtin("stairs", b_stairs);
    table.add_builtin("errorbar", b_errorbar);
    table.add_builtin("plot3", b_plot3);
    table.add_builtin("peaks", b_peaks);
    table.add_builtin("view", b_view);
}

/// Split args into `(x, y)`, supplying the implicit `x = 1:n` when only `y` is
/// given. Used by the chart-type builtins (`bar`, `stem`, `stairs`).
fn xy_args(args: &[Array]) -> (Vec<f64>, Vec<f64>) {
    if args.len() >= 2 && !args[1].is_char() {
        (to_f64_vec(&args[0]), to_f64_vec(&args[1]))
    } else {
        let y = to_f64_vec(&args[0]);
        let x: Vec<f64> = (1..=y.len()).map(|k| k as f64).collect();
        (x, y)
    }
}

/// Read a single string argument (linespec, label text, on/off, ...).
fn str_arg(args: &[Array], i: usize) -> Option<String> {
    args.get(i).and_then(Array::as_string)
}

/// A scalar `double` array (handles are MATLAB-style doubles).
fn scalar(v: f64) -> Array {
    Array::Scalar(fm_core::ScalarValue::Double(v))
}

/// A handle value as a scalar double.
fn handle(h: u64) -> Array {
    scalar(h as f64)
}

// ---- figure / axes / subplot ------------------------------------------------

fn b_figure(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let id = if let Some(n) = args.first().and_then(Array::as_f64) {
        n.max(1.0) as u64
    } else {
        i.graphics.next_figure_id()
    };
    i.graphics.select_figure(id);
    i.graphics.dirty = true;
    Ok(vec![handle(id)])
}

/// `axes` (new full-frame axes) / `axes(h)` (select an existing axes).
fn b_axes(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if let Some(h) = args.first().and_then(Array::as_f64) {
        let h = h as u64;
        if i.graphics.select_axes(h) {
            i.graphics.dirty = true;
            return Ok(vec![handle(h)]);
        }
        return err(format!("axes: {h} is not a valid axes handle"));
    }
    let h = i.graphics.create_axes([0.0, 0.0, 1.0, 1.0]);
    Ok(vec![handle(h)])
}

/// `subplot(m, n, p)` — create/select the axes for grid cell `p` (1-based,
/// row-major) and return its handle. The cell's normalized position is
/// `[left, bottom, width, height]` with width `1/n`, height `1/m`, the top row
/// first (MATLAB numbering).
fn b_subplot(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    // Support subplot(mnp) with a single 3-digit number too.
    let (m, n, p) = if args.len() == 1 {
        let v = args[0].as_f64().unwrap_or(111.0).round() as i64;
        if !(111..=999).contains(&v) {
            return err("subplot: single-argument form needs a 3-digit code (e.g. 211)");
        }
        (
            (v / 100) as usize,
            (v / 10 % 10) as usize,
            (v % 10) as usize,
        )
    } else if args.len() >= 3 {
        let m = args[0].as_f64().unwrap_or(1.0).round().max(1.0) as usize;
        let n = args[1].as_f64().unwrap_or(1.0).round().max(1.0) as usize;
        let p = args[2].as_f64().unwrap_or(1.0).round().max(1.0) as usize;
        (m, n, p)
    } else {
        return err("subplot: expected subplot(m,n,p)");
    };
    if p < 1 || p > m * n {
        return err(format!(
            "subplot: index {p} out of range for a {m}x{n} grid"
        ));
    }
    let position = subplot_position(m, n, p);
    let fig = i.graphics.ensure_figure();
    // Reuse an axes already occupying this exact cell (re-selecting it),
    // otherwise delete any axes whose rectangle *overlaps* the new cell (the
    // default full-frame axes, or a coarser-grid cell) and create a fresh one —
    // mirroring toolbox `subplot.m`.
    let existing = i.graphics.scene.figure(fig).and_then(|f| {
        f.axes
            .iter()
            .find(|a| positions_match(a.position, position))
            .map(|a| a.handle)
    });
    let h = if let Some(h) = existing {
        i.graphics.select_axes(h);
        h
    } else {
        let overlapping: Vec<u64> = i
            .graphics
            .scene
            .figure(fig)
            .map(|f| {
                f.axes
                    .iter()
                    .filter(|a| positions_overlap(a.position, position))
                    .map(|a| a.handle)
                    .collect()
            })
            .unwrap_or_default();
        for h in overlapping {
            i.graphics.delete(h);
        }
        i.graphics.create_axes(position)
    };
    i.graphics.dirty = true;
    Ok(vec![handle(h)])
}

/// Normalized `[left, bottom, width, height]` for grid cell `p` of an m×n grid.
fn subplot_position(m: usize, n: usize, p: usize) -> [f64; 4] {
    let p0 = p - 1;
    let row = p0 / n; // 0 = top row
    let col = p0 % n;
    let width = 1.0 / n as f64;
    let height = 1.0 / m as f64;
    let left = col as f64 * width;
    // Row 0 is the top, so its bottom edge is the highest.
    let bottom = 1.0 - (row as f64 + 1.0) * height;
    [left, bottom, width, height]
}

/// True if two normalized position rectangles coincide (to a small tolerance).
fn positions_match(a: [f64; 4], b: [f64; 4]) -> bool {
    a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() < 1e-9)
}

/// True if two `[left, bottom, width, height]` rectangles overlap by a
/// non-trivial area (the `subplot.m` overlap test).
fn positions_overlap(a: [f64; 4], b: [f64; 4]) -> bool {
    let nleft = a[0].max(b[0]);
    let nbottom = a[1].max(b[1]);
    let nright = (a[0] + a[2]).min(b[0] + b[2]);
    let ntop = (a[1] + a[3]).min(b[1] + b[3]);
    if nright <= nleft || ntop <= nbottom {
        false
    } else {
        (nright - nleft) * (ntop - nbottom) > 0.01
    }
}

// ---- gcf / gca / gco / cla / clf / close / delete ---------------------------

fn b_gcf(i: &mut Interpreter, _a: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let h = i.graphics.current_figure_handle();
    Ok(vec![handle(h)])
}

fn b_gca(i: &mut Interpreter, _a: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let h = i.graphics.current_axes_handle();
    Ok(vec![handle(h)])
}

fn b_gco(i: &mut Interpreter, _a: &[Array], _n: usize) -> Flow<Vec<Array>> {
    Ok(vec![handle(i.graphics.current_object)])
}

fn b_cla(i: &mut Interpreter, _a: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let ax = i.graphics.current_figure_mut().current_axes_mut();
    ax.series.clear();
    ax.xscale = Scale::Linear;
    ax.yscale = Scale::Linear;
    ax.limits = None;
    i.graphics.dirty = true;
    Ok(vec![])
}

fn b_clf(i: &mut Interpreter, _a: &[Array], _n: usize) -> Flow<Vec<Array>> {
    // Reset the current figure to a single default axes, dropping its children.
    let fig = i.graphics.current_figure_handle();
    i.graphics.delete_children_of_figure(fig);
    i.graphics.dirty = true;
    Ok(vec![])
}

fn b_close(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    match str_arg(args, 0).as_deref() {
        Some("all") => i.graphics.close_all(),
        _ => {
            let h = args
                .first()
                .and_then(Array::as_f64)
                .map(|v| v as u64)
                .unwrap_or_else(|| i.graphics.current_figure_handle());
            i.graphics.delete(h);
        }
    }
    Ok(vec![])
}

fn b_delete(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    for a in args {
        for v in to_f64_vec(a) {
            i.graphics.delete(v as u64);
        }
    }
    Ok(vec![])
}

fn b_drawnow(i: &mut Interpreter, _a: &[Array], _n: usize) -> Flow<Vec<Array>> {
    i.graphics.flush();
    Ok(vec![])
}

// ---- ishandle / findobj -----------------------------------------------------

fn b_ishandle(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let h = args.first().and_then(Array::as_f64).map(|v| v as u64);
    let result = match (h, str_arg(args, 1)) {
        (Some(h), Some(kind)) => i
            .graphics
            .kind_of(h)
            .is_some_and(|k| k.type_name().eq_ignore_ascii_case(&kind)),
        (Some(h), None) => i.graphics.is_handle(h),
        _ => false,
    };
    Ok(vec![Array::Scalar(fm_core::ScalarValue::Bool(result))])
}

/// `findobj` / `findobj('type', 'axes')` — return matching handles as a column.
fn b_findobj(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    // Optional ('Type', name) filter (the common case used by toolbox code).
    let mut type_filter: Option<String> = None;
    let mut idx = 0;
    while idx + 1 < args.len() {
        if let Some(prop) = str_arg(args, idx)
            && prop.eq_ignore_ascii_case("type")
        {
            type_filter = str_arg(args, idx + 1);
        }
        idx += 2;
    }
    let handles: Vec<f64> = i
        .graphics
        .all_handles()
        .into_iter()
        .filter(|&h| match &type_filter {
            Some(t) => i
                .graphics
                .kind_of(h)
                .is_some_and(|k| k.type_name().eq_ignore_ascii_case(t)),
            None => true,
        })
        .map(|h| h as f64)
        .collect();
    let n = handles.len();
    Ok(vec![build_real(DataClass::Double, &[n, 1], handles)])
}

// ---- set / get --------------------------------------------------------------

fn b_set(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("set: expected a handle");
    }
    let h = args[0].as_f64().unwrap_or(0.0) as u64;
    if !i.graphics.is_handle(h) {
        return err(format!("set: {h} is not a valid handle"));
    }
    // Property/value pairs follow the handle.
    let mut k = 1;
    while k + 1 < args.len() + 1 && k < args.len() {
        let Some(prop) = str_arg(args, k) else {
            return err("set: expected a property name string");
        };
        let Some(value) = args.get(k + 1) else {
            return err(format!("set: no value given for property '{prop}'"));
        };
        apply_property(i, h, &prop, value)?;
        k += 2;
    }
    i.graphics.dirty = true;
    Ok(vec![])
}

fn b_get(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("get: expected a handle");
    }
    let h = args[0].as_f64().unwrap_or(0.0) as u64;
    if !i.graphics.is_handle(h) {
        return err(format!("get: {h} is not a valid handle"));
    }
    let Some(prop) = str_arg(args, 1) else {
        // get(h) with no property — return the type string (a minimal summary).
        let ty = i.graphics.kind_of(h).map(ObjKind::type_name).unwrap_or("");
        return Ok(vec![Array::char_string(ty)]);
    };
    Ok(vec![read_property(i, h, &prop)])
}

/// Apply a single property/value pair to the handle's object.
fn apply_property(i: &mut Interpreter, h: u64, prop: &str, value: &Array) -> Flow<()> {
    let prop = prop.to_ascii_lowercase();
    let loc = i.graphics.resolve(h);
    // Axes-level properties.
    if let Some(loc) = loc
        && matches!(loc, ObjLocation::Axes { .. })
        && let Some(ax) = i.graphics.axes_at_mut(loc)
    {
        let v = to_f64_vec(value);
        match prop.as_str() {
            "title" => ax.title = value.as_string().unwrap_or_default(),
            "xlabel" => ax.xlabel = value.as_string().unwrap_or_default(),
            "ylabel" => ax.ylabel = value.as_string().unwrap_or_default(),
            "zlabel" => ax.zlabel = value.as_string().unwrap_or_default(),
            "xlim" if v.len() >= 2 => {
                let mut lim = ax.limits.unwrap_or(AxisLimits {
                    xmin: v[0],
                    xmax: v[1],
                    ymin: 0.0,
                    ymax: 1.0,
                });
                lim.xmin = v[0];
                lim.xmax = v[1];
                ax.limits = Some(lim);
            }
            "ylim" if v.len() >= 2 => {
                let mut lim = ax.limits.unwrap_or(AxisLimits {
                    xmin: 0.0,
                    xmax: 1.0,
                    ymin: v[0],
                    ymax: v[1],
                });
                lim.ymin = v[0];
                lim.ymax = v[1];
                ax.limits = Some(lim);
            }
            "xscale" => ax.xscale = scale_of(&value.as_string().unwrap_or_default()),
            "yscale" => ax.yscale = scale_of(&value.as_string().unwrap_or_default()),
            "grid" => ax.grid = on_off(&value.as_string().unwrap_or_default()),
            "position" | "outerposition" if v.len() >= 4 => {
                ax.position = [v[0], v[1], v[2], v[3]];
            }
            _ => {
                i.graphics.set_extra(h, &prop, value.clone());
            }
        }
        return Ok(());
    }
    // Series-level properties.
    if let Some(loc) = loc
        && matches!(loc, ObjLocation::Series { .. })
        && let Some(Series::Line(line)) = i.graphics.series_at_mut(loc)
    {
        match prop.as_str() {
            "color" | "linecolor" => line.color = value.as_string().unwrap_or_default(),
            "linestyle" => line.line_style = value.as_string().unwrap_or_default(),
            "marker" => line.marker = value.as_string().unwrap_or_default(),
            "displayname" => line.name = value.as_string().unwrap_or_default(),
            "xdata" => line.x = to_f64_vec(value),
            "ydata" => line.y = to_f64_vec(value),
            _ => i.graphics.set_extra(h, &prop, value.clone()),
        }
        return Ok(());
    }
    // Figure-level / fallback: store in the bag.
    i.graphics.set_extra(h, &prop, value.clone());
    Ok(())
}

/// Read a single property from the handle's object.
fn read_property(i: &Interpreter, h: u64, prop: &str) -> Array {
    let prop_l = prop.to_ascii_lowercase();
    if let Some(loc) = i.graphics.resolve(h) {
        match loc {
            ObjLocation::Figure { fig } => {
                if prop_l == "type" {
                    return Array::char_string("figure");
                }
                if prop_l == "number" {
                    return scalar(fig as f64);
                }
            }
            ObjLocation::Axes { fig, axes } => {
                if let Some(ax) = i.graphics.scene.figure(fig).and_then(|f| f.axes.get(axes)) {
                    match prop_l.as_str() {
                        "type" => return Array::char_string("axes"),
                        "title" => return Array::char_string(&ax.title),
                        "xlabel" => return Array::char_string(&ax.xlabel),
                        "ylabel" => return Array::char_string(&ax.ylabel),
                        "zlabel" => return Array::char_string(&ax.zlabel),
                        "position" | "outerposition" => {
                            return build_real(DataClass::Double, &[1, 4], ax.position.to_vec());
                        }
                        "xscale" => return Array::char_string(scale_name(ax.xscale)),
                        "yscale" => return Array::char_string(scale_name(ax.yscale)),
                        "grid" => return Array::char_string(if ax.grid { "on" } else { "off" }),
                        "xlim" => {
                            if let Some(l) = ax.limits {
                                return build_real(
                                    DataClass::Double,
                                    &[1, 2],
                                    vec![l.xmin, l.xmax],
                                );
                            }
                        }
                        "ylim" => {
                            if let Some(l) = ax.limits {
                                return build_real(
                                    DataClass::Double,
                                    &[1, 2],
                                    vec![l.ymin, l.ymax],
                                );
                            }
                        }
                        _ => {}
                    }
                }
            }
            ObjLocation::Series { fig, axes, series } => {
                if let Some(Series::Line(line)) = i
                    .graphics
                    .scene
                    .figure(fig)
                    .and_then(|f| f.axes.get(axes))
                    .and_then(|a| a.series.get(series))
                {
                    match prop_l.as_str() {
                        "type" => return Array::char_string("line"),
                        "color" | "linecolor" => return Array::char_string(&line.color),
                        "linestyle" => return Array::char_string(&line.line_style),
                        "marker" => return Array::char_string(&line.marker),
                        "displayname" => return Array::char_string(&line.name),
                        "xdata" => {
                            return build_real(
                                DataClass::Double,
                                &[1, line.x.len()],
                                line.x.clone(),
                            );
                        }
                        "ydata" => {
                            return build_real(
                                DataClass::Double,
                                &[1, line.y.len()],
                                line.y.clone(),
                            );
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    // Unknown / free-form: echo from the bag, or empty.
    i.graphics
        .get_extra(h, prop)
        .unwrap_or_else(|| build_real(DataClass::Double, &[0, 0], Vec::new()))
}

fn scale_of(s: &str) -> Scale {
    if s.eq_ignore_ascii_case("log") {
        Scale::Log
    } else {
        Scale::Linear
    }
}

fn scale_name(s: Scale) -> &'static str {
    match s {
        Scale::Log => "log",
        Scale::Linear => "linear",
    }
}

fn on_off(s: &str) -> bool {
    s.eq_ignore_ascii_case("on")
}

// ---- plot / line ------------------------------------------------------------

fn b_plot(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("plot: not enough arguments");
    }
    // newplot semantics: unless `hold on`, clear the current axes' series.
    clear_current_series_unless_hold(i);
    let mut handles: Vec<f64> = Vec::new();
    let mut idx = 0;
    while idx < args.len() {
        let (x, y, mut next) = if idx + 1 < args.len() && !args[idx + 1].is_char() {
            (to_f64_vec(&args[idx]), to_f64_vec(&args[idx + 1]), idx + 2)
        } else {
            let y = to_f64_vec(&args[idx]);
            let x: Vec<f64> = (1..=y.len()).map(|k| k as f64).collect();
            (x, y, idx + 1)
        };
        let mut spec = fm_graphics::LineSpec::default();
        if let Some(s) = args.get(next).and_then(Array::as_string) {
            let parsed = parse_linespec(&s);
            if parsed.valid {
                spec = parsed;
                next += 1;
            }
        }
        handles.push(add_line(i, x, y, &spec) as f64);
        idx = next;
    }
    i.graphics.dirty = true;
    let n = handles.len();
    Ok(vec![build_real(DataClass::Double, &[n, 1], handles)])
}

/// Clear the current axes' series unless it is in `hold on` mode.
fn clear_current_series_unless_hold(i: &mut Interpreter) {
    let ax = i.graphics.current_figure_mut().current_axes_mut();
    if !ax.hold {
        ax.series.clear();
    }
}

/// Append a line series to the current axes; return its handle.
fn add_line(i: &mut Interpreter, x: Vec<f64>, y: Vec<f64>, spec: &fm_graphics::LineSpec) -> u64 {
    let (fig, axes_idx) = current_axes_loc(i);
    let ax = i.graphics.current_figure_mut().current_axes_mut();
    let series_index = ax.series.len();
    let line_style = if spec.line_style.is_empty() && spec.marker.is_empty() {
        "-".to_string()
    } else {
        spec.line_style.clone()
    };
    let color = if spec.color.is_empty() {
        default_color(series_index)
    } else {
        spec.color.clone()
    };
    ax.series.push(Series::Line(LineSeries {
        x,
        y,
        line_style,
        marker: spec.marker.clone(),
        color,
        name: String::new(),
    }));
    i.graphics.register_series(fig, axes_idx, ObjKind::Line)
}

/// The (figure id, current-axes index) of the current axes.
fn current_axes_loc(i: &mut Interpreter) -> (u64, usize) {
    let fig = i.graphics.ensure_figure();
    let axes = i
        .graphics
        .scene
        .figure(fig)
        .map(|f| f.current_axes)
        .unwrap_or(0);
    (fig, axes)
}

fn b_line(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.len() < 2 {
        return err("line: expected x and y vectors");
    }
    let x = to_f64_vec(&args[0]);
    let y = to_f64_vec(&args[1]);
    let mut spec = fm_graphics::LineSpec::default();
    if let Some(s) = args.get(2).and_then(Array::as_string) {
        let parsed = parse_linespec(&s);
        if parsed.valid {
            spec = parsed;
        }
    }
    let h = add_line(i, x, y, &spec);
    i.graphics.dirty = true;
    Ok(vec![handle(h)])
}

// ---- contour ----------------------------------------------------------------

/// `contour(Z)` / `contour(Z, n)` / `contour(Z, levels)` /
/// `contour(X, Y, Z[, ...])`.
fn b_contour(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("contour: expected a Z matrix");
    }
    // Decide whether the first three args are X, Y, Z.
    let (z_arg, x, y, level_arg) =
        if args.len() >= 3 && args[2].dims().iter().product::<usize>() > 1 {
            (
                &args[2],
                to_f64_vec(&args[0]),
                to_f64_vec(&args[1]),
                args.get(3),
            )
        } else {
            (&args[0], Vec::new(), Vec::new(), args.get(1))
        };
    let (z, _r, _c) = grid_of(z_arg);
    let levels = match level_arg {
        Some(a) => {
            let v = to_f64_vec(a);
            if v.len() == 1 {
                // A scalar count → that many evenly spaced levels.
                let count = v[0].max(1.0) as usize;
                let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
                for row in &z {
                    for &val in row {
                        lo = lo.min(val);
                        hi = hi.max(val);
                    }
                }
                if lo.is_finite() && hi.is_finite() && count > 1 {
                    (0..count)
                        .map(|k| lo + (hi - lo) * k as f64 / (count as f64 - 1.0))
                        .collect()
                } else {
                    Vec::new()
                }
            } else {
                v
            }
        }
        None => Vec::new(),
    };
    clear_current_series_unless_hold(i);
    let (fig, axes_idx) = current_axes_loc(i);
    i.graphics
        .current_figure_mut()
        .current_axes_mut()
        .series
        .push(Series::Contour(ContourSeries {
            z,
            x,
            y,
            levels,
            colormap: "Viridis".into(),
        }));
    let h = i.graphics.register_series(fig, axes_idx, ObjKind::Contour);
    i.graphics.dirty = true;
    Ok(vec![handle(h)])
}

// ---- view (3-D camera) ------------------------------------------------------

/// `view(2)` / `view(3)` / `view(az, el)` — set the 3-D viewing angles.
///
/// FreeMat/MATLAB conventions:
/// - `view(2)` → straight-down 2-D view (az=0, el=90),
/// - `view(3)` → default 3-D view (az=-37.5, el=30),
/// - `view(az, el)` → explicit azimuth/elevation in degrees.
///
/// We record the requested `[az, el]` on the current axes; the renderers map it
/// to the Plotly `scene.camera.eye`. The call never errors (so doc examples run).
fn b_view(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let view = match args {
        [a, b] => Some([a.as_f64().unwrap_or(0.0), b.as_f64().unwrap_or(0.0)]),
        [a] => {
            // A single argument is a mode selector (2 or 3) — or a 1x2/2x1 vector.
            let v = to_f64_vec(a);
            if v.len() >= 2 {
                Some([v[0], v[1]])
            } else {
                match a.as_f64().unwrap_or(3.0).round() as i64 {
                    2 => Some([0.0, 90.0]),
                    _ => Some([-37.5, 30.0]),
                }
            }
        }
        // `view()` with no args is a query in MATLAB; we just no-op.
        _ => None,
    };
    if let Some(v) = view {
        i.graphics.current_figure_mut().current_axes_mut().view = Some(v);
        i.graphics.dirty = true;
    }
    Ok(vec![])
}

// ---- labels / title ---------------------------------------------------------

enum Axis {
    X,
    Y,
    Z,
}

fn label(i: &mut Interpreter, args: &[Array], which: Axis) -> Flow<Vec<Array>> {
    let text = str_arg(args, 0).unwrap_or_default();
    let ax = i.graphics.current_figure_mut().current_axes_mut();
    match which {
        Axis::X => ax.xlabel = text,
        Axis::Y => ax.ylabel = text,
        Axis::Z => ax.zlabel = text,
    }
    i.graphics.dirty = true;
    Ok(vec![])
}

fn b_title(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let text = str_arg(args, 0).unwrap_or_default();
    i.graphics.current_figure_mut().current_axes_mut().title = text;
    i.graphics.dirty = true;
    Ok(vec![])
}

// ---- legend / hold / grid / axis --------------------------------------------

fn b_legend(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let first = str_arg(args, 0);
    let ax = i.graphics.current_figure_mut().current_axes_mut();
    if first.as_deref() == Some("off") {
        ax.legend = Some(Legend {
            visible: false,
            names: Vec::new(),
        });
    } else {
        let names: Vec<String> = args.iter().filter_map(Array::as_string).collect();
        ax.legend = Some(Legend {
            visible: true,
            names,
        });
    }
    i.graphics.dirty = true;
    Ok(vec![])
}

fn b_hold(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let ax = i.graphics.current_figure_mut().current_axes_mut();
    match str_arg(args, 0).as_deref() {
        Some("on") => ax.hold = true,
        Some("off") => ax.hold = false,
        _ => ax.hold = !ax.hold,
    }
    Ok(vec![])
}

fn b_grid(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let ax = i.graphics.current_figure_mut().current_axes_mut();
    match str_arg(args, 0).as_deref() {
        Some("off") => ax.grid = false,
        Some("on") => ax.grid = true,
        _ => ax.grid = !ax.grid,
    }
    i.graphics.dirty = true;
    Ok(vec![])
}

fn b_axis(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let ax = i.graphics.current_figure_mut().current_axes_mut();
    if let Some(s) = str_arg(args, 0) {
        match s.as_str() {
            "equal" => ax.equal = true,
            "normal" | "auto" => {
                ax.equal = false;
                ax.limits = None;
            }
            "tight" | "square" => {}
            _ => {}
        }
    } else if let Some(a) = args.first() {
        let v = to_f64_vec(a);
        if v.len() >= 4 {
            ax.limits = Some(AxisLimits {
                xmin: v[0],
                xmax: v[1],
                ymin: v[2],
                ymax: v[3],
            });
        }
    }
    i.graphics.dirty = true;
    Ok(vec![])
}

// ---- surf / mesh / image ----------------------------------------------------

/// Read an array as a row-major grid of `rows × cols` for surface/image traces.
fn grid_of(a: &Array) -> (Vec<Vec<f64>>, usize, usize) {
    let dims = a.dims();
    let rows = dims.first().copied().unwrap_or(0);
    let cols = dims.get(1).copied().unwrap_or(1);
    let col_major = to_f64_vec(a);
    let mut grid = vec![vec![0.0; cols]; rows];
    for c in 0..cols {
        for r in 0..rows {
            grid[r][c] = col_major[c * rows + r];
        }
    }
    (grid, rows, cols)
}

fn surface(i: &mut Interpreter, args: &[Array], wireframe: bool) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("surf/mesh: expected a Z matrix");
    }
    let (z_arg, x, y) = if args.len() >= 3 {
        (&args[2], to_f64_vec(&args[0]), to_f64_vec(&args[1]))
    } else {
        (&args[0], Vec::new(), Vec::new())
    };
    let (z, _r, _c) = grid_of(z_arg);
    clear_current_series_unless_hold(i);
    let (fig, axes_idx) = current_axes_loc(i);
    i.graphics
        .current_figure_mut()
        .current_axes_mut()
        .series
        .push(Series::Surface(SurfaceSeries {
            z,
            x,
            y,
            colormap: "Viridis".into(),
            wireframe,
        }));
    let h = i.graphics.register_series(fig, axes_idx, ObjKind::Surface);
    i.graphics.dirty = true;
    Ok(vec![handle(h)])
}

fn b_image(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("image: expected a data matrix");
    }
    let (data, _r, _c) = grid_of(&args[0]);
    clear_current_series_unless_hold(i);
    let (fig, axes_idx) = current_axes_loc(i);
    i.graphics
        .current_figure_mut()
        .current_axes_mut()
        .series
        .push(Series::Image(ImageSeries {
            data,
            colormap: "Viridis".into(),
        }));
    let h = i.graphics.register_series(fig, axes_idx, ObjKind::Image);
    i.graphics.dirty = true;
    Ok(vec![handle(h)])
}

// ---- bar / barh / hist ------------------------------------------------------

/// `bar(y)` / `bar(x,y)` / `barh(...)` — a vertical (or horizontal) bar chart.
fn bar(i: &mut Interpreter, args: &[Array], horizontal: bool) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("bar: not enough arguments");
    }
    let (x, y) = xy_args(args);
    clear_current_series_unless_hold(i);
    let (fig, axes_idx) = current_axes_loc(i);
    let ax = i.graphics.current_figure_mut().current_axes_mut();
    let color = default_color(ax.series.len());
    ax.series.push(Series::Bar(BarSeries {
        x,
        y,
        horizontal,
        color,
        name: String::new(),
    }));
    let h = i.graphics.register_series(fig, axes_idx, ObjKind::Line);
    i.graphics.dirty = true;
    Ok(vec![handle(h)])
}

/// `hist(y)` / `hist(y, nbins)` — compute a histogram (bins + counts) and draw
/// it as a bar chart. With output args, return `[n, x]` (counts and centers)
/// like MATLAB/FreeMat (no plot in that case).
fn b_hist(i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("hist: not enough arguments");
    }
    let data = to_f64_vec(&args[0]);
    let finite: Vec<f64> = data.iter().copied().filter(|v| v.is_finite()).collect();
    // Second arg: either a bin count (scalar) or explicit bin centers (vector).
    let centers: Vec<f64> = match args.get(1) {
        Some(a) if to_f64_vec(a).len() > 1 => to_f64_vec(a),
        other => {
            let nbins = other
                .map(|a| a.as_f64().unwrap_or(10.0).round().max(1.0) as usize)
                .unwrap_or(10);
            histogram_centers(&finite, nbins)
        }
    };
    let counts = histogram_counts(&finite, &centers);
    if nargout >= 1 {
        // Return [n, x] without plotting.
        let n = counts.len();
        let mut out = vec![build_real(DataClass::Double, &[1, n], counts)];
        if nargout >= 2 {
            out.push(build_real(DataClass::Double, &[1, centers.len()], centers));
        }
        return Ok(out);
    }
    clear_current_series_unless_hold(i);
    let (fig, axes_idx) = current_axes_loc(i);
    let ax = i.graphics.current_figure_mut().current_axes_mut();
    let color = default_color(ax.series.len());
    ax.series.push(Series::Bar(BarSeries {
        x: centers,
        y: counts,
        horizontal: false,
        color,
        name: String::new(),
    }));
    let h = i.graphics.register_series(fig, axes_idx, ObjKind::Line);
    i.graphics.dirty = true;
    Ok(vec![handle(h)])
}

/// Evenly-spaced bin centers spanning the data range (FreeMat `hist` default).
fn histogram_centers(data: &[f64], nbins: usize) -> Vec<f64> {
    let nbins = nbins.max(1);
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for &v in data {
        lo = lo.min(v);
        hi = hi.max(v);
    }
    if !lo.is_finite() || !hi.is_finite() {
        lo = 0.0;
        hi = 1.0;
    }
    if (hi - lo).abs() < f64::EPSILON {
        lo -= 0.5;
        hi += 0.5;
    }
    let width = (hi - lo) / nbins as f64;
    (0..nbins).map(|k| lo + width * (k as f64 + 0.5)).collect()
}

/// Count data points into bins whose edges are midway between the centers
/// (FreeMat/MATLAB `hist` binning).
fn histogram_counts(data: &[f64], centers: &[f64]) -> Vec<f64> {
    let mut counts = vec![0.0; centers.len()];
    if centers.is_empty() {
        return counts;
    }
    // Edges: midpoints between adjacent centers, with the outer bins unbounded.
    let mut edges = Vec::with_capacity(centers.len() + 1);
    edges.push(f64::NEG_INFINITY);
    for w in centers.windows(2) {
        edges.push((w[0] + w[1]) / 2.0);
    }
    edges.push(f64::INFINITY);
    for &v in data {
        // Find the bin whose [edge[k], edge[k+1]) contains v.
        let mut bin = 0;
        for k in 0..centers.len() {
            if v >= edges[k] && (v < edges[k + 1] || k + 1 == centers.len()) {
                bin = k;
                break;
            }
        }
        counts[bin] += 1.0;
    }
    counts
}

// ---- stem / stairs ----------------------------------------------------------

/// `stem(y)` / `stem(x,y)` — markers on vertical stems.
fn b_stem(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("stem: not enough arguments");
    }
    let (x, y) = xy_args(args);
    clear_current_series_unless_hold(i);
    let (fig, axes_idx) = current_axes_loc(i);
    let ax = i.graphics.current_figure_mut().current_axes_mut();
    let color = default_color(ax.series.len());
    ax.series.push(Series::Stem(StemSeries {
        x,
        y,
        color,
        marker: "o".into(),
        name: String::new(),
    }));
    let h = i.graphics.register_series(fig, axes_idx, ObjKind::Line);
    i.graphics.dirty = true;
    Ok(vec![handle(h)])
}

/// `stairs(y)` / `stairs(x,y)` — staircase step plot.
fn b_stairs(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("stairs: not enough arguments");
    }
    let (x, y) = xy_args(args);
    clear_current_series_unless_hold(i);
    let (fig, axes_idx) = current_axes_loc(i);
    let ax = i.graphics.current_figure_mut().current_axes_mut();
    let color = default_color(ax.series.len());
    ax.series.push(Series::Stairs(StairsSeries {
        x,
        y,
        color,
        name: String::new(),
    }));
    let h = i.graphics.register_series(fig, axes_idx, ObjKind::Line);
    i.graphics.dirty = true;
    Ok(vec![handle(h)])
}

// ---- errorbar ---------------------------------------------------------------

/// `errorbar(x, y, e)` (or `errorbar(y, e)`) — line with symmetric error bars.
fn b_errorbar(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let (x, y, e) = match args.len() {
        0 | 1 => return err("errorbar: expected errorbar(x,y,e) or errorbar(y,e)"),
        2 => {
            let y = to_f64_vec(&args[0]);
            let x: Vec<f64> = (1..=y.len()).map(|k| k as f64).collect();
            (x, y, to_f64_vec(&args[1]))
        }
        _ => (
            to_f64_vec(&args[0]),
            to_f64_vec(&args[1]),
            to_f64_vec(&args[2]),
        ),
    };
    clear_current_series_unless_hold(i);
    let (fig, axes_idx) = current_axes_loc(i);
    let ax = i.graphics.current_figure_mut().current_axes_mut();
    let color = default_color(ax.series.len());
    ax.series.push(Series::Errorbar(ErrorbarSeries {
        x,
        y,
        e,
        color,
        name: String::new(),
    }));
    let h = i.graphics.register_series(fig, axes_idx, ObjKind::Line);
    i.graphics.dirty = true;
    Ok(vec![handle(h)])
}

// ---- plot3 ------------------------------------------------------------------

/// `plot3(x, y, z[, linespec])` — a 3-D line plot.
fn b_plot3(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.len() < 3 {
        return err("plot3: expected plot3(x,y,z)");
    }
    clear_current_series_unless_hold(i);
    let x = to_f64_vec(&args[0]);
    let y = to_f64_vec(&args[1]);
    let z = to_f64_vec(&args[2]);
    let mut spec = fm_graphics::LineSpec::default();
    if let Some(s) = args.get(3).and_then(Array::as_string) {
        let parsed = parse_linespec(&s);
        if parsed.valid {
            spec = parsed;
        }
    }
    let (fig, axes_idx) = current_axes_loc(i);
    let ax = i.graphics.current_figure_mut().current_axes_mut();
    let color = if spec.color.is_empty() {
        default_color(ax.series.len())
    } else {
        spec.color.clone()
    };
    let line_style = if spec.line_style.is_empty() && spec.marker.is_empty() {
        "-".to_string()
    } else {
        spec.line_style.clone()
    };
    ax.series.push(Series::Line3d(Line3dSeries {
        x,
        y,
        z,
        line_style,
        marker: spec.marker.clone(),
        color,
        name: String::new(),
    }));
    let h = i.graphics.register_series(fig, axes_idx, ObjKind::Line);
    i.graphics.dirty = true;
    Ok(vec![handle(h)])
}

// ---- peaks ------------------------------------------------------------------

/// `peaks` / `peaks(n)` — the classic demo surface. Returns the `n×n` matrix.
///
/// FreeMat/MATLAB's `peaks` plots a `surf` when called with *zero* output
/// arguments. The interpreter always invokes builtins with `nargout >= 1`
/// (every expression statement requests one output, bound to `ans`), so we
/// cannot distinguish a bare `peaks` from `Z = peaks` here — and returning the
/// matrix is the form needed by the dominant uses (`surf(peaks)`, `Z=peaks`,
/// `mesh(peaks(30))`). To plot the demo surface, use `surf(peaks)`.
fn b_peaks(_i: &mut Interpreter, args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    let n = args
        .first()
        .and_then(Array::as_f64)
        .map(|v| v.round().max(1.0) as usize)
        .unwrap_or(49);
    let (z, rows, cols) = peaks_matrix(n);
    // Column-major flatten for the Array.
    let mut col_major = vec![0.0; rows * cols];
    for r in 0..rows {
        for c in 0..cols {
            col_major[c * rows + r] = z[r][c];
        }
    }
    Ok(vec![build_real(
        DataClass::Double,
        &[rows, cols],
        col_major,
    )])
}

/// The classic `peaks` surface on an `n×n` grid over `[-3, 3]²`.
/// `z = 3*(1-x)^2*exp(-x^2-(y+1)^2) - 10*(x/5-x^3-y^5)*exp(-x^2-y^2)
///       - 1/3*exp(-(x+1)^2-y^2)` (FreeMat/MATLAB definition).
fn peaks_matrix(n: usize) -> (Vec<Vec<f64>>, usize, usize) {
    let n = n.max(1);
    let lin: Vec<f64> = if n == 1 {
        vec![0.0]
    } else {
        (0..n)
            .map(|k| -3.0 + 6.0 * k as f64 / (n as f64 - 1.0))
            .collect()
    };
    let mut z = vec![vec![0.0; n]; n];
    for r in 0..n {
        let y = lin[r];
        for c in 0..n {
            let x = lin[c];
            z[r][c] = 3.0 * (1.0 - x).powi(2) * (-(x * x) - (y + 1.0).powi(2)).exp()
                - 10.0 * (x / 5.0 - x.powi(3) - y.powi(5)) * (-(x * x) - y * y).exp()
                - (1.0 / 3.0) * (-(x + 1.0).powi(2) - y * y).exp();
        }
    }
    (z, n, n)
}

// `semilogx`/`semilogy`/`loglog` set the scale then delegate to plot.
pub(crate) fn register_log_plots(table: &mut FunctionTable) {
    table.add_builtin("semilogx", |i, a, n| log_plot(i, a, n, true, false));
    table.add_builtin("semilogy", |i, a, n| log_plot(i, a, n, false, true));
    table.add_builtin("loglog", |i, a, n| log_plot(i, a, n, true, true));
}

fn log_plot(
    i: &mut Interpreter,
    args: &[Array],
    n: usize,
    logx: bool,
    logy: bool,
) -> Flow<Vec<Array>> {
    let r = b_plot(i, args, n)?;
    let ax = i.graphics.current_figure_mut().current_axes_mut();
    if logx {
        ax.xscale = Scale::Log;
    }
    if logy {
        ax.yscale = Scale::Log;
    }
    i.graphics.dirty = true;
    Ok(r)
}
