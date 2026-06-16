# FreeMat-rs — progress tracker

Source of truth for what is done across sessions. To work on the project, read
`docs/PLAN.md` (the full plan), pick a stage, implement it to the Definition of Done,
then tick it here and commit. Leave notes for the next session under each stage.

## Stage checklist

- [x] **Stage 0 — Workspace scaffold & conventions**
- [x] **Stage 1 — `fm-core`: types & Array**
- [ ] **Stage 2 — `fm-parser`: lexer + parser + AST (miette)**
- [ ] **Stage 3 — `fm-interp`: evaluator, scope, registry, `.m` loader**
- [ ] **Stage 4 — Conformance harness**
- [ ] **Stage 5 — `fm-linalg` + core math builtins  ·  ★ Milestone 1**
- [ ] **Stage 6 — `fm-builtins`: remaining core functions**
- [ ] **Stage 7 — `fm-graphics` + webserver + Plotly  ·  ★ Milestone 2**
- [ ] **Stage 8 — `fm-io`: MAT files, file I/O, FFT, regex**
- [ ] **Stage 9 — Advanced / optional**
- [ ] **Stage 10 — Debugging & editor integration (DAP + `db*` engine; optional LSP)**

## Definition of Done (every stage)

- `cargo build --workspace` succeeds.
- `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- `cargo fmt --all --check` passes.
- `cargo test --workspace` is green; `.m` conformance tests ported where the feature can run them.
- User-facing errors are `miette` diagnostics with spans.
- This file updated + committed.

## Notes by stage

### Stage 0 — done
- Workspace at `freemat-rs/` (sibling to `FreeMat/`), edition 2024, resolver 2, MSRV 1.96.
- Crates: `fm-core`, `fm-parser`, `fm-interp`, `fm-linalg`, `fm-builtins`, `fm-graphics`,
  `fm-io` (libs) + `fm-cli` (bin `fm`). Each has a trivial passing test.
- Lints inherited via `[workspace.lints]` (`rust_2018_idioms` + `clippy::all` at warn);
  CI escalates to `-D warnings`.
- `toolbox/` = the 317 FreeMat `.m` files copied verbatim from `FreeMat/toolbox/`.
- CI in `.github/workflows/ci.yml`: fmt-check, clippy `-D warnings`, build, test.
- License: `GPL-2.0-or-later` (FreeMat is GPL v2 per `FreeMat/COPYING`). **Confirm with owner.**
- Working on branch `rust-port` (not `master`).
- External deps deliberately empty so far — each stage adds its own to
  `[workspace.dependencies]` and opts in per crate.

### Stage 1 — done (`fm-core`)
- **Deps:** `ndarray = "0.17"` (resolved 0.17.2 — newer than the plan's ~0.16 guess) and
  `num-complex = "0.4"` (0.4.6). Added to `[workspace.dependencies]`; `fm-core` opts in.
- **Module layout (`crates/fm-core/src/`):**
  - `class.rs` — `DataClass` enum (mirrors FreeMat `DataClass`, no sparse) + name/predicate helpers.
  - `complex.rs` — `C32`/`C64` aliases over `num_complex::Complex` (interleaved, `repr(C)`).
  - `scalar.rs` — `ScalarValue`: the inline, heap-free scalar union (every numeric class + char)
    with `class`/`is_complex`/`as_f64`/`add_scalar`.
  - `array.rs` — the `Array` enum: inline `Scalar(ScalarValue)` fast path + dense
    `Arc<ArrayD<T>>` variants (column-major / F-order) for every class, plus `Cell` and `Struct`.
    Constructors, class/shape queries, scalar/string/cell/struct extraction, and per-type
    `make_mut_*` COW accessors via `Arc::make_mut`.
  - `struct_array.rs` — `StructArray` (ordered fields, one `Array` per element, column-major).
  - `promote.rs` — `promote(a, b)` type lattice (double-dominant; single only over bool/char/self;
    integer dominates non-integer numerics; mixed-integer / reference classes are errors).
  - `format.rs` — `FormatMode` (Short/Long) + `Array::format`/`display`: MATLAB-style body output
    (integers without decimals, 4/15 decimals otherwise, exponential for very large/small,
    `re + imi` complex, right-aligned column-major matrices, ND page headers, cell/struct/empty).
  - `error.rs` — `CoreError`/`Result` (plain Rust errors; the interpreter will wrap in `miette`).
- **Performance:** inline `Scalar` variant means scalar temporaries never touch the heap. A
  counting global allocator (thread-local flag + thread-local counter so parallel test threads
  don't pollute each other) asserts **zero** allocations across a 100k-iteration scalar-add loop
  and inline-scalar construction; a control test confirms dense construction *does* allocate.
- **Storage:** dense buffers are F-order (`IxDyn(dims).f()`); a column-major data `Vec` slots in
  directly, and `as_slice_memory_order()` returns the column-major buffer (verified by test) —
  keeps the Stage 5 `faer` boundary near zero-copy.
- **Tests (40):** `construction.rs`, `promotion.rs`, `cow.rs`, `formatting.rs` (golden strings),
  `no_alloc.rs`. The Stage-0 `scaffold_builds` placeholder was removed.
- **Design decisions / deferrals:**
  - Display golden strings are written against the MATLAB-style output we implement (no live
    FreeMat to diff). FreeMat's matrix-wide `1.0e+NN *` common scale factor and "Columns N through
    M" terminal-width splitting are **deferred** — current matrix display right-aligns each element
    in a shared column width without a common scale factor. Revisit when the REPL needs exact diffs.
  - `promote` forbids mixing two distinct integer classes (MATLAB semantics) and returns an error
    for cell/struct, rather than silently coercing.
  - Sparse arrays are out of scope (deferred per plan); no `Sparse` variant.
  - `ScalarValue::add_scalar` is a minimal heap-free scalar add to anchor the perf test; full typed
    operator dispatch / broadcasting lives in `fm-interp` (Stage 3).

### Debugging (Stage 10, design locked — build deferred to after Stages 7–8)
- Decision: editor+debugger via **DAP/LSP** (drive from VS Code/Neovim) — no built-in editor,
  no GUI. Debug *engine* lives in `fm-interp`; new crates `fm-dap` (+ optional `fm-lsp`).
- **Stage 3 must add the cheap enabling seams now** (not a retrofit): single statement-execution
  chokepoint, per-scope source line/span, switchable active scope (for `dbup`/`dbdown`).
- FreeMat reference to port later: `Interpreter.cpp` `bpStack`/`processBreakpoints`/`doDebugCycle`/
  `dbup`/`dbdown`; `libCore/Debug.cpp`; observer model in `libXP/Editor.cpp` becomes a Rust
  event trait the terminal and DAP both consume.
