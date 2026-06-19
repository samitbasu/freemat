# FreeMat-rs — remaining work / backlog

Snapshot: conformance **672/677 ≈ 99.3%** against FreeMat's own `.m` suite. The interpreter is
effectively feature-complete for the corpus; the 5 still-red are all out-of-scope (`file1` ×2
buggy corpus test, `ctype1` C-FFI, `parallel_fft1` ×2 threads). What remains is one last big
*feature* (the debugger) plus optional polish.

How to use: pick an item, implement to the **Definition of Done** in `docs/PLAN.md` (build +
`clippy -D warnings` + `fmt --check` + `test` all green, regression tests, no conformance
regression, update `PROGRESS.md`, commit on `rust-port`). Reconfirm with
`cargo run --release -q -p fm-conformance -- --failures`. C++ oracle: `../FreeMat/`.

---

## A. Conformance gaps — all addressable tests now pass

### A0 — DONE (REMAINING.md backlog pass)

All cleared, no regressions (see `PROGRESS.md` → "REMAINING.md backlog pass"):
- **`fitfun` + `gausfit`/`gfitfun`** — Levenberg–Marquardt (`fm-builtins::fitfun`) + embedded
  toolbox M-source. `test_fitfun1/2/3`, `test_gausfit1`.
- **`source`** — `fm-builtins::interp_ops`, plus `which`-returns-file-path tracking. `test_source`.
- **`int2bin`/`bin2int` N-D** — VectorOp semantics in `baseconv.rs`. `test_bin2int1`.
- **`test_sparse75`** — sparse-preserving indexed assignment (no densify) + `lu` erroring on
  non-square / non-double sparse input (matches `SparseLUDecompose`).
- **`imwrite`/`imread`** — `fm-io::image_io` on the `image` crate (bmp/png/jpeg/gif/tiff).
  `test_imwrite_imread`.

### A1 — DONE (sparse de-densification pass)

See `PROGRESS.md` → "Sparse de-densification". No regressions; cleared `test_sparse45`.
- **`eigs`** — pure-Rust shift-invert Arnoldi (`fm-linalg::eigs`) on faer's sparse `sp_lu` solve +
  a CSC mat-vec; no native/ARPACK dependency. `test_sparse45`.
- **Native sparse `\` / `/`** — faer `sp_lu` (square) / `sp_qr` lstsq, no densification of `A`.
- **Sparsity-preserving ops** — `.*` (sparse·sparse), `real`/`imag`/`conj`, conjugate-transpose,
  `diag` (build/extract), `repmat` now stay sparse.

---

## B. Bigger features not yet built (beyond conformance)

**Stage 10 — Debugger (DAP + `db*` engine; optional LSP)**
- The last big *planned* feature. Full spec already in `docs/PLAN.md` → "Stage 10". Zero
  conformance value, but completes the editor/debug story. The enabling seams are already in
  `fm-interp` from Stage 3 (statement chokepoint, per-scope source span, switchable active scope).
- *Effort:* Large. *Value:* High if interactive debugging matters; otherwise optional.

**Graphics handle-system polish** (deferred in Stage 7.5)
- Full MATLAB property catalogue (ticks/dir/aspect/`nextplot`/`children`/`clim`), text-object
  handles, `linkaxes`, `colorbar`, `copyobj`, `print`, and wiring the deeper `toolbox/graph/*.m`
  files. *Effort:* Medium, incremental. *Value:* Moderate for serious plotting.

**Remaining sparse-numerics polish** (post de-densification):
- **Factor-returning `lu(sparse)` / `det(sparse)`** still densify — faer's sparse `Lu` is
  solve-only (no L/U/P extraction), so a native version needs our own sparse LU (e.g.
  Gilbert–Peierls). FreeMat returns the same dense *result*, so this is a memory/scale gap only.
- **Error-parity for `reshape`/`permute`/`fft` on sparse** — FreeMat *errors*; we likely densify
  silently. Small fidelity fix (add `is_sparse()` guards) once FreeMat's exact error text is
  confirmed. (Not "supported sparse functions" — these are ops FreeMat rejects on sparse.)
- **Sparse reductions** (`sum`/`prod`/`max`/`min`/`mean`) currently densify; FreeMat also densifies
  for full reductions, so results match — keeping them sparse is an optional efficiency win.
*Value:* Low unless very large sparse systems matter.

---

## C. Out of scope / not real bugs (don't chase)

- **`ctypedefine` (FFI / imported C functions)** — `suite/test_ctype1`. The libffi/imported-function
  feature the plan **dropped** (native-only, deprioritized). Won't pass without re-scoping.
- **Threads / parallel** — `suite|transforms/test_parallel_fft1` (`threadnew`). **Out of scope** per
  the plan.
- **`file1`** — `io|suite/test_file1`. A buggy FreeMat corpus test: `return (p==q)` is dead code, so
  the output is never assigned and the function returns `[]` ⇒ an honest FAIL **in FreeMat too**.
  Not fixable on our side; leave as-is.

---

## Quick map: where things live
- Interpreter / eval / indexing / scopes: `crates/fm-interp/`
- Values / `Array` (dense + `Array::Sparse` + `Array::FunctionHandle`), formatting: `crates/fm-core/`
- Lexer / parser / AST: `crates/fm-parser/`
- Dense + sparse linear algebra (faer): `crates/fm-linalg/`
- Builtins (math/strings/array/poly/bitops/baseconv/handles/graphics/…): `crates/fm-builtins/`
- MAT / file I/O / FFT / regex: `crates/fm-io/`
- REPL + graphics webserver: `crates/fm-cli/`; browser frontend: `web/index.html`
- Conformance harness + corpus: `crates/fm-conformance/` (`-- --failures` lists what's red)
- Coverage report (generated from `fm --list-builtins`): `docs/COVERAGE.md`
