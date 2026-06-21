//! String builtins: `strncmp`, `upper`/`lower`, `strtrim`, `strrep`,
//! `strfind`, `strsplit`/`strjoin`, `num2str`/`str2num`/`str2double`,
//! `blanks`, `deblank`, and `sprintf`/`printf`.
//!
//! (`strcmp`/`strcmpi` already live in `inspection`.)

use fm_core::{Array, DataClass};
use fm_interp::error::Flow;
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};

use crate::util::{err, need};

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("strncmp", |_i, a, _n| strncmp(a, false));
    table.add_builtin("strncmpi", |_i, a, _n| strncmp(a, true));
    table.add_builtin("upper", |_i, a, _n| map_case(a, true));
    table.add_builtin("toupper", |_i, a, _n| map_case(a, true));
    table.add_builtin("lower", |_i, a, _n| map_case(a, false));
    table.add_builtin("tolower", |_i, a, _n| map_case(a, false));
    table.add_builtin("strtrim", b_strtrim);
    table.add_builtin("deblank", b_deblank);
    table.add_builtin("blanks", b_blanks);
    table.add_builtin("strrep", b_strrep);
    table.add_builtin("strfind", b_strfind);
    table.add_builtin("strsplit", b_strsplit);
    table.add_builtin("strjoin", b_strjoin);
    table.add_builtin("str2num", b_str2num);
    table.add_builtin("str2double", b_str2double);
    table.add_builtin("sprintf", b_sprintf);
    table.add_builtin("printf", b_printf);
    table.add_builtin("num2str", b_num2str);
    table.add_builtin("mat2str", b_mat2str);
    table.add_builtin("fileparts", b_fileparts);
    table.add_builtin("cellstr", b_cellstr);
    table.add_builtin("strstr", b_strstr);
    table.add_builtin("isalpha", |_i, a, _n| {
        char_pred(a, "isalpha", |c| c.is_ascii_alphabetic())
    });
    table.add_builtin("isdigit", |_i, a, _n| {
        char_pred(a, "isdigit", |c| c.is_ascii_digit())
    });
    table.add_builtin("isspace", |_i, a, _n| {
        char_pred(a, "isspace", |c| c.is_whitespace())
    });
    table.add_builtin("fullfile", b_fullfile);
    table.add_builtin("getenv", b_getenv);
}

/// `fullfile(a, b, ...)` — join path components with `/` (FreeMat uses the
/// forward slash uniformly), collapsing redundant separators.
fn b_fullfile(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "fullfile")?;
    let mut out = String::new();
    for (k, a) in args.iter().enumerate() {
        let part = str_of(a);
        if k > 0 && !out.is_empty() && !out.ends_with('/') {
            out.push('/');
        }
        out.push_str(part.trim_end_matches('/'));
    }
    Ok(vec![Array::char_string(&out)])
}

/// `getenv(name)` — value of an environment variable, or `''` if unset.
fn b_getenv(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "getenv")?;
    let name = str_of(&args[0]);
    let val = std::env::var(&name).unwrap_or_default();
    Ok(vec![Array::char_string(&val)])
}

/// `cellstr(C)` — convert a char matrix into a cell array of strings, one row
/// per cell, with trailing blanks removed (FreeMat `cellstr`).
fn b_cellstr(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "cellstr")?;
    let a = &args[0];
    // A cell array passes through (validated to be all strings).
    if let Some(cells) = a.as_cell() {
        let data: Vec<Array> = cells.iter().cloned().collect();
        return Ok(vec![Array::cell(&a.dims(), data)]);
    }
    let dims = a.dims();
    let chars: Vec<char> = match a.as_string() {
        Some(s) => s.chars().collect(),
        None => return err("cellstr: argument must be a string"),
    };
    let (rows, cols) = if dims.len() >= 2 {
        (dims[0], dims[1])
    } else {
        (1, chars.len())
    };
    if rows == 0 {
        return Ok(vec![Array::cell(&[0, 1], vec![])]);
    }
    // `as_string` yields the chars in logical (row-major) order, so row `i`
    // occupies `chars[i*cols .. i*cols + cols]`.
    let out: Vec<Array> = (0..rows)
        .map(|i| {
            let row: String = (0..cols).map(|j| chars[i * cols + j]).collect();
            Array::char_string(row.trim_end())
        })
        .collect();
    Ok(vec![Array::cell(&[rows, 1], out)])
}

/// `strstr(X, Y)` — 1-based index of the first occurrence of `Y` in `X`, or 0
/// if not found (FreeMat's C-style substring search).
fn b_strstr(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "strstr")?;
    let hay = str_of(&args[0]);
    let needle = str_of(&args[1]);
    // Byte search; the corpus is ASCII. Convert byte offset to a 1-based index.
    let pos = hay
        .find(&needle)
        .map(|b| hay[..b].chars().count() + 1)
        .unwrap_or(0);
    Ok(vec![build_real(
        DataClass::Double,
        &[1, 1],
        vec![pos as f64],
    )])
}

/// Shared helper for `isalpha`/`isdigit`/`isspace`: a logical row vector marking
/// which characters of the input string satisfy `pred`.
fn char_pred(args: &[Array], name: &str, pred: impl Fn(char) -> bool) -> Flow<Vec<Array>> {
    need(args, 1, name)?;
    let s = match args[0].as_string() {
        Some(s) => s,
        None => return err(format!("{name}: argument must be a string")),
    };
    let data: Vec<f64> = s.chars().map(|c| pred(c) as i32 as f64).collect();
    let n = data.len();
    Ok(vec![build_real(DataClass::Bool, &[1, n], data)])
}

/// `[path, name, ext, ver] = fileparts(filename)` — split a path into its
/// directory, base name, and extension. Mirrors FreeMat's `FilePartsFunction`
/// (Qt `QFileInfo`): `path` is the directory (no trailing slash), `name` is the
/// complete base name (everything before the LAST `.`), `ext` is the last
/// extension *including* the dot (or `''`), and `ver` is always an empty string
/// (MATLAB compatibility). A leading-dot file (e.g. `.bashrc`) is treated as a
/// name with no extension, matching Qt.
fn b_fileparts(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "fileparts")?;
    let full = str_of(&args[0]);

    // Split off the directory at the last path separator (accept both `/` and
    // platform separators; the corpus uses `/`).
    let sep = full.rfind(['/', '\\']);
    let (dir, file) = match sep {
        Some(i) => (&full[..i], &full[i + 1..]),
        None => ("", full.as_str()),
    };

    // Within the file portion, the extension is everything after the LAST dot,
    // unless that dot is the first character (a dotfile has no extension).
    let (name, ext) = match file.rfind('.') {
        Some(i) if i > 0 => (&file[..i], &file[i..]),
        _ => (file, ""),
    };

    Ok(vec![
        Array::char_string(dir),
        Array::char_string(name),
        Array::char_string(ext),
        Array::char_string(""),
    ])
}

fn str_of(a: &Array) -> String {
    a.as_string().unwrap_or_default()
}

/// `strncmp(a, b, n)` — compare the first `n` characters.
fn strncmp(args: &[Array], ci: bool) -> Flow<Vec<Array>> {
    need(args, 3, "strncmp")?;
    let a = str_of(&args[0]);
    let b = str_of(&args[1]);
    let n = args[2].as_f64().unwrap_or(0.0) as usize;
    let take = |s: &str| -> String { s.chars().take(n).collect() };
    let (x, y) = (take(&a), take(&b));
    let eq = a.chars().count() >= n
        && b.chars().count() >= n
        && if ci {
            x.eq_ignore_ascii_case(&y)
        } else {
            x == y
        };
    Ok(vec![Array::bool(eq)])
}

fn map_case(args: &[Array], upper: bool) -> Flow<Vec<Array>> {
    need(args, 1, "upper")?;
    let a = &args[0];
    if let Some(cells) = a.as_cell() {
        let data: Vec<Array> = cells
            .iter()
            .map(|c| {
                let s = c.as_string().unwrap_or_default();
                Array::char_string(&case(&s, upper))
            })
            .collect();
        return Ok(vec![Array::cell(&a.dims(), data)]);
    }
    let s = str_of(a);
    Ok(vec![Array::char_string(&case(&s, upper))])
}

fn case(s: &str, upper: bool) -> String {
    if upper {
        s.to_uppercase()
    } else {
        s.to_lowercase()
    }
}

fn b_strtrim(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "strtrim")?;
    let s = str_of(&args[0]);
    Ok(vec![Array::char_string(s.trim())])
}

fn b_deblank(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "deblank")?;
    let s = str_of(&args[0]);
    Ok(vec![Array::char_string(s.trim_end())])
}

fn b_blanks(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "blanks")?;
    let n = args[0].as_f64().unwrap_or(0.0).max(0.0) as usize;
    Ok(vec![Array::char_string(&" ".repeat(n))])
}

fn b_strrep(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 3, "strrep")?;
    let s = str_of(&args[0]);
    let from = str_of(&args[1]);
    let to = str_of(&args[2]);
    let out = if from.is_empty() {
        s
    } else {
        s.replace(&from, &to)
    };
    Ok(vec![Array::char_string(&out)])
}

/// `strfind(text, pat)` — 1-based start positions of every (overlapping) match.
fn b_strfind(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "strfind")?;
    let s: Vec<char> = str_of(&args[0]).chars().collect();
    let pat: Vec<char> = str_of(&args[1]).chars().collect();
    let mut hits = Vec::new();
    if !pat.is_empty() && pat.len() <= s.len() {
        for i in 0..=(s.len() - pat.len()) {
            if s[i..i + pat.len()] == pat[..] {
                hits.push((i + 1) as f64);
            }
        }
    }
    let n = hits.len();
    Ok(vec![build_real(DataClass::Double, &[1, n], hits)])
}

/// `strsplit(text)` / `strsplit(text, delim)` — split into a cell of strings.
fn b_strsplit(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "strsplit")?;
    let s = str_of(&args[0]);
    let parts: Vec<String> = if args.len() >= 2 {
        let d = str_of(&args[1]);
        if d.is_empty() {
            vec![s]
        } else {
            s.split(&d).map(str::to_string).collect()
        }
    } else {
        s.split_whitespace().map(str::to_string).collect()
    };
    let n = parts.len();
    let data: Vec<Array> = parts.iter().map(|p| Array::char_string(p)).collect();
    Ok(vec![Array::cell(&[1, n], data)])
}

/// `strjoin(cellstr)` / `strjoin(cellstr, delim)` — join with a delimiter.
fn b_strjoin(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "strjoin")?;
    let cells = args[0]
        .as_cell()
        .ok_or_else(|| crate::util::err_signal("strjoin: first argument must be a cell array"))?;
    let parts: Vec<String> = cells
        .iter()
        .map(|c| c.as_string().unwrap_or_default())
        .collect();
    let delim = if args.len() >= 2 {
        str_of(&args[1])
    } else {
        " ".to_string()
    };
    Ok(vec![Array::char_string(&parts.join(&delim))])
}

/// `str2num(s)` — evaluate a numeric string. We parse the common forms (a single
/// number or a simple bracketed row vector) without re-entering the evaluator.
fn b_str2num(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "str2num")?;
    let s = str_of(&args[0]);
    let inner = s.trim().trim_start_matches('[').trim_end_matches(']');
    let nums: Vec<f64> = inner
        .split([',', ' ', ';', '\t'])
        .filter(|t| !t.is_empty())
        .filter_map(|t| t.parse::<f64>().ok())
        .collect();
    if nums.is_empty() {
        return Ok(vec![Array::empty()]);
    }
    let n = nums.len();
    Ok(vec![build_real(DataClass::Double, &[1, n], nums)])
}

/// `str2double(s)` — parse a single double; `NaN` on failure. Cell input maps.
fn b_str2double(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "str2double")?;
    let a = &args[0];
    if let Some(cells) = a.as_cell() {
        let data: Vec<f64> = cells
            .iter()
            .map(|c| parse_double(&c.as_string().unwrap_or_default()))
            .collect();
        return Ok(vec![build_real(DataClass::Double, &a.dims(), data)]);
    }
    Ok(vec![Array::double(parse_double(&str_of(a)))])
}

fn parse_double(s: &str) -> f64 {
    s.trim().parse::<f64>().unwrap_or(f64::NAN)
}

fn b_sprintf(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "sprintf")?;
    let fmt = str_of(&args[0]);
    let out = sprintf_impl(&fmt, &args[1..])?;
    Ok(vec![Array::char_string(&out)])
}

fn b_printf(i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "printf")?;
    let fmt = str_of(&args[0]);
    let out = sprintf_impl(&fmt, &args[1..])?;
    i.emit(&out);
    Ok(vec![])
}

/// `num2str` — like FreeMat: scalars use up to 4 significant decimals, matrices
/// format each element. Honours an optional precision / format-string argument.
fn b_num2str(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "num2str")?;
    let a = &args[0];
    if a.is_char() {
        return Ok(vec![a.clone()]);
    }
    // num2str(x, fmt) form.
    if args.len() >= 2
        && let Some(fmt) = args[1].as_string()
    {
        let out = sprintf_impl(&fmt, std::slice::from_ref(a))?;
        return Ok(vec![Array::char_string(&out)]);
    }
    if let Some(v) = a.as_f64() {
        return Ok(vec![Array::char_string(&fmt_num(v))]);
    }
    // Matrix: rows separated by newlines, columns by spaces.
    let dims = a.dims();
    let data = to_f64_vec(a);
    if dims.len() != 2 {
        return Ok(vec![Array::char_string(
            &a.format(fm_core::FormatMode::Short),
        )]);
    }
    let (r, c) = (dims[0], dims[1]);
    let mut lines = Vec::with_capacity(r);
    for i in 0..r {
        let row: Vec<String> = (0..c).map(|j| fmt_num(data[i + j * r])).collect();
        lines.push(row.join("  "));
    }
    Ok(vec![Array::char_string(&lines.join("\n"))])
}

fn fmt_num(v: f64) -> String {
    if v.fract() == 0.0 && v.is_finite() && v.abs() < 1e15 {
        format!("{v}")
    } else {
        // 5 significant digits, trimmed (MATLAB-ish).
        let s = format!("{v:.4}");
        s
    }
}

/// `mat2str(x)` — a string that re-creates the matrix (e.g. `[1 2;3 4]`).
fn b_mat2str(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "mat2str")?;
    let a = &args[0];
    if let Some(v) = a.as_f64() {
        return Ok(vec![Array::char_string(&fmt_num(v))]);
    }
    let dims = a.dims();
    if dims.len() != 2 {
        return err("mat2str: only 2-D inputs are supported");
    }
    let (r, c) = (dims[0], dims[1]);
    let data = to_f64_vec(a);
    let mut rows = Vec::with_capacity(r);
    for i in 0..r {
        let row: Vec<String> = (0..c).map(|j| fmt_num(data[i + j * r])).collect();
        rows.push(row.join(" "));
    }
    Ok(vec![Array::char_string(&format!("[{}]", rows.join(";")))])
}

/// A minimal `printf`-style formatter covering `%d %i %u %f %e %g %s %c %x %o`
/// with optional width/precision/flags, recycling the argument list over the
/// format string (MATLAB semantics: the format repeats until the args run out).
fn sprintf_impl(fmt: &str, args: &[Array]) -> Flow<String> {
    // Flatten every argument into a queue of scalar values (column-major), with
    // a marker for string arguments consumed whole by `%s`.
    let mut queue: Vec<Field> = Vec::new();
    for a in args {
        if a.is_char() {
            queue.push(Field::Str(a.as_string().unwrap_or_default()));
        } else {
            for v in to_f64_vec(a) {
                queue.push(Field::Num(v));
            }
        }
    }

    let chars: Vec<char> = fmt.chars().collect();
    let mut out = String::new();
    let mut qi = 0usize;
    // Whether the format contains any conversion that consumes an argument.
    let consumes = format_consumes(&chars);

    loop {
        let mut i = 0usize;
        let start_qi = qi;
        while i < chars.len() {
            let ch = chars[i];
            if ch == '%' {
                if i + 1 < chars.len() && chars[i + 1] == '%' {
                    out.push('%');
                    i += 2;
                    continue;
                }
                let (spec, ni) = parse_spec(&chars, i);
                i = ni;
                apply_spec(&mut out, &spec, &mut queue, &mut qi);
            } else if ch == '\\' && i + 1 < chars.len() {
                out.push(match chars[i + 1] {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '\\' => '\\',
                    other => other,
                });
                i += 2;
            } else {
                out.push(ch);
                i += 1;
            }
        }
        // Stop if the format consumes nothing, the args are exhausted, or no
        // progress was made this pass.
        if !consumes || qi >= queue.len() || qi == start_qi {
            break;
        }
    }
    Ok(out)
}

enum Field {
    Num(f64),
    Str(String),
}

/// A parsed conversion spec: flags + width + precision + conversion char.
struct Spec {
    flags: String,
    width: Option<usize>,
    precision: Option<usize>,
    conv: char,
}

fn format_consumes(chars: &[char]) -> bool {
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' {
            if i + 1 < chars.len() && chars[i + 1] == '%' {
                i += 2;
                continue;
            }
            return true;
        }
        i += 1;
    }
    false
}

fn parse_spec(chars: &[char], start: usize) -> (Spec, usize) {
    let mut i = start + 1; // skip '%'
    let mut flags = String::new();
    while i < chars.len() && matches!(chars[i], '-' | '+' | ' ' | '0' | '#') {
        flags.push(chars[i]);
        i += 1;
    }
    let mut width = None;
    let mut w = String::new();
    while i < chars.len() && chars[i].is_ascii_digit() {
        w.push(chars[i]);
        i += 1;
    }
    if !w.is_empty() {
        width = w.parse().ok();
    }
    let mut precision = None;
    if i < chars.len() && chars[i] == '.' {
        i += 1;
        let mut p = String::new();
        while i < chars.len() && chars[i].is_ascii_digit() {
            p.push(chars[i]);
            i += 1;
        }
        precision = Some(p.parse().unwrap_or(0));
    }
    let conv = chars.get(i).copied().unwrap_or('s');
    i += 1;
    (
        Spec {
            flags,
            width,
            precision,
            conv,
        },
        i,
    )
}

fn next_num(queue: &mut [Field], qi: &mut usize) -> f64 {
    match queue.get(*qi) {
        Some(Field::Num(v)) => {
            *qi += 1;
            *v
        }
        Some(Field::Str(s)) => {
            // A string used in a numeric conversion contributes its char codes;
            // take the first code point (rare path).
            let v = s.chars().next().map_or(0.0, |c| u32::from(c) as f64);
            *qi += 1;
            v
        }
        None => 0.0,
    }
}

fn next_str(queue: &mut [Field], qi: &mut usize) -> String {
    match queue.get(*qi) {
        Some(Field::Str(s)) => {
            let out = s.clone();
            *qi += 1;
            out
        }
        Some(Field::Num(v)) => {
            *qi += 1;
            fmt_num(*v)
        }
        None => String::new(),
    }
}

fn apply_spec(out: &mut String, spec: &Spec, queue: &mut [Field], qi: &mut usize) {
    let body = match spec.conv {
        'd' | 'i' => format!("{}", next_num(queue, qi).round() as i64),
        'u' => format!("{}", next_num(queue, qi).round().max(0.0) as u64),
        'x' => format!("{:x}", next_num(queue, qi).round() as i64),
        'X' => format!("{:X}", next_num(queue, qi).round() as i64),
        'o' => format!("{:o}", next_num(queue, qi).round() as i64),
        'f' | 'F' => {
            let p = spec.precision.unwrap_or(6);
            format!("{:.*}", p, next_num(queue, qi))
        }
        'e' | 'E' => {
            let p = spec.precision.unwrap_or(6);
            let s = format!("{:.*e}", p, next_num(queue, qi));
            fix_exp(&s, spec.conv == 'E')
        }
        'g' | 'G' => {
            let v = next_num(queue, qi);
            let s = format!("{v}");
            if spec.conv == 'G' {
                s.to_uppercase()
            } else {
                s
            }
        }
        'c' => {
            let v = next_num(queue, qi);
            char::from_u32(v as u32).unwrap_or('\u{fffd}').to_string()
        }
        's' => {
            let s = next_str(queue, qi);
            if let Some(p) = spec.precision {
                s.chars().take(p).collect()
            } else {
                s
            }
        }
        other => other.to_string(),
    };
    out.push_str(&pad(&body, spec));
}

/// Rust formats exponents as `1e2`; C/MATLAB use `1.000000e+02`. Normalize.
fn fix_exp(s: &str, upper: bool) -> String {
    let (mantissa, exp) = match s.split_once('e') {
        Some((m, e)) => (m, e),
        None => return s.to_string(),
    };
    let (sign, digits) = if let Some(d) = exp.strip_prefix('-') {
        ('-', d)
    } else if let Some(d) = exp.strip_prefix('+') {
        ('+', d)
    } else {
        ('+', exp)
    };
    let e_ch = if upper { 'E' } else { 'e' };
    format!("{mantissa}{e_ch}{sign}{digits:0>2}")
}

fn pad(body: &str, spec: &Spec) -> String {
    let Some(width) = spec.width else {
        return body.to_string();
    };
    let len = body.chars().count();
    if len >= width {
        return body.to_string();
    }
    let fill = width - len;
    if spec.flags.contains('-') {
        format!("{body}{}", " ".repeat(fill))
    } else if spec.flags.contains('0')
        && matches!(
            spec.conv,
            'd' | 'i' | 'u' | 'f' | 'F' | 'e' | 'E' | 'x' | 'X' | 'o'
        )
    {
        // Zero-pad numerics, after any sign.
        if let Some(rest) = body.strip_prefix('-') {
            format!("-{}{rest}", "0".repeat(fill))
        } else {
            format!("{}{body}", "0".repeat(fill))
        }
    } else {
        format!("{}{body}", " ".repeat(fill))
    }
}
