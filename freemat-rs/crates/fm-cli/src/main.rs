//! `fm` — FreeMat-rs terminal REPL.
//!
//! A lean `rustyline` read–eval–print loop: it builds an [`Interpreter`],
//! registers the full [`fm_builtins`] standard library, evaluates each entered
//! line, prints results MATLAB-style (a trailing `;` suppresses the echo), and
//! renders runtime errors with `miette`'s graphical (`fancy`) reporter.
//!
//! On startup it also launches the embedded `axum` graphics webserver (see
//! [`fm_cli::server`]) and installs it as the interpreter's graphics sink, so
//! plotting commands render live in a browser over a websocket.

use std::io::Write;

use fm_interp::Interpreter;
use miette::{GraphicalReportHandler, GraphicalTheme};
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

fn main() -> std::process::ExitCode {
    print_banner();

    let mut interp = Interpreter::new();
    fm_builtins::register_standard_library(&mut interp);

    // Start the embedded graphics webserver and install it as the graphics sink
    // so plotting commands render in the browser. A failure here is non-fatal —
    // the REPL still works, graphics just won't display.
    match fm_cli::start(0) {
        Ok(handle) => {
            let url = handle.url();
            println!("Graphics server: {url}");
            // Best-effort auto-open; ignore failures (headless / no browser).
            let _ = webbrowser::open(&url);
            interp.set_graphics_sink(Box::new(handle));
        }
        Err(e) => {
            eprintln!("graphics server unavailable ({e}); plotting disabled");
        }
    }

    let mut rl = match DefaultEditor::new() {
        Ok(rl) => rl,
        Err(e) => {
            eprintln!("failed to start line editor: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let reporter = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());

    loop {
        match rl.readline("--> ") {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let _ = rl.add_history_entry(line.as_str());
                if matches!(trimmed, "quit" | "exit") {
                    break;
                }
                eval_line(&mut interp, &line, &reporter);
            }
            // Ctrl-C: abandon the current line, keep going.
            Err(ReadlineError::Interrupted) => continue,
            // Ctrl-D / EOF: exit cleanly.
            Err(ReadlineError::Eof) => break,
            Err(e) => {
                eprintln!("input error: {e}");
                break;
            }
        }
    }
    std::process::ExitCode::SUCCESS
}

/// Evaluate one input line, flushing buffered output and rendering errors.
fn eval_line(interp: &mut Interpreter, line: &str, reporter: &GraphicalReportHandler) {
    match interp.run(line) {
        Ok(()) => {
            let out = interp.take_output();
            print!("{out}");
            let _ = std::io::stdout().flush();
        }
        Err(e) => {
            // Print anything emitted before the error, then the diagnostic.
            let out = interp.take_output();
            print!("{out}");
            let mut buf = String::new();
            if reporter.render_report(&mut buf, &e).is_ok() {
                eprint!("{buf}");
            } else {
                eprintln!("error: {e}");
            }
            let _ = std::io::stdout().flush();
        }
    }
}

fn print_banner() {
    println!(
        "FreeMat-rs {} — a Rust port of FreeMat",
        env!("CARGO_PKG_VERSION")
    );
    println!("Type an expression, or `quit` / Ctrl-D to exit.");
}
