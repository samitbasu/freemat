//! Registration of the `fm-io` builtins into the interpreter's function table:
//! MAT `save`/`load`, file I/O, FFT, and regex.

use std::path::Path;

use fm_core::{Array, StructArray};
use fm_interp::error::{Flow, InterpError, Signal};
use fm_interp::{FunctionTable, Interpreter};
use fm_parser::Span;

use crate::matfile::{MatVar, read_mat, write_mat};
use crate::{fft, fileio, osio, regexp, scanf, wav};

/// Register every `fm-io` builtin into `table`.
pub fn register(table: &mut FunctionTable) {
    // MAT persistence.
    table.add_builtin("save", b_save);
    table.add_builtin("load", b_load);
    // File I/O.
    table.add_builtin("fopen", |_i, a, n| fileio::fopen(a, n));
    table.add_builtin("fclose", |_i, a, n| fileio::fclose(a, n));
    table.add_builtin("fwrite", |_i, a, n| fileio::fwrite(a, n));
    table.add_builtin("fread", |_i, a, n| fileio::fread(a, n));
    table.add_builtin("frewind", |_i, a, n| fileio::frewind(a, n));
    table.add_builtin("fseek", |_i, a, n| fileio::fseek(a, n));
    table.add_builtin("ftell", |_i, a, n| fileio::ftell(a, n));
    table.add_builtin("feof", |_i, a, n| fileio::feof(a, n));
    table.add_builtin("fgetl", |_i, a, n| fileio::fgetl(a, n));
    table.add_builtin("fgets", |_i, a, n| fileio::fgets(a, n));
    table.add_builtin("fgetline", |_i, a, n| fileio::fgetl(a, n));
    table.add_builtin("fflush", |_i, a, n| fileio::fflush(a, n));
    table.add_builtin("getline", |_i, a, _n| fileio::getline(a));
    table.add_builtin("rawread", |_i, a, n| fileio::rawread(a, n));
    table.add_builtin("rawwrite", |_i, a, n| fileio::rawwrite(a, n));
    table.add_builtin("fprintf", b_fprintf);
    table.add_builtin("fscanf", b_fscanf);
    table.add_builtin("sscanf", b_sscanf);
    table.add_builtin("fileread", b_fileread);
    table.add_builtin("dlmread", b_dlmread);
    table.add_builtin("dlmwrite", b_dlmwrite);
    table.add_builtin("csvread", b_csvread);
    table.add_builtin("csvwrite", b_csvwrite);
    table.add_builtin("exist", b_exist);
    // Filesystem.
    table.add_builtin("pwd", b_pwd);
    table.add_builtin("cd", b_cd);
    table.add_builtin("dir", b_dir);
    table.add_builtin("ls", b_ls);
    table.add_builtin("which", b_which);
    table.add_builtin("type", b_type);
    table.add_builtin("mkdir", b_mkdir);
    table.add_builtin("rmdir", b_rmdir);
    table.add_builtin("copyfile", b_copyfile);
    table.add_builtin("fileattrib", b_fileattrib);
    table.add_builtin("exit", b_exit);
    // Path / separator builtins.
    table.add_builtin("filesep", b_filesep);
    table.add_builtin("dirsep", b_filesep);
    table.add_builtin("pathsep", b_pathsep);
    table.add_builtin("path", b_path);
    table.add_builtin("addpath", b_addpath);
    table.add_builtin("getpath", b_getpath);
    table.add_builtin("setpath", b_setpath);
    table.add_builtin("rehash", b_rehash);
    table.add_builtin("rescan", b_rehash);
    table.add_builtin("pathtool", b_pathtool);
    table.add_builtin("what", b_what);
    // FFT.
    table.add_builtin("fft", |_i, a, _n| fft::fft1(a, false));
    table.add_builtin("ifft", |_i, a, _n| fft::fft1(a, true));
    table.add_builtin("fft2", |_i, a, _n| fft::fft2(a, false));
    table.add_builtin("ifft2", |_i, a, _n| fft::fft2(a, true));
    table.add_builtin("fftn", |_i, a, _n| fft::fftn(a, false));
    table.add_builtin("ifftn", |_i, a, _n| fft::fftn(a, true));
    table.add_builtin("fftshift", |_i, a, _n| fft::fftshift(a, true));
    table.add_builtin("ifftshift", |_i, a, _n| fft::fftshift(a, false));
    table.add_builtin("hilbert", |_i, a, n| fft::hilbert(a, n));
    // Regex.
    table.add_builtin("imwrite", |_i, a, _n| crate::image_io::imwrite(a));
    table.add_builtin("imread", |_i, a, _n| crate::image_io::imread(a));

    table.add_builtin("regexp", |_i, a, n| regexp::regexp(a, false, n));
    table.add_builtin("regexpi", |_i, a, n| regexp::regexp(a, true, n));
    table.add_builtin("regexprep", |_i, a, n| regexp::regexprep(a, n));

    table.add_builtin("system", |_i, a, n| osio::system(a, n));
    table.add_builtin("diary", |_i, a, n| osio::diary(a, n));
    table.add_builtin("license", |_i, a, _n| osio::license(a));
    table.add_builtin("wavwrite", |_i, a, _n| wav::wavwrite(a));
    table.add_builtin("wavread", |_i, a, _n| wav::wavread(a));
}

pub(crate) fn err<T>(msg: impl Into<String>) -> Flow<T> {
    Err(Signal::Error(InterpError::msg(msg)))
}

// ---- save / load --------------------------------------------------------

/// `save filename var1 var2 ...` — write named variables (or all locals) to a
/// MAT file. With no variable names, every variable in the current scope is
/// saved. The `-ascii` / `-mat` flags are recognized; `-ascii` writes a numeric
/// text matrix instead of a MAT file.
fn b_save(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("save: filename required");
    }
    let mut filename = args[0].as_string().unwrap_or_default();
    let mut names: Vec<String> = Vec::new();
    let mut ascii = false;
    for a in &args[1..] {
        if let Some(s) = a.as_string() {
            match s.as_str() {
                "-ascii" => ascii = true,
                "-mat" | "-v7" | "-v6" => {}
                _ => names.push(s),
            }
        }
    }
    // Default to all locals if none named.
    if names.is_empty() {
        names = i
            .context
            .top()
            .local_names()
            .into_iter()
            .map(str::to_string)
            .collect();
        names.sort();
    }

    if ascii {
        return save_ascii(i, &filename, &names);
    }
    // Append `.mat` only for a bare name with no extension (FreeMat keeps an
    // explicit extension like `.dat` as-is and writes its native format there,
    // which `load` reads back through the same MAT codec).
    if !filename.contains('.') {
        filename.push_str(".mat");
    }
    let mut vars = Vec::with_capacity(names.len());
    for name in &names {
        if let Some(v) = i.context.lookup(name) {
            vars.push(MatVar {
                name: name.clone(),
                value: v.clone(),
            });
        }
    }
    let bytes = write_mat(&vars);
    std::fs::write(&filename, bytes)
        .map_err(|e| Signal::Error(InterpError::msg(format!("save: {e}"))))?;
    Ok(vec![])
}

/// `-ascii` save: write each named variable's numeric rows as space-separated
/// text (MATLAB's `%.4f`-ish format; we use a high-precision general format).
fn save_ascii(i: &mut Interpreter, filename: &str, names: &[String]) -> Flow<Vec<Array>> {
    let mut text = String::new();
    for name in names {
        if let Some(v) = i.context.lookup(name) {
            let dims = v.dims();
            let rows = dims.first().copied().unwrap_or(0);
            let cols = dims.get(1).copied().unwrap_or(0);
            let data = fm_interp::value::to_f64_vec(v); // column-major
            for r in 0..rows {
                let mut line = String::new();
                for c in 0..cols {
                    if c > 0 {
                        line.push(' ');
                    }
                    line.push_str(&format!("{:.10e}", data[c * rows + r]));
                }
                text.push_str(&line);
                text.push('\n');
            }
        }
    }
    std::fs::write(filename, text)
        .map_err(|e| Signal::Error(InterpError::msg(format!("save: {e}"))))?;
    Ok(vec![])
}

/// `load filename` / `S = load('filename')` / `load('f','-ascii')`.
///
/// As a command (`nargout == 0`) the variables are injected into the current
/// scope; as an expression (`nargout >= 1`) they are returned in a struct.
fn b_load(i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("load: filename required");
    }
    let filename = args[0].as_string().unwrap_or_default();
    let ascii = args[1..]
        .iter()
        .any(|a| a.as_string().as_deref() == Some("-ascii"));
    // Explicit variable-name filters (anything after the filename that isn't a flag).
    let filters: Vec<String> = args[1..]
        .iter()
        .filter_map(Array::as_string)
        .filter(|s| !s.starts_with('-'))
        .collect();

    let path = resolve_load_path(&filename, ascii);
    if ascii || (!path.to_string_lossy().to_lowercase().ends_with(".mat") && looks_ascii(&path)) {
        return load_ascii(i, &path, &filename, nargout);
    }

    let bytes = std::fs::read(&path).map_err(|e| {
        Signal::Error(InterpError::msg(format!(
            "load: cannot read {filename}: {e}"
        )))
    })?;
    let vars = read_mat(&bytes).map_err(|e| Signal::Error(InterpError::msg(e.to_string())))?;
    let vars: Vec<MatVar> = vars
        .into_iter()
        .filter(|v| filters.is_empty() || filters.iter().any(|f| f == &v.name))
        .collect();

    if nargout >= 1 {
        let fields: Vec<(String, Vec<Array>)> =
            vars.into_iter().map(|v| (v.name, vec![v.value])).collect();
        return Ok(vec![Array::struct_array(StructArray::from_fields(
            vec![1, 1],
            fields,
        ))]);
    }
    for v in vars {
        i.context.assign(&v.name, v.value);
    }
    Ok(vec![])
}

fn resolve_load_path(filename: &str, ascii: bool) -> std::path::PathBuf {
    let p = Path::new(filename);
    if p.exists() || ascii || filename.contains('.') {
        return p.to_path_buf();
    }
    // Bare name: try appending .mat.
    let with_mat = format!("{filename}.mat");
    if Path::new(&with_mat).exists() {
        return std::path::PathBuf::from(with_mat);
    }
    p.to_path_buf()
}

fn looks_ascii(path: &Path) -> bool {
    // Heuristic: a file that doesn't start with the 128-byte MAT header text.
    if let Ok(bytes) = std::fs::read(path) {
        if bytes.len() >= 128 && &bytes[126..128] == b"IM" {
            return false;
        }
        if bytes.iter().take(64).all(|&b| b.is_ascii()) {
            return true;
        }
    }
    false
}

/// `-ascii` load: parse a whitespace/`,`/`;`-delimited numeric matrix.
fn load_ascii(
    i: &mut Interpreter,
    path: &Path,
    filename: &str,
    nargout: usize,
) -> Flow<Vec<Array>> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| Signal::Error(InterpError::msg(format!("load: {e}"))))?;
    let mut rows: Vec<Vec<f64>> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('%') {
            continue;
        }
        let nums: Vec<f64> = line
            .split([' ', '\t', ',', ';'])
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();
        if !nums.is_empty() {
            rows.push(nums);
        }
    }
    let r = rows.len();
    let c = rows.first().map_or(0, Vec::len);
    // Build column-major.
    let mut data = vec![0.0; r * c];
    for (ri, row) in rows.iter().enumerate() {
        for (ci, &v) in row.iter().enumerate() {
            if ci < c {
                data[ci * r + ri] = v;
            }
        }
    }
    let arr = Array::double_matrix(&[r, c], data);
    if nargout >= 1 {
        return Ok(vec![arr]);
    }
    // As a command, bind to a variable named after the file stem.
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("ans")
        .to_string();
    i.context.assign(&stem, arr);
    Ok(vec![])
}

// ---- fprintf / scanf / fileread / exist ---------------------------------

/// `fprintf([fid,] fmt, args...)` — format and write. To a file id, the text
/// goes to disk; to fid 1/2 (or omitted) it is emitted on the console.
fn b_fprintf(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("fprintf: format required");
    }
    // Detect a leading numeric fid: a scalar number whose following arg is a
    // string format. If the first arg is a string, fid defaults to stdout.
    let (fid, fmt_idx) = if args[0].is_char() {
        (1, 0)
    } else {
        (args[0].as_f64().unwrap_or(1.0) as i32, 1)
    };
    let format_args = &args[fmt_idx..];
    let text = sprintf(i, format_args)?;
    if let Some(console) = fileio::fprintf_to(fid, &text)? {
        i.emit(&console);
    }
    Ok(vec![Array::double(text.len() as f64)])
}

/// Format via the registered `sprintf` builtin (reuses its printf engine).
fn sprintf(i: &mut Interpreter, args: &[Array]) -> Flow<String> {
    let out = i.call_function("sprintf", args, 1, "", Span::empty(0))?;
    Ok(out.first().and_then(Array::as_string).unwrap_or_default())
}

/// `sscanf(str, fmt)`.
fn b_sscanf(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.len() < 2 {
        return err("sscanf: expected (str, format)");
    }
    let s = args[0].as_string().unwrap_or_default();
    let fmt = args[1].as_string().unwrap_or_default();
    scanf::sscanf(&s, &fmt)
}

/// `fscanf(fid, fmt)` — read the whole file (or remaining bytes) then scan.
fn b_fscanf(i: &mut Interpreter, args: &[Array], n: usize) -> Flow<Vec<Array>> {
    if args.len() < 2 {
        return err("fscanf: expected (fid, format)");
    }
    let fid = args[0].as_f64().unwrap_or(0.0) as i32;
    // Read remaining content via fread as uchar, then scan.
    let read = fileio::fread(
        &[
            Array::double(f64::from(fid)),
            Array::double(f64::INFINITY),
            Array::char_string("char"),
        ],
        1,
    )?;
    let _ = i; // fscanf does not touch interpreter state beyond the file table
    let s: String = match read.first() {
        Some(a) => fm_interp::value::to_f64_vec(a)
            .iter()
            .map(|&v| char::from_u32(v as u32).unwrap_or('\u{fffd}'))
            .collect(),
        None => String::new(),
    };
    let fmt = args[1].as_string().unwrap_or_default();
    let _ = n;
    scanf::sscanf(&s, &fmt)
}

/// `fileread(name)` — read an entire file as a char row vector.
fn b_fileread(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("fileread: filename required");
    }
    let name = args[0].as_string().unwrap_or_default();
    let text = std::fs::read_to_string(&name)
        .map_err(|e| Signal::Error(InterpError::msg(format!("fileread: {e}"))))?;
    Ok(vec![Array::char_string(&text)])
}

// ---- delimited text I/O -------------------------------------------------

/// `dlmread(name [, delim [, R, C]])` — read a numeric matrix from a delimited
/// text file. `delim` defaults to comma; an empty/whitespace delimiter splits on
/// any whitespace. `R`/`C` give a zero-based starting row/column offset.
fn b_dlmread(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("dlmread: filename required");
    }
    let name = args[0].as_string().unwrap_or_default();
    let delim = args.get(1).and_then(Array::as_string);
    let r0 = args.get(2).and_then(Array::as_f64).unwrap_or(0.0).max(0.0) as usize;
    let c0 = args.get(3).and_then(Array::as_f64).unwrap_or(0.0).max(0.0) as usize;
    let text = std::fs::read_to_string(&name)
        .map_err(|e| Signal::Error(InterpError::msg(format!("dlmread: {e}"))))?;
    Ok(vec![parse_delimited(&text, delim.as_deref(), r0, c0)])
}

/// `csvread(name [, R, C])` — comma-delimited read (a `dlmread` with `','`).
fn b_csvread(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("csvread: filename required");
    }
    let name = args[0].as_string().unwrap_or_default();
    let r0 = args.get(1).and_then(Array::as_f64).unwrap_or(0.0).max(0.0) as usize;
    let c0 = args.get(2).and_then(Array::as_f64).unwrap_or(0.0).max(0.0) as usize;
    let text = std::fs::read_to_string(&name)
        .map_err(|e| Signal::Error(InterpError::msg(format!("csvread: {e}"))))?;
    Ok(vec![parse_delimited(&text, Some(","), r0, c0)])
}

/// Parse a delimited numeric matrix, column-major, skipping the first `r0` rows
/// and `c0` columns.
fn parse_delimited(text: &str, delim: Option<&str>, r0: usize, c0: usize) -> Array {
    let mut rows: Vec<Vec<f64>> = Vec::new();
    for line in text.lines().skip(r0) {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            continue;
        }
        let fields: Vec<&str> = match delim {
            Some(d) if !d.is_empty() => trimmed.split(d).collect(),
            _ => trimmed.split_whitespace().collect(),
        };
        let nums: Vec<f64> = fields
            .iter()
            .skip(c0)
            .map(|s| s.trim().parse::<f64>().unwrap_or(0.0))
            .collect();
        rows.push(nums);
    }
    let r = rows.len();
    let c = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut data = vec![0.0f64; r * c];
    for (ri, row) in rows.iter().enumerate() {
        for (ci, &v) in row.iter().enumerate() {
            data[ci * r + ri] = v;
        }
    }
    Array::double_matrix(&[r, c], data)
}

/// `dlmwrite(name, M [, delim])` — write a numeric matrix as delimited text.
fn b_dlmwrite(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.len() < 2 {
        return err("dlmwrite: expected (filename, matrix [, delim])");
    }
    let name = args[0].as_string().unwrap_or_default();
    let delim = args
        .get(2)
        .and_then(Array::as_string)
        .unwrap_or_else(|| ",".to_string());
    write_delimited(i, &name, &args[1], &delim)
}

/// `csvwrite(name, M)` — comma-delimited write.
fn b_csvwrite(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.len() < 2 {
        return err("csvwrite: expected (filename, matrix)");
    }
    let name = args[0].as_string().unwrap_or_default();
    write_delimited(i, &name, &args[1], ",")
}

fn write_delimited(_i: &mut Interpreter, name: &str, m: &Array, delim: &str) -> Flow<Vec<Array>> {
    let dims = m.dims();
    let rows = dims.first().copied().unwrap_or(0);
    let cols = dims.get(1).copied().unwrap_or(0);
    let data = fm_interp::value::to_f64_vec(m); // column-major
    let mut text = String::new();
    for r in 0..rows {
        for c in 0..cols {
            if c > 0 {
                text.push_str(delim);
            }
            let v = data[c * rows + r];
            if v.fract() == 0.0 && v.abs() < 1e15 {
                text.push_str(&format!("{}", v as i64));
            } else {
                text.push_str(&format!("{v}"));
            }
        }
        text.push('\n');
    }
    std::fs::write(name, text)
        .map_err(|e| Signal::Error(InterpError::msg(format!("dlmwrite: {e}"))))?;
    Ok(vec![])
}

// ---- filesystem ----------------------------------------------------------

/// `pwd` — the current working directory.
fn b_pwd(_i: &mut Interpreter, _args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let dir = std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(vec![Array::char_string(&dir)])
}

/// `cd [dir]` — change (or report) the working directory.
fn b_cd(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if let Some(d) = args.first().and_then(Array::as_string) {
        std::env::set_current_dir(&d)
            .map_err(|e| Signal::Error(InterpError::msg(format!("cd: {e}"))))?;
    }
    let dir = std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(vec![Array::char_string(&dir)])
}

/// `dir [pattern]` — a struct array of directory entries with `name`, `bytes`,
/// and `isdir` fields (a useful subset of MATLAB's `dir`).
fn b_dir(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let pattern = args
        .first()
        .and_then(Array::as_string)
        .unwrap_or_else(|| ".".to_string());
    let (dir, glob) = split_dir_pattern(&pattern);
    let mut names = Vec::new();
    let mut bytes = Vec::new();
    let mut isdir = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for entry in rd.flatten() {
            let fname = entry.file_name().to_string_lossy().into_owned();
            if let Some(g) = &glob
                && !glob_match(g, &fname)
            {
                continue;
            }
            let meta = entry.metadata().ok();
            names.push(Array::char_string(&fname));
            bytes.push(Array::double(meta.as_ref().map_or(0.0, |m| m.len() as f64)));
            isdir.push(Array::bool(
                meta.as_ref().is_some_and(std::fs::Metadata::is_dir),
            ));
        }
    }
    let n = names.len();
    let fields = vec![
        ("name".to_string(), names),
        ("bytes".to_string(), bytes),
        ("isdir".to_string(), isdir),
    ];
    Ok(vec![Array::struct_array(StructArray::from_fields(
        vec![n, 1],
        fields,
    ))])
}

/// `ls [pattern]` — newline-joined directory listing (a char row vector).
fn b_ls(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let pattern = args
        .first()
        .and_then(Array::as_string)
        .unwrap_or_else(|| ".".to_string());
    let (dir, glob) = split_dir_pattern(&pattern);
    let mut entries: Vec<String> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for entry in rd.flatten() {
            let fname = entry.file_name().to_string_lossy().into_owned();
            if let Some(g) = &glob
                && !glob_match(g, &fname)
            {
                continue;
            }
            entries.push(fname);
        }
    }
    entries.sort();
    Ok(vec![Array::char_string(&entries.join("\n"))])
}

/// `which(name)` — locate a function: returns `'<name> is a built-in function'`
/// for a registered builtin, an on-disk `.m` path if found, else `''`.
fn b_which(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("which: name required");
    }
    let name = args[0].as_string().unwrap_or_default();
    // A function loaded from an `.m` file: report its path (FreeMat behaviour).
    if let Some(path) = i.functions.source_path(&name) {
        return Ok(vec![Array::char_string(&path)]);
    }
    // A loaded/registered function with no backing file (builtin / embedded).
    if i.functions.contains(&name) {
        return Ok(vec![Array::char_string(&format!(
            "{name} is a built-in function"
        ))]);
    }
    // An `.m` file on the path (cwd only — no search path model yet).
    let candidate = format!("{name}.m");
    if Path::new(&candidate).exists() {
        let abs = std::fs::canonicalize(&candidate)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or(candidate);
        return Ok(vec![Array::char_string(&abs)]);
    }
    Ok(vec![Array::char_string("")])
}

/// `type name` / `type('name')` — display the contents of a function or file.
///
/// Resolution order (mirrors `which`): an interpreted `.m` function prints its
/// stored source text; a registered builtin prints a deterministic one-line
/// notice; an `.m` file in the current directory is read and printed verbatim;
/// otherwise an error is raised. Output goes through `i.emit` (no return value),
/// matching FreeMat where `type` is a console command.
fn b_type(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("type: name required");
    }
    let name = args[0].as_string().unwrap_or_default();
    // An interpreted function loaded from source: print its source text.
    if let Some(src) = i.functions.source_text(&name) {
        i.emit(&src);
        if !src.ends_with('\n') {
            i.emit("\n");
        }
        return Ok(vec![]);
    }
    // A registered builtin with no backing source: a deterministic notice
    // (consistent with `which`'s "is a built-in function" wording).
    if i.functions.contains(&name) {
        i.emit(&format!("{name} is a built-in function\n"));
        return Ok(vec![]);
    }
    // An `.m` file in the current directory: print its contents verbatim.
    let candidate = if name.ends_with(".m") {
        name.clone()
    } else {
        format!("{name}.m")
    };
    if Path::new(&candidate).exists() {
        let text = std::fs::read_to_string(&candidate)
            .map_err(|e| Signal::Error(InterpError::msg(format!("type: {e}"))))?;
        i.emit(&text);
        if !text.ends_with('\n') {
            i.emit("\n");
        }
        return Ok(vec![]);
    }
    err(format!("type: '{name}' not found"))
}

/// Split a `dir`/`ls` pattern into a directory and an optional glob of its last
/// component (only `*` wildcards are supported).
fn split_dir_pattern(pattern: &str) -> (String, Option<String>) {
    let p = Path::new(pattern);
    if p.is_dir() {
        return (pattern.to_string(), None);
    }
    let parent = p
        .parent()
        .map(|d| {
            if d.as_os_str().is_empty() {
                ".".to_string()
            } else {
                d.to_string_lossy().into_owned()
            }
        })
        .unwrap_or_else(|| ".".to_string());
    let glob = p
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .filter(|f| f.contains('*'));
    match glob {
        Some(g) => (parent, Some(g)),
        None => (pattern.to_string(), None),
    }
}

/// Minimal `*`-glob match (used by `dir`/`ls`).
fn glob_match(pat: &str, name: &str) -> bool {
    let parts: Vec<&str> = pat.split('*').collect();
    if parts.len() == 1 {
        return pat == name;
    }
    let mut pos = 0usize;
    for (idx, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if idx == 0 {
            if !name[pos..].starts_with(part) {
                return false;
            }
            pos += part.len();
        } else if idx + 1 == parts.len() {
            return name[pos..].ends_with(part);
        } else if let Some(found) = name[pos..].find(part) {
            pos += found + part.len();
        } else {
            return false;
        }
    }
    true
}

/// `exist(name [, kind])` — extends the interpreter's `exist` with file checks:
/// 2 if `name` is a file on disk. Variable → 1, file → 2, function/builtin → 5.
fn b_exist(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("exist: name required");
    }
    let name = args[0].as_string().unwrap_or_default();
    let code = if i.context.exists(&name) {
        1.0
    } else if Path::new(&name).exists() {
        2.0
    } else if i.functions.contains(&name) {
        5.0
    } else {
        0.0
    };
    Ok(vec![Array::double(code)])
}

/// `mkdir(dirname)` / `mkdir(parentdir, dirname)` — create a directory (and any
/// missing parents, like FreeMat's `QDir::mkpath`). Returns `[success, msg,
/// msgid]`: a logical `1` on success (or if the directory already exists), `0`
/// on failure. Does not throw on an existing directory.
fn b_mkdir(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("mkdir requires at least one argument (the directory to create)");
    }
    let target = match args.len() {
        1 => args[0].as_string().unwrap_or_default(),
        _ => {
            let parent = args[0].as_string().unwrap_or_default();
            let child = args[1].as_string().unwrap_or_default();
            let mut p = std::path::PathBuf::from(parent);
            p.push(child);
            p.to_string_lossy().into_owned()
        }
    };
    let path = Path::new(&target);
    let (success, msg, msgid) = if path.is_dir() {
        (true, "directory already exists".to_string(), String::new())
    } else {
        match std::fs::create_dir_all(path) {
            Ok(()) => (true, String::new(), String::new()),
            Err(e) => (false, e.to_string(), "FreeMat:mkdir".to_string()),
        }
    };
    Ok(vec![
        Array::bool(success),
        Array::char_string(&msg),
        Array::char_string(&msgid),
    ])
}

/// `rmdir(dirname)` / `rmdir(dirname, 's')` — remove a directory. Without the
/// `'s'` flag only an empty directory is removed; with `'s'` (or `'S'`) the
/// directory and all its contents are removed recursively. Returns a logical
/// success flag (does not throw on a missing/non-empty directory).
fn b_rmdir(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("rmdir requires at least one argument (the directory to remove)");
    }
    let dirname = args[0].as_string().unwrap_or_default();
    let recursive = match args.get(1).and_then(Array::as_string) {
        None => false,
        Some(flag) if flag.eq_ignore_ascii_case("s") => true,
        Some(_) => {
            return err("rmdir second argument must be 's' to do a recursive delete");
        }
    };
    let result = if recursive {
        std::fs::remove_dir_all(&dirname)
    } else {
        std::fs::remove_dir(&dirname)
    };
    let (success, msg, msgid) = match result {
        Ok(()) => (true, String::new(), String::new()),
        Err(e) => (false, e.to_string(), "FreeMat:rmdir".to_string()),
    };
    Ok(vec![
        Array::bool(success),
        Array::char_string(&msg),
        Array::char_string(&msgid),
    ])
}

/// `copyfile(src, dst)` — copy a file. If `dst` is an existing directory the
/// file is copied into it under its original name (FreeMat behaviour). Returns
/// a logical success flag (does not throw on a missing source).
fn b_copyfile(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.len() < 2 {
        return err("copyfile requires two arguments - source and destination");
    }
    let src = args[0].as_string().unwrap_or_default();
    let dst = args[1].as_string().unwrap_or_default();
    let src_path = Path::new(&src);
    let dst_path = Path::new(&dst);
    // If the destination is a directory, copy into it under the source's name.
    let final_dst = if dst_path.is_dir() {
        match src_path.file_name() {
            Some(name) => dst_path.join(name),
            None => dst_path.to_path_buf(),
        }
    } else {
        dst_path.to_path_buf()
    };
    let (success, msg, msgid) = match std::fs::copy(src_path, &final_dst) {
        Ok(_) => (true, String::new(), String::new()),
        Err(e) => (false, e.to_string(), "FreeMat:copyfile".to_string()),
    };
    Ok(vec![
        Array::bool(success),
        Array::char_string(&msg),
        Array::char_string(&msgid),
    ])
}

/// `fileattrib(path)` — report a file's attributes. Returns `[success, attribs,
/// msgid]` where `attribs` is a struct with `Name`, `directory`, and the
/// `UserRead`/`UserWrite`/`UserExecute` (plus group/other) permission flags,
/// matching FreeMat's `FileAttrib` field set. On a missing path `success` is `0`
/// and `attribs` is empty (does not throw).
fn b_fileattrib(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return err("fileattrib requires at least one argument (the file name)");
    }
    let name = args[0].as_string().unwrap_or_default();
    let path = Path::new(&name);
    let Ok(meta) = std::fs::metadata(path) else {
        return Ok(vec![
            Array::bool(false),
            Array::char_string(""),
            Array::char_string("FreeMat:fileattrib"),
        ]);
    };
    let abs = std::fs::canonicalize(path)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| name.clone());
    let is_dir = meta.is_dir();
    let perm = meta.permissions();

    // Decode read/write/execute for user/group/other. On unix we read the mode
    // bits; elsewhere we fall back to the read-only flag for the user triad.
    let (ur, uw, ux, gr, gw, gx, or_, ow, ox);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = perm.mode();
        ur = mode & 0o400 != 0;
        uw = mode & 0o200 != 0;
        ux = mode & 0o100 != 0;
        gr = mode & 0o040 != 0;
        gw = mode & 0o020 != 0;
        gx = mode & 0o010 != 0;
        or_ = mode & 0o004 != 0;
        ow = mode & 0o002 != 0;
        ox = mode & 0o001 != 0;
    }
    #[cfg(not(unix))]
    {
        let writable = !perm.readonly();
        ur = true;
        uw = writable;
        ux = is_dir;
        gr = true;
        gw = writable;
        gx = is_dir;
        or_ = true;
        ow = writable;
        ox = is_dir;
    }

    let fields = vec![
        ("Name".to_string(), vec![Array::char_string(&abs)]),
        ("archive".to_string(), vec![Array::double(0.0)]),
        ("system".to_string(), vec![Array::double(0.0)]),
        ("hidden".to_string(), vec![Array::double(0.0)]),
        ("directory".to_string(), vec![Array::bool(is_dir)]),
        ("UserRead".to_string(), vec![Array::bool(ur)]),
        ("UserWrite".to_string(), vec![Array::bool(uw)]),
        ("UserExecute".to_string(), vec![Array::bool(ux)]),
        ("GroupRead".to_string(), vec![Array::bool(gr)]),
        ("GroupWrite".to_string(), vec![Array::bool(gw)]),
        ("GroupExecute".to_string(), vec![Array::bool(gx)]),
        ("OtherRead".to_string(), vec![Array::bool(or_)]),
        ("OtherWrite".to_string(), vec![Array::bool(ow)]),
        ("OtherExecute".to_string(), vec![Array::bool(ox)]),
    ];
    Ok(vec![
        Array::bool(true),
        Array::struct_array(StructArray::from_fields(vec![1, 1], fields)),
        Array::char_string(""),
    ])
}

/// `exit` / `exit(code)` — quit FreeMat. A synonym for `quit`: in this
/// interpreter `quit` is a keyword that signals `Signal::Return` to unwind out
/// of the current scope, and the CLI's REPL loop separately breaks on the
/// literal `exit`/`quit` input. We mirror `quit` exactly — emit `Signal::Return`
/// — so there is no `process::exit` in library code and a test that evaluates
/// `exit` only sees a benign return (it cannot kill the test runner). The
/// optional numeric exit code is accepted and ignored (there is no process to
/// hand it to from inside the interpreter).
fn b_exit(_i: &mut Interpreter, _args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    Err(Signal::Return)
}

// ---- path / separators ---------------------------------------------------

/// The OS path-list separator (`;` on Windows, `:` elsewhere).
fn path_list_sep() -> char {
    if cfg!(windows) { ';' } else { ':' }
}

/// `filesep` / `dirsep` — the directory separator character for this platform
/// (`/` on unix, `\` on Windows).
fn b_filesep(_i: &mut Interpreter, _args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    Ok(vec![Array::char_string(std::path::MAIN_SEPARATOR_STR)])
}

/// `pathsep` — the search-path list separator (`;` on Windows, `:` elsewhere).
fn b_pathsep(_i: &mut Interpreter, _args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    Ok(vec![Array::char_string(&path_list_sep().to_string())])
}

/// Join the interpreter's search path into a single `pathsep`-delimited string.
fn path_string(i: &Interpreter) -> String {
    let sep = path_list_sep().to_string();
    i.search_path.join(&sep)
}

/// Replace the interpreter's search path from a `pathsep`-delimited string.
fn set_path_string(i: &mut Interpreter, s: &str) {
    let sep = path_list_sep();
    i.search_path = s
        .split(sep)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect();
}

/// `getpath` — the current FreeMat search path as a `pathsep`-delimited string.
fn b_getpath(i: &mut Interpreter, _args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    Ok(vec![Array::char_string(&path_string(i))])
}

/// `setpath(y)` — set the search path from a `pathsep`-delimited string.
fn b_setpath(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let s = args.first().and_then(Array::as_string).unwrap_or_default();
    set_path_string(i, &s);
    Ok(vec![])
}

/// `path` — with no args (and no output) prints the search path one entry per
/// line; with an output returns it as a string; `path(p)` sets it; `path(p1,p2)`
/// concatenates `p1` and `p2` (FreeMat's `path.m`).
fn b_path(i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    if args.is_empty() && nargout == 0 {
        let p = path_string(i);
        if p.is_empty() {
            i.emit("\n");
        } else {
            for entry in p.split(path_list_sep()) {
                i.emit(entry);
                i.emit("\n");
            }
        }
        return Ok(vec![]);
    }
    let out = if nargout >= 1 {
        vec![Array::char_string(&path_string(i))]
    } else {
        vec![]
    };
    match args.len() {
        1 => {
            let s = args[0].as_string().unwrap_or_default();
            set_path_string(i, &s);
        }
        n if n >= 2 => {
            let a = args[0].as_string().unwrap_or_default();
            let b = args[1].as_string().unwrap_or_default();
            set_path_string(i, &format!("{a}{}{b}", path_list_sep()));
        }
        _ => {}
    }
    Ok(out)
}

/// `addpath(d1, d2, ..., [-begin|-end|-0|-1])` — add directories to the search
/// path, prepending by default or appending with `-end`/`-1` (FreeMat's
/// `addpath.m`).
fn b_addpath(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        return Ok(vec![]);
    }
    let mut dirs: Vec<String> = args.iter().filter_map(Array::as_string).collect();
    let mut at_begin = true;
    if let Some(last) = dirs.last() {
        match last.as_str() {
            "-0" | "-begin" => {
                dirs.pop();
            }
            "-1" | "-end" => {
                at_begin = false;
                dirs.pop();
            }
            _ => {}
        }
    }
    if at_begin {
        // Prepend, preserving the given order ahead of the existing path.
        for d in dirs.into_iter().rev() {
            i.search_path.insert(0, d);
        }
    } else {
        i.search_path.extend(dirs);
    }
    Ok(vec![])
}

/// `rehash` / `rescan` — refresh the function cache. FreeMat re-`cd`s to the
/// current directory; here the interpreter loads `.m` files on demand, so this
/// is a clean no-op that returns successfully.
fn b_rehash(_i: &mut Interpreter, _args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    Ok(vec![])
}

/// `pathtool` — opens a GUI path editor in FreeMat; in this port there is no
/// GUI, so it is a clean no-op.
fn b_pathtool(_i: &mut Interpreter, _args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    Ok(vec![])
}

/// `what [path]` — list the FreeMat-relevant files (`.m` and `.mat`) in a
/// directory (the current directory if none given). Returns a 1x1 struct with
/// `path`, `m`, and `mat` fields (a useful subset of MATLAB's `what`).
fn b_what(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let dir = args
        .first()
        .and_then(Array::as_string)
        .unwrap_or_else(|| ".".to_string());
    let abs = std::fs::canonicalize(&dir)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| dir.clone());
    let mut m_files: Vec<String> = Vec::new();
    let mut mat_files: Vec<String> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let lower = name.to_lowercase();
            if lower.ends_with(".m") {
                m_files.push(name);
            } else if lower.ends_with(".mat") {
                mat_files.push(name);
            }
        }
    }
    m_files.sort();
    mat_files.sort();
    let m_cell = Array::cell(
        &[m_files.len(), 1],
        m_files.iter().map(|s| Array::char_string(s)).collect(),
    );
    let mat_cell = Array::cell(
        &[mat_files.len(), 1],
        mat_files.iter().map(|s| Array::char_string(s)).collect(),
    );
    let fields = vec![
        ("path".to_string(), vec![Array::char_string(&abs)]),
        ("m".to_string(), vec![m_cell]),
        ("mat".to_string(), vec![mat_cell]),
    ];
    Ok(vec![Array::struct_array(StructArray::from_fields(
        vec![1, 1],
        fields,
    ))])
}
