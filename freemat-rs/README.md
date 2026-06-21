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
| `fm-doc` | help-system core: doc model, `register_doc!` macro, markdown renderers |
| `fm-cli` | the **`fm`** binary: REPL + graphics/help webserver |
| `xtask` | dev tasks: `cargo xtask docgen` (help DB), `migrate-docs`/`migrate-place` |

## Status

Work in progress, built in stages. See [`docs/PLAN.md`](docs/PLAN.md) for the full plan and
[`PROGRESS.md`](PROGRESS.md) for the current state.

## Build & run

Run everything from this directory (`freemat-rs/`). Requires a Rust toolchain
(edition 2024, Rust ≥ 1.96).

> **Note:** the crate is named `fm-cli`, but the binary it produces is named **`fm`**.
> You build/run the *crate* with `-p fm-cli`, and the resulting executable is `fm`.

```sh
# Start the interactive REPL (builds fm-cli, then runs the `fm` binary):
cargo run -p fm-cli

# …or build first, then run the binary directly:
cargo build -p fm-cli
./target/debug/fm

# Optimized build (binary at ./target/release/fm):
cargo run --release -p fm-cli
```

On startup `fm` drops you into the REPL (prompt `--> `) and launches the embedded
graphics/help webserver, printing its URL (`Graphics server: http://127.0.0.1:<port>`).
Exit with `quit` or Ctrl-D.

### Command-line flags

Pass flags after `--` when using `cargo run` (so Cargo forwards them to `fm`); the
direct binary needs no `--`:

```sh
cargo run -p fm-cli -- --no-gfx          # terminal only; skip the webserver
cargo run -p fm-cli -- --list-builtins   # print registered builtins and exit
./target/debug/fm --no-gfx               # equivalent, direct binary
```

### Help system

```text
--> help            # list documentation sections
--> help sin        # one topic as terminal text (+ a clickable browser URL)
--> helpwin sin     # open the rich browser page (math, highlighting, plots)
```

### Common workspace tasks

```sh
cargo build --workspace        # build all crates
cargo test  --workspace        # run the full test suite
cargo xtask docgen             # regenerate the help fragment DB
cargo xtask docgen --check     # CI gate: verify the help DB is up to date
```

Editing a doc? The captured example transcripts are compiled in, so the loop is
**edit `register_doc!` → `cargo xtask docgen` → rebuild → run**. See
[`docs/WRITING_DOCS.md`](docs/WRITING_DOCS.md).
