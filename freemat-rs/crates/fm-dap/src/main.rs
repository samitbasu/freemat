//! `fm-dap` — the FreeMat-rs Debug Adapter Protocol server binary.
//!
//! Usage:
//! - `fm-dap` — serve one session over stdin/stdout (how editors launch a debug
//!   adapter).
//! - `fm-dap --port 4711` — listen on `127.0.0.1:4711` and serve each incoming
//!   connection (handy for `attach`-style debugging and manual TCP testing).

use std::io::{BufReader, Write};
use std::net::TcpListener;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();

    if let Some(pos) = args.iter().position(|a| a == "--port") {
        let port: u16 = match args.get(pos + 1).and_then(|p| p.parse().ok()) {
            Some(p) => p,
            None => {
                eprintln!("--port requires a port number");
                return std::process::ExitCode::FAILURE;
            }
        };
        return serve_tcp(port);
    }

    serve_stdio()
}

/// Serve a single session over stdin/stdout.
fn serve_stdio() -> std::process::ExitCode {
    let reader = BufReader::new(std::io::stdin());
    let writer = std::io::stdout();
    match fm_dap::serve(Box::new(reader), Box::new(writer)) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("fm-dap: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// Listen on `127.0.0.1:<port>` and serve connections one at a time.
fn serve_tcp(port: u16) -> std::process::ExitCode {
    let listener = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("fm-dap: cannot bind 127.0.0.1:{port}: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    eprintln!("fm-dap: listening on 127.0.0.1:{port}");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let reader = match stream.try_clone() {
                    Ok(r) => BufReader::new(r),
                    Err(e) => {
                        eprintln!("fm-dap: cannot split stream: {e}");
                        continue;
                    }
                };
                let writer: Box<dyn Write> = Box::new(stream);
                if let Err(e) = fm_dap::serve(Box::new(reader), writer) {
                    eprintln!("fm-dap: session ended: {e}");
                }
            }
            Err(e) => eprintln!("fm-dap: accept failed: {e}"),
        }
    }
    std::process::ExitCode::SUCCESS
}
