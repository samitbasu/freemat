//! `fm` — FreeMat-rs terminal REPL and embedded graphics webserver.
//!
//! Scaffold only (Stage 0). The `crossterm`/`rustyline` REPL arrives in Stage 3
//! and the `axum` graphics webserver in Stage 7.

fn main() {
    println!(
        "FreeMat-rs {} — Rust port of FreeMat",
        env!("CARGO_PKG_VERSION")
    );
    println!("Stage 0 scaffold: workspace builds. Interactive REPL arrives in Stage 3.");
}

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_builds() {
        assert_eq!(2 + 2, 4);
    }
}
