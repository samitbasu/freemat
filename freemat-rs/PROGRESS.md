# FreeMat-rs — progress tracker

Source of truth for what is done across sessions. To work on the project, read
`docs/PLAN.md` (the full plan), pick a stage, implement it to the Definition of Done,
then tick it here and commit. Leave notes for the next session under each stage.

## Stage checklist

- [x] **Stage 0 — Workspace scaffold & conventions**
- [ ] **Stage 1 — `fm-core`: types & Array**
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

### Next: Stage 1 (`fm-core`)
- Reference `FreeMat/libs/libFreeMat/Array.hpp`, `BasicArray.hpp`, `Complex.hpp`, `Types.hpp`.
- Mandatory: column-major (ndarray F-order) storage, `Arc`+`Arc::make_mut` COW, and an inline
  `Scalar` variant so scalar temporaries never heap-allocate.

### Debugging (Stage 10, design locked — build deferred to after Stages 7–8)
- Decision: editor+debugger via **DAP/LSP** (drive from VS Code/Neovim) — no built-in editor,
  no GUI. Debug *engine* lives in `fm-interp`; new crates `fm-dap` (+ optional `fm-lsp`).
- **Stage 3 must add the cheap enabling seams now** (not a retrofit): single statement-execution
  chokepoint, per-scope source line/span, switchable active scope (for `dbup`/`dbdown`).
- FreeMat reference to port later: `Interpreter.cpp` `bpStack`/`processBreakpoints`/`doDebugCycle`/
  `dbup`/`dbdown`; `libCore/Debug.cpp`; observer model in `libXP/Editor.cpp` becomes a Rust
  event trait the terminal and DAP both consume.
