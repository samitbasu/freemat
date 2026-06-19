//! Number-base conversion builtins: `dec2hex`, `hex2dec`, `dec2bin`,
//! `bin2dec`, `num2hex`, `hex2num`, and the FreeMat `int2bin`/`bin2int`.

use fm_core::{Array, DataClass};
use fm_interp::error::Flow;
use fm_interp::value::{build_real, char_matrix, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};

use crate::util::need;

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("dec2hex", b_dec2hex);
    table.add_builtin("hex2dec", b_hex2dec);
    table.add_builtin("dec2bin", b_dec2bin);
    table.add_builtin("bin2dec", b_bin2dec);
    table.add_builtin("num2hex", b_num2hex);
    table.add_builtin("hex2num", b_hex2num);
    table.add_builtin("int2bin", b_int2bin);
    table.add_builtin("bin2int", b_bin2int);
}

/// Build a char matrix (one row per string), right-justified/zero-padded to a
/// common width. Strings are assumed already padded to equal length.
fn char_rows(rows: &[String]) -> Array {
    if rows.len() == 1 {
        return Array::char_string(&rows[0]);
    }
    let width = rows.first().map_or(0, |s| s.chars().count());
    let m = rows.len();
    // Column-major: element (i,j) at index i + j*m.
    let mut data = vec![' '; m * width];
    for (i, row) in rows.iter().enumerate() {
        for (j, c) in row.chars().enumerate() {
            data[i + j * m] = c;
        }
    }
    char_matrix(&[m, width], data)
}

/// `dec2hex(D)` / `dec2hex(D, n)` — uppercase hex strings, zero-padded to at
/// least `n` (or the longest) digits.
fn b_dec2hex(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "dec2hex")?;
    let vals = to_f64_vec(&args[0]);
    let min_width = args.get(1).and_then(Array::as_f64).unwrap_or(0.0) as usize;
    let raw: Vec<String> = vals.iter().map(|&v| format!("{:X}", v as u64)).collect();
    let width = raw
        .iter()
        .map(String::len)
        .max()
        .unwrap_or(0)
        .max(min_width);
    let rows: Vec<String> = raw.iter().map(|s| format!("{s:0>width$}")).collect();
    Ok(vec![char_rows(&rows)])
}

/// `hex2dec(S)` — parse hex string(s) to double.
fn b_hex2dec(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "hex2dec")?;
    let rows = string_rows(&args[0]);
    let mut out = Vec::with_capacity(rows.len());
    for s in &rows {
        let t = s.trim();
        let v = u64::from_str_radix(t, 16)
            .map_err(|_| crate::util::err_signal(format!("hex2dec: invalid hex digit in '{t}'")))?;
        out.push(v as f64);
    }
    Ok(vec![column(out)])
}

/// `dec2bin(D)` / `dec2bin(D, n)` — binary strings zero-padded to `n`/longest.
fn b_dec2bin(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "dec2bin")?;
    let vals = to_f64_vec(&args[0]);
    let min_width = args.get(1).and_then(Array::as_f64).unwrap_or(0.0) as usize;
    let raw: Vec<String> = vals.iter().map(|&v| format!("{:b}", v as u64)).collect();
    let width = raw
        .iter()
        .map(String::len)
        .max()
        .unwrap_or(1)
        .max(min_width)
        .max(1);
    let rows: Vec<String> = raw.iter().map(|s| format!("{s:0>width$}")).collect();
    Ok(vec![char_rows(&rows)])
}

/// `bin2dec(S)` — parse binary string(s) to double.
fn b_bin2dec(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "bin2dec")?;
    let rows = string_rows(&args[0]);
    let mut out = Vec::with_capacity(rows.len());
    for s in &rows {
        let t: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        let v = u64::from_str_radix(&t, 2).map_err(|_| {
            crate::util::err_signal(format!("bin2dec: invalid binary digit in '{t}'"))
        })?;
        out.push(v as f64);
    }
    Ok(vec![column(out)])
}

/// `num2hex(X)` — the IEEE-754 hex representation of a double (16 hex digits).
fn b_num2hex(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "num2hex")?;
    let vals = to_f64_vec(&args[0]);
    let rows: Vec<String> = vals
        .iter()
        .map(|&v| format!("{:016x}", v.to_bits()))
        .collect();
    Ok(vec![char_rows(&rows)])
}

/// `hex2num(S)` — interpret a 16-digit IEEE-754 hex string as a double.
fn b_hex2num(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "hex2num")?;
    let rows = string_rows(&args[0]);
    let mut out = Vec::with_capacity(rows.len());
    for s in &rows {
        let mut t = s.trim().to_string();
        // Right-pad with zeros to 16 hex digits (MATLAB semantics).
        while t.len() < 16 {
            t.push('0');
        }
        let bits = u64::from_str_radix(&t, 16)
            .map_err(|_| crate::util::err_signal(format!("hex2num: invalid hex string '{t}'")))?;
        out.push(f64::from_bits(bits));
    }
    Ok(vec![column(out)])
}

/// `int2bin(A, n)` — FreeMat: each input element expands into `n` bits (MSB
/// first) laid out along a new trailing dimension (a trailing singleton dim is
/// reused). So a column vector `m x 1` becomes `m x n`, while an N-D array
/// `[d1..dk]` becomes `[d1..dk n]`. Bits use two's-complement for negatives.
fn b_int2bin(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "int2bin")?;
    let in_dims = args[0].dims();
    let vals = to_f64_vec(&args[0]);
    let nbits = (args[1].as_f64().unwrap_or(0.0).clamp(0.0, 64.0) as usize).max(1);
    let numel = vals.len();
    // The bits dimension is the (new) outermost dim, so its stride is `numel`.
    let mut data = vec![0.0f64; numel * nbits];
    for (e, &v) in vals.iter().enumerate() {
        let iv = v as i64 as u64;
        for b in 0..nbits {
            // column/plane b (MSB first) holds bit (nbits-1-b).
            let bit = (iv >> (nbits - 1 - b)) & 1;
            data[e + b * numel] = bit as f64;
        }
    }
    let mut out_dims: Vec<usize> = in_dims.to_vec();
    if out_dims.len() >= 2 && *out_dims.last().unwrap() == 1 {
        *out_dims.last_mut().unwrap() = nbits;
    } else {
        out_dims.push(nbits);
    }
    Ok(vec![build_real(DataClass::Double, &out_dims, data)])
}

/// `bin2int(B)` — inverse of `int2bin`: collapses the last non-singleton
/// dimension (the bits, MSB first) into a single integer, recovering the
/// original N-D shape.
fn b_bin2int(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "bin2int")?;
    let dims = args[0].dims();
    // Work dimension = last non-singleton dim (FreeMat's `lastNotOne`).
    let mut wd = 0usize;
    for (k, &d) in dims.iter().enumerate() {
        if d > 1 {
            wd = k;
        }
    }
    let nbits = dims.get(wd).copied().unwrap_or(1).max(1);
    let data = to_f64_vec(&args[0]);
    // The work dim is the outermost non-singleton, so its stride is `base`.
    let base = data.len() / nbits;
    let mut out = vec![0.0f64; base];
    for e in 0..base {
        let mut acc: u64 = 0;
        for b in 0..nbits {
            let bit = data[e + b * base] as i64 as u64 & 1;
            acc = (acc << 1) | bit;
        }
        out[e] = acc as f64;
    }
    let mut out_dims: Vec<usize> = dims.to_vec();
    if wd < out_dims.len() {
        out_dims[wd] = 1;
    }
    while out_dims.len() > 2 && *out_dims.last().unwrap() == 1 {
        out_dims.pop();
    }
    Ok(vec![build_real(DataClass::Double, &out_dims, out)])
}

/// Read an array as a list of row strings: a single char row vector -> one
/// string; a char matrix -> one string per row; a cell of strings -> per cell.
fn string_rows(a: &Array) -> Vec<String> {
    if let Some(cell) = a.as_cell() {
        return cell
            .iter()
            .map(|e| e.as_string().unwrap_or_default())
            .collect();
    }
    let dims = a.dims();
    if dims.len() == 2 && dims[0] > 1 && matches!(a.class(), DataClass::Char) {
        // Char matrix: each row is a string (column-major read).
        let m = dims[0];
        let cols = dims[1];
        let chars: Vec<char> = to_f64_vec(a)
            .iter()
            .map(|&c| char::from_u32(c as u32).unwrap_or(' '))
            .collect();
        let mut rows = Vec::with_capacity(m);
        for i in 0..m {
            let s: String = (0..cols).map(|j| chars[i + j * m]).collect();
            rows.push(s);
        }
        return rows;
    }
    vec![a.as_string().unwrap_or_default()]
}

fn column(data: Vec<f64>) -> Array {
    let n = data.len();
    if n == 1 {
        build_real(DataClass::Double, &[1, 1], data)
    } else {
        build_real(DataClass::Double, &[n, 1], data)
    }
}
