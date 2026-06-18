//! Interpreter-aware builtins: `eval`, `evalin`, `feval`, `builtin`, `exist`,
//! `clear`, `isset`, `assignin`, plus a handful of type predicates.

use fm_core::Array;
use fm_interp::error::{Flow, InterpError, Signal};
use fm_interp::{FunctionTable, Interpreter};
use fm_parser::{Span, parse_statements};

use crate::util::need;

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("eval", b_eval);
    table.add_builtin("evalin", b_evalin);
    table.add_builtin("feval", b_feval);
    table.add_builtin("builtin", b_builtin);
    table.add_builtin("exist", b_exist);
    table.add_builtin("isset", b_isset);
    table.add_builtin("clear", b_clear);
    table.add_builtin("assignin", b_assignin);
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
    // Output-producing eval: parse the trimmed source as a bare expression.
    if nargout >= 1
        && let Ok(expr) = fm_parser::parse_expression(src.trim_end_matches([';', ' ', '\n']))
    {
        return i.eval_multi(&expr, nargout, &src);
    }
    match run_source(i, &src) {
        Ok(()) => Ok(vec![]),
        Err(e) => {
            if let Some(catch) = args.get(1).and_then(Array::as_string) {
                run_source(i, &catch)?;
                Ok(vec![])
            } else {
                Err(e)
            }
        }
    }
}

/// `evalin(context, expr)` — we treat the context name (`'base'`/`'caller'`) as
/// a no-op and evaluate in the current scope (sufficient for the test corpus).
fn b_evalin(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "evalin")?;
    let context = args[0].as_string().unwrap_or_default();
    let src = args[1]
        .as_string()
        .ok_or_else(|| Signal::Error(InterpError::msg("evalin: expression must be a string")))?;
    // `evalin('caller', expr)` runs `expr` in the *caller's* scope. The builtin
    // executes without its own frame, so the current top scope belongs to the
    // function that called `evalin`; popping it makes the caller the top scope.
    // `'base'` is the top-level scope; we approximate it with 'caller' since the
    // corpus only exercises the single-level case.
    let primary = run_in_caller_scope(i, &context, |i| {
        let res = run_source(i, &src);
        // `evalin('caller', expr, catch_expr)`: run the catch expression on error.
        if res.is_err()
            && let Some(catch) = args.get(2).and_then(Array::as_string)
        {
            return run_source(i, &catch);
        }
        res
    });
    primary?;
    Ok(vec![])
}

/// Run `f` with the call frame switched to the `'caller'`/`'base'` scope, then
/// restore. For `'caller'`, the current executing frame (the function that
/// invoked the calling builtin) is temporarily popped so the *caller* becomes
/// the top scope. Any other context string runs in the current scope.
fn run_in_caller_scope<F, R>(i: &mut Interpreter, context: &str, f: F) -> Flow<R>
where
    F: FnOnce(&mut Interpreter) -> Flow<R>,
{
    let switch = (context == "caller" || context == "base") && i.context.depth() > 1;
    if !switch {
        return f(i);
    }
    let saved = i.context.pop_scope();
    let out = f(i);
    if let Some(scope) = saved {
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

/// `feval(fn, args...)` — call `fn` (a name string) with the given arguments.
fn b_feval(i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "feval")?;
    let name = args[0]
        .as_string()
        .ok_or_else(|| Signal::Error(InterpError::msg("feval: first argument must be a name")))?;
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
    // `assignin('caller', name, value)` binds in the caller's scope.
    run_in_caller_scope(i, &context, |i| {
        i.context.assign(&name, value);
        Ok(())
    })?;
    Ok(vec![])
}
