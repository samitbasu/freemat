//! Low-level file I/O builtins: `fopen`/`fclose`/`fread`/`fwrite`/`fprintf`/
//! `fscanf`/`sscanf`/`fgetl`/`fgets`/`frewind`/`feof`/`fileread`/`exist`.
//!
//! Open files live in a process-wide table keyed by a small integer descriptor
//! (mirroring MATLAB's `fid`s; 0/1/2 reserved for stdin/stdout/stderr). The
//! interpreter doesn't carry per-instance file state, so the table is a
//! thread-local — fine for the single-threaded REPL and the per-test isolation
//! in the conformance harness.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

use fm_core::{Array, DataClass};
use fm_interp::error::{Flow, InterpError, Signal};
use fm_interp::value::build_real;

/// An open file with a cursor and an at-EOF flag (set after a read past end).
struct OpenFile {
    file: File,
    /// `true` once a read consumed fewer bytes than requested (MATLAB `feof`).
    eof: bool,
    /// Open for writing (vs read-only).
    writable: bool,
}

thread_local! {
    static FILES: RefCell<HashMap<i32, OpenFile>> = RefCell::new(HashMap::new());
    static NEXT_FID: RefCell<i32> = const { RefCell::new(3) };
}

fn ferr<T>(msg: impl Into<String>) -> Flow<T> {
    Err(Signal::Error(InterpError::msg(msg)))
}

fn str_of(a: &Array) -> String {
    a.as_string().unwrap_or_default()
}

/// `fopen(name [, mode])` → `fid` (or -1 on failure), plus an empty message.
pub fn fopen(args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return ferr("fopen: filename required");
    }
    let name = str_of(&args[0]);
    let mode = args.get(1).map(str_of).unwrap_or_else(|| "r".to_string());
    let mode = mode.trim_end_matches(['t', 'b']); // text/binary suffix is a no-op
    let mut opts = std::fs::OpenOptions::new();
    let writable;
    match mode {
        "r" | "r+" => {
            opts.read(true);
            writable = mode == "r+";
            if writable {
                opts.write(true);
            }
        }
        "w" | "w+" => {
            opts.write(true).create(true).truncate(true);
            opts.read(mode == "w+");
            writable = true;
        }
        "a" | "a+" => {
            opts.append(true).create(true);
            opts.read(mode == "a+");
            writable = true;
        }
        _ => {
            return Ok(vec![
                Array::double(-1.0),
                Array::char_string("invalid mode"),
            ]);
        }
    }
    match opts.open(&name) {
        Ok(file) => {
            let fid = NEXT_FID.with(|n| {
                let mut n = n.borrow_mut();
                let id = *n;
                *n += 1;
                id
            });
            FILES.with(|f| {
                f.borrow_mut().insert(
                    fid,
                    OpenFile {
                        file,
                        eof: false,
                        writable,
                    },
                );
            });
            Ok(vec![Array::double(f64::from(fid)), Array::char_string("")])
        }
        Err(e) => Ok(vec![
            Array::double(-1.0),
            Array::char_string(&e.to_string()),
        ]),
    }
}

/// `fclose(fid)` or `fclose('all')` → 0 on success, -1 on failure.
pub fn fclose(args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if let Some(s) = args.first().and_then(Array::as_string)
        && s == "all"
    {
        FILES.with(|f| f.borrow_mut().clear());
        return Ok(vec![Array::double(0.0)]);
    }
    let fid = fid_of(args, 0, "fclose")?;
    let removed = FILES.with(|f| f.borrow_mut().remove(&fid).is_some());
    Ok(vec![Array::double(if removed { 0.0 } else { -1.0 })])
}

fn fid_of(args: &[Array], i: usize, name: &str) -> Flow<i32> {
    match args.get(i).and_then(Array::as_f64) {
        Some(v) => Ok(v as i32),
        None => ferr(format!("{name}: expected a file id")),
    }
}

/// `fwrite(fid, data [, precision])` → number of elements written.
pub fn fwrite(args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let fid = fid_of(args, 0, "fwrite")?;
    if args.len() < 2 {
        return ferr("fwrite: data required");
    }
    let data = &args[1];
    let precision = args.get(2).map(str_of);
    let bytes = encode_for_write(data, precision.as_deref());
    let count = data.numel();
    FILES.with(|f| -> Flow<()> {
        let mut map = f.borrow_mut();
        let of = map
            .get_mut(&fid)
            .ok_or_else(|| Signal::Error(InterpError::msg("fwrite: invalid file id")))?;
        of.file
            .write_all(&bytes)
            .map_err(|e| Signal::Error(InterpError::msg(format!("fwrite: {e}"))))?;
        Ok(())
    })?;
    Ok(vec![Array::double(count as f64)])
}

/// Encode `data` to bytes for `fwrite`, honoring `precision` (the class name).
fn encode_for_write(data: &Array, precision: Option<&str>) -> Vec<u8> {
    use byteorder::{LittleEndian, WriteBytesExt};
    let class = match precision {
        Some(p) => class_of_precision(p).unwrap_or_else(|| data.class()),
        None => data.class(),
    };
    let vals = fm_interp::value::to_f64_vec(data);
    let mut out = Vec::new();
    for &v in &vals {
        match class {
            DataClass::Int8 => out.write_i8(v as i8).unwrap(),
            DataClass::UInt8 | DataClass::Char => out.push(v as u8),
            DataClass::Int16 => out.write_i16::<LittleEndian>(v as i16).unwrap(),
            DataClass::UInt16 => out.write_u16::<LittleEndian>(v as u16).unwrap(),
            DataClass::Int32 => out.write_i32::<LittleEndian>(v as i32).unwrap(),
            DataClass::UInt32 => out.write_u32::<LittleEndian>(v as u32).unwrap(),
            DataClass::Int64 => out.write_i64::<LittleEndian>(v as i64).unwrap(),
            DataClass::UInt64 => out.write_u64::<LittleEndian>(v as u64).unwrap(),
            DataClass::Float => out.write_f32::<LittleEndian>(v as f32).unwrap(),
            DataClass::Bool => out.push(u8::from(v != 0.0)),
            _ => out.write_f64::<LittleEndian>(v).unwrap(),
        }
    }
    out
}

fn class_of_precision(p: &str) -> Option<DataClass> {
    Some(match p {
        "int8" | "char" | "schar" => DataClass::Int8,
        "uint8" | "uchar" => DataClass::UInt8,
        "int16" | "short" => DataClass::Int16,
        "uint16" | "ushort" => DataClass::UInt16,
        "int32" | "int" => DataClass::Int32,
        "uint32" | "uint" => DataClass::UInt32,
        "int64" | "long" => DataClass::Int64,
        "uint64" | "ulong" => DataClass::UInt64,
        "single" | "float" | "float32" => DataClass::Float,
        "double" | "float64" => DataClass::Double,
        _ => return None,
    })
}

fn byte_size(class: DataClass) -> usize {
    match class {
        DataClass::Int8 | DataClass::UInt8 | DataClass::Char | DataClass::Bool => 1,
        DataClass::Int16 | DataClass::UInt16 => 2,
        DataClass::Int32 | DataClass::UInt32 | DataClass::Float => 4,
        _ => 8,
    }
}

/// `fread(fid [, count [, precision]])` → a column vector of the read values.
pub fn fread(args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let fid = fid_of(args, 0, "fread")?;
    let precision = args
        .get(2)
        .map(str_of)
        .unwrap_or_else(|| "uchar".to_string());
    let class = class_of_precision(&precision).unwrap_or(DataClass::Double);
    let elem_size = byte_size(class);
    let count = args.get(1).and_then(Array::as_f64);

    FILES.with(|f| -> Flow<Vec<Array>> {
        let mut map = f.borrow_mut();
        let of = map
            .get_mut(&fid)
            .ok_or_else(|| Signal::Error(InterpError::msg("fread: invalid file id")))?;
        let mut buf = Vec::new();
        match count {
            Some(c) if c.is_finite() => {
                let want = (c as usize) * elem_size;
                let mut chunk = vec![0u8; want];
                let got = read_fill(&mut of.file, &mut chunk);
                if got < want {
                    of.eof = true;
                }
                buf.extend_from_slice(&chunk[..got]);
            }
            _ => {
                of.file.read_to_end(&mut buf).ok();
                of.eof = true;
            }
        }
        let vals = decode_read(&buf, class);
        let dims = [vals.len(), 1];
        Ok(vec![build_read_result(class, &dims, vals)])
    })
}

fn read_fill(file: &mut File, buf: &mut [u8]) -> usize {
    let mut total = 0;
    while total < buf.len() {
        match file.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(_) => break,
        }
    }
    total
}

fn decode_read(bytes: &[u8], class: DataClass) -> Vec<f64> {
    use byteorder::{LittleEndian, ReadBytesExt};
    let mut cur = std::io::Cursor::new(bytes);
    let mut out = Vec::new();
    loop {
        let v: Option<f64> = match class {
            DataClass::Int8 => cur.read_i8().ok().map(f64::from),
            DataClass::UInt8 | DataClass::Char | DataClass::Bool => {
                cur.read_u8().ok().map(f64::from)
            }
            DataClass::Int16 => cur.read_i16::<LittleEndian>().ok().map(f64::from),
            DataClass::UInt16 => cur.read_u16::<LittleEndian>().ok().map(f64::from),
            DataClass::Int32 => cur.read_i32::<LittleEndian>().ok().map(f64::from),
            DataClass::UInt32 => cur.read_u32::<LittleEndian>().ok().map(f64::from),
            DataClass::Int64 => cur.read_i64::<LittleEndian>().ok().map(|v| v as f64),
            DataClass::UInt64 => cur.read_u64::<LittleEndian>().ok().map(|v| v as f64),
            DataClass::Float => cur.read_f32::<LittleEndian>().ok().map(f64::from),
            _ => cur.read_f64::<LittleEndian>().ok(),
        };
        match v {
            Some(x) => out.push(x),
            None => break,
        }
    }
    out
}

fn build_read_result(class: DataClass, dims: &[usize], vals: Vec<f64>) -> Array {
    // `fread` returns the values in the requested class (double for double/float
    // precisions; the integer classes preserve their type).
    let out_class = match class {
        DataClass::Char => DataClass::Char,
        DataClass::Bool => DataClass::Bool,
        c if c.is_integer() => c,
        _ => DataClass::Double,
    };
    build_real(out_class, dims, vals)
}

/// `frewind(fid)` — seek to the start of the file.
pub fn frewind(args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let fid = fid_of(args, 0, "frewind")?;
    FILES.with(|f| -> Flow<()> {
        let mut map = f.borrow_mut();
        if let Some(of) = map.get_mut(&fid) {
            of.file.seek(SeekFrom::Start(0)).ok();
            of.eof = false;
        }
        Ok(())
    })?;
    Ok(vec![])
}

/// `feof(fid)` — true once a read has hit end-of-file.
pub fn feof(args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let fid = fid_of(args, 0, "feof")?;
    let eof = FILES.with(|f| f.borrow().get(&fid).map(|of| of.eof).unwrap_or(true));
    Ok(vec![Array::bool(eof)])
}

/// Read one line; returns `(line, found)` where `found` is false at EOF.
fn read_line(fid: i32) -> Flow<(Option<String>, bool)> {
    FILES.with(|f| -> Flow<(Option<String>, bool)> {
        let mut map = f.borrow_mut();
        let of = map
            .get_mut(&fid)
            .ok_or_else(|| Signal::Error(InterpError::msg("fgetl: invalid file id")))?;
        let mut line = Vec::new();
        let mut byte = [0u8; 1];
        let mut any = false;
        loop {
            match of.file.read(&mut byte) {
                Ok(0) => {
                    of.eof = true;
                    break;
                }
                Ok(_) => {
                    any = true;
                    if byte[0] == b'\n' {
                        break;
                    }
                    line.push(byte[0]);
                }
                Err(_) => break,
            }
        }
        if !any {
            return Ok((None, false));
        }
        Ok((Some(String::from_utf8_lossy(&line).to_string()), true))
    })
}

/// `fgetl(fid)` — read a line without the trailing newline; -1 at EOF.
pub fn fgetl(args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let fid = fid_of(args, 0, "fgetl")?;
    let (line, found) = read_line(fid)?;
    Ok(vec![match (found, line) {
        (true, Some(s)) => Array::char_string(&strip_cr(&s)),
        _ => Array::double(-1.0),
    }])
}

/// `fgets(fid)` — like `fgetl` but keeps the trailing newline; -1 at EOF.
pub fn fgets(args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let fid = fid_of(args, 0, "fgets")?;
    let (line, found) = read_line(fid)?;
    Ok(vec![match (found, line) {
        (true, Some(s)) => Array::char_string(&format!("{}\n", strip_cr(&s))),
        _ => Array::double(-1.0),
    }])
}

fn strip_cr(s: &str) -> String {
    s.strip_suffix('\r').unwrap_or(s).to_string()
}

/// `fprintf` writing to a file id (the interpreter prefixes the formatted
/// string). When `fid` is 1/2 or omitted the output is returned to the caller to
/// emit on the console.
pub fn fprintf_to(fid: i32, text: &str) -> Flow<Option<String>> {
    if fid <= 2 {
        // stdout/stderr — let the interpreter emit it.
        return Ok(Some(text.to_string()));
    }
    FILES.with(|f| -> Flow<Option<String>> {
        let mut map = f.borrow_mut();
        let of = map
            .get_mut(&fid)
            .ok_or_else(|| Signal::Error(InterpError::msg("fprintf: invalid file id")))?;
        if !of.writable {
            return ferr("fprintf: file not open for writing");
        }
        of.file
            .write_all(text.as_bytes())
            .map_err(|e| Signal::Error(InterpError::msg(format!("fprintf: {e}"))))?;
        Ok(None)
    })
}
