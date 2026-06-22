//! MATLAB-style display formatting for [`Array`] values.
//!
//! This implements the *value body* MATLAB/FreeMat prints (the `x =` /
//! `ans =` prefix is the interpreter's job, Stage 3). It follows FreeMat's
//! `Print.cpp` conventions and standard MATLAB display:
//!
//! - **Integers / integer-valued reals** print with no decimal point.
//! - **Non-integer reals** print with 4 significant decimals (`format short`)
//!   or 15 (`format long`).
//! - **Matrices** factor out a common `1.0e+NN *` scale factor when the largest
//!   magnitude is very large or very small, matching FreeMat's thresholds.
//! - **Complex** values print as `re + imi` / `re - imi`.
//! - **Char** values print as text; **empty** prints `[]`.
//!
//! Golden-string tests live in `tests/` and assert exactly this output.

use std::fmt::Write as _;

use crate::array::Array;
use crate::scalar::ScalarValue;

/// MATLAB `format` mode controlling precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FormatMode {
    /// `format short` — ~5 significant digits (4 decimals).
    #[default]
    Short,
    /// `format long` — ~15 significant digits.
    Long,
    /// `format short e` — 4 decimals, always exponential.
    ShortE,
    /// `format long e` — 15 decimals, always exponential.
    LongE,
    /// `format short g` — compact, ~5 significant digits.
    ShortG,
    /// `format long g` — compact, ~15 significant digits.
    LongG,
}

impl FormatMode {
    /// Decimal places used for non-integer reals.
    const fn decimals(self) -> usize {
        match self {
            FormatMode::Short | FormatMode::ShortE | FormatMode::ShortG => 4,
            FormatMode::Long | FormatMode::LongE | FormatMode::LongG => 15,
        }
    }

    /// Whether this mode always renders reals in exponential notation.
    const fn always_exponential(self) -> bool {
        matches!(self, FormatMode::ShortE | FormatMode::LongE)
    }

    /// Whether this mode uses the compact (`%g`-style) presentation, which
    /// drops trailing zeros and picks fixed/exponential per value.
    const fn compact(self) -> bool {
        matches!(self, FormatMode::ShortG | FormatMode::LongG)
    }

    /// Parse the FreeMat `format` argument(s) (already lower-cased and joined by
    /// a single space, e.g. `"short e"`). Returns `None` for unknown modes.
    #[must_use]
    pub fn from_arg(s: &str) -> Option<FormatMode> {
        Some(match s.trim() {
            "short" | "" => FormatMode::Short,
            "long" => FormatMode::Long,
            "short e" | "shorte" => FormatMode::ShortE,
            "long e" | "longe" => FormatMode::LongE,
            "short g" | "shortg" => FormatMode::ShortG,
            "long g" | "longg" => FormatMode::LongG,
            _ => return None,
        })
    }

    /// The canonical name string returned by `t = format`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            FormatMode::Short => "short",
            FormatMode::Long => "long",
            FormatMode::ShortE => "short e",
            FormatMode::LongE => "long e",
            FormatMode::ShortG => "short g",
            FormatMode::LongG => "long g",
        }
    }
}

impl Array {
    /// Render this value the way MATLAB/FreeMat displays it, in the given mode.
    ///
    /// The returned string is the value *body* — no `x =` prefix and no leading
    /// blank line; the interpreter adds those.
    #[must_use]
    pub fn format(&self, mode: FormatMode) -> String {
        // Char strings print verbatim (row vector → the text itself).
        if let Some(s) = self.as_string_for_display() {
            return s;
        }
        if self.is_empty() {
            return "[]".to_string();
        }
        if let Some(s) = self.as_scalar() {
            return format_scalar(s, mode);
        }
        if let Some(cells) = self.as_cell() {
            return format_cell(cells, &self.dims(), mode);
        }
        if let Some(st) = self.as_struct() {
            return format_struct(st);
        }
        if let Some(sp) = self.as_sparse() {
            return format_sparse(sp, mode);
        }
        if let Some(h) = self.as_function_handle() {
            return h.display_str();
        }
        format_matrix(self, mode)
    }

    /// Convenience: format using `format short`.
    #[must_use]
    pub fn display(&self) -> String {
        self.format(FormatMode::Short)
    }

    /// Char value as the literal text, only when it is a row vector / scalar.
    fn as_string_for_display(&self) -> Option<String> {
        if !self.is_char() {
            return None;
        }
        let dims = self.dims();
        // 1xN (or scalar char) prints as plain text.
        if dims.len() == 2 && dims[0] <= 1 {
            return self.as_string();
        }
        None
    }
}

/// Format a single scalar value.
fn format_scalar(s: ScalarValue, mode: FormatMode) -> String {
    match s {
        ScalarValue::Bool(v) => (if v { "1" } else { "0" }).to_string(),
        ScalarValue::Char(c) => c.to_string(),
        ScalarValue::Int8(v) => v.to_string(),
        ScalarValue::UInt8(v) => v.to_string(),
        ScalarValue::Int16(v) => v.to_string(),
        ScalarValue::UInt16(v) => v.to_string(),
        ScalarValue::Int32(v) => v.to_string(),
        ScalarValue::UInt32(v) => v.to_string(),
        ScalarValue::Int64(v) => v.to_string(),
        ScalarValue::UInt64(v) => v.to_string(),
        ScalarValue::Float(v) => format_real(f64::from(v), mode),
        ScalarValue::Double(v) => format_real(v, mode),
        ScalarValue::Complex32(v) => format_complex(f64::from(v.re), f64::from(v.im), mode),
        ScalarValue::Complex64(v) => format_complex(v.re, v.im, mode),
    }
}

/// Format one real `f64` the MATLAB way (integers without a decimal point).
fn format_real(v: f64, mode: FormatMode) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    if v.is_nan() {
        return "NaN".to_string();
    }
    if v.is_infinite() {
        return if v < 0.0 { "-Inf" } else { "Inf" }.to_string();
    }
    let d = mode.decimals();
    // The `e` modes always use exponential notation, even for integer values.
    if mode.always_exponential() {
        let s = format!("{v:.*e}", d);
        return normalize_exp(&s);
    }
    // The `g` modes use the compact `%g`-style presentation.
    if mode.compact() {
        return format_compact(v, d);
    }
    if is_integer_valued(v) && v.abs() < 1e15 {
        // Print as an integer.
        return format!("{}", v as i64);
    }
    let mag = v.abs();
    // Use exponential notation for very large/small magnitudes, like MATLAB.
    if !(1e-5..1e5).contains(&mag) {
        // e.g. 1.2346e+07
        let s = format!("{v:.*e}", d);
        normalize_exp(&s)
    } else {
        format!("{v:.*}", d)
    }
}

/// Compact (`%g`-style) rendering used by `format short g` / `long g`:
/// `d` significant digits, trailing zeros dropped, exponential only when the
/// magnitude is very large or very small.
fn format_compact(v: f64, d: usize) -> String {
    if is_integer_valued(v) && v.abs() < 1e15 {
        return format!("{}", v as i64);
    }
    let mag = v.abs();
    if !(1e-5..1e5).contains(&mag) {
        // Significant-digit exponential, trailing zeros trimmed.
        let s = format!("{v:.*e}", d.saturating_sub(1));
        let (mantissa, exp) = s.split_once('e').unwrap_or((s.as_str(), ""));
        let mantissa = trim_trailing_zeros(mantissa);
        normalize_exp(&format!("{mantissa}e{exp}"))
    } else {
        let s = format!("{v:.*}", d);
        trim_trailing_zeros(&s)
    }
}

/// Drop trailing zeros (and a dangling decimal point) from a fixed-point string.
fn trim_trailing_zeros(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    let trimmed = s.trim_end_matches('0');
    trimmed.trim_end_matches('.').to_string()
}

/// Format a complex value as `re + imi` / `re - imi`.
fn format_complex(re: f64, im: f64, mode: FormatMode) -> String {
    let re_s = format_real(re, mode);
    let im_abs = format_real(im.abs(), mode);
    let sign = if im < 0.0 || (im == 0.0 && im.is_sign_negative()) {
        "-"
    } else {
        "+"
    };
    format!("{re_s} {sign} {im_abs}i")
}

/// Normalize Rust's exponent (`1.23e7`) into MATLAB's `1.2300e+07` shape:
/// always a sign and at least two exponent digits.
fn normalize_exp(s: &str) -> String {
    let Some((mantissa, exp)) = s.split_once('e') else {
        return s.to_string();
    };
    let (sign, digits) = match exp.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("+", exp.strip_prefix('+').unwrap_or(exp)),
    };
    format!("{mantissa}e{sign}{digits:0>2}")
}

/// Whether `v` is mathematically an integer (used to drop the decimal point).
fn is_integer_valued(v: f64) -> bool {
    v.is_finite() && v.fract() == 0.0
}

/// Format a 2D (or higher) numeric matrix.
fn format_matrix(a: &Array, mode: FormatMode) -> String {
    let dims = a.dims();
    // For >2D, print page by page ("ans(:,:,k) ="-style header).
    if dims.len() > 2 {
        return format_nd(a, &dims, mode);
    }
    let rows = dims[0];
    let cols = dims[1];
    let cells = render_cells(a, mode);
    // Column width = max rendered width in that column-major layout.
    let width = cells.iter().map(String::len).max().unwrap_or(1);
    let mut out = String::new();
    for r in 0..rows {
        for c in 0..cols {
            let idx = c * rows + r; // column-major flat index
            let _ = write!(out, "   {:>width$}", cells[idx]);
        }
        out.push('\n');
    }
    // Trim the single trailing newline to keep the body clean.
    if out.ends_with('\n') {
        out.pop();
    }
    out
}

/// Higher-dimensional arrays: emit each 2D page with a header.
fn format_nd(a: &Array, dims: &[usize], mode: FormatMode) -> String {
    let rows = dims[0];
    let cols = dims[1];
    let page_size = rows * cols;
    let pages: usize = dims[2..].iter().product();
    let cells = render_cells(a, mode);
    let width = cells.iter().map(String::len).max().unwrap_or(1);
    let mut out = String::new();
    for p in 0..pages {
        // Build the trailing subscripts for the header.
        let mut subs = Vec::new();
        let mut rem = p;
        for &d in &dims[2..] {
            subs.push(rem % d + 1);
            rem /= d;
        }
        let subs_str: Vec<String> = subs.iter().map(usize::to_string).collect();
        let _ = writeln!(out, "(:,:,{}) =", subs_str.join(","));
        out.push('\n');
        let base = p * page_size;
        for r in 0..rows {
            for c in 0..cols {
                let idx = base + c * rows + r;
                let _ = write!(out, "   {:>width$}", cells[idx]);
            }
            out.push('\n');
        }
        out.push('\n');
    }
    while out.ends_with('\n') {
        out.pop();
    }
    out
}

/// Render every element of a numeric array to its display string, column-major.
fn render_cells(a: &Array, mode: FormatMode) -> Vec<String> {
    let n = a.numel();
    let mut cells = Vec::with_capacity(n);
    // Iterate in column-major order to match storage and MATLAB display.
    let dims = a.dims();
    for flat in col_major_indices(&dims) {
        let s = scalar_at(a, flat).map_or_else(|| String::from("?"), |sv| format_scalar(sv, mode));
        cells.push(s);
    }
    cells
}

/// Read the element at column-major flat index `flat` as a [`ScalarValue`].
fn scalar_at(a: &Array, flat: usize) -> Option<ScalarValue> {
    macro_rules! at {
        ($arr:expr, $ctor:path) => {{
            // ndarray iterates in logical (row-major) order regardless of memory
            // order, but `.t()`-free flat indexing via `as_slice_memory_order`
            // gives us the column-major buffer directly.
            $arr.as_slice_memory_order()
                .and_then(|s| s.get(flat).copied())
                .map($ctor)
        }};
    }
    match a {
        Array::Scalar(s) => (flat == 0).then_some(*s),
        Array::Bool(x) => at!(x, ScalarValue::Bool),
        Array::Int8(x) => at!(x, ScalarValue::Int8),
        Array::UInt8(x) => at!(x, ScalarValue::UInt8),
        Array::Int16(x) => at!(x, ScalarValue::Int16),
        Array::UInt16(x) => at!(x, ScalarValue::UInt16),
        Array::Int32(x) => at!(x, ScalarValue::Int32),
        Array::UInt32(x) => at!(x, ScalarValue::UInt32),
        Array::Int64(x) => at!(x, ScalarValue::Int64),
        Array::UInt64(x) => at!(x, ScalarValue::UInt64),
        Array::Float(x) => at!(x, ScalarValue::Float),
        Array::Double(x) => at!(x, ScalarValue::Double),
        Array::Complex32(x) => at!(x, ScalarValue::Complex32),
        Array::Complex64(x) => at!(x, ScalarValue::Complex64),
        Array::Char(x) => at!(x, ScalarValue::Char),
        Array::Cell(_) | Array::Struct(_) | Array::Sparse(_) | Array::FunctionHandle(_) => None,
    }
}

/// Format a sparse matrix the way MATLAB/FreeMat does: a list of
/// `(i,j)  value` triplets in column-major order (1-based indices).
fn format_sparse(s: &crate::sparse::SparseMatrix, mode: FormatMode) -> String {
    let (i, j, v, im) = s.find_triplets();
    if i.is_empty() {
        return format!("All zero sparse: {}-by-{}", s.rows(), s.cols());
    }
    let label_w = i
        .iter()
        .zip(&j)
        .map(|(&ii, &jj)| format!("({},{})", ii as usize, jj as usize).len())
        .max()
        .unwrap_or(0);
    let mut out = String::new();
    for k in 0..i.len() {
        let label = format!("({},{})", i[k] as usize, j[k] as usize);
        let val = if let Some(iv) = &im {
            format_scalar(ScalarValue::Complex64(crate::C64::new(v[k], iv[k])), mode)
        } else {
            format_scalar(ScalarValue::Double(v[k]), mode)
        };
        let _ = writeln!(out, "  {label:<label_w$}      {val}");
    }
    // Drop the trailing newline to match the value-body convention.
    if out.ends_with('\n') {
        out.pop();
    }
    out
}

/// Column-major flat indices `0..numel` for the given `dims`. Because storage
/// is column-major, the flat memory index *is* the column-major index, so this
/// is just `0..product(dims)`.
fn col_major_indices(dims: &[usize]) -> std::ops::Range<usize> {
    0..dims.iter().product::<usize>()
}

/// Format a cell array as `{ ... }` rows of element summaries.
fn format_cell(cells: &ndarray::ArrayD<Array>, dims: &[usize], mode: FormatMode) -> String {
    let rows = *dims.first().unwrap_or(&0);
    let cols = dims.get(1).copied().unwrap_or(0);
    let flat: Vec<&Array> = cells
        .as_slice_memory_order()
        .map(|s| s.iter().collect())
        .unwrap_or_default();
    let mut out = String::new();
    out.push_str("{\n");
    for r in 0..rows {
        for c in 0..cols {
            let idx = c * rows + r;
            if let Some(el) = flat.get(idx) {
                let _ = write!(out, "  [{}]", cell_summary(el, mode));
            }
        }
        out.push('\n');
    }
    out.push('}');
    out
}

/// A short one-line summary for a cell element.
fn cell_summary(el: &Array, mode: FormatMode) -> String {
    if el.is_scalar() && el.class().is_numeric() {
        el.format(mode)
    } else if el.is_char() {
        format!("'{}'", el.as_string().unwrap_or_default())
    } else {
        let dims = el
            .dims()
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join("x");
        format!("{dims} {}", el.class_name())
    }
}

/// Format a struct array as a field listing.
fn format_struct(st: &crate::struct_array::StructArray) -> String {
    let dims = st
        .dims()
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join("x");
    let mut out = format!("{dims} struct array with fields:\n");
    for name in st.field_names() {
        let _ = writeln!(out, "    {name}");
    }
    if out.ends_with('\n') {
        out.pop();
    }
    out
}
