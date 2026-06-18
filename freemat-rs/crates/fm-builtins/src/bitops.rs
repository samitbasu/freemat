//! Bit-manipulation builtins: `bitand`, `bitor`, `bitxor`, `bitcmp`,
//! `bitshift`. Operands are treated as unsigned 64-bit integers (MATLAB
//! semantics: inputs are non-negative integers; the result class follows the
//! input integer class where applicable, else `double`).

use fm_core::{Array, DataClass};
use fm_interp::error::Flow;
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};

use crate::util::{err, need};

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("bitand", |_i, a, _n| binary(a, "bitand", |x, y| x & y));
    table.add_builtin("bitor", |_i, a, _n| binary(a, "bitor", |x, y| x | y));
    table.add_builtin("bitxor", |_i, a, _n| binary(a, "bitxor", |x, y| x ^ y));
    table.add_builtin("bitcmp", b_bitcmp);
    table.add_builtin("bitshift", b_bitshift);
}

/// Number of bits for a given integer class (for `bitcmp` complement and the
/// result class of bit ops).
fn class_bits(class: DataClass) -> u32 {
    match class {
        DataClass::Int8 | DataClass::UInt8 => 8,
        DataClass::Int16 | DataClass::UInt16 => 16,
        DataClass::Int32 | DataClass::UInt32 => 32,
        _ => 64,
    }
}

/// Result class for a bit op: prefer an integer class if either input is one,
/// otherwise `double` (FreeMat returns the input class).
fn result_class(a: &Array, b: &Array) -> DataClass {
    let ca = a.class();
    let cb = b.class();
    if ca.is_integer() {
        ca
    } else if cb.is_integer() {
        cb
    } else {
        DataClass::Double
    }
}

fn binary(args: &[Array], name: &str, op: impl Fn(u64, u64) -> u64) -> Flow<Vec<Array>> {
    need(args, 2, name)?;
    let x = to_f64_vec(&args[0]);
    let y = to_f64_vec(&args[1]);
    let (n, dims) = if x.len() >= y.len() {
        (x.len(), args[0].dims())
    } else {
        (y.len(), args[1].dims())
    };
    if x.len() != 1 && y.len() != 1 && x.len() != y.len() {
        return err(format!("{name}: array dimensions must agree"));
    }
    let cls = result_class(&args[0], &args[1]);
    let data: Vec<f64> = (0..n)
        .map(|i| {
            let xi = if x.len() == 1 { x[0] } else { x[i] } as u64;
            let yi = if y.len() == 1 { y[0] } else { y[i] } as u64;
            op(xi, yi) as f64
        })
        .collect();
    Ok(vec![build_real(cls, &dims, data)])
}

/// `bitcmp(A)` / `bitcmp(A, n)` — bitwise complement within `n` bits (default
/// the class width, 64 for `double`).
fn b_bitcmp(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "bitcmp")?;
    let cls = if args[0].class().is_integer() {
        args[0].class()
    } else {
        DataClass::Double
    };
    let bits = match args.get(1) {
        Some(b) => {
            if let Some(s) = b.as_string() {
                // 'uint8' etc. or a numeric string.
                match s.as_str() {
                    "uint8" | "int8" => 8,
                    "uint16" | "int16" => 16,
                    "uint32" | "int32" => 32,
                    "uint64" | "int64" => 64,
                    _ => class_bits(cls),
                }
            } else {
                b.as_f64().unwrap_or(class_bits(cls) as f64) as u32
            }
        }
        None => class_bits(cls),
    };
    let mask: u64 = if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    let x = to_f64_vec(&args[0]);
    let dims = args[0].dims();
    let data: Vec<f64> = x.iter().map(|&v| (!(v as u64) & mask) as f64).collect();
    Ok(vec![build_real(cls, &dims, data)])
}

/// `bitshift(A, k)` — left shift for `k > 0`, right shift (toward zero) for
/// `k < 0`. Result is masked to the input class width.
fn b_bitshift(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "bitshift")?;
    let cls = if args[0].class().is_integer() {
        args[0].class()
    } else {
        DataClass::Double
    };
    let bits = class_bits(cls);
    let mask: u64 = if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    let x = to_f64_vec(&args[0]);
    let k = to_f64_vec(&args[1]);
    let dims = args[0].dims();
    let n = x.len();
    let data: Vec<f64> = (0..n)
        .map(|i| {
            let xi = x[i] as u64;
            let ki = if k.len() == 1 { k[0] } else { k[i] } as i64;
            let shifted = if ki >= 0 {
                let s = ki.min(63) as u32;
                xi.wrapping_shl(s)
            } else {
                let s = (-ki).min(63) as u32;
                xi >> s
            };
            (shifted & mask) as f64
        })
        .collect();
    Ok(vec![build_real(cls, &dims, data)])
}
