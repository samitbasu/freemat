//! Interpreter-aware builtins: `eval`, `evalin`, `feval`, `builtin`, `exist`,
//! `clear`, `isset`, `assignin`, plus a handful of type predicates.

use fm_core::{Array, FunctionHandle};
use fm_interp::error::{Flow, InterpError, Signal};
use fm_interp::{FunctionTable, Interpreter};
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
    table.add_builtin("clear", b_clear);
    table.add_builtin("assignin", b_assignin);
    table.add_builtin("func2str", b_func2str);
    table.add_builtin("str2func", b_str2func);
    table.add_builtin("is_function_handle", b_is_function_handle);
    table.add_builtin("isfunctionhandle", b_is_function_handle);
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
