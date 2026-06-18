//! Cell and struct builtins: `cell`, `struct`, `fieldnames`, `isfield`,
//! `rmfield`, `getfield`/`setfield`, `orderfields`, `cell2mat`, `num2cell`,
//! `struct2cell`/`cell2struct`, `cellfun`/`structfun`.

use fm_core::{Array, DataClass, StructArray};
use fm_interp::error::Flow;
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};
use fm_parser::Span;

use crate::util::{err, err_signal, need};

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("cell", b_cell);
    table.add_builtin("struct", b_struct);
    table.add_builtin("fieldnames", b_fieldnames);
    table.add_builtin("isfield", b_isfield);
    table.add_builtin("rmfield", b_rmfield);
    table.add_builtin("getfield", b_getfield);
    table.add_builtin("setfield", b_setfield);
    table.add_builtin("orderfields", b_orderfields);
    table.add_builtin("cell2mat", b_cell2mat);
    table.add_builtin("num2cell", b_num2cell);
    table.add_builtin("struct2cell", b_struct2cell);
    table.add_builtin("cell2struct", b_cell2struct);
    table.add_builtin("cellfun", b_cellfun);
    table.add_builtin("arrayfun", b_arrayfun);
    table.add_builtin("structfun", b_structfun);
}

/// `arrayfun(fn, A, ...)` — apply `fn` to each element of the array operand(s),
/// collecting the results. With `'UniformOutput', false` the results are packed
/// into a cell array; otherwise scalar results pack into a numeric/logical array
/// shaped like the input.
fn b_arrayfun(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "arrayfun")?;
    let callable = args[0].clone();
    let mut operands: Vec<Array> = Vec::new();
    let mut uniform = true;
    let mut idx = 1;
    while idx < args.len() {
        if args[idx].as_string().as_deref() == Some("UniformOutput") {
            uniform = args.get(idx + 1).and_then(Array::as_f64).unwrap_or(1.0) != 0.0;
            idx += 2;
        } else {
            operands.push(args[idx].clone());
            idx += 1;
        }
    }
    if operands.is_empty() {
        return err("arrayfun: at least one array operand is required");
    }
    let dims = operands[0].dims();
    let count = operands[0].numel();
    // Read each operand's elements as 1×1 arrays (preserving class), column-major.
    let elemwise: Vec<Vec<Array>> = operands.iter().map(split_elements).collect();
    let mut results: Vec<Array> = Vec::with_capacity(count);
    for k in 0..count {
        let call_args: Vec<Array> = elemwise
            .iter()
            .map(|o| o.get(k).cloned().unwrap_or_else(Array::empty))
            .collect();
        results.push(apply_callable(i, &callable, &call_args)?);
    }
    if uniform {
        let all_scalar = results.iter().all(|r| r.numel() == 1);
        if all_scalar {
            let any_bool =
                !results.is_empty() && results.iter().all(|r| r.class() == DataClass::Bool);
            let data: Vec<f64> = results.iter().map(|r| r.as_f64().unwrap_or(0.0)).collect();
            let class = if any_bool {
                DataClass::Bool
            } else {
                DataClass::Double
            };
            return Ok(vec![build_real(class, &dims, data)]);
        }
    }
    Ok(vec![Array::cell(&dims, results)])
}

/// Split an array into its elements as 1×1 arrays, column-major, preserving the
/// element class (so `arrayfun` passes typed scalars to the callable).
fn split_elements(a: &Array) -> Vec<Array> {
    let n = a.numel();
    (0..n)
        .map(|k| {
            let plan = fm_interp::index::plan_index(
                &a.dims(),
                &[fm_interp::index::IndexArg::Value(Array::double(
                    (k + 1) as f64,
                ))],
            );
            match plan {
                Ok(p) => fm_interp::index::gather(a, &p).unwrap_or_else(|_| Array::empty()),
                Err(_) => Array::empty(),
            }
        })
        .collect()
}

fn dims_arg(args: &[Array]) -> Vec<usize> {
    if args.is_empty() {
        return vec![0, 0];
    }
    if args.len() == 1 {
        if args[0].numel() == 1 {
            let n = args[0].as_f64().unwrap_or(0.0).max(0.0) as usize;
            return vec![n, n];
        }
        return to_f64_vec(&args[0])
            .into_iter()
            .map(|x| x.max(0.0) as usize)
            .collect();
    }
    args.iter()
        .map(|a| a.as_f64().unwrap_or(0.0).max(0.0) as usize)
        .collect()
}

/// `cell(n)` / `cell(m, n, ...)` — a cell array of empty matrices.
fn b_cell(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let mut dims = dims_arg(args);
    while dims.len() < 2 {
        dims.push(1);
    }
    let n: usize = dims.iter().product();
    Ok(vec![Array::cell(&dims, vec![Array::empty(); n])])
}

/// `struct('f1', v1, 'f2', v2, ...)` — build a scalar (or array) struct. If any
/// value is a cell array, the struct array takes that cell's shape.
fn b_struct(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return Ok(vec![Array::struct_array(StructArray::scalar([]))]);
    }
    if !args.len().is_multiple_of(2) {
        return err("struct: arguments must be field/value pairs");
    }
    // Determine the struct-array shape from any cell-valued argument.
    let mut dims = vec![1usize, 1];
    for pair in args.chunks(2) {
        if let Some(c) = pair[1].as_cell()
            && c.len() != 1
        {
            dims = pair[1].dims();
        }
    }
    let numel: usize = dims.iter().product();
    let mut fields: Vec<(String, Vec<Array>)> = Vec::new();
    for pair in args.chunks(2) {
        let name = pair[0]
            .as_string()
            .ok_or_else(|| err_signal("struct: field names must be strings"))?;
        let col: Vec<Array> = if pair[1].as_cell().is_some() {
            let flat: Vec<Array> = crate::util::cell_mem_order(&pair[1]);
            if flat.len() == 1 {
                vec![flat[0].clone(); numel]
            } else {
                flat
            }
        } else {
            vec![pair[1].clone(); numel]
        };
        fields.push((name, col));
    }
    Ok(vec![Array::struct_array(StructArray::from_fields(
        dims, fields,
    ))])
}

fn b_fieldnames(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "fieldnames")?;
    let s = args[0]
        .as_struct()
        .ok_or_else(|| err_signal("fieldnames: argument must be a struct"))?;
    let names = s.field_name_strings();
    let n = names.len();
    let data: Vec<Array> = names.iter().map(|x| Array::char_string(x)).collect();
    Ok(vec![Array::cell(&[n, 1], data)])
}

fn b_isfield(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "isfield")?;
    let Some(s) = args[0].as_struct() else {
        return Ok(vec![Array::bool(false)]);
    };
    // isfield(s, {'a','b'}) → logical vector.
    if let Some(cells) = args[1].as_cell() {
        let mask: Vec<bool> = cells
            .iter()
            .map(|c| c.as_string().is_some_and(|n| s.has_field(&n)))
            .collect();
        let dims = args[1].dims();
        return Ok(vec![Array::bool_matrix(&dims, mask)]);
    }
    let name = args[1].as_string().unwrap_or_default();
    Ok(vec![Array::bool(s.has_field(&name))])
}

fn b_rmfield(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "rmfield")?;
    let s = args[0]
        .as_struct()
        .ok_or_else(|| err_signal("rmfield: argument must be a struct"))?;
    let drop = args[1].as_string().unwrap_or_default();
    if !s.has_field(&drop) {
        return err(format!("rmfield: no field named '{drop}'"));
    }
    let fields: Vec<(String, Vec<Array>)> = s
        .field_pairs()
        .iter()
        .filter(|(n, _)| *n != drop)
        .cloned()
        .collect();
    Ok(vec![Array::struct_array(StructArray::from_fields(
        s.dims().to_vec(),
        fields,
    ))])
}

fn b_getfield(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "getfield")?;
    // `getfield(s, idx1, name1, idx2, ...)`: each trailing argument is either a
    // cell of subscripts `{i,j}` (paren-index the current value) or a field-name
    // string (`.name`). Walk them left to right.
    let mut cur = args[0].clone();
    for arg in &args[1..] {
        if let Some(cell) = arg.as_cell() {
            // Cell of subscripts → paren-index `cur` at those positions.
            let subs: Vec<Array> = crate::util::cell_mem_order(arg);
            let _ = cell;
            let plan = fm_interp::index::plan_index(
                &cur.dims(),
                &subs
                    .iter()
                    .map(|s| fm_interp::index::IndexArg::Value(s.clone()))
                    .collect::<Vec<_>>(),
            )?;
            cur = fm_interp::index::gather(&cur, &plan)?;
        } else {
            let name = arg.as_string().ok_or_else(|| {
                err_signal("getfield: expected a field name or cell of subscripts")
            })?;
            cur = fm_interp::index::field_read(&cur, &name)
                .map_err(|_| err_signal(format!("getfield: no field named '{name}'")))?;
        }
    }
    let _ = i;
    Ok(vec![cur])
}

fn b_setfield(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 3, "setfield")?;
    let name = args[1].as_string().unwrap_or_default();
    let value = args[2].clone();
    let mut pairs: Vec<(String, Array)> = match args[0].as_struct() {
        Some(s) => s
            .field_pairs()
            .iter()
            .filter(|(n, _)| *n != name)
            .map(|(n, v)| (n.clone(), v.first().cloned().unwrap_or_else(Array::empty)))
            .collect(),
        None => Vec::new(),
    };
    pairs.push((name, value));
    Ok(vec![Array::struct_array(StructArray::scalar(pairs))])
}

fn b_orderfields(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "orderfields")?;
    let s = args[0]
        .as_struct()
        .ok_or_else(|| err_signal("orderfields: argument must be a struct"))?;
    let mut fields: Vec<(String, Vec<Array>)> = s.field_pairs().to_vec();
    fields.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(vec![Array::struct_array(StructArray::from_fields(
        s.dims().to_vec(),
        fields,
    ))])
}

/// `cell2mat(C)` — concatenate the contents of a 2-D cell array into a matrix.
fn b_cell2mat(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "cell2mat")?;
    let Some(cells) = args[0].as_cell() else {
        // Passing a non-cell returns it unchanged (MATLAB tolerates this).
        return Ok(vec![args[0].clone()]);
    };
    let dims = args[0].dims();
    if dims.len() != 2 {
        return err("cell2mat: only 2-D cell arrays are supported");
    }
    let (r, c) = (dims[0], dims[1]);
    let flat: Vec<Array> = cells.iter().cloned().collect();
    // Reconstruct column-major access: build each row by horzcat, then vertcat.
    let get = |row: usize, col: usize| -> Array {
        // `cells` iterates logical (row-major); map (row,col) accordingly.
        flat[row * c + col].clone()
    };
    let mut row_arrays = Vec::with_capacity(r);
    for row in 0..r {
        let row_cells: Vec<Array> = (0..c).map(|col| get(row, col)).collect();
        row_arrays.push(i.concat_values(2, &row_cells)?);
    }
    Ok(vec![i.concat_values(1, &row_arrays)?])
}

/// `num2cell(A)` — wrap each element of `A` in its own cell.
fn b_num2cell(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "num2cell")?;
    let a = &args[0];
    let dims = a.dims();
    let v = to_f64_vec(a);
    let data: Vec<Array> = v
        .into_iter()
        .map(|x| build_real(a.class(), &[1, 1], vec![x]))
        .collect();
    Ok(vec![Array::cell(&dims, data)])
}

/// `struct2cell(s)` — a cell column of the scalar struct's field values.
fn b_struct2cell(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "struct2cell")?;
    let s = args[0]
        .as_struct()
        .ok_or_else(|| err_signal("struct2cell: argument must be a struct"))?;
    let data: Vec<Array> = s
        .field_pairs()
        .iter()
        .map(|(_, v)| v.first().cloned().unwrap_or_else(Array::empty))
        .collect();
    let n = data.len();
    Ok(vec![Array::cell(&[n, 1], data)])
}

/// `cell2struct(C, fields, dim)` — build a scalar struct from a cell of values
/// and a cell (or char-matrix) of field names.
fn b_cell2struct(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "cell2struct")?;
    if args[0].as_cell().is_none() {
        return err("cell2struct: first argument must be a cell array");
    }
    let names: Vec<String> = if args[1].as_cell().is_some() {
        crate::util::cell_mem_order(&args[1])
            .iter()
            .filter_map(Array::as_string)
            .collect()
    } else {
        vec![args[1].as_string().unwrap_or_default()]
    };
    let vals: Vec<Array> = crate::util::cell_mem_order(&args[0]);
    if names.len() != vals.len() {
        return err("cell2struct: number of fields must match the number of cells");
    }
    let pairs: Vec<(String, Array)> = names.into_iter().zip(vals).collect();
    Ok(vec![Array::struct_array(StructArray::scalar(pairs))])
}

/// Resolve a function-name / function-handle argument to a callable name.
fn func_name(a: &Array) -> Flow<String> {
    a.as_string()
        .ok_or_else(|| err_signal("expected a function name string"))
}

/// Apply a callable (a function-name string **or** a function handle) to `args`,
/// requesting one output. Shared by `cellfun` / `arrayfun` / `structfun`.
pub(crate) fn apply_callable(i: &mut Interpreter, callable: &Array, args: &[Array]) -> Flow<Array> {
    let out = if callable.is_function_handle() {
        i.invoke_handle(callable, args, 1, "", Span::empty(0))?
    } else {
        let name = func_name(callable)?;
        i.call_function(&name, args, 1, "", Span::empty(0))?
    };
    Ok(out.into_iter().next().unwrap_or_else(Array::empty))
}

/// `cellfun(fn, C, ...)` — apply `fn` to each cell, collecting the results.
fn b_cellfun(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "cellfun")?;
    let callable = args[0].clone();
    // Collect the cell operands (every trailing cell argument before options).
    let mut operands: Vec<Vec<Array>> = Vec::new();
    let mut uniform = true;
    let mut idx = 1;
    while idx < args.len() {
        if args[idx].as_cell().is_some() {
            operands.push(crate::util::cell_mem_order(&args[idx]));
            idx += 1;
        } else if args[idx].as_string().as_deref() == Some("UniformOutput") {
            uniform = args.get(idx + 1).and_then(Array::as_f64).unwrap_or(1.0) != 0.0;
            idx += 2;
        } else {
            idx += 1;
        }
    }
    if operands.is_empty() {
        return err("cellfun: at least one cell array operand is required");
    }
    let count = operands[0].len();
    let dims = args[1].dims();
    let mut results: Vec<Array> = Vec::with_capacity(count);
    for k in 0..count {
        let call_args: Vec<Array> = operands.iter().map(|o| o[k].clone()).collect();
        results.push(apply_callable(i, &callable, &call_args)?);
    }
    if uniform {
        // Pack scalar numeric/logical results into an array of the cell's shape.
        let all_scalar = results.iter().all(|r| r.numel() == 1);
        if all_scalar {
            let any_bool = results.iter().all(|r| r.class() == DataClass::Bool);
            let data: Vec<f64> = results.iter().map(|r| r.as_f64().unwrap_or(0.0)).collect();
            let class = if any_bool {
                DataClass::Bool
            } else {
                DataClass::Double
            };
            return Ok(vec![build_real(class, &dims, data)]);
        }
    }
    Ok(vec![Array::cell(&dims, results)])
}

/// `structfun(fn, s)` — apply `fn` to each field of a scalar struct.
fn b_structfun(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "structfun")?;
    let callable = args[0].clone();
    let s = args[1]
        .as_struct()
        .ok_or_else(|| err_signal("structfun: second argument must be a struct"))?;
    let pairs = s.field_pairs().to_vec();
    let mut results = Vec::with_capacity(pairs.len());
    for (_, v) in &pairs {
        let val = v.first().cloned().unwrap_or_else(Array::empty);
        results.push(apply_callable(i, &callable, std::slice::from_ref(&val))?);
    }
    let all_scalar = results.iter().all(|r| r.numel() == 1);
    if all_scalar {
        let data: Vec<f64> = results.iter().map(|r| r.as_f64().unwrap_or(0.0)).collect();
        let n = data.len();
        return Ok(vec![build_real(DataClass::Double, &[n, 1], data)]);
    }
    err("structfun: non-scalar results require UniformOutput=false (not supported)")
}
