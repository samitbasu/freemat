//! Interpreter-aware builtins: `eval`, `evalin`, `feval`, `builtin`, `exist`,
//! `clear`, `isset`, `assignin`, plus a handful of type predicates.

use std::collections::HashMap;
use std::sync::Arc;

use fm_core::{Array, DataClass, FunctionHandle};
use fm_interp::error::{Flow, InterpError, Signal};
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter, collect_free_vars};
use fm_parser::{Span, parse_expression, parse_statements};

use crate::util::need;

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("eval", b_eval);
    table.add_builtin("evalin", b_evalin);
    table.add_builtin("source", b_source);
    table.add_builtin("feval", b_feval);
    table.add_builtin("builtin", b_builtin);
    table.add_builtin("exist", b_exist);
    table.add_builtin("isset", b_isset);
    table.add_builtin("who", b_who);
    table.add_builtin("whos", b_whos);
    table.add_builtin("where", b_where);
    table.add_builtin("lasterr", b_lasterr);
    table.add_builtin("clear", b_clear);
    table.add_builtin("assignin", b_assignin);
    table.add_builtin("inline", b_inline);
    table.add_builtin("symvar", b_symvar);
    table.add_builtin("func2str", b_func2str);
    table.add_builtin("str2func", b_str2func);
    table.add_builtin("is_function_handle", b_is_function_handle);
    table.add_builtin("isfunctionhandle", b_is_function_handle);
    table.add_builtin("ode45", b_ode45);
    table.add_builtin("mpower", b_mpower);
}

/// `mpower(a, b)` — the matrix power `a ^ b`. Delegates to the exact same
/// operator path as the `^` operator so the results match identically.
fn b_mpower(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "mpower")?;
    let out = fm_interp::ops::binary(fm_parser::ast::BinaryOp::Pow, &args[0], &args[1])?;
    Ok(vec![out])
}

/// `ode45(f, tspan, y0[, options])` — solve the IVP `y' = f(t,y)`, `y(t0)=y0`
/// using the Dormand–Prince embedded RK45 method with adaptive step control.
///
/// Return conventions match FreeMat / MATLAB:
///   * `[t,y] = ode45(...)` → `t` is a column vector of time points, `y` is a
///     matrix with one row per time point and one column per state component.
///   * `SOL = ode45(...)` → a struct with fields `x` (row vector of times) and
///     `y` (state matrix, states × times).
///
/// When `tspan` is `[t0 tf]` the solver returns its natural adaptive steps;
/// when it is a longer vector the solution is reported at those exact times.
fn b_ode45(i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 3, "ode45")?;
    if !args[0].is_function_handle() && args[0].as_string().is_none() {
        return Err(Signal::Error(InterpError::msg(
            "ode45: first argument must be a function handle or function name",
        )));
    }

    let tspan = to_f64_vec(&args[1]);
    if tspan.len() < 2 {
        return Err(Signal::Error(InterpError::msg(
            "ode45: tspan must have at least two elements",
        )));
    }
    let t0 = tspan[0];
    let tf = *tspan.last().unwrap();
    let y0 = to_f64_vec(&args[2]);
    let n = y0.len();
    if n == 0 {
        return Err(Signal::Error(InterpError::msg(
            "ode45: initial condition y0 must be non-empty",
        )));
    }

    // Tolerances (MATLAB defaults).
    let reltol = 1e-3_f64;
    let abstol = 1e-6_f64;
    let direction = if tf >= t0 { 1.0 } else { -1.0 };

    // Output mode: 2-element span → natural steps; longer vector → requested times.
    let requested: Option<Vec<f64>> = if tspan.len() > 2 {
        Some(tspan.clone())
    } else {
        None
    };

    // Helper to evaluate the derivative f(t, y) -> Vec<f64> of length n.
    let eval_f = |i: &mut Interpreter, t: f64, y: &[f64]| -> Flow<Vec<f64>> {
        let ta = Array::Scalar(fm_core::ScalarValue::Double(t));
        let ya = build_real(DataClass::Double, &[y.len(), 1], y.to_vec());
        let out = i.invoke_handle(&args[0], &[ta, ya], 1, "", Span::empty(0))?;
        let dv = out
            .first()
            .map(to_f64_vec)
            .ok_or_else(|| Signal::Error(InterpError::msg("ode45: f returned no output")))?;
        if dv.len() != y.len() {
            return Err(Signal::Error(InterpError::msg(format!(
                "ode45: f returned a vector of length {} but expected {}",
                dv.len(),
                y.len()
            ))));
        }
        Ok(dv)
    };

    // Dormand–Prince (DOPRI5) Butcher tableau.
    // c nodes
    const C2: f64 = 1.0 / 5.0;
    const C3: f64 = 3.0 / 10.0;
    const C4: f64 = 4.0 / 5.0;
    const C5: f64 = 8.0 / 9.0;
    // a coefficients
    const A21: f64 = 1.0 / 5.0;
    const A31: f64 = 3.0 / 40.0;
    const A32: f64 = 9.0 / 40.0;
    const A41: f64 = 44.0 / 45.0;
    const A42: f64 = -56.0 / 15.0;
    const A43: f64 = 32.0 / 9.0;
    const A51: f64 = 19372.0 / 6561.0;
    const A52: f64 = -25360.0 / 2187.0;
    const A53: f64 = 64448.0 / 6561.0;
    const A54: f64 = -212.0 / 729.0;
    const A61: f64 = 9017.0 / 3168.0;
    const A62: f64 = -355.0 / 33.0;
    const A63: f64 = 46732.0 / 5247.0;
    const A64: f64 = 49.0 / 176.0;
    const A65: f64 = -5103.0 / 18656.0;
    // 5th-order solution weights == row-7 a-coefficients (FSAL).
    const B1: f64 = 35.0 / 384.0;
    const B3: f64 = 500.0 / 1113.0;
    const B4: f64 = 125.0 / 192.0;
    const B5: f64 = -2187.0 / 6784.0;
    const B6: f64 = 11.0 / 84.0;
    // 4th-order embedded weights.
    const E1: f64 = 5179.0 / 57600.0;
    const E3: f64 = 7571.0 / 16695.0;
    const E4: f64 = 393.0 / 640.0;
    const E5: f64 = -92097.0 / 339200.0;
    const E6: f64 = 187.0 / 2100.0;
    const E7: f64 = 1.0 / 40.0;

    let span = (tf - t0).abs();
    // Initial step heuristic; deterministic for fixed inputs.
    let mut h = direction * (span / 100.0).max(span * 1e-6).max(1e-12);
    let hmax = span / 10.0;

    // Accumulated accepted points (natural-step mode) and FSAL stage.
    let mut t = t0;
    let mut y = y0.clone();
    let mut f0 = eval_f(i, t, &y)?;

    // Storage for output.
    let mut t_out: Vec<f64> = Vec::new();
    let mut y_out: Vec<f64> = Vec::new(); // row-major rows = time points
    let mut req_idx = 0usize; // next requested-time index to emit (skip t0 handled below)

    // Always record the initial point.
    match &requested {
        Some(rt) => {
            // Emit requested times that coincide with t0 (typically the first).
            while req_idx < rt.len() && (rt[req_idx] - t0).abs() <= 1e-12 * (1.0 + t0.abs()) {
                t_out.push(rt[req_idx]);
                y_out.extend_from_slice(&y);
                req_idx += 1;
            }
            if t_out.is_empty() {
                t_out.push(t0);
                y_out.extend_from_slice(&y);
            }
        }
        None => {
            t_out.push(t0);
            y_out.extend_from_slice(&y);
        }
    }

    let still_going = |t: f64| direction * (tf - t) > 0.0;
    let mut iter_guard = 0u64;
    let max_iters = 2_000_000u64;

    while still_going(t) {
        iter_guard += 1;
        if iter_guard > max_iters {
            return Err(Signal::Error(InterpError::msg(
                "ode45: too many steps (is the problem singular?)",
            )));
        }
        // Clamp step magnitude.
        if h.abs() > hmax {
            h = direction * hmax;
        }
        // Don't step past tf.
        if direction * (t + h - tf) > 0.0 {
            h = tf - t;
        }
        if h == 0.0 {
            break;
        }

        // Stages.
        let k1 = f0.clone();
        let k1 = &k1;
        let mut ys = vec![0.0; n];
        for j in 0..n {
            ys[j] = y[j] + h * A21 * k1[j];
        }
        let k2 = eval_f(i, t + C2 * h, &ys)?;
        for j in 0..n {
            ys[j] = y[j] + h * (A31 * k1[j] + A32 * k2[j]);
        }
        let k3 = eval_f(i, t + C3 * h, &ys)?;
        for j in 0..n {
            ys[j] = y[j] + h * (A41 * k1[j] + A42 * k2[j] + A43 * k3[j]);
        }
        let k4 = eval_f(i, t + C4 * h, &ys)?;
        for j in 0..n {
            ys[j] = y[j] + h * (A51 * k1[j] + A52 * k2[j] + A53 * k3[j] + A54 * k4[j]);
        }
        let k5 = eval_f(i, t + C5 * h, &ys)?;
        for j in 0..n {
            ys[j] =
                y[j] + h * (A61 * k1[j] + A62 * k2[j] + A63 * k3[j] + A64 * k4[j] + A65 * k5[j]);
        }
        let k6 = eval_f(i, t + h, &ys)?;
        // 5th-order solution.
        let mut ynew = vec![0.0; n];
        for j in 0..n {
            ynew[j] = y[j] + h * (B1 * k1[j] + B3 * k3[j] + B4 * k4[j] + B5 * k5[j] + B6 * k6[j]);
        }
        let k7 = eval_f(i, t + h, &ynew)?;

        // Error estimate (difference of 5th- and 4th-order solutions).
        let mut err_norm = 0.0_f64;
        for j in 0..n {
            let yerr = h
                * ((B1 - E1) * k1[j]
                    + (B3 - E3) * k3[j]
                    + (B4 - E4) * k4[j]
                    + (B5 - E5) * k5[j]
                    + (B6 - E6) * k6[j]
                    - E7 * k7[j]);
            let sc = abstol + reltol * y[j].abs().max(ynew[j].abs());
            let ratio = yerr / sc;
            err_norm += ratio * ratio;
        }
        err_norm = (err_norm / n as f64).sqrt();

        if err_norm <= 1.0 || h.abs() <= span * 1e-12 {
            // Accept the step.
            let t_prev = t;
            let y_prev = y.clone();
            t += h;
            y = ynew.clone();
            f0 = k7; // FSAL: last stage is f(t+h, ynew).

            match &requested {
                Some(rt) => {
                    // Emit any requested times within (t_prev, t] via 4th-order
                    // Hermite interpolation using the endpoint derivatives.
                    while req_idx < rt.len()
                        && direction * (t - rt[req_idx]) >= -1e-12 * (1.0 + t.abs())
                        && direction * (rt[req_idx] - t_prev) > 0.0
                    {
                        let tr = rt[req_idx];
                        let s = (tr - t_prev) / (t - t_prev);
                        let yi = hermite(&y_prev, &y, k1, &f0, t - t_prev, s, n);
                        t_out.push(tr);
                        y_out.extend_from_slice(&yi);
                        req_idx += 1;
                    }
                }
                None => {
                    t_out.push(t);
                    y_out.extend_from_slice(&y);
                }
            }

            // Step-size growth.
            let factor = if err_norm == 0.0 {
                5.0
            } else {
                (0.8 * err_norm.powf(-0.2)).clamp(0.2, 5.0)
            };
            h *= factor;
        } else {
            // Reject and shrink.
            let factor = (0.8 * err_norm.powf(-0.2)).clamp(0.1, 1.0);
            h *= factor;
        }
    }

    let npts = t_out.len();
    if nargout >= 2 {
        // t as column vector, y as (npts × n) matrix.
        let t_arr = build_real(DataClass::Double, &[npts, 1], t_out);
        // y_out is row-major (npts rows, n cols); convert to column-major.
        let mut col_major = vec![0.0; npts * n];
        for r in 0..npts {
            for c in 0..n {
                col_major[c * npts + r] = y_out[r * n + c];
            }
        }
        let y_arr = build_real(DataClass::Double, &[npts, n], col_major);
        Ok(vec![t_arr, y_arr])
    } else {
        // SOL struct: x = row vector of times, y = states × times matrix.
        let x_arr = build_real(DataClass::Double, &[1, npts], t_out);
        // y field: (n states × npts times) matrix, column-major. Element
        // (state c, time r) lives at index c + r*n; y_out is row-major
        // (time r, state c) at r*n + c — the same layout here.
        let mut col_major = vec![0.0; npts * n];
        for r in 0..npts {
            for c in 0..n {
                col_major[r * n + c] = y_out[r * n + c];
            }
        }
        let y_arr = build_real(DataClass::Double, &[n, npts], col_major);
        let sol = fm_core::StructArray::scalar([
            ("x".to_string(), x_arr),
            ("y".to_string(), y_arr),
            ("solver".to_string(), Array::char_string("ode45")),
        ]);
        Ok(vec![Array::struct_array(sol)])
    }
}

/// Cubic Hermite interpolation between two solver points using the endpoint
/// derivatives (`f_a`, `f_b`) over step `dt`, at normalized position `s` in
/// `[0,1]`. Returns the interpolated state vector of length `n`.
fn hermite(
    ya: &[f64],
    yb: &[f64],
    f_a: &[f64],
    f_b: &[f64],
    dt: f64,
    s: f64,
    n: usize,
) -> Vec<f64> {
    let s2 = s * s;
    let s3 = s2 * s;
    let h00 = 2.0 * s3 - 3.0 * s2 + 1.0;
    let h10 = s3 - 2.0 * s2 + s;
    let h01 = -2.0 * s3 + 3.0 * s2;
    let h11 = s3 - s2;
    let mut out = vec![0.0; n];
    for j in 0..n {
        out[j] = h00 * ya[j] + h10 * dt * f_a[j] + h01 * yb[j] + h11 * dt * f_b[j];
    }
    out
}

/// `func2str(h)` — the source text of a handle: `name` for `@name`, `@(x) expr`
/// for an anonymous closure (FreeMat/MATLAB).
fn b_func2str(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "func2str")?;
    let h = args[0].as_function_handle().ok_or_else(|| {
        Signal::Error(InterpError::msg(
            "func2str: argument must be a function handle",
        ))
    })?;
    Ok(vec![Array::char_string(&h.to_str())])
}

/// `str2func(s)` — build a handle from a string. `'name'` → `@name`; a string
/// beginning with `@` is parsed as an anonymous-function expression.
fn b_str2func(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "str2func")?;
    let s = args[0]
        .as_string()
        .ok_or_else(|| Signal::Error(InterpError::msg("str2func: argument must be a string")))?;
    let trimmed = s.trim();
    if trimmed.starts_with('@') {
        // Parse and evaluate the `@(...)...` expression in the current scope so a
        // closure captures live variables, exactly like writing it literally.
        let expr = parse_expression(trimmed)
            .map_err(|e| Signal::Error(InterpError::msg(format!("str2func: parse error: {e}"))))?;
        return Ok(vec![i.eval(&expr, trimmed)?]);
    }
    Ok(vec![Array::function_handle(FunctionHandle::named(trimmed))])
}

/// `is_function_handle(x)` — true iff `x` is a function-handle value.
fn b_is_function_handle(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "is_function_handle")?;
    Ok(vec![Array::bool(args[0].is_function_handle())])
}

/// `eval(expr)` / `eval(expr, catch)` — parse and run `expr` in the current
/// scope. If it raises and a catch string is given, run that instead. When the
/// caller requests outputs (`[a,b] = eval('f(x)')`) and the source is a single
/// expression, the expression's values are returned.
fn b_eval(i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "eval")?;
    let src = args[0]
        .as_string()
        .ok_or_else(|| Signal::Error(InterpError::msg("eval: argument must be a string")))?;
    match eval_string(i, &src, nargout) {
        Ok(v) => Ok(v),
        Err(e) => {
            // Two-argument form `eval(try, catch)`: if the first string raises,
            // run the catch string instead — and, like the try, produce the
            // requested outputs (so `b = eval('z','a+1')` assigns the catch's
            // value, not an error). The catch path applies to BOTH the
            // statement form and the output-producing form.
            if let Some(catch) = args.get(1).and_then(Array::as_string) {
                eval_string(i, &catch, nargout)
            } else {
                Err(e)
            }
        }
    }
}

/// Evaluate one `eval` source string. When an output is requested (`nargout >=
/// 1`) and the source parses as a bare expression, return its value(s);
/// otherwise run it as statements (assignments/calls display in the current
/// scope) and yield no outputs.
fn eval_string(i: &mut Interpreter, src: &str, nargout: usize) -> Flow<Vec<Array>> {
    if nargout >= 1
        && let Ok(expr) = fm_parser::parse_expression(src.trim_end_matches([';', ' ', '\n']))
    {
        return i.eval_multi(&expr, nargout, src);
    }
    run_source(i, src).map(|()| vec![])
}

/// `source(filename)` — read a script file and execute its statements in the
/// *current* scope (FreeMat's `Source.cpp`). Unlike calling a function file,
/// the script's assignments land in the caller's workspace, and unlike running
/// a file as a script it does not re-execute the final line.
fn b_source(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "source")?;
    let path = args[0]
        .as_string()
        .ok_or_else(|| Signal::Error(InterpError::msg("source: argument must be a filename")))?;
    let contents = std::fs::read_to_string(&path).map_err(|e| {
        Signal::Error(InterpError::msg(format!(
            "source: cannot read '{path}': {e}"
        )))
    })?;
    run_source(i, &contents)?;
    Ok(vec![])
}

/// `evalin(context, expr)` — we treat the context name (`'base'`/`'caller'`) as
/// a no-op and evaluate in the current scope (sufficient for the test corpus).
fn b_evalin(i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 2, "evalin")?;
    let context = args[0].as_string().unwrap_or_default();
    let src = args[1]
        .as_string()
        .ok_or_else(|| Signal::Error(InterpError::msg("evalin: expression must be a string")))?;
    // `evalin('caller'/'base', expr)` runs `expr` in the caller's / base scope
    // (see `run_in_scope`). When an output is requested, the trimmed source is
    // evaluated as a bare expression and its value returned (mirroring `eval`).
    run_in_scope(i, &context, |i| {
        if nargout >= 1
            && let Ok(expr) = fm_parser::parse_expression(src.trim_end_matches([';', ' ', '\n']))
        {
            return i.eval_multi(&expr, nargout, &src);
        }
        let res = run_source(i, &src);
        // `evalin(ctx, expr, catch_expr)`: run the catch expression on error.
        if res.is_err()
            && let Some(catch) = args.get(2).and_then(Array::as_string)
        {
            run_source(i, &catch)?;
            return Ok(vec![]);
        }
        res.map(|()| vec![])
    })
}

/// Run `f` with the call frame switched to the named scope, then restore the
/// popped frames. `'caller'` pops the current executing frame (the function
/// that invoked the calling builtin) so the *caller* becomes the top scope;
/// `'base'` pops down to the base (top-level) scope. Any other context string
/// runs in the current scope.
fn run_in_scope<F, R>(i: &mut Interpreter, context: &str, f: F) -> Flow<R>
where
    F: FnOnce(&mut Interpreter) -> Flow<R>,
{
    let pop = match context {
        "caller" if i.context.depth() > 1 => 1,
        "base" => i.context.depth().saturating_sub(1),
        _ => 0,
    };
    let mut saved = Vec::with_capacity(pop);
    for _ in 0..pop {
        if let Some(s) = i.context.pop_scope() {
            saved.push(s);
        }
    }
    let out = f(i);
    while let Some(scope) = saved.pop() {
        i.context.restore_scope(scope);
    }
    out
}

/// Parse and execute `src` as a statement list in the current scope.
fn run_source(i: &mut Interpreter, src: &str) -> Flow<()> {
    let stmts =
        parse_statements(src).map_err(|e| Signal::Error(InterpError::msg(e.to_string())))?;
    // `run_block` swallows Break/Continue/Return; run statements directly so a
    // bare `return` inside the eval'd string still propagates, but tolerate the
    // common assignment/expr forms.
    for stmt in &stmts {
        i.exec_statement(stmt, src)?;
    }
    Ok(())
}

/// `feval(fn, args...)` — call `fn` (a name string **or** a function handle)
/// with the given arguments.
fn b_feval(i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "feval")?;
    if args[0].is_function_handle() {
        return i.invoke_handle(&args[0], &args[1..], nargout.max(1), "", Span::empty(0));
    }
    let name = args[0].as_string().ok_or_else(|| {
        Signal::Error(InterpError::msg(
            "feval: first argument must be a name or function handle",
        ))
    })?;
    i.call_function(&name, &args[1..], nargout.max(1), "", Span::empty(0))
}

/// `builtin(fn, args...)` — like `feval`, but always calls the *native* builtin
/// `fn`, bypassing any user-defined function / subfunction that shadows it.
fn b_builtin(i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "builtin")?;
    let name = args[0]
        .as_string()
        .ok_or_else(|| Signal::Error(InterpError::msg("builtin: first argument must be a name")))?;
    i.call_builtin(&name, &args[1..], nargout.max(1))
}

/// `exist(name)` — 1 if `name` is a variable, 2 if a file/function, 5 if a
/// builtin, else 0. We map: variable → 1, function/builtin → 5, else 0.
fn b_exist(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "exist")?;
    let name = args[0].as_string().unwrap_or_default();
    let code = if i.context.exists(&name) {
        1.0
    } else if i.functions.contains(&name) {
        5.0
    } else {
        0.0
    };
    Ok(vec![Array::double(code)])
}

/// `isset(name)` — FreeMat: true iff a variable named `name` is defined.
fn b_isset(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "isset")?;
    let name = args[0].as_string().unwrap_or_default();
    // FreeMat's `isset` is true only when the variable is defined *and*
    // non-empty (`isDefed && !d->isEmpty()`).
    let set = i.context.lookup(&name).is_some_and(|v| v.numel() > 0);
    Ok(vec![Array::bool(set)])
}

/// Right-justify `s` in a field of `width` (pad on the left; never truncate),
/// matching FreeMat's `QString::rightJustified(width, ' ', false)`.
fn right_just(s: &str, width: usize) -> String {
    if s.len() >= width {
        s.to_string()
    } else {
        format!("{}{}", " ".repeat(width - s.len()), s)
    }
}

/// Render a dimension vector the way FreeMat's `Dimensions::toString` does:
/// `AxBxC`, trailing singleton dimensions trimmed (a scalar prints as `1`).
fn dims_to_string(dims: &[usize]) -> String {
    // Index one past the last dimension that isn't 1 (FreeMat's `lastNotOne`).
    let mut last_not_one = 0;
    for (idx, &d) in dims.iter().enumerate() {
        if d != 1 {
            last_not_one = idx + 1;
        }
    }
    let mut s = format!("{}", dims.first().copied().unwrap_or(0));
    for &d in &dims[1..last_not_one.max(1).min(dims.len())] {
        s.push_str(&format!("x{d}"));
    }
    s
}

/// The names to report for `who`/`whos`: the explicit argument list, or — when
/// none is given — every variable in the active scope, sorted (FreeMat sorts so
/// the listing is stable).
fn who_names(i: &Interpreter, args: &[Array]) -> Vec<String> {
    let mut names: Vec<String> = if args.is_empty() {
        i.context
            .active()
            .local_names()
            .into_iter()
            .map(str::to_string)
            .collect()
    } else {
        args.iter().filter_map(Array::as_string).collect()
    };
    names.sort();
    names
}

/// Append the per-variable flag/size columns shared by `who` and `whos`.
fn who_describe(i: &Interpreter, name: &str, out: &mut String) {
    out.push_str(&right_just(name, 15));
    match i.context.lookup(name) {
        None => out.push_str("   <undefined>"),
        Some(v) => {
            out.push_str(&right_just(v.class_name(), 10));
            out.push_str(if v.as_sparse().is_some() {
                "   sparse"
            } else {
                "         "
            });
            // Global/persistent flags are not surfaced through the public scope
            // API here; locals are the common case for the help corpus.
            out.push_str("         ");
            out.push_str(&format!("  [{}]", dims_to_string(&v.dims())));
        }
    }
}

/// Per-element byte size of a data class (for `whos`).
fn class_byte_size(c: fm_core::DataClass) -> usize {
    use fm_core::DataClass::*;
    match c {
        Bool | Int8 | UInt8 | Char => 1,
        Int16 | UInt16 => 2,
        Int32 | UInt32 | Float => 4,
        Int64 | UInt64 | Double => 8,
        Cell | Struct | FunctionHandle => std::mem::size_of::<usize>(),
    }
}

/// `who` — list the variables in the current scope (name, type, flags, size).
/// `who a b ...` / `who('a','b',...)` restrict the listing to named variables.
fn b_who(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let names = who_names(i, args);
    let mut out = String::from("  Variable Name       Type   Flags             Size\n");
    for name in &names {
        who_describe(i, name, &mut out);
        out.push('\n');
    }
    i.emit(&out);
    Ok(vec![])
}

/// `whos` — like `who`, but with a trailing byte-count column.
fn b_whos(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let names = who_names(i, args);
    let mut out = String::from("  Variable Name       Type   Flags             Size       Bytes\n");
    for name in &names {
        who_describe(i, name, &mut out);
        if let Some(v) = i.context.lookup(name) {
            let elt = if v.is_complex() { 2 } else { 1 };
            let bytes = v.numel() * elt * class_byte_size(v.class());
            // Pad the size column to width 15 so the byte count lines up, as in
            // FreeMat's `whos` (`txt.leftJustified(15)`); `who_describe` already
            // appended the `[size]`, so just append the bytes.
            out.push_str(&format!("   {bytes}"));
        }
        out.push('\n');
    }
    i.emit(&out);
    Ok(vec![])
}

/// `lasterr` — get or set the last caught error message. `lasterr` returns the
/// stored message; `lasterr('msg')` replaces it (and returns nothing).
fn b_lasterr(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if let Some(a) = args.first() {
        i.last_error = a.as_string().unwrap_or_default();
        return Ok(vec![]);
    }
    Ok(vec![Array::char_string(&i.last_error)])
}

/// Whether `name` denotes a function or built-in constant (and thus is *not* a
/// free symbolic variable, for `inline`/`symvar`). FreeMat eliminates anything
/// that resolves to a function; we also exclude the interpreter's constant
/// pseudo-variables (`pi`, `eps`, `i`, …), which are not in the function table.
fn is_function_or_const(i: &Interpreter, name: &str) -> bool {
    i.functions.contains(name)
        || matches!(
            name,
            "pi" | "e"
                | "eps"
                | "i"
                | "j"
                | "I"
                | "J"
                | "Inf"
                | "inf"
                | "NaN"
                | "nan"
                | "nargin"
                | "nargout"
                | "true"
                | "false"
        )
}

/// The symbolic variables of an expression string: free identifiers that are not
/// functions or constants, sorted (FreeMat's `symvar` ordering).
fn symbolic_vars(i: &Interpreter, expr: &fm_parser::ast::Expr) -> Vec<String> {
    let mut free = Vec::new();
    collect_free_vars(expr, &mut free);
    free.retain(|n| !is_function_or_const(i, n));
    free.sort();
    free
}

/// `inline(expr)` / `inline(expr, 'a', 'b', ...)` — build a callable function
/// object from an expression string. With explicit argument names they become
/// the parameters in order; otherwise the parameters are the expression's
/// symbolic variables (sorted). Implemented as an anonymous function handle, so
/// it is callable as `f(...)` and via `feval`.
fn b_inline(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "inline")?;
    let expr_str = args[0].as_string().ok_or_else(|| {
        Signal::Error(InterpError::msg("inline: first argument must be a string"))
    })?;
    let expr = parse_expression(&expr_str)
        .map_err(|e| Signal::Error(InterpError::msg(format!("inline: {e}"))))?;
    // Parameters: explicit names, or the auto-detected symbolic variables.
    let params: Vec<String> = if args.len() > 1 {
        args[1..].iter().filter_map(Array::as_string).collect()
    } else {
        symbolic_vars(i, &expr)
    };
    // Capture any remaining free variables (not parameters) by value, like an
    // anonymous function (`inline('a*x','x')` with `a` in scope closes over `a`).
    let mut captures = HashMap::new();
    let mut free = Vec::new();
    collect_free_vars(&expr, &mut free);
    for n in free {
        if params.contains(&n) {
            continue;
        }
        if let Some(v) = i.context.lookup(&n) {
            captures.insert(n, v.clone());
        }
    }
    let text = format!("@({}) {expr_str}", params.join(","));
    let handle =
        FunctionHandle::anonymous(params, expr, captures, text, Arc::new(expr_str.clone()));
    Ok(vec![Array::function_handle(handle)])
}

/// `symvar(expr)` — the symbolic variables in the expression string, returned as
/// a cell array of names (sorted; functions and constants are excluded).
fn b_symvar(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "symvar")?;
    let expr_str = args[0]
        .as_string()
        .ok_or_else(|| Signal::Error(InterpError::msg("symvar: argument must be a string")))?;
    let expr = parse_expression(&expr_str)
        .map_err(|e| Signal::Error(InterpError::msg(format!("symvar: {e}"))))?;
    let vars = symbolic_vars(i, &expr);
    let n = vars.len();
    let data: Vec<Array> = vars.into_iter().map(|s| Array::char_string(&s)).collect();
    Ok(vec![Array::cell(&[n, 1], data)])
}

/// `where` — print a stack trace of the current call stack (base → top).
fn b_where(i: &mut Interpreter, _args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let mut out = String::new();
    for (name, line) in i.context.stack_trace() {
        let label = if name.is_empty() { "base" } else { &name };
        match line {
            Some(l) => out.push_str(&format!("   {label} (line {l})\n")),
            None => out.push_str(&format!("   {label}\n")),
        }
    }
    i.emit(&out);
    Ok(vec![])
}

/// `clear(name, ...)` / `clear('all')` — remove variables from the scope.
fn b_clear(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return Ok(vec![]);
    }
    for a in args {
        if let Some(name) = a.as_string() {
            if name == "all" || name == "-all" {
                let names: Vec<String> = i
                    .context
                    .top()
                    .local_names()
                    .into_iter()
                    .map(str::to_string)
                    .collect();
                for n in names {
                    i.context.remove(&n);
                }
            } else {
                i.context.remove(&name);
            }
        }
    }
    Ok(vec![])
}

/// `assignin(context, name, value)` — assign in the current scope (context is a
/// no-op for the test corpus).
fn b_assignin(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 3, "assignin")?;
    let context = args[0].as_string().unwrap_or_default();
    let name = args[1]
        .as_string()
        .ok_or_else(|| Signal::Error(InterpError::msg("assignin: name must be a string")))?;
    let value = args[2].clone();
    // `assignin('caller'/'base', name, value)` binds in the caller / base scope.
    run_in_scope(i, &context, |i| {
        i.context.assign(&name, value);
        Ok(())
    })?;
    Ok(vec![])
}
