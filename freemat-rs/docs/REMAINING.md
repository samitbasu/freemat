# FreeMat-rs — remaining work / backlog

Snapshot: conformance **670/677 ≈ 99.0%** against FreeMat's own `.m` suite. The interpreter is
effectively feature-complete for the corpus; what remains is one last big *feature* (the
debugger), **one** deferred numerical feature (`eigs`), and a handful of out-of-scope /
not-real-bug items.

How to use: pick an item, implement to the **Definition of Done** in `docs/PLAN.md` (build +
`clippy -D warnings` + `fmt --check` + `test` all green, regression tests, no conformance
regression, update `PROGRESS.md`, commit on `rust-port`). Reconfirm with
`cargo run --release -q -p fm-conformance -- --failures`. C++ oracle: `../FreeMat/`.

---

## A. Conformance gaps (the 7 still-failing tests)

### A0 — DONE in the REMAINING.md backlog pass (was A1/A2 + imwrite)

All cleared, no regressions (see `PROGRESS.md` → "REMAINING.md backlog pass"):
- **`fitfun` + `gausfit`/`gfitfun`** — Levenberg–Marquardt (`fm-builtins::fitfun`) + embedded
  toolbox M-source. `test_fitfun1/2/3`, `test_gausfit1`.
- **`source`** — `fm-builtins::interp_ops`, plus `which`-returns-file-path tracking. `test_source`.
- **`int2bin`/`bin2int` N-D** — VectorOp semantics in `baseconv.rs`. `test_bin2int1`.
- **`test_sparse75`** — sparse-preserving indexed assignment (no densify) + `lu` erroring on
  non-square / non-double sparse input (matches `SparseLUDecompose`).
- **`imwrite`/`imread`** — `fm-io::image_io` on the `image` crate (bmp/png/jpeg/gif/tiff).
  `test_imwrite_imread`.

### A1 — Still open

**`eigs` — iterative sparse eigensolver** · unblocks `suite/test_sparse45`, `sparse/test_sparse45` (1)
- *Deferred* (must NOT densify — see [[sparse-no-densify]] rule). A correct version is a shift-invert
  Arnoldi: build sparse `A − σI`, factor with faer's `SparseColMat::sp_lu()`, run (non-restarted, or
  ideally implicitly-restarted) Arnoldi on `OP = (A − σI)⁻¹` via the sparse solve, take Ritz values
  `λ = σ + 1/θ`. The `'lm'`/no-sigma case runs Arnoldi on `A` directly via sparse mat-vec. The test
  is `eigs(A,4,0.634)` (4 eigenvalues nearest 0.634, 1 output).
- *Effort:* Medium–High. *Value:* Low (one unique test).

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

**Other deferred numerics:** complex-sparse heavy linalg (sparse LU/QR/chol on complex), and
sparse direct/iterative solvers beyond the current densify-fallback. *Value:* Low unless large
sparse systems matter.

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
