//! Transport-agnostic DAP building blocks.
//!
//! These turn interpreter state into DAP response *bodies* (and back), with no
//! knowledge of how messages are framed or where they go. Both the standalone
//! [`Session`](crate::Session) (which owns a socket) and the embedded
//! engine-attached debugger in `fm-cli` (which talks to the interpreter actor
//! over channels) build their responses from these, so the two debuggers report
//! variables, stacks, and capabilities identically.

use std::collections::HashMap;

use fm_core::{Array, FormatMode};
use fm_interp::Interpreter;
use serde_json::{Value, json};

/// The single thread id we report — FreeMat-rs runs one interpreter thread.
pub const MAIN_THREAD: i64 = 1;

/// Build a success `response` message for `request`, carrying `body`, stamped
/// with sequence `seq`.
#[must_use]
pub fn response(request: &Value, body: Value, seq: i64) -> Value {
    json!({
        "seq": seq,
        "type": "response",
        "request_seq": request.get("seq").and_then(Value::as_i64).unwrap_or(0),
        "success": true,
        "command": request.get("command").and_then(Value::as_str).unwrap_or(""),
        "body": body,
    })
}

/// Build a failure `response` carrying a human-readable `message`.
#[must_use]
pub fn error_response(request: &Value, message: &str, seq: i64) -> Value {
    json!({
        "seq": seq,
        "type": "response",
        "request_seq": request.get("seq").and_then(Value::as_i64).unwrap_or(0),
        "success": false,
        "command": request.get("command").and_then(Value::as_str).unwrap_or(""),
        "message": message,
    })
}

/// Build an `event` message stamped with sequence `seq`.
#[must_use]
pub fn event(name: &str, body: Value, seq: i64) -> Value {
    json!({ "seq": seq, "type": "event", "event": name, "body": body })
}

/// The `command` field of a request (empty string if absent).
#[must_use]
pub fn command_of(request: &Value) -> &str {
    request.get("command").and_then(Value::as_str).unwrap_or("")
}

/// The `stopped` event body for a single-thread stop with the given `reason`.
#[must_use]
pub fn stopped_body(reason: &str) -> Value {
    json!({ "reason": reason, "threadId": MAIN_THREAD, "allThreadsStopped": true })
}

/// The capabilities advertised in the `initialize` response.
#[must_use]
pub fn capabilities() -> Value {
    json!({
        "supportsConfigurationDoneRequest": true,
        "supportsEvaluateForHovers": true,
        "supportsTerminateRequest": true,
        "supportsSingleThreadExecutionRequests": false,
    })
}

/// The `threads` response body (one thread, "main").
#[must_use]
pub fn threads_body() -> Value {
    json!({ "threads": [{ "id": MAIN_THREAD, "name": "main" }] })
}

/// Parse the `line` numbers from a `setBreakpoints` request's arguments.
#[must_use]
pub fn breakpoint_lines(args: &Value) -> Vec<usize> {
    args.get("breakpoints")
        .and_then(Value::as_array)
        .map(|bps| {
            bps.iter()
                .filter_map(|bp| bp.get("line").and_then(Value::as_i64))
                .map(|l| l as usize)
                .collect()
        })
        .unwrap_or_default()
}

/// The `setBreakpoints` response body confirming each line as verified.
#[must_use]
pub fn verified_breakpoints_body(lines: &[usize]) -> Value {
    let verified: Vec<Value> = lines
        .iter()
        .map(|&line| json!({ "verified": true, "line": line }))
        .collect();
    json!({ "breakpoints": verified })
}

/// The `stackTrace` body for the current call stack, innermost frame first
/// (DAP convention). Frame `id` is the scope index, so a `scopes`/`variables`
/// request can map a `frameId` straight back to a scope.
#[must_use]
pub fn stack_trace_body(interp: &Interpreter, src_name: &str, src_path: &str) -> Value {
    let trace = interp.context.stack_trace(); // base → top
    let total = trace.len();
    let mut frames = Vec::with_capacity(total);
    for (idx, (name, line)) in trace.iter().enumerate().rev() {
        let display = if name.is_empty() {
            "(main)".to_string()
        } else {
            name.clone()
        };
        frames.push(json!({
            "id": idx,
            "name": display,
            "line": line.unwrap_or(0),
            "column": 1,
            "source": { "name": src_name, "path": src_path },
        }));
    }
    json!({ "stackFrames": frames, "totalFrames": total })
}

/// Per-stop `variablesReference` allocator: maps the opaque references handed out
/// by `scopes` back to the call-frame index a later `variables` request reads.
/// Reset on every stop (references from a previous stop are invalid).
pub struct Frames {
    var_refs: HashMap<i64, usize>,
    next: i64,
}

impl Default for Frames {
    fn default() -> Self {
        Self::new()
    }
}

impl Frames {
    /// A fresh allocator. References start at 1000 (DAP needs them `> 0`, and
    /// staying clear of small frame ids keeps debugging output legible).
    #[must_use]
    pub fn new() -> Self {
        Frames {
            var_refs: HashMap::new(),
            next: 1000,
        }
    }

    /// Invalidate all references (call when entering a new stop).
    pub fn reset(&mut self) {
        self.var_refs.clear();
        self.next = 1000;
    }

    /// The `scopes` body for `frame_id`: a single "Locals" scope whose fresh
    /// `variablesReference` resolves back to that frame.
    pub fn scopes_body(&mut self, frame_id: i64) -> Value {
        let var_ref = self.next;
        self.next += 1;
        self.var_refs.insert(var_ref, frame_id as usize);
        json!({
            "scopes": [{
                "name": "Locals",
                "variablesReference": var_ref,
                "expensive": false,
            }],
        })
    }

    /// The `variables` body listing the locals of the frame `var_ref` points at.
    #[must_use]
    pub fn variables_body(&self, interp: &Interpreter, var_ref: i64) -> Value {
        let Some(&frame_idx) = self.var_refs.get(&var_ref) else {
            return json!({ "variables": [] });
        };
        let mut variables = Vec::new();
        if let Some(scope) = interp.context.scope_at(frame_idx) {
            let mut names: Vec<&str> = scope.local_names();
            names.sort_unstable();
            for name in names {
                if let Some(arr) = scope.get_local(name) {
                    let (value, ty) = summarize(arr);
                    variables.push(json!({
                        "name": name,
                        "value": value,
                        "type": ty,
                        "variablesReference": 0,
                    }));
                }
            }
        }
        json!({ "variables": variables })
    }
}

/// The `evaluate` response body, or an error string. Evaluates `expr` in the
/// interpreter's current top frame with the seam suppressed (a watch/hover
/// expression must never trip a breakpoint).
pub fn evaluate_body(interp: &mut Interpreter, expr: &str) -> Result<Value, String> {
    let arr = eval_expression(interp, expr)?;
    let (value, ty) = summarize(&arr);
    Ok(json!({ "result": value, "type": ty, "variablesReference": 0 }))
}

/// Parse and evaluate `expr` in the interpreter's current top frame, returning a
/// concise error string on failure.
fn eval_expression(interp: &mut Interpreter, expr: &str) -> Result<Array, String> {
    use fm_parser::ast::{Program, StmtKind};
    let program = fm_parser::parse_program(expr).map_err(|e| e.to_string())?;
    let stmts = match program {
        Program::Script(stmts) => stmts,
        Program::Functions(_) => return Err("cannot evaluate a function definition".to_string()),
    };
    let stmt = stmts
        .first()
        .ok_or_else(|| "empty expression".to_string())?;
    match &stmt.kind {
        // Suppress the seam while evaluating: a watch/hover expression that calls
        // a function must not trip a breakpoint (which, mid-stop, would re-enter
        // the hook).
        StmtKind::Expr(e) => interp
            .eval_suppressed(|i| i.eval(e, expr))
            .map_err(|sig| format!("{sig:?}")),
        _ => Err("only expressions can be evaluated".to_string()),
    }
}

/// Render an array as a `(value, type)` pair for the variables / evaluate views:
/// scalars inline, strings quoted, everything else summarized by class + shape.
#[must_use]
pub fn summarize(arr: &Array) -> (String, String) {
    let ty = arr.class_name().to_string();
    if let Some(s) = arr.as_string() {
        return (format!("'{s}'"), "char".to_string());
    }
    if arr.as_cell().is_some() {
        return (format!("{} cell", dims_str(arr)), ty);
    }
    if arr.as_struct().is_some() {
        return (format!("{} struct", dims_str(arr)), ty);
    }
    if arr.numel() == 1 {
        // Flatten the formatter's (possibly padded/multi-line) scalar output to a
        // single token, e.g. "   42\n" → "42", "3 + 4i".
        let flat = arr
            .format(FormatMode::Short)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        return (flat, ty);
    }
    (format!("[{} {ty}]", dims_str(arr)), ty)
}

/// `"2x3"`-style shape string.
fn dims_str(arr: &Array) -> String {
    arr.dims()
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join("x")
}
