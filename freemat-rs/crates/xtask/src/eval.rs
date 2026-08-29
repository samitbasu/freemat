//! `cargo xtask eval` — run FreeMat code headlessly through the interpreter.
//!
//! The `fm` binary only offers an interactive REPL; this task fills the gap of a
//! non-interactive runner, which is the workhorse for scripted testing and for
//! trying a feature out from the shell:
//!
//! ```text
//! cargo xtask eval "x = 1:5, sum(x)"        # run a one-liner
//! cargo xtask eval --file scratch.m          # run a .m script file
//! echo "disp(magic(4))" | cargo xtask eval - # read the program from stdin
//! cargo xtask eval --figure fig.json "plot(sin(0:0.1:6))"
//! ```
//!
//! It builds the same headless [`Interpreter`] (full standard library, no
//! graphics webserver) that the docs-capture engine uses, runs the program, and
//! prints whatever the interpreter emitted — the MATLAB-style `x = …` echoes,
//! `disp`/`printf` output, and so on — exactly as the REPL would. A runtime
//! error is rendered through `miette`'s graphical reporter (matching the live
//! REPL) and makes the task exit non-zero, so `eval` is usable as a test oracle.
//!
//! Unlike the docs-capture engine, `eval` does **not** seed the RNG or change
//! the working directory: it runs in the caller's cwd so relative file reads and
//! `rand`/`randn` behave as they would for a user at the prompt.

use std::io::{Read, Write};

use fm_interp::Interpreter;
use miette::{GraphicalReportHandler, GraphicalTheme};

/// Resolve the program source (and a diagnostic label) from the `eval` flags:
/// a `--file`, positional `code`, or — when neither is given, or `code` is just
/// `-` — stdin.
fn source(code: &[String], file: Option<&str>) -> Result<(String, String), String> {
    if let Some(path) = file {
        let src =
            std::fs::read_to_string(path).map_err(|e| format!("eval: reading '{path}': {e}"))?;
        return Ok((src, path.to_string()));
    }
    if code.is_empty() || code == ["-"] {
        let mut s = String::new();
        std::io::stdin()
            .read_to_string(&mut s)
            .map_err(|e| format!("eval: reading stdin: {e}"))?;
        return Ok((s, "<stdin>".to_string()));
    }
    // Join multiple positionals with newlines so `eval "a=1" "b=2"` runs both.
    Ok((code.join("\n"), "<cmdline>".to_string()))
}

/// Run the program and print its output. Returns `Err` (with a short summary; the
/// full diagnostic has already been rendered to stderr) if it raised a runtime
/// error, so the process exits non-zero.
pub fn eval(code: &[String], file: Option<&str>, figure: Option<&str>) -> Result<(), String> {
    let (src, label) = source(code, file)?;

    let mut interp = Interpreter::new();
    fm_builtins::register_standard_library(&mut interp);

    let result = interp.run(&src);

    // Print anything the program emitted before returning/erroring — the same
    // `take_output()` buffer the REPL prints (`x = …` echoes, disp/printf, …).
    print!("{}", interp.take_output());
    let _ = std::io::stdout().flush();

    if let Err(e) = result {
        let reporter = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());
        let mut buf = String::new();
        if reporter.render_report(&mut buf, &e).is_ok() {
            eprint!("{buf}");
        } else {
            eprintln!("error: {e}");
        }
        return Err(format!("eval: {label} raised a runtime error"));
    }

    if let Some(path) = figure {
        let scene = &interp.graphics.scene;
        if scene.figures.is_empty() {
            eprintln!(
                "eval: --figure given but {label} produced no figure (no plotting command ran)"
            );
        } else {
            let json = scene
                .to_message()
                .map_err(|e| format!("eval: serializing figure: {e}"))?;
            std::fs::write(path, &json).map_err(|e| format!("eval: writing '{path}': {e}"))?;
            println!("eval: wrote figure JSON ({} bytes) to {path}", json.len());
        }
    }

    Ok(())
}
