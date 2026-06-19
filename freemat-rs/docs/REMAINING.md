# FreeMat-rs — remaining work / backlog

Snapshot: HEAD `c796560af`, conformance **658/677 ≈ 97.2%** against FreeMat's own `.m` suite.
The interpreter is effectively feature-complete for the corpus; what remains is one last big
*feature* (the debugger), a few moderate/niche numerical features, and a handful of
out-of-scope or not-real-bug items.

How to use: pick an item, implement to the **Definition of Done** in `docs/PLAN.md` (build +
`clippy -D warnings` + `fmt --check` + `test` all green, regression tests, no conformance
regression, update `PROGRESS.md`, commit on `rust-port`). Reconfirm with
`cargo run --release -q -p fm-conformance -- --failures`. C++ oracle: `../FreeMat/`.

---

## A. Conformance gaps (the ~19 still-failing tests)

### A1 — Worth doing

**Curve fitting — `fitfun`, `gausfit`** · unblocks `suite/test_fitfun1/2/3`, `suite/test_gausfit1` (4)
- *Gap:* nonlinear least-squares is unimplemented. `fitfun` is the core; `gausfit` builds on it.
- *Work:* implement Levenberg–Marquardt — either port FreeMat's bundled `libs/libFN/levmar-2.3/`,
  or use a Rust crate (`levenberg-marquardt`, or `argmin`). Add the `fitfun` builtin (it takes a
  function/expression + data); `gausfit` is likely expressible as a toolbox `.m` once `fitfun`
  exists. Reference FreeMat `libs/libFN` (`FitFunFunction`).
- *Effort:* Medium. *Value:* Moderate — the only remaining genuinely-useful capability.

**`source` builtin** · unblocks `freemat/test_source`, `suite/test_source` (1 unique)
- *Gap:* `source('file')` (run a script file's statements in the caller's scope) is missing.
- *Work:* small `fm-interp` builtin — parse the file and `exec` its statements in the current
  scope (distinct from running it as a function). Reference FreeMat `Source.cpp`.
- *Effort:* Low–Medium. *Value:* Low.

### A2 — Cheap niche fixes

**`int2bin`/`bin2int` on N-D input** · unblocks `suite/test_bin2int1`, `typecast/test_bin2int1` (1 unique)
- *Gap:* the test feeds `rand(4,4,3)` (3-D); our base-conversion builtins assume ≤2-D.
- *Work:* generalize `crates/fm-builtins/src/baseconv.rs` `int2bin`/`bin2int` to preserve N-D shape.
- *Effort:* Low. *Value:* Low.

**`lu` must error on non-finite** · unblocks `suite/test_sparse75`, `transforms/test_sparse75` (1 unique)
- *Gap:* the test sets an element to `inf` and expects `[l,u]=lu(...)` to throw; we don't.
- *Work:* in `fm-linalg` `lu` (and the sparse densify path), detect non-finite input and return an
  error matching FreeMat. *Effort:* Low. *Value:* Low.

### A3 — Larger / low-value features

**`eigs` — iterative sparse eigensolver** · unblocks `suite/test_sparse45`, `sparse/test_sparse45` (1)
- *Work:* Lanczos (symmetric) / Arnoldi (general) restarted iteration — the ARPACK role the plan
  deferred. Consider a Rust crate or a minimal Lanczos for the symmetric case.
- *Effort:* Medium–High. *Value:* Low.

**`imwrite` / `imread` — image I/O** · unblocks `io/test_imwrite_imread`, `suite/test_imwrite_imread` (1)
- *Work:* add the `image` crate; map FreeMat's `imwrite(A,'file.png')` / `imread`. New external dep.
- *Effort:* Medium. *Value:* Low.

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
