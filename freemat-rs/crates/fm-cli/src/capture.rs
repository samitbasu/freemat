//! Fragment capture engine (help-system phase P2).
//!
//! Runs a scripted REPL session headlessly and captures the resulting
//! transcript **byte-for-byte identical** to what a user sees typing the same
//! lines into the live `fm` REPL. This is the engine `cargo xtask docgen`
//! (phase P3) drives to fill `fm-exec` / `fm-exec:figure` doc fragments with
//! real interpreter output.
//!
//! # Byte-for-byte REPL parity
//! The live REPL ([`crate::main`]'s `eval_line`) does exactly two things per
//! line: it prints [`Interpreter::take_output`] (which already contains the
//! `ans = …` / `<var> = …` displays produced by `Interpreter::echo` →
//! `Array::format`), and on error it renders the [`InterpError`] with
//! `miette`'s `GraphicalReportHandler` themed `unicode()`. This module reuses
//! **that exact code path** — it never re-implements value/matrix/number
//! formatting — and prefixes each echoed input line with the REPL prompt
//! `--> `. See `crates/fm-interp/src/interp.rs:1403` (`Interpreter::echo`) for
//! the display source of truth and `crates/fm-cli/src/main.rs` (`eval_line`)
//! for the REPL path this mirrors.
//!
//! # Determinism
//! `rand` / `randn` / `randi` draw from a thread-local seeded `StdRng` once the
//! `seed` builtin has been called (see `crates/fm-builtins/src/random.rs`).
//! Capture runs `seed(1,0);` as the very first statement of every session, so
//! fragments that use random numbers produce identical transcripts on every
//! run. (`tic`/`toc`/`clock`/`now` are not stubbed here — docs are expected to
//! avoid them, per `docs/HELP_SYSTEM.md` §4.3.)

//! The shared `FragmentScript` / `CapturedFragment` model is owned by `fm-doc`
//! (the single source of truth — it is what gets serialized into the generated
//! fragment DB). This module re-uses those types directly; there is no local
//! copy. The `fm --capture-fragment` JSON path and the in-process
//! [`run_fragment`] API are unchanged.

use std::fs;
use std::path::PathBuf;

use fm_doc::{CapturedFragment, FragmentScript};
use fm_interp::Interpreter;
use miette::{GraphicalReportHandler, GraphicalTheme};

/// The REPL prompt prefix used to echo input lines, matching `fm`'s prompt.
const PROMPT: &str = "--> ";

/// The fixed RNG seed installed at the start of every capture session, so
/// `rand`/`randn`/`randi` are reproducible. Mirrors a user typing `seed(1,0)`.
const SEED_PRELUDE: &str = "seed(1,0);";

/// Serializes the process-global working-directory mutation inside
/// [`run_fragment`] so concurrent callers (notably the parallel capture unit
/// tests) don't race on the shared cwd. See its acquisition site for the exact
/// race this prevents.
static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Run a fragment script headlessly and capture its REPL transcript.
///
/// This is the in-process library entry point (used by xtask in P3). It:
/// 1. builds a headless [`Interpreter`] (full standard library, **no** graphics
///    webserver — graphics builtins still mutate the retained scene, they just
///    don't broadcast),
/// 2. seeds the RNG for determinism,
/// 3. writes the `files` into a temp dir and makes that dir the working
///    directory so `.m` aux files and relative reads resolve, loading any `.m`
///    files so their functions become callable,
/// 4. runs each input block, echoing `--> <line>` per physical line and
///    appending the interpreter's output / rendered error,
/// 5. captures the figure `Scene` JSON if requested.
///
/// The transcript matches the live REPL byte-for-byte (same display path, same
/// error renderer); per-line trailing whitespace is trimmed and the transcript
/// ends with exactly one newline.
#[must_use]
pub fn run_fragment(script: &FragmentScript) -> CapturedFragment {
    // Staging chdir's the *process-global* current directory, so concurrent
    // `run_fragment` calls would race on it. The failure is subtle: one thread's
    // `remove_dir_all` (staging cleanup) can unlink the directory another thread
    // is currently `cwd`'d into, so that thread's next `current_dir()` fails with
    // `ENOENT`, `stage_files` bails to `None`, and its aux `.m` files silently
    // never load. Serialize the whole chdir'd section behind a process-global
    // lock. `docgen` runs fragments sequentially, so this is uncontended in
    // production; it only orders the parallel capture unit tests. A poisoned lock
    // (from a prior panic) is tolerated — we guard only the cwd, no shared data.
    let _cwd_guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let mut interp = Interpreter::new();
    fm_builtins::register_standard_library(&mut interp);

    // Determinism: seed before anything user-visible runs. Suppressed, so it
    // contributes no transcript output.
    let _ = interp.run(SEED_PRELUDE);
    let _ = interp.take_output();

    // Stage the aux files in a unique temp dir and chdir into it so relative
    // `.m` reads (e.g. `source`/file builtins) and function loading resolve.
    let staging = stage_files(script);

    let reporter = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());
    let mut transcript = String::new();
    let mut error_count = 0usize;

    // Load any `.m` aux files so their functions / scripts are callable in the
    // session. A `.m` file is either a function file (registers its functions)
    // or a script file (statements). `load_file` ignores script files, so we
    // register those separately under their base name so invoking the name runs
    // the script in the caller's workspace.
    if let Some(dir) = staging.as_ref() {
        for (name, body) in &script.files {
            if name.ends_with(".m") {
                let _ = interp.load_file(dir.join(name));
                let base = name.strip_suffix(".m").unwrap_or(name);
                let base = std::path::Path::new(base)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(base);
                let _ = interp.register_script(base, body);
            }
        }
    }

    for block in &script.inputs {
        for chunk in logical_chunks(block) {
            // Echo every physical line of the chunk with the REPL prompt.
            for line in chunk.lines() {
                transcript.push_str(PROMPT);
                transcript.push_str(line);
                transcript.push('\n');
            }
            match interp.run(&chunk) {
                Ok(()) => {
                    transcript.push_str(&interp.take_output());
                }
                Err(e) => {
                    // Print anything emitted before the error, then the
                    // diagnostic — exactly as the REPL's `eval_line` does.
                    transcript.push_str(&interp.take_output());
                    let mut buf = String::new();
                    if reporter.render_report(&mut buf, &e).is_ok() {
                        transcript.push_str(&buf);
                    } else {
                        transcript.push_str(&format!("error: {e}\n"));
                    }
                    error_count += 1;
                }
            }
        }
    }

    let figure = if script.want_figure {
        capture_figure(&interp)
    } else {
        None
    };

    // Restore the working directory and clean up the staging dir.
    if let Some(dir) = staging {
        let _ = std::env::set_current_dir(dir.prev());
        dir.cleanup();
    }

    CapturedFragment {
        transcript: normalize(&transcript),
        error_count,
        figure,
    }
}

/// Capture the retained graphics scene as Plotly wire JSON, if non-empty.
///
/// No network sink is involved: graphics builtins mutate `interp.graphics.scene`
/// directly, so we serialize that snapshot via the same `Scene::to_message()`
/// the webserver uses.
fn capture_figure(interp: &Interpreter) -> Option<String> {
    let scene = &interp.graphics.scene;
    if scene.figures.is_empty() {
        return None;
    }
    scene.to_message().ok()
}

/// Normalize a raw transcript per §4.2: trim trailing whitespace on every line,
/// then ensure the whole thing ends with exactly one newline.
fn normalize(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for line in raw.split_inclusive('\n') {
        let had_newline = line.ends_with('\n');
        let body = line.strip_suffix('\n').unwrap_or(line);
        out.push_str(body.trim_end());
        if had_newline {
            out.push('\n');
        }
    }
    // Collapse trailing blank lines to a single terminating newline.
    let trimmed = out.trim_end_matches('\n');
    if trimmed.is_empty() {
        String::new()
    } else {
        let mut s = trimmed.to_string();
        s.push('\n');
        s
    }
}

/// Split an `fm-exec` block into logical chunks runnable one at a time, so that
/// output interleaves with input the way the live REPL does for the common
/// one-statement-per-line case, while multi-line block constructs
/// (`for`/`while`/`if`/`switch`/`try`/`function`/`parfor`) and `...`
/// continuations are grouped and run as a single unit.
///
/// Grouping is purely lexical (keyword/`end` nesting depth), so it needs no
/// dependency on the parser. A line that opens a block raises the depth; an
/// `end`/`endfor`/… closes it; while depth > 0 (or a line ends with `...`),
/// lines accumulate into the current chunk.
fn logical_chunks(block: &str) -> Vec<String> {
    let mut chunks: Vec<String> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    let mut depth: i32 = 0;

    for line in block.lines() {
        current.push(line);
        depth += block_delta(line);
        // A trailing `...` continues onto the next physical line.
        let continued = line.trim_end().ends_with("...");
        if depth <= 0 && !continued {
            depth = 0;
            chunks.push(current.join("\n"));
            current.clear();
        }
    }
    if !current.is_empty() {
        chunks.push(current.join("\n"));
    }
    chunks
}

/// The net block-nesting change a line contributes: openers `+1`, closers `-1`.
/// Ignores keywords inside strings/comments only crudely (good enough for doc
/// fragments, which are well-formed); the worst case merely groups extra lines
/// into one chunk, which still runs correctly.
fn block_delta(line: &str) -> i32 {
    let code = strip_comment(line);
    let mut delta = 0i32;
    for tok in tokenize_words(&code) {
        match tok {
            "for" | "parfor" | "while" | "if" | "switch" | "try" | "function" => delta += 1,
            "end" | "endfor" | "endwhile" | "endif" | "endswitch" | "end_try_catch"
            | "endfunction" => delta -= 1,
            _ => {}
        }
    }
    delta
}

/// Drop a trailing `%`/`#` line comment (outside of strings — approximated by
/// honoring single/double quotes).
fn strip_comment(line: &str) -> String {
    let mut out = String::new();
    let mut in_s = false;
    let mut in_d = false;
    for c in line.chars() {
        match c {
            '\'' if !in_d => in_s = !in_s,
            '"' if !in_s => in_d = !in_d,
            '%' | '#' if !in_s && !in_d => break,
            _ => {}
        }
        out.push(c);
    }
    out
}

/// Split a code line into identifier-like words for keyword detection.
fn tokenize_words(code: &str) -> Vec<&str> {
    code.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| !w.is_empty())
        .collect()
}

/// A staged temp directory holding the fragment's aux files; restores the
/// previous working directory and removes itself on [`Self::cleanup`].
struct Staging {
    dir: PathBuf,
    prev: PathBuf,
}

impl Staging {
    fn join(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
    fn prev(&self) -> &PathBuf {
        &self.prev
    }
    fn cleanup(self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Create a fresh temp dir, write the fragment's `files` into it, and chdir
/// there. Done for *every* fragment (even file-less ones) so that fragments
/// which write files (`save`, `csvwrite`, `fwrite`, …) never pollute the
/// caller's working directory (e.g. the repo root during `docgen`). Returns
/// `None` only if the temp dir can't be created, in which case capture falls
/// back to the current dir.
fn stage_files(script: &FragmentScript) -> Option<Staging> {
    let prev = std::env::current_dir().ok()?;
    // Unique temp dir without an external crate: temp_dir + pid + a counter.
    let base = std::env::temp_dir();
    let unique = format!("fm-capture-{}-{}", std::process::id(), next_counter());
    let dir = base.join(unique);
    fs::create_dir_all(&dir).ok()?;
    for (name, body) in &script.files {
        // Guard against path escapes: only the final component is honored.
        let safe = std::path::Path::new(name)
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(name));
        let _ = fs::write(dir.join(safe), body);
    }
    if std::env::set_current_dir(&dir).is_err() {
        let _ = fs::remove_dir_all(&dir);
        return None;
    }
    Some(Staging { dir, prev })
}

/// A process-local monotonic counter for unique staging dir names.
fn next_counter() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static C: AtomicU64 = AtomicU64::new(0);
    C.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The capture path and the live-REPL display path must agree byte for
    /// byte. Both ultimately run `interp.run` + `interp.take_output`, so we
    /// drive a fresh interpreter the same way `eval_line` does and compare.
    fn repl_reference(lines: &[&str]) -> String {
        let mut interp = Interpreter::new();
        fm_builtins::register_standard_library(&mut interp);
        let _ = interp.run(SEED_PRELUDE);
        let _ = interp.take_output();
        let reporter = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());
        let mut out = String::new();
        for line in lines {
            out.push_str(PROMPT);
            out.push_str(line);
            out.push('\n');
            match interp.run(line) {
                Ok(()) => out.push_str(&interp.take_output()),
                Err(e) => {
                    out.push_str(&interp.take_output());
                    let mut buf = String::new();
                    let _ = reporter.render_report(&mut buf, &e);
                    out.push_str(&buf);
                }
            }
        }
        normalize(&out)
    }

    fn capture_lines(lines: &[&str]) -> CapturedFragment {
        run_fragment(&FragmentScript {
            files: vec![],
            inputs: vec![lines.join("\n")],
            expect_errors: 0,
            want_figure: false,
        })
    }

    #[test]
    fn scalar_display_matches_repl() {
        let lines = ["x = 3"];
        let got = capture_lines(&lines);
        assert_eq!(got.transcript, repl_reference(&lines));
        assert!(got.transcript.contains("--> x = 3"));
        assert!(got.transcript.contains("x ="));
        assert_eq!(got.error_count, 0);
    }

    #[test]
    fn matrix_display_matches_repl() {
        let lines = ["y = [1 2; 3 4]"];
        let got = capture_lines(&lines);
        assert_eq!(got.transcript, repl_reference(&lines));
        assert!(got.transcript.contains("y ="));
    }

    #[test]
    fn complex_display_matches_repl() {
        let lines = ["z = 1 + 2i"];
        let got = capture_lines(&lines);
        assert_eq!(got.transcript, repl_reference(&lines));
    }

    #[test]
    fn string_display_matches_repl() {
        let lines = ["s = 'hello'"];
        let got = capture_lines(&lines);
        assert_eq!(got.transcript, repl_reference(&lines));
        assert!(got.transcript.contains("hello"));
    }

    #[test]
    fn suppressed_line_shows_no_value() {
        let got = capture_lines(&["w = 42;"]);
        assert_eq!(got.transcript, "--> w = 42;\n");
        assert_eq!(got.error_count, 0);
    }

    #[test]
    fn multi_output_matches_repl() {
        let lines = ["[a, b] = size([1 2 3])"];
        let got = capture_lines(&lines);
        assert_eq!(got.transcript, repl_reference(&lines));
        assert!(got.transcript.contains("a ="));
        assert!(got.transcript.contains("b ="));
    }

    #[test]
    fn error_is_counted_and_rendered() {
        let lines = ["this_is_not_defined_xyz"];
        let got = capture_lines(&lines);
        assert_eq!(got.error_count, 1);
        assert_eq!(got.transcript, repl_reference(&lines));
        // Errors render via miette; the line is still echoed first.
        assert!(got.transcript.starts_with("--> this_is_not_defined_xyz\n"));
    }

    #[test]
    fn state_persists_across_blocks() {
        let got = run_fragment(&FragmentScript {
            files: vec![],
            inputs: vec!["a = 5;".into(), "b = a + 1".into()],
            expect_errors: 0,
            want_figure: false,
        });
        assert!(got.transcript.contains("b ="));
        assert!(got.transcript.contains('6'));
        assert_eq!(got.error_count, 0);
    }

    #[test]
    fn rng_is_deterministic() {
        let a = capture_lines(&["v = rand(1,3)"]);
        let b = capture_lines(&["v = rand(1,3)"]);
        assert_eq!(a.transcript, b.transcript);
    }

    #[test]
    fn multiline_for_loop_runs_as_one_chunk() {
        // A for-loop split across physical lines must be grouped and run, not
        // fed line-by-line (which would error on the partial `for`).
        let got = run_fragment(&FragmentScript {
            files: vec![],
            inputs: vec!["s = 0;\nfor i=1:3\n  s = s + i;\nend\ns".into()],
            expect_errors: 0,
            want_figure: false,
        });
        assert_eq!(got.error_count, 0, "transcript was: {}", got.transcript);
        assert!(got.transcript.contains("s ="));
        assert!(got.transcript.contains('6'));
        // Every physical line is echoed.
        assert!(got.transcript.contains("--> for i=1:3"));
        assert!(got.transcript.contains("--> end"));
    }

    #[test]
    fn aux_file_function_is_callable() {
        let got = run_fragment(&FragmentScript {
            files: vec![(
                "addtwo.m".into(),
                "function y = addtwo(x)\n  y = x + 2;\nend\n".into(),
            )],
            inputs: vec!["r = addtwo(40)".into()],
            expect_errors: 0,
            want_figure: false,
        });
        assert_eq!(got.error_count, 0, "transcript was: {}", got.transcript);
        assert!(got.transcript.contains("42"));
    }

    #[test]
    fn figure_is_captured_when_requested() {
        let got = run_fragment(&FragmentScript {
            files: vec![],
            inputs: vec!["x = linspace(0,1);\nplot(x, x.^2)".into()],
            expect_errors: 0,
            want_figure: true,
        });
        let fig = got.figure.expect("figure captured");
        assert!(fig.contains("\"type\":\"scene\""));
        assert!(fig.contains("figures"));
    }

    #[test]
    fn transcript_ends_with_single_newline() {
        let got = capture_lines(&["x = 1"]);
        assert!(got.transcript.ends_with('\n'));
        assert!(!got.transcript.ends_with("\n\n"));
    }

    #[test]
    fn no_figure_when_not_requested() {
        let got = capture_lines(&["plot([1 2 3])"]);
        assert!(got.figure.is_none());
    }
}
