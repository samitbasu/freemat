//! Graphics builtins: `figure`, `plot`, `line`, `title`, `xlabel`/`ylabel`/
//! `zlabel`, `legend`, `hold`, `axis`, `grid`, `clf`, `gcf`/`gca`, `drawnow`,
//! and basic `surf`/`mesh`/`image`.
//!
//! These build the semantic [`fm_graphics`] scene directly in interpreter state
//! and mark it dirty; the implicit draw at the end of a top-level command (or an
//! explicit `drawnow`) flushes it through the optional sink. This is the
//! pragmatic Milestone-2 path — it does **not** reproduce FreeMat's full
//! handle-property `set`/`get` system (see PROGRESS.md for deferred fidelity).

use fm_core::Array;
use fm_graphics::{
    AxisLimits, ImageSeries, Legend, LineSeries, Scale, Series, SurfaceSeries, default_color,
    parse_linespec,
};
use fm_interp::error::Flow;
use fm_interp::value::to_f64_vec;
use fm_interp::{FunctionTable, Interpreter};

use crate::util::err;

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("figure", b_figure);
    table.add_builtin("plot", b_plot);
    table.add_builtin("line", b_line);
    table.add_builtin("title", b_title);
    table.add_builtin("xlabel", |i, a, _n| label(i, a, Axis::X));
    table.add_builtin("ylabel", |i, a, _n| label(i, a, Axis::Y));
    table.add_builtin("zlabel", |i, a, _n| label(i, a, Axis::Z));
    table.add_builtin("legend", b_legend);
    table.add_builtin("hold", b_hold);
    table.add_builtin("axis", b_axis);
    table.add_builtin("grid", b_grid);
    table.add_builtin("clf", b_clf);
    table.add_builtin("gcf", b_gcf);
    table.add_builtin("gca", b_gca);
    table.add_builtin("drawnow", b_drawnow);
    table.add_builtin("surf", |i, a, _n| surface(i, a, false));
    table.add_builtin("mesh", |i, a, _n| surface(i, a, true));
    table.add_builtin("image", b_image);
    table.add_builtin("imagesc", b_image);
}

/// Read a single string argument (linespec, label text, on/off, ...).
fn str_arg(args: &[Array], i: usize) -> Option<String> {
    args.get(i).and_then(Array::as_string)
}

/// The figure number, as a scalar `double` array (FreeMat returns the handle).
fn scalar(v: f64) -> Array {
    Array::Scalar(fm_core::ScalarValue::Double(v))
}

// ---- figure / gcf / gca / clf -----------------------------------------------

fn b_figure(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let id = if let Some(n) = args.first().and_then(Array::as_f64) {
        n.max(1.0) as u64
    } else {
        i.graphics.next_figure_id()
    };
    i.graphics.select_figure(id);
    i.graphics.dirty = true;
    Ok(vec![scalar(id as f64)])
}

fn b_gcf(i: &mut Interpreter, _a: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let id = i.graphics.ensure_figure();
    Ok(vec![scalar(id as f64)])
}

fn b_gca(i: &mut Interpreter, _a: &[Array], _n: usize) -> Flow<Vec<Array>> {
    // We model a single axes per figure; return the figure handle as a stand-in.
    let id = i.graphics.ensure_figure();
    Ok(vec![scalar(id as f64)])
}

fn b_clf(i: &mut Interpreter, _a: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let fig = i.graphics.current_figure_mut();
    fig.axes = vec![fm_graphics::Axes::new()];
    i.graphics.dirty = true;
    Ok(vec![])
}

fn b_drawnow(i: &mut Interpreter, _a: &[Array], _n: usize) -> Flow<Vec<Array>> {
    i.graphics.flush();
    Ok(vec![])
}

// ---- plot / line ------------------------------------------------------------

fn b_plot(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("plot: not enough arguments");
    }
    // newplot semantics: unless `hold on`, clear the current axes' series.
    {
        let fig = i.graphics.current_figure_mut();
        let ax = fig.current_axes_mut();
        if !ax.hold {
            ax.series.clear();
        }
    }
    // Parse the (x?, y, linespec?) argument groups, MATLAB-style.
    let mut idx = 0;
    while idx < args.len() {
        // Determine x, y for this group.
        let (x, y, mut next) = if idx + 1 < args.len() && !args[idx + 1].is_char() {
            // plot(x, y, ...)
            (to_f64_vec(&args[idx]), to_f64_vec(&args[idx + 1]), idx + 2)
        } else {
            // plot(y, ...) — implicit x = 1:n
            let y = to_f64_vec(&args[idx]);
            let x: Vec<f64> = (1..=y.len()).map(|k| k as f64).collect();
            (x, y, idx + 1)
        };
        // Optional trailing linespec string for this group.
        let mut spec = fm_graphics::LineSpec::default();
        if let Some(s) = args.get(next).and_then(Array::as_string) {
            let parsed = parse_linespec(&s);
            if parsed.valid {
                spec = parsed;
                next += 1;
            }
        }
        add_line(i, x, y, &spec);
        idx = next;
    }
    i.graphics.dirty = true;
    let id = i.graphics.current_figure;
    Ok(vec![scalar(id as f64)])
}

/// Append a line series to the current axes, defaulting style/color.
fn add_line(i: &mut Interpreter, x: Vec<f64>, y: Vec<f64>, spec: &fm_graphics::LineSpec) {
    let ax = i.graphics.current_figure_mut().current_axes_mut();
    let series_index = ax.series.len();
    let line_style = if spec.line_style.is_empty() && spec.marker.is_empty() {
        "-".to_string() // default solid line
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
}

fn b_line(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.len() < 2 {
        return err("line: expected x and y vectors");
    }
    // `line` always adds (never clears), regardless of hold.
    let x = to_f64_vec(&args[0]);
    let y = to_f64_vec(&args[1]);
    let mut spec = fm_graphics::LineSpec::default();
    if let Some(s) = args.get(2).and_then(Array::as_string) {
        let parsed = parse_linespec(&s);
        if parsed.valid {
            spec = parsed;
        }
    }
    add_line(i, x, y, &spec);
    i.graphics.dirty = true;
    let id = i.graphics.current_figure;
    Ok(vec![scalar(id as f64)])
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
    // `legend off` hides; `legend(...)` with strings sets entry names + shows.
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
        // `hold` with no argument toggles.
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
            "tight" | "square" => {} // accepted; no-op for now
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
    let col_major = to_f64_vec(a); // column-major flat
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
    // surf(Z) or surf(X, Y, Z).
    let (z_arg, x, y) = if args.len() >= 3 {
        (&args[2], to_f64_vec(&args[0]), to_f64_vec(&args[1]))
    } else {
        (&args[0], Vec::new(), Vec::new())
    };
    let (z, _r, _c) = grid_of(z_arg);
    {
        let ax = i.graphics.current_figure_mut().current_axes_mut();
        if !ax.hold {
            ax.series.clear();
        }
        ax.series.push(Series::Surface(SurfaceSeries {
            z,
            x,
            y,
            colormap: "Viridis".into(),
            wireframe,
        }));
    }
    i.graphics.dirty = true;
    let id = i.graphics.current_figure;
    Ok(vec![scalar(id as f64)])
}

fn b_image(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("image: expected a data matrix");
    }
    let (data, _r, _c) = grid_of(&args[0]);
    {
        let ax = i.graphics.current_figure_mut().current_axes_mut();
        if !ax.hold {
            ax.series.clear();
        }
        ax.series.push(Series::Image(ImageSeries {
            data,
            colormap: "Viridis".into(),
        }));
    }
    i.graphics.dirty = true;
    let id = i.graphics.current_figure;
    Ok(vec![scalar(id as f64)])
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
