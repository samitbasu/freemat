//! Registration of the `fm-io` builtins into the interpreter's function table:
//! MAT `save`/`load`, file I/O, FFT, and regex.

use std::path::Path;

use fm_core::{Array, StructArray};
use fm_interp::error::{Flow, InterpError, Signal};
use fm_interp::{FunctionTable, Interpreter};
use fm_parser::Span;

use crate::matfile::{MatVar, read_mat, write_mat};
use crate::{fft, fileio, regexp, scanf};

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
    table.add_builtin("feof", |_i, a, n| fileio::feof(a, n));
    table.add_builtin("fgetl", |_i, a, n| fileio::fgetl(a, n));
    table.add_builtin("fgets", |_i, a, n| fileio::fgets(a, n));
    table.add_builtin("fgetline", |_i, a, n| fileio::fgetl(a, n));
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
    // FFT.
    table.add_builtin("fft", |_i, a, _n| fft::fft1(a, false));
    table.add_builtin("ifft", |_i, a, _n| fft::fft1(a, true));
    table.add_builtin("fft2", |_i, a, _n| fft::fft2(a, false));
    table.add_builtin("ifft2", |_i, a, _n| fft::fft2(a, true));
    // Regex.
    table.add_builtin("imwrite", |_i, a, _n| crate::image_io::imwrite(a));
    table.add_builtin("imread", |_i, a, _n| crate::image_io::imread(a));

    table.add_builtin("regexp", |_i, a, n| regexp::regexp(a, false, n));
    table.add_builtin("regexpi", |_i, a, n| regexp::regexp(a, true, n));
    table.add_builtin("regexprep", |_i, a, n| regexp::regexprep(a, n));
}

fn err<T>(msg: impl Into<String>) -> Flow<T> {
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
