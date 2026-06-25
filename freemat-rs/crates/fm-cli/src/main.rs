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
    // `--list-builtins` prints every registered function name (sorted), one per
    // line, then exits. This is the authoritative freemat-rs builtin set used to
    // generate `docs/COVERAGE.md`.
    if std::env::args().any(|a| a == "--list-builtins") {
        let mut interp = Interpreter::new();
        fm_builtins::register_standard_library(&mut interp);
        for name in interp.functions.names() {
            println!("{name}");
        }
        return std::process::ExitCode::SUCCESS;
    }

    // `--capture-fragment <script.json>` (or `-` / omitted = stdin) reads a
    // `FragmentScript` as JSON, runs it through the headless capture engine
    // (help-system P2), and writes the resulting `CapturedFragment` as JSON to
    // stdout. Used by `cargo xtask docgen` for crash-isolated fragment capture.
    {
        let args: Vec<String> = std::env::args().collect();
        if let Some(pos) = args.iter().position(|a| a == "--capture-fragment") {
            return capture_fragment_cli(args.get(pos + 1).map(String::as_str));
        }
    }

    // `--no-gfx` skips the embedded graphics webserver (and browser auto-open).
    // Handy for headless runs, scripted/piped input, and benchmarking, where a
    // background tokio server is pure overhead.
    let no_gfx = std::env::args().any(|a| a == "--no-gfx");

    // `--dap-port N` starts an embedded Debug Adapter Protocol server on
    // `127.0.0.1:N` (use `0` for an ephemeral port). A debugger that connects
    // can set breakpoints and step the *live* REPL session. See
    // `docs/INTERP_SERVICE_PLAN.md`.
    let dap_port = parse_flag_value("--dap-port").and_then(|v| v.parse::<u16>().ok());

    print_banner();

    // Start the embedded graphics webserver on this thread so we can print its
    // URL and open a browser, then hand the sink to the engine thread (the
    // `ServerHandle` is `Send`). A failure here is non-fatal — the REPL still
    // works, graphics just won't display. Skipped under `--no-gfx`.
    let sink = if no_gfx {
        println!("Graphics disabled (--no-gfx).");
        None
    } else {
        match fm_cli::start(0) {
            Ok(handle) => {
                let url = handle.url();
                println!("Graphics server: {url}");
                // Best-effort auto-open; ignore failures (headless / no browser).
                let _ = webbrowser::open(&url);
                Some(handle)
            }
            Err(e) => {
                eprintln!("graphics server unavailable ({e}); plotting disabled");
                None
            }
        }
    };

    // The interpreter now lives behind the engine actor (see
    // `docs/INTERP_SERVICE_PLAN.md`); the REPL drives it by message. The setup
    // closure runs on the engine thread, installing the graphics sink there.
    let setup = move |interp: &mut Interpreter| {
        if let Some(handle) = sink {
            interp.set_graphics_sink(Box::new(handle));
        }
    };
    let engine = match dap_port {
        Some(port) => match fm_cli::Engine::spawn_with_dap(port, setup) {
            Ok((engine, bound)) => {
                println!("Debug adapter (DAP) listening on 127.0.0.1:{bound}");
                engine
            }
            Err(e) => {
                eprintln!("DAP server unavailable ({e}); debugging disabled");
                fm_cli::Engine::spawn(|_| {})
            }
        },
        None => fm_cli::Engine::spawn(setup),
    };

    // Ctrl-C during a run now aborts it to the prompt (cooperative interrupt)
    // instead of killing the process.
    install_interrupt_handler(engine.interrupt_flag());

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
                eval_line(&engine, &line, &reporter);
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

/// Evaluate one input line through the engine, flushing buffered output and
/// rendering any error. Output is printed in both cases (matching the old
/// behavior of showing whatever was emitted before an error).
fn eval_line(engine: &fm_cli::Engine, line: &str, reporter: &GraphicalReportHandler) {
    // Clear any interrupt raised while idle at the prompt, so a stale Ctrl-C
    // doesn't abort this fresh line on its first statement.
    engine
        .interrupt_flag()
        .store(false, std::sync::atomic::Ordering::Relaxed);
    let outcome = engine.eval(line);
    print!("{}", outcome.output);
    if let Some(e) = outcome.error {
        let mut buf = String::new();
        if reporter.render_report(&mut buf, &e).is_ok() {
            eprint!("{buf}");
        } else {
            eprintln!("error: {e}");
        }
    }
    let _ = std::io::stdout().flush();
}

/// Raw pointer to the engine's interrupt `AtomicBool`, reachable from the
/// async-signal context. Set once at startup to a leaked (process-lifetime)
/// pointer, so the `SIGINT` handler only ever performs an atomic store.
static INTERRUPT_PTR: std::sync::atomic::AtomicPtr<std::sync::atomic::AtomicBool> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

#[cfg(unix)]
extern "C" fn handle_sigint(_sig: libc::c_int) {
    let ptr = INTERRUPT_PTR.load(std::sync::atomic::Ordering::Relaxed);
    if !ptr.is_null() {
        // SAFETY: `ptr` points at a leaked, process-lifetime `AtomicBool`; an
        // atomic store is async-signal-safe.
        unsafe { (*ptr).store(true, std::sync::atomic::Ordering::Relaxed) };
    }
}

/// Install a `SIGINT` (Ctrl-C) handler that raises the engine's interrupt flag,
/// so Ctrl-C during a long run aborts it to the prompt instead of killing the
/// process. No-op on non-Unix.
fn install_interrupt_handler(flag: std::sync::Arc<std::sync::atomic::AtomicBool>) {
    // Leak one strong ref so the `AtomicBool` outlives any signal delivery.
    let raw = std::sync::Arc::into_raw(flag) as *mut std::sync::atomic::AtomicBool;
    INTERRUPT_PTR.store(raw, std::sync::atomic::Ordering::Relaxed);
    #[cfg(unix)]
    // SAFETY: the handler only performs an atomic store (async-signal-safe).
    unsafe {
        libc::signal(
            libc::SIGINT,
            handle_sigint as *const () as libc::sighandler_t,
        );
    }
}

/// Return the value following `flag` in the process arguments, if present
/// (`--flag value` form).
fn parse_flag_value(flag: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    let pos = args.iter().position(|a| a == flag)?;
    args.get(pos + 1).cloned()
}

/// Handle `fm --capture-fragment [<script.json>|-]`: read a `FragmentScript`
/// JSON (from the given path, or stdin if the path is missing or `-`), run it,
/// and print the `CapturedFragment` JSON to stdout. Diagnostics go to stderr.
fn capture_fragment_cli(path: Option<&str>) -> std::process::ExitCode {
    let input = match path {
        None | Some("-") => {
            let mut s = String::new();
            if let Err(e) = std::io::Read::read_to_string(&mut std::io::stdin(), &mut s) {
                eprintln!("error reading fragment script from stdin: {e}");
                return std::process::ExitCode::FAILURE;
            }
            s
        }
        Some(p) => match std::fs::read_to_string(p) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error reading fragment script '{p}': {e}");
                return std::process::ExitCode::FAILURE;
            }
        },
    };
    let script: fm_cli::FragmentScript = match serde_json::from_str(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error parsing fragment script JSON: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let captured = fm_cli::run_fragment(&script);
    match serde_json::to_string(&captured) {
        Ok(json) => {
            println!("{json}");
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error serializing captured fragment: {e}");
            std::process::ExitCode::FAILURE
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
