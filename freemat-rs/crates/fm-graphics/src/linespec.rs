//! MATLAB linespec parsing (`'r--o'` → color/style/marker), matching FreeMat's
//! `islinespec` semantics in `toolbox/graph`.

/// A parsed MATLAB linespec: color, line-style, marker (any may be empty).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineSpec {
    /// `rgb(...)` color string, or empty if the spec named no color.
    pub color: String,
    /// Line style (`-`, `--`, `:`, `-.`), or empty.
    pub line_style: String,
    /// Marker symbol (`o`, `+`, `*`, `.`, `x`, `s`, `d`, `^`, `v`, `>`, `<`,
    /// `p`, `h`), or empty.
    pub marker: String,
    /// True if every char was a valid linespec char (so the caller can decide
    /// it really *was* a linespec and not, say, a property name).
    pub valid: bool,
}

/// Map a single MATLAB color letter to a CSS `rgb(...)` string.
fn color_letter(c: char) -> Option<&'static str> {
    Some(match c {
        'b' => "rgb(0,0,255)",
        'g' => "rgb(0,128,0)",
        'r' => "rgb(255,0,0)",
        'c' => "rgb(0,191,191)",
        'm' => "rgb(191,0,191)",
        'y' => "rgb(191,191,0)",
        'k' => "rgb(0,0,0)",
        'w' => "rgb(255,255,255)",
        _ => return None,
    })
}

const MARKERS: &[char] = &[
    'o', '+', '*', '.', 'x', 's', 'd', '^', 'v', '>', '<', 'p', 'h',
];

/// Parse a linespec string (e.g. `"r--o"`, `":g"`, `"*"`).
///
/// Sets `valid = false` if any character is not a recognized
/// color/style/marker, so the caller can reject non-linespec strings (FreeMat's
/// `plot.m` only treats an argument as a linespec when `islinespec` is true).
#[must_use]
pub fn parse(s: &str) -> LineSpec {
    let mut spec = LineSpec {
        valid: true,
        ..Default::default()
    };
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // Two-char line styles first (`--`, `-.`), then single `-`, `:`.
        if c == '-' && i + 1 < chars.len() && (chars[i + 1] == '-' || chars[i + 1] == '.') {
            spec.line_style = chars[i..i + 2].iter().collect();
            i += 2;
            continue;
        }
        if c == '-' {
            spec.line_style = "-".into();
        } else if c == ':' {
            spec.line_style = ":".into();
        } else if let Some(rgb) = color_letter(c) {
            spec.color = rgb.into();
        } else if MARKERS.contains(&c) {
            spec.marker = c.to_string();
        } else {
            spec.valid = false;
        }
        i += 1;
    }
    spec
}
