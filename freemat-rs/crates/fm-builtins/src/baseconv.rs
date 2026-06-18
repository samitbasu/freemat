//! Number-base conversion builtins: `dec2hex`, `hex2dec`, `dec2bin`,
//! `bin2dec`, `num2hex`, `hex2num`, and the FreeMat `int2bin`/`bin2int`.

use fm_core::{Array, DataClass};
use fm_interp::error::Flow;
use fm_interp::value::{build_real, char_matrix, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};

use crate::util::{err, need};

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

/// `int2bin(A, n)` — FreeMat: returns an `n`-wide bit matrix where each input
/// element becomes a row of bits (MSB first), stacked along the last dim. We
/// implement the common 2-D case: rows of input map to rows of an `numel x n`
/// double matrix of 0/1.
fn b_int2bin(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "int2bin")?;
    let vals = to_f64_vec(&args[0]);
    let nbits = args[1].as_f64().unwrap_or(0.0) as usize;
    let m = vals.len();
    // Result is m x nbits double, MSB first. Column-major layout.
    let mut data = vec![0.0f64; m * nbits];
    for (i, &v) in vals.iter().enumerate() {
        let iv = v as u64;
        for b in 0..nbits {
            // bit (nbits-1-b) is column b (MSB first).
            let bit = (iv >> (nbits - 1 - b)) & 1;
            data[i + b * m] = bit as f64;
        }
    }
    if m == 1 {
        Ok(vec![build_real(DataClass::Double, &[1, nbits], data)])
    } else {
        Ok(vec![build_real(DataClass::Double, &[m, nbits], data)])
    }
}

/// `bin2int(B)` — inverse of `int2bin`: each row of the `m x nbits` bit matrix
/// (MSB first) becomes an integer.
fn b_bin2int(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "bin2int")?;
    let dims = args[0].dims();
    if dims.len() != 2 {
        return err("bin2int: input must be a 2-D bit matrix");
    }
    let (m, nbits) = (dims[0], dims[1]);
    let data = to_f64_vec(&args[0]);
    let mut out = vec![0.0f64; m];
    for i in 0..m {
        let mut acc: u64 = 0;
        for b in 0..nbits {
            let bit = data[i + b * m] as u64 & 1;
            acc = (acc << 1) | bit;
        }
        out[i] = acc as f64;
    }
    if m == 1 {
        Ok(vec![build_real(DataClass::Double, &[1, 1], out)])
    } else {
        Ok(vec![build_real(DataClass::Double, &[m, 1], out)])
    }
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
