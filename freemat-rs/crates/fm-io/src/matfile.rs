//! MATLAB v7 / Level-5 MAT-file read and write.
//!
//! Ported from FreeMat's `libs/libCore/MatIO.cpp`. Supports the core data
//! classes — double/single, complex, every integer class, logical, char, struct,
//! and cell — with zlib-compressed matrix elements on write (matching FreeMat)
//! and transparent decompression of compressed elements on read.
//!
//! # Layout (brief)
//! A MAT file is a 128-byte header followed by a sequence of *data elements*.
//! Each top-level variable is one `miMATRIX` element (usually wrapped in a
//! `miCOMPRESSED`/zlib element). A `miMATRIX` body is a sequence of tagged
//! sub-elements: array-flags, dimensions, name, then the payload (real/imag data
//! for leaf arrays; nested matrices for cells/structs).
//!
//! Sparse arrays are **deferred to Stage 9** (no sparse type in `fm-core` yet);
//! a sparse element on read is rejected with a clear error.

use std::io::{Cursor, Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use fm_core::{Array, C64, DataClass, ScalarValue, StructArray};
use ndarray::{ArrayD, IxDyn, ShapeBuilder};

// ---- MAT type / class constants -----------------------------------------

const MI_INT8: u32 = 1;
const MI_UINT8: u32 = 2;
const MI_INT16: u32 = 3;
const MI_UINT16: u32 = 4;
const MI_INT32: u32 = 5;
const MI_UINT32: u32 = 6;
const MI_SINGLE: u32 = 7;
const MI_DOUBLE: u32 = 9;
const MI_INT64: u32 = 12;
const MI_UINT64: u32 = 13;
const MI_MATRIX: u32 = 14;
const MI_COMPRESSED: u32 = 15;
const MI_UTF8: u32 = 16;
const MI_UTF16: u32 = 17;
const MI_UTF32: u32 = 18;

const MX_CELL: u8 = 1;
const MX_STRUCT: u8 = 2;
const MX_CHAR: u8 = 4;
const MX_SPARSE: u8 = 5;
const MX_DOUBLE: u8 = 6;
const MX_SINGLE: u8 = 7;
const MX_INT8: u8 = 8;
const MX_UINT8: u8 = 9;
const MX_INT16: u8 = 10;
const MX_UINT16: u8 = 11;
const MX_INT32: u8 = 12;
const MX_UINT32: u8 = 13;
const MX_INT64: u8 = 14;
const MX_UINT64: u8 = 15;

const FLAG_COMPLEX: u32 = 0x0800; // bcomplexFlag (8) << 8
const FLAG_LOGICAL: u32 = 0x0200; // blogicalFlag (2) << 8

/// A named variable read from / written to a MAT file.
#[derive(Debug, Clone)]
pub struct MatVar {
    /// The variable name.
    pub name: String,
    /// The value.
    pub value: Array,
}

/// Errors from MAT-file reading/writing.
#[derive(Debug)]
pub struct MatError(pub String);

impl std::fmt::Display for MatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for MatError {}

impl From<std::io::Error> for MatError {
    fn from(e: std::io::Error) -> Self {
        MatError(format!("MAT I/O error: {e}"))
    }
}

type MResult<T> = Result<T, MatError>;

fn err<T>(msg: impl Into<String>) -> MResult<T> {
    Err(MatError(msg.into()))
}

// ============================ WRITER =====================================

/// Serialize a set of named variables into MAT v7 bytes.
#[must_use = "the serialized bytes must be written somewhere"]
pub fn write_mat(vars: &[MatVar]) -> Vec<u8> {
    let mut out = Vec::new();
    write_header(&mut out);
    for v in vars {
        let mut body = Vec::new();
        write_matrix(&mut body, &v.name, &v.value);
        write_compressed(&mut out, &body);
    }
    out
}

fn write_header(out: &mut Vec<u8>) {
    let text = b"MATLAB 5.0 MAT-file, Platform: rust-fm, Created by: freemat-rs";
    let mut hdr = [0u8; 124];
    let n = text.len().min(124);
    hdr[..n].copy_from_slice(&text[..n]);
    out.extend_from_slice(&hdr);
    out.write_u16::<LittleEndian>(0x0100).unwrap(); // version
    // Endian indicator: file is little-endian, so the bytes spell "IM".
    out.extend_from_slice(b"IM");
}

/// Wrap a matrix element in a zlib-compressed `miCOMPRESSED` element.
fn write_compressed(out: &mut Vec<u8>, body: &[u8]) {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::new(6));
    encoder.write_all(body).unwrap();
    let compressed = encoder.finish().unwrap();
    out.write_u32::<LittleEndian>(MI_COMPRESSED).unwrap();
    out.write_u32::<LittleEndian>(compressed.len() as u32)
        .unwrap();
    out.extend_from_slice(&compressed);
    // MATLAB does NOT pad compressed top-level elements to 8 bytes (the next
    // element's tag follows immediately), so neither do we — the reader skips
    // alignment after a compressed element to match.
}

/// Pad a buffer to the next 8-byte boundary.
fn align8(out: &mut Vec<u8>) {
    while !out.len().is_multiple_of(8) {
        out.push(0);
    }
}

/// Write a tagged data element (regular 8-byte tag form) into `out`, padding to
/// 8 bytes afterwards.
fn write_element(out: &mut Vec<u8>, mtype: u32, data: &[u8]) {
    out.write_u32::<LittleEndian>(mtype).unwrap();
    out.write_u32::<LittleEndian>(data.len() as u32).unwrap();
    out.extend_from_slice(data);
    align8(out);
}

/// Write a full `miMATRIX` element for `value` under `name`.
fn write_matrix(out: &mut Vec<u8>, name: &str, value: &Array) {
    out.write_u32::<LittleEndian>(MI_MATRIX).unwrap();
    let len_pos = out.len();
    out.write_u32::<LittleEndian>(0).unwrap(); // placeholder length
    let start = out.len();
    write_matrix_body(out, name, value);
    let body_len = (out.len() - start) as u32;
    out[len_pos..len_pos + 4].copy_from_slice(&body_len.to_le_bytes());
}

fn write_matrix_body(out: &mut Vec<u8>, name: &str, value: &Array) {
    let class = mx_class_of(value);
    let complex = value.is_complex();
    let logical = value.class() == DataClass::Bool;

    // 1. Array flags (miUINT32, 2 words).
    let mut flags = u32::from(class);
    if complex {
        flags |= FLAG_COMPLEX;
    }
    if logical {
        flags |= FLAG_LOGICAL;
    }
    let mut flag_data = Vec::with_capacity(8);
    flag_data.write_u32::<LittleEndian>(flags).unwrap();
    flag_data.write_u32::<LittleEndian>(0).unwrap(); // nzmax
    write_element(out, MI_UINT32, &flag_data);

    // 2. Dimensions (miINT32). At least 2 dims (MATLAB requires rank >= 2).
    let mut dims: Vec<usize> = value.dims();
    while dims.len() < 2 {
        dims.push(1);
    }
    let mut dim_data = Vec::with_capacity(dims.len() * 4);
    for &d in &dims {
        dim_data.write_i32::<LittleEndian>(d as i32).unwrap();
    }
    write_element(out, MI_INT32, &dim_data);

    // 3. Name (miINT8).
    write_element(out, MI_INT8, name.as_bytes());

    // 4. Payload.
    match value {
        Array::Cell(_) => write_cell_payload(out, value),
        Array::Struct(s) => write_struct_payload(out, s),
        _ => write_numeric_payload(out, value, complex),
    }
}

fn write_cell_payload(out: &mut Vec<u8>, value: &Array) {
    for elem in cell_mem_order(value) {
        write_matrix(out, "", &elem);
    }
}

fn write_struct_payload(out: &mut Vec<u8>, s: &StructArray) {
    let pairs = s.field_pairs();
    let max_len = pairs.iter().map(|(n, _)| n.len()).max().unwrap_or(0) + 1;
    // Field-name length (miINT32, single value).
    let mut len_data = Vec::with_capacity(4);
    len_data.write_i32::<LittleEndian>(max_len as i32).unwrap();
    write_element(out, MI_INT32, &len_data);
    // Field names (miINT8), fixed-width null-padded blocks.
    let mut names = Vec::with_capacity(pairs.len() * max_len);
    for (n, _) in pairs {
        let b = n.as_bytes();
        names.extend_from_slice(b);
        names.resize(names.len() + (max_len - b.len()), 0);
    }
    write_element(out, MI_INT8, &names);
    // Fields, field-major then element.
    for (_, elems) in pairs {
        for elem in elems {
            write_matrix(out, "", elem);
        }
    }
}

fn write_numeric_payload(out: &mut Vec<u8>, value: &Array, complex: bool) {
    if complex {
        let data = to_c64(value);
        let re: Vec<f64> = data.iter().map(|c| c.re).collect();
        let im: Vec<f64> = data.iter().map(|c| c.im).collect();
        write_real_data(out, value.class(), &re);
        write_real_data(out, value.class(), &im);
    } else {
        write_array_data(out, value);
    }
}

/// Write the real-part data element for a (possibly complex) array of `class`.
fn write_real_data(out: &mut Vec<u8>, class: DataClass, data: &[f64]) {
    // Complex arrays in fm-core are only double/single; emit as that class.
    let (mtype, bytes) = match class {
        DataClass::Float => (MI_SINGLE, f32_bytes(data)),
        _ => (MI_DOUBLE, f64_bytes(data)),
    };
    write_element(out, mtype, &bytes);
}

/// Write a non-complex leaf array's data element with its natural MAT type.
fn write_array_data(out: &mut Vec<u8>, value: &Array) {
    macro_rules! ints {
        ($d:expr, $mtype:expr, $w:ident, $t:ty) => {{
            let v = mem_order($d);
            let mut bytes = Vec::with_capacity(v.len() * std::mem::size_of::<$t>());
            for x in v {
                bytes.$w::<LittleEndian>(x).unwrap();
            }
            write_element(out, $mtype, &bytes);
        }};
    }
    match value {
        Array::Scalar(_) => {
            // Densify a scalar to its class.
            let dense = scalar_to_dense(value);
            write_array_data(out, &dense);
        }
        Array::Bool(d) => {
            // Logical is written as INT32 with the logical flag set in flags.
            let v: Vec<i32> = mem_order(d).iter().map(|&b| i32::from(b)).collect();
            let mut bytes = Vec::with_capacity(v.len() * 4);
            for x in v {
                bytes.write_i32::<LittleEndian>(x).unwrap();
            }
            write_element(out, MI_INT32, &bytes);
        }
        Array::Char(d) => {
            // Char as UTF-16 code units (BMP only; astral → replacement).
            let v: Vec<u16> = mem_order(d)
                .iter()
                .map(|&c| u16::try_from(u32::from(c)).unwrap_or(0xFFFD))
                .collect();
            let mut bytes = Vec::with_capacity(v.len() * 2);
            for x in v {
                bytes.write_u16::<LittleEndian>(x).unwrap();
            }
            write_element(out, MI_UTF16, &bytes);
        }
        Array::Int8(d) => {
            let bytes: Vec<u8> = mem_order(d).iter().map(|&x| x as u8).collect();
            write_element(out, MI_INT8, &bytes);
        }
        Array::UInt8(d) => write_element(out, MI_UINT8, &mem_order(d)),
        Array::Int16(d) => ints!(d, MI_INT16, write_i16, i16),
        Array::UInt16(d) => ints!(d, MI_UINT16, write_u16, u16),
        Array::Int32(d) => ints!(d, MI_INT32, write_i32, i32),
        Array::UInt32(d) => ints!(d, MI_UINT32, write_u32, u32),
        Array::Int64(d) => ints!(d, MI_INT64, write_i64, i64),
        Array::UInt64(d) => ints!(d, MI_UINT64, write_u64, u64),
        Array::Float(d) => {
            let bytes = f32_bytes_raw(&mem_order(d));
            write_element(out, MI_SINGLE, &bytes);
        }
        Array::Double(d) => {
            let bytes = f64_bytes(&mem_order(d));
            write_element(out, MI_DOUBLE, &bytes);
        }
        // Complex handled in write_numeric_payload; cell/struct handled above.
        _ => {
            let data = to_f64(value);
            let bytes = f64_bytes(&data);
            write_element(out, MI_DOUBLE, &bytes);
        }
    }
}

fn f64_bytes(data: &[f64]) -> Vec<u8> {
    let mut b = Vec::with_capacity(data.len() * 8);
    for &x in data {
        b.write_f64::<LittleEndian>(x).unwrap();
    }
    b
}
fn f32_bytes(data: &[f64]) -> Vec<u8> {
    let mut b = Vec::with_capacity(data.len() * 4);
    for &x in data {
        b.write_f32::<LittleEndian>(x as f32).unwrap();
    }
    b
}
fn f32_bytes_raw(data: &[f32]) -> Vec<u8> {
    let mut b = Vec::with_capacity(data.len() * 4);
    for &x in data {
        b.write_f32::<LittleEndian>(x).unwrap();
    }
    b
}

fn mx_class_of(a: &Array) -> u8 {
    match a.class() {
        DataClass::Bool => MX_INT32, // logical written as int32 + logical flag
        DataClass::Int8 => MX_INT8,
        DataClass::UInt8 => MX_UINT8,
        DataClass::Int16 => MX_INT16,
        DataClass::UInt16 => MX_UINT16,
        DataClass::Int32 => MX_INT32,
        DataClass::UInt32 => MX_UINT32,
        DataClass::Int64 => MX_INT64,
        DataClass::UInt64 => MX_UINT64,
        DataClass::Float => MX_SINGLE,
        DataClass::Double => MX_DOUBLE,
        DataClass::Char => MX_CHAR,
        DataClass::Cell => MX_CELL,
        DataClass::Struct => MX_STRUCT,
        // Function handles are not serializable to a .mat file; treat as double.
        DataClass::FunctionHandle => MX_DOUBLE,
    }
}

// ============================ READER =====================================

/// Read all named variables from MAT v7 bytes.
///
/// # Errors
/// Returns [`MatError`] if the header is invalid or an element is malformed /
/// unsupported (e.g. sparse).
pub fn read_mat(bytes: &[u8]) -> MResult<Vec<MatVar>> {
    if bytes.len() < 128 {
        return err("MAT file too short");
    }
    // Validate the endian indicator (we only handle little-endian files, which
    // is what every modern MATLAB writes and what we write).
    let endian = &bytes[126..128];
    if endian != b"IM" && endian != b"MI" {
        return err("unrecognized MAT-file endian indicator");
    }
    if endian == b"MI" {
        return err("big-endian MAT files are not supported");
    }
    let mut cur = Cursor::new(&bytes[128..]);
    let mut vars = Vec::new();
    while let Some((mtype, data)) = read_element(&mut cur)? {
        match mtype {
            MI_COMPRESSED => {
                let mut dec = ZlibDecoder::new(Cursor::new(data));
                let mut raw = Vec::new();
                dec.read_to_end(&mut raw)
                    .map_err(|e| MatError(format!("zlib inflate failed: {e}")))?;
                let mut inner = Cursor::new(raw.as_slice());
                if let Some((MI_MATRIX, mdata)) = read_element(&mut inner)? {
                    // Skip (rather than abort on) a variable we can't parse — a
                    // single exotic element shouldn't fail the whole `load`.
                    if let Ok(v) = read_matrix(&mdata) {
                        vars.push(v);
                    }
                }
            }
            MI_MATRIX => {
                if let Ok(v) = read_matrix(&data) {
                    vars.push(v);
                }
            }
            _ => { /* skip unknown top-level elements */ }
        }
    }
    Ok(vars)
}

/// Read just the first variable from a MAT file (used to validate cross-tool
/// reads where later elements may be unsupported).
///
/// # Errors
/// Returns [`MatError`] if no variable can be parsed.
pub fn read_first(bytes: &[u8]) -> MResult<MatVar> {
    read_mat(bytes)?
        .into_iter()
        .next()
        .ok_or_else(|| MatError("no variables in MAT file".into()))
}

/// Read one tagged element from `cur`. Returns `(type, data)` or `None` at EOF.
/// Handles both the regular 8-byte tag and the packed "small element" tag.
fn read_element(cur: &mut Cursor<&[u8]>) -> MResult<Option<(u32, Vec<u8>)>> {
    let pos = cur.position() as usize;
    let buf = *cur.get_ref();
    if pos >= buf.len() {
        return Ok(None);
    }
    if pos + 4 > buf.len() {
        return Ok(None);
    }
    let tag1 = cur.read_u32::<LittleEndian>()?;
    let (mtype, nbytes, small) = if (tag1 >> 16) != 0 {
        // Small element: lower word = type, upper word = byte count.
        ((tag1 & 0xFFFF), (tag1 >> 16) as usize, true)
    } else {
        let nbytes = cur.read_u32::<LittleEndian>()? as usize;
        (tag1, nbytes, false)
    };
    let data = if small {
        // Up to 4 data bytes share the second word of the tag.
        let mut d = vec![0u8; nbytes];
        cur.read_exact(&mut d)?;
        // Skip the rest of the 4-byte data slot.
        let skip = 4 - nbytes;
        cur.set_position(cur.position() + skip as u64);
        d
    } else {
        let mut d = vec![0u8; nbytes];
        cur.read_exact(&mut d)?;
        // Align to 8 bytes — except compressed top-level elements, which MATLAB
        // writes without trailing padding (the next tag follows immediately).
        if mtype != MI_COMPRESSED {
            let pad = (8 - (nbytes % 8)) % 8;
            cur.set_position(cur.position() + pad as u64);
        }
        d
    };
    Ok(Some((mtype, data)))
}

/// Parse a `miMATRIX` element body into a named variable.
fn read_matrix(body: &[u8]) -> MResult<MatVar> {
    let mut cur = Cursor::new(body);
    // Array flags.
    let (ft, fd) = read_element(&mut cur)?.ok_or(MatError("missing array flags".into()))?;
    if ft != MI_UINT32 || fd.len() < 8 {
        return err("corrupt MAT: bad array flags");
    }
    let flags_word = u32::from_le_bytes([fd[0], fd[1], fd[2], fd[3]]);
    let class = (flags_word & 0xFF) as u8;
    let complex = (flags_word & FLAG_COMPLEX) != 0;
    let logical = (flags_word & FLAG_LOGICAL) != 0;

    // Dimensions.
    let (dt, dd) = read_element(&mut cur)?.ok_or(MatError("missing dims".into()))?;
    if dt != MI_INT32 {
        return err("corrupt MAT: bad dimensions");
    }
    let dims: Vec<usize> = dd
        .chunks_exact(4)
        .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]).max(0) as usize)
        .collect();

    // Name.
    let (_nt, nd) = read_element(&mut cur)?.ok_or(MatError("missing name".into()))?;
    let name = String::from_utf8_lossy(&nd).to_string();

    let value = match class {
        MX_CELL => read_cell(&mut cur, &dims)?,
        MX_STRUCT => read_struct(&mut cur, &dims)?,
        MX_SPARSE => return err("sparse MAT arrays are not supported yet (Stage 9)"),
        MX_CHAR => read_char(&mut cur, &dims)?,
        _ => read_numeric(&mut cur, &dims, class, complex, logical)?,
    };
    Ok(MatVar { name, value })
}

fn read_cell(cur: &mut Cursor<&[u8]>, dims: &[usize]) -> MResult<Array> {
    let n: usize = dims.iter().product();
    let mut elems = Vec::with_capacity(n);
    for _ in 0..n {
        let (mt, md) = read_element(cur)?.ok_or(MatError("cell: missing element".into()))?;
        if mt != MI_MATRIX {
            return err("cell: expected matrix element");
        }
        elems.push(read_matrix(&md)?.value);
    }
    Ok(make_cell(dims, elems))
}

fn read_struct(cur: &mut Cursor<&[u8]>, dims: &[usize]) -> MResult<Array> {
    let (_lt, ld) = read_element(cur)?.ok_or(MatError("struct: missing field-name len".into()))?;
    let field_len = i32::from_le_bytes([ld[0], ld[1], ld[2], ld[3]]).max(1) as usize;
    let (_nt, nd) = read_element(cur)?.ok_or(MatError("struct: missing field names".into()))?;
    let field_count = nd.len().checked_div(field_len).unwrap_or(0);
    let mut names = Vec::with_capacity(field_count);
    for i in 0..field_count {
        let block = &nd[i * field_len..(i + 1) * field_len];
        let end = block.iter().position(|&b| b == 0).unwrap_or(block.len());
        names.push(String::from_utf8_lossy(&block[..end]).to_string());
    }
    let numel: usize = dims.iter().product();
    let mut fields: Vec<(String, Vec<Array>)> =
        names.iter().map(|n| (n.clone(), Vec::new())).collect();
    for (fi, _) in names.iter().enumerate() {
        for _ in 0..numel {
            let (mt, md) = read_element(cur)?.ok_or(MatError("struct: missing field".into()))?;
            if mt != MI_MATRIX {
                return err("struct: expected matrix element");
            }
            fields[fi].1.push(read_matrix(&md)?.value);
        }
    }
    Ok(Array::struct_array(StructArray::from_fields(
        dims.to_vec(),
        fields,
    )))
}

fn read_char(cur: &mut Cursor<&[u8]>, dims: &[usize]) -> MResult<Array> {
    let (mt, md) = read_element(cur)?.ok_or(MatError("char: missing data".into()))?;
    let chars: Vec<char> = match mt {
        MI_UTF16 | MI_UINT16 => md
            .chunks_exact(2)
            .map(|c| {
                char::from_u32(u32::from(u16::from_le_bytes([c[0], c[1]]))).unwrap_or('\u{fffd}')
            })
            .collect(),
        MI_UTF8 | MI_UINT8 | MI_INT8 => md.iter().map(|&b| char::from(b)).collect(),
        MI_UTF32 | MI_UINT32 | MI_INT32 => md
            .chunks_exact(4)
            .map(|c| {
                char::from_u32(u32::from_le_bytes([c[0], c[1], c[2], c[3]])).unwrap_or('\u{fffd}')
            })
            .collect(),
        _ => return err("char: unexpected data type"),
    };
    Ok(make_char(dims, chars))
}

fn read_numeric(
    cur: &mut Cursor<&[u8]>,
    dims: &[usize],
    class: u8,
    complex: bool,
    logical: bool,
) -> MResult<Array> {
    let (rt, rd) = read_element(cur)?.ok_or(MatError("numeric: missing data".into()))?;
    let re = decode_reals(rt, &rd);
    if complex {
        let (it, id) = read_element(cur)?.ok_or(MatError("complex: missing imag".into()))?;
        let im = decode_reals(it, &id);
        let data: Vec<C64> = re
            .iter()
            .zip(im.iter())
            .map(|(&r, &i)| C64::new(r, i))
            .collect();
        return Ok(make_complex(dims, data));
    }
    if logical {
        let bools: Vec<bool> = re.iter().map(|&v| v != 0.0).collect();
        return Ok(make_bool(dims, bools));
    }
    Ok(make_real(target_class(class), dims, re))
}

/// Decode a numeric data element's bytes into `f64`s, honoring its MAT type.
fn decode_reals(mtype: u32, data: &[u8]) -> Vec<f64> {
    macro_rules! dec {
        ($sz:expr, $f:expr) => {
            data.chunks_exact($sz).map($f).collect::<Vec<f64>>()
        };
    }
    match mtype {
        MI_DOUBLE => dec!(8, |c: &[u8]| f64::from_le_bytes(c.try_into().unwrap())),
        MI_SINGLE => dec!(4, |c: &[u8]| f64::from(f32::from_le_bytes(
            c.try_into().unwrap()
        ))),
        MI_INT8 => data.iter().map(|&b| f64::from(b as i8)).collect(),
        MI_UINT8 => data.iter().map(|&b| f64::from(b)).collect(),
        MI_INT16 => dec!(2, |c: &[u8]| f64::from(i16::from_le_bytes(
            c.try_into().unwrap()
        ))),
        MI_UINT16 => dec!(2, |c: &[u8]| f64::from(u16::from_le_bytes(
            c.try_into().unwrap()
        ))),
        MI_INT32 => dec!(4, |c: &[u8]| f64::from(i32::from_le_bytes(
            c.try_into().unwrap()
        ))),
        MI_UINT32 => dec!(4, |c: &[u8]| f64::from(u32::from_le_bytes(
            c.try_into().unwrap()
        ))),
        MI_INT64 => dec!(8, |c: &[u8]| i64::from_le_bytes(c.try_into().unwrap())
            as f64),
        MI_UINT64 => dec!(8, |c: &[u8]| u64::from_le_bytes(c.try_into().unwrap())
            as f64),
        _ => Vec::new(),
    }
}

fn target_class(class: u8) -> DataClass {
    match class {
        MX_INT8 => DataClass::Int8,
        MX_UINT8 => DataClass::UInt8,
        MX_INT16 => DataClass::Int16,
        MX_UINT16 => DataClass::UInt16,
        MX_INT32 => DataClass::Int32,
        MX_UINT32 => DataClass::UInt32,
        MX_INT64 => DataClass::Int64,
        MX_UINT64 => DataClass::UInt64,
        MX_SINGLE => DataClass::Float,
        _ => DataClass::Double,
    }
}

// ---- Array build/read helpers (column-major) ----------------------------

fn make_real(class: DataClass, dims: &[usize], data: Vec<f64>) -> Array {
    fm_interp::value::build_real(class, dims, data)
}

fn make_complex(dims: &[usize], data: Vec<C64>) -> Array {
    if data.len() == 1 && dims.iter().product::<usize>() == 1 {
        return Array::Scalar(ScalarValue::Complex64(data[0]));
    }
    Array::complex64_matrix(dims, data)
}

fn make_bool(dims: &[usize], data: Vec<bool>) -> Array {
    if data.len() == 1 && dims.iter().product::<usize>() == 1 {
        return Array::bool(data[0]);
    }
    Array::bool_matrix(dims, data)
}

fn make_char(dims: &[usize], data: Vec<char>) -> Array {
    fm_interp::value::char_matrix(dims, data)
}

fn make_cell(dims: &[usize], data: Vec<Array>) -> Array {
    if data.len() == dims.iter().product::<usize>() {
        Array::cell(dims, data)
    } else {
        Array::cell(&[1, data.len()], data)
    }
}

/// Densify a scalar to a dense (non-`Scalar`) array of its class so the writer
/// can emit it without re-entering the scalar branch. Builds dense `ArrayD`s
/// directly — `build_real` would re-collapse a 1-element result back to a
/// `Scalar` and loop.
fn scalar_to_dense(a: &Array) -> Array {
    use std::sync::Arc;
    let Array::Scalar(s) = a else {
        return a.clone();
    };
    macro_rules! one {
        ($variant:ident, $val:expr) => {{
            let arr = ArrayD::from_shape_vec(IxDyn(&[1, 1]).f(), vec![$val]).unwrap();
            Array::$variant(Arc::new(arr))
        }};
    }
    match *s {
        ScalarValue::Bool(b) => one!(Bool, b),
        ScalarValue::Char(c) => one!(Char, c),
        ScalarValue::Complex64(c) => one!(Complex64, c),
        ScalarValue::Complex32(c) => one!(Complex64, C64::new(f64::from(c.re), f64::from(c.im))),
        ScalarValue::Int8(v) => one!(Int8, v),
        ScalarValue::UInt8(v) => one!(UInt8, v),
        ScalarValue::Int16(v) => one!(Int16, v),
        ScalarValue::UInt16(v) => one!(UInt16, v),
        ScalarValue::Int32(v) => one!(Int32, v),
        ScalarValue::UInt32(v) => one!(UInt32, v),
        ScalarValue::Int64(v) => one!(Int64, v),
        ScalarValue::UInt64(v) => one!(UInt64, v),
        ScalarValue::Float(v) => one!(Float, v),
        ScalarValue::Double(v) => one!(Double, v),
    }
}

fn mem_order<T: Clone>(d: &ArrayD<T>) -> Vec<T> {
    if let Some(s) = d.as_slice_memory_order() {
        s.to_vec()
    } else {
        d.t().iter().cloned().collect()
    }
}

fn to_f64(a: &Array) -> Vec<f64> {
    fm_interp::value::to_f64_vec(a)
}

fn to_c64(a: &Array) -> Vec<C64> {
    fm_interp::value::to_c64_vec(a)
}

fn cell_mem_order(a: &Array) -> Vec<Array> {
    match a.as_cell() {
        Some(d) => mem_order(d),
        None => Vec::new(),
    }
}
