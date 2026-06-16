# FreeMat-rs

A Rust port of [FreeMat](../FreeMat), a MATLAB-compatible numerical computing environment.

## Architecture

- **Native terminal CLI** (`fm`) — a real TTY REPL driving the interpreter; first-class on
  Windows/macOS/Linux. No GUI toolkit.
- **Graphics in the browser** — the CLI embeds a small webserver; plot commands serialize a
  scene-graph as JSON over a websocket to a static page that renders it with **Plotly.js**.
  Nothing Rust runs in a browser.
- **Pure-Rust math** — linear algebra via [`faer`](https://crates.io/crates/faer) over
  [`ndarray`](https://crates.io/crates/ndarray) storage; no LAPACK/BLAS/Fortran.
- **Idiomatic reimplementation** — the proven FreeMat design (Array model, hand-written
  parser, tree-walking interpreter, handle graphics) rewritten in idiomatic Rust, validated
  against FreeMat's own `.m` test suite.

## Crates

| Crate | Role |
|---|---|
| `fm-core` | numeric types + the ndarray-backed `Array` value |
| `fm-parser` | lexer, recursive-descent parser, AST, `miette` diagnostics |
| `fm-interp` | tree-walking evaluator, scopes, builtin registry, `.m` loader |
| `fm-linalg` | `faer`-backed linear algebra |
| `fm-builtins` | builtin functions ported from FreeMat's libCore |
| `fm-graphics` | handle-graphics scene model + JSON wire protocol |
| `fm-io` | file I/O, MAT files, FFT, regex |
| `fm-cli` | the `fm` binary: REPL + graphics webserver |

## Status

Work in progress, built in stages. See [`docs/PLAN.md`](docs/PLAN.md) for the full plan and
[`PROGRESS.md`](PROGRESS.md) for the current state.

## Building

```sh
cargo build --workspace
cargo test  --workspace
cargo run -p fm-cli
```
