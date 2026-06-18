//! `regexp` / `regexpi` / `regexprep` via the [`regex`] crate.
//!
//! Translates the common MATLAB options. Default `regexp(str, pat)` returns the
//! 1-based start indices of each match. Option keywords select alternative
//! outputs: `'start'`, `'end'`, `'match'`, `'tokens'`, `'split'`, plus `'once'`
//! (return the first match only, unwrapped). `regexprep(str, pat, repl)`
//! substitutes, translating MATLAB `$N` backreferences to the regex-crate form.

use fm_core::Array;
use fm_interp::error::{Flow, InterpError, Signal};
use regex::Regex;

fn rerr<T>(msg: impl Into<String>) -> Flow<T> {
    Err(Signal::Error(InterpError::msg(msg)))
}

fn str_of(a: &Array) -> String {
    a.as_string().unwrap_or_default()
}

fn compile(pat: &str, case_insensitive: bool) -> Flow<Regex> {
    let prefix = if case_insensitive { "(?i)" } else { "" };
    Regex::new(&format!("{prefix}{pat}"))
        .map_err(|e| Signal::Error(InterpError::msg(format!("regexp: invalid pattern: {e}"))))
}

/// Map a byte offset into `s` to a 1-based character index (MATLAB indices are
/// over characters, but the corpus is ASCII so this is also the byte index +1).
fn char_index(s: &str, byte_off: usize) -> f64 {
    (s[..byte_off].chars().count() + 1) as f64
}

/// `regexp(str, pat [, opts...])` / `regexpi` (case-insensitive).
pub fn regexp(args: &[Array], case_insensitive: bool, nargout: usize) -> Flow<Vec<Array>> {
    if args.len() < 2 {
        return rerr("regexp: expected (str, pattern)");
    }
    let s = str_of(&args[0]);
    let pat = str_of(&args[1]);
    let re = compile(&pat, case_insensitive)?;

    // Collect option keywords.
    let mut once = false;
    let mut selectors: Vec<String> = Vec::new();
    for a in &args[2..] {
        if let Some(opt) = a.as_string() {
            match opt.as_str() {
                "once" => once = true,
                "start" | "end" | "match" | "tokens" | "split" | "names" => selectors.push(opt),
                _ => {}
            }
        }
    }

    // Gather matches.
    let matches: Vec<regex::Captures<'_>> = if once {
        re.captures(&s).into_iter().collect()
    } else {
        re.captures_iter(&s).collect()
    };

    let build_selector = |sel: &str| -> Array {
        match sel {
            "start" => {
                let v: Vec<f64> = matches
                    .iter()
                    .map(|c| char_index(&s, c.get(0).unwrap().start()))
                    .collect();
                if once {
                    v.first().map_or_else(Array::empty, |&x| Array::double(x))
                } else {
                    Array::double_matrix(&[1, v.len()], v)
                }
            }
            "end" => {
                let v: Vec<f64> = matches
                    .iter()
                    .map(|c| char_index(&s, c.get(0).unwrap().end()) - 1.0)
                    .collect();
                if once {
                    v.first().map_or_else(Array::empty, |&x| Array::double(x))
                } else {
                    Array::double_matrix(&[1, v.len()], v)
                }
            }
            "match" => {
                let v: Vec<Array> = matches
                    .iter()
                    .map(|c| Array::char_string(c.get(0).unwrap().as_str()))
                    .collect();
                if once {
                    v.into_iter()
                        .next()
                        .unwrap_or_else(|| Array::char_string(""))
                } else {
                    Array::cell(&[1, v.len()], v)
                }
            }
            "tokens" => {
                let toks: Vec<Array> = matches
                    .iter()
                    .map(|c| {
                        let groups: Vec<Array> = (1..c.len())
                            .map(|g| Array::char_string(c.get(g).map_or("", |m| m.as_str())))
                            .collect();
                        let n = groups.len();
                        Array::cell(&[1, n], groups)
                    })
                    .collect();
                if once {
                    toks.into_iter()
                        .next()
                        .unwrap_or_else(|| Array::cell(&[1, 0], vec![]))
                } else {
                    Array::cell(&[1, toks.len()], toks)
                }
            }
            "split" => {
                let parts: Vec<Array> = re.split(&s).map(Array::char_string).collect();
                Array::cell(&[1, parts.len()], parts)
            }
            _ => Array::empty(),
        }
    };

    if !selectors.is_empty() {
        // Explicit selectors: return them in the order requested.
        return Ok(selectors.iter().map(|s| build_selector(s)).collect());
    }

    // Default multi-output order: start, end, te, match, tokens, names, split.
    let defaults = ["start", "end", "match", "tokens", "split"];
    let want = nargout.max(1).min(defaults.len());
    Ok(defaults[..want].iter().map(|s| build_selector(s)).collect())
}

/// `regexprep(str, pat, repl [, opts])`.
pub fn regexprep(args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.len() < 3 {
        return rerr("regexprep: expected (str, pattern, replacement)");
    }
    let s = str_of(&args[0]);
    let pat = str_of(&args[1]);
    let repl = translate_replacement(&str_of(&args[2]));

    let mut case_insensitive = false;
    let mut once = false;
    for a in &args[3..] {
        if let Some(opt) = a.as_string() {
            match opt.as_str() {
                "ignorecase" => case_insensitive = true,
                "once" => once = true,
                _ => {}
            }
        }
    }
    let re = compile(&pat, case_insensitive)?;
    let out = if once {
        re.replacen(&s, 1, repl.as_str()).into_owned()
    } else {
        re.replace_all(&s, repl.as_str()).into_owned()
    };
    Ok(vec![Array::char_string(&out)])
}

/// Translate MATLAB `$1` backreferences to the regex-crate `${1}` form (so a
/// following digit doesn't merge into the group number). `$0` → `${0}`.
fn translate_replacement(repl: &str) -> String {
    let mut out = String::with_capacity(repl.len());
    let mut chars = repl.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '$' {
            if chars.peek() == Some(&'$') {
                out.push_str("$$");
                chars.next();
                continue;
            }
            let mut num = String::new();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    num.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            if num.is_empty() {
                out.push('$');
            } else {
                out.push_str(&format!("${{{num}}}"));
            }
        } else {
            out.push(c);
        }
    }
    out
}
