//! `sscanf` / `fscanf` — parse formatted input.
//!
//! A pragmatic implementation covering the common numeric (`%d %i %u %f %e %g`)
//! and string (`%s %c`) conversions used by the FreeMat test corpus. The format
//! string is applied repeatedly over the input (MATLAB recycling) until the
//! input is exhausted, collecting numeric results into a column vector.

use fm_core::Array;
use fm_interp::error::Flow;

/// `sscanf(str, fmt)` → a numeric column vector (or char row for `%s`/`%c`).
pub fn sscanf(input: &str, fmt: &str) -> Flow<Vec<Array>> {
    let conversions = parse_format(fmt);
    let mut numbers: Vec<f64> = Vec::new();
    let mut strings: Vec<String> = Vec::new();
    let mut has_str = false;

    let mut rest = input;
    // Apply the conversion list repeatedly over the input.
    loop {
        let before = rest.len();
        let mut made_progress = false;
        for conv in &conversions {
            match conv {
                Conv::Literal(s) => {
                    rest = rest.trim_start();
                    if let Some(stripped) = rest.strip_prefix(s.as_str()) {
                        rest = stripped;
                    }
                }
                Conv::Whitespace => {
                    rest = rest.trim_start_matches([' ', '\t', '\n', '\r']);
                }
                Conv::Number => {
                    rest = rest.trim_start();
                    let (val, tail) = scan_number(rest);
                    match val {
                        Some(v) => {
                            numbers.push(v);
                            rest = tail;
                            made_progress = true;
                        }
                        None => {
                            return Ok(vec![finish(numbers, strings, has_str)]);
                        }
                    }
                }
                Conv::Str => {
                    rest = rest.trim_start();
                    let end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
                    if end == 0 {
                        return Ok(vec![finish(numbers, strings, has_str)]);
                    }
                    strings.push(rest[..end].to_string());
                    has_str = true;
                    rest = &rest[end..];
                    made_progress = true;
                }
                Conv::Char => {
                    if let Some(c) = rest.chars().next() {
                        strings.push(c.to_string());
                        has_str = true;
                        rest = &rest[c.len_utf8()..];
                        made_progress = true;
                    } else {
                        return Ok(vec![finish(numbers, strings, has_str)]);
                    }
                }
            }
        }
        // Stop if the whole input was consumed or no conversion advanced.
        if rest.trim().is_empty() || (!made_progress && rest.len() == before) {
            break;
        }
        if rest.len() == before {
            break;
        }
    }
    Ok(vec![finish(numbers, strings, has_str)])
}

fn finish(numbers: Vec<f64>, strings: Vec<String>, has_str: bool) -> Array {
    if has_str && numbers.is_empty() {
        return Array::char_string(&strings.concat());
    }
    let n = numbers.len();
    Array::double_matrix(&[n, 1], numbers)
}

/// Scan a leading numeric token from `s`; return the value and the remainder.
fn scan_number(s: &str) -> (Option<f64>, &str) {
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut seen_digit = false;
    let mut seen_dot = false;
    let mut seen_exp = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_digit() {
            seen_digit = true;
            i += 1;
        } else if c == b'.' && !seen_dot && !seen_exp {
            seen_dot = true;
            i += 1;
        } else if (c == b'e' || c == b'E') && seen_digit && !seen_exp {
            seen_exp = true;
            i += 1;
            if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
                i += 1;
            }
        } else {
            break;
        }
    }
    if !seen_digit {
        return (None, s);
    }
    match s[..i].parse::<f64>() {
        Ok(v) => (Some(v), &s[i..]),
        Err(_) => (None, s),
    }
}

enum Conv {
    Number,
    Str,
    Char,
    Whitespace,
    Literal(String),
}

/// Parse a scanf format string into a list of conversions.
fn parse_format(fmt: &str) -> Vec<Conv> {
    let mut out = Vec::new();
    let mut chars = fmt.chars().peekable();
    let mut literal = String::new();
    let flush = |out: &mut Vec<Conv>, literal: &mut String| {
        if !literal.is_empty() {
            out.push(Conv::Literal(std::mem::take(literal)));
        }
    };
    while let Some(c) = chars.next() {
        if c == '%' {
            flush(&mut out, &mut literal);
            // Skip width/flags until the conversion letter.
            let mut spec = String::new();
            while let Some(&nc) = chars.peek() {
                if nc.is_ascii_alphabetic() || nc == '%' {
                    break;
                }
                spec.push(nc);
                chars.next();
            }
            match chars.next() {
                Some('d' | 'i' | 'u' | 'f' | 'e' | 'g' | 'E' | 'G' | 'x' | 'o') => {
                    out.push(Conv::Number);
                }
                Some('s') => out.push(Conv::Str),
                Some('c') => out.push(Conv::Char),
                Some('%') => literal.push('%'),
                _ => {}
            }
        } else if c.is_whitespace() {
            flush(&mut out, &mut literal);
            out.push(Conv::Whitespace);
        } else {
            literal.push(c);
        }
    }
    flush(&mut out, &mut literal);
    out
}
