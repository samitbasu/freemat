# Porting FreeMat to Rust — native CLI + browser (Plotly) graphics

> **Multi-session execution plan.** This is not expected to finish in one session. It is
> organized as a sequence of self-contained **stages**. To work on the project, point a fresh
> Claude session at this file and say *"implement Stage N"*. Each stage section is written to be
> executable on its own, assuming earlier stages are done.

## How to use this plan (read this first, every session)

1. Read **Context** and **Standing rules (Definition of Done)** below — they apply to every stage.
2. Open **`docs/PLAN.md` and `PROGRESS.md` in the repo** (Stage 0 copies this plan there and
   creates the tracker). `PROGRESS.md` is the source of truth for what is done.
3. Find the stage you were asked to implement. Do only that stage unless told otherwise.
4. When finished: ensure the Definition of Done holds, tick the stage in `PROGRESS.md`, and
   commit. Leave notes in `PROGRESS.md` about anything deferred or surprising for the next session.

## Context

FreeMat is a mature MATLAB-compatible environment in C++/Qt (~51k LOC core + graphics, plus
LAPACK/BLAS/FFTW/UMFPACK/ARPACK/LLVM deps), living at `/home/samitbasu/Devel/freemat/FreeMat`.
The goal is a fresh Rust implementation that drops Qt, replaces the Fortran/C math stack with
pure-Rust crates, and renders graphics in a browser via Plotly.js.

A key finding reframes the effort — this is **not** a 50k-line transliteration:
- `FreeMat/toolbox/` holds **317 `.m` files (~8,700 lines)** written in the FreeMat language.
  Once the Rust interpreter runs, these execute **unchanged** — zero port cost.
- `FreeMat/tests/` holds **913 `.m` test files + 375 `.mat` fixtures** — a ready-made
  **conformance oracle** to validate the Rust interpreter against original behavior.
- The real port surface is the **engine** (`FreeMat/libs/libFreeMat`) and the **~70 C++ builtins**
  (`FreeMat/libs/libCore`), plus the **handle-graphics** model (`FreeMat/libs/libGraphics`).

### Architecture (final)

The simplest, least-code shape: a native terminal CLI + an embedded webserver that a browser
connects to for graphics.

- **No GUI toolkit, no Qt, no egui, no WASM.** Modern terminals give real cross-platform parity,
  and using the browser as the plot backend cleanly avoids the old Windows console/GUI-subsystem
  problem that motivated a rewrite. Nothing Rust ever runs in a browser.
- **Interpreter interaction is terminal-only** — a real TTY REPL (`crossterm` + `rustyline`),
  first-class on Windows/macOS/Linux.
- **All graphics render in a web browser via Plotly.js (plain JavaScript).** The CLI embeds a
  small webserver (`axum` + `tokio`) serving a static page (Plotly + a tiny websocket client).
  On a plot command the interpreter serializes a semantic scene (serde JSON) and pushes it over
  the websocket; Plotly draws it (2D first; interactive 3D `surf`/`mesh` later).
- **Linear algebra is pure-Rust `faer`** (LU/QR/SVD/eig/Cholesky) over **`ndarray`** storage.
- **Parser/runtime errors use `miette`** (+ `thiserror`): byte-span diagnostics rendered as
  annotated, source-highlighted messages on the terminal.
- **Port philosophy: idiomatic reimplementation.** Keep the proven design (Array model,
  hand-written lexer/parser, tree-walking interpreter, scope model, handle-graphics scene),
  rewrite in idiomatic Rust, validate against the `.m` conformance suite.
- **Debugging & editing via your own editor (DAP/LSP).** No built-in editor, no GUI: the debug
  *engine* lives in `fm-interp` (breakpoints, stepping, variable inspection at the paused scope —
  FreeMat's `db*` commands, terminal-native). Visual debugging is exposed via a **Debug Adapter
  Protocol** server so VS Code/Neovim drive breakpoints/stepping/inspection, with an optional
  **LSP** for `.m` smarts. This mirrors FreeMat's clean engine-vs-GUI split (its Qt editor only
  *observed* the interpreter through signals). Built in Stage 10; the cheap enabling seams go
  into Stage 3.

## Standing rules (Definition of Done — applies to EVERY stage)

A stage is done only when all of these hold:

- **Builds clean:** `cargo build --workspace` succeeds.
- **Clippy clean:** `cargo clippy --workspace --all-targets -- -D warnings` passes with zero
  warnings. Do not `#[allow(...)]` to silence lints without a one-line justification comment.
- **Formatted:** `cargo fmt --all` applied; CI checks `cargo fmt --all --check`.
- **Tested, and tests ported wherever possible:**
  - Every layer below the interpreter (types, lexer, parser, linalg adapters) gets **Rust unit
    tests**.
  - For anything the interpreter can run, **port the relevant FreeMat `.m` tests** from
    `FreeMat/tests/` into the conformance harness (Stage 4) and make them pass — do not rewrite
    them, run them as-is. Each feature stage must leave new `.m` tests green and list which
    `tests/` subdirectories it now covers.
  - `cargo test --workspace` is green.
- **Idiomatic Rust:** reference the original C++ for *semantics*, not structure. Enums over
  tagged unions, `Result`/`?` over error codes, iterators where natural.
- **Diagnostics:** user-facing errors (parse + runtime) are `miette` diagnostics with spans.
- **Tracked:** tick the stage in `PROGRESS.md`, note anything deferred, and commit.

## Workspace layout

```
freemat-rs/                 (new Cargo workspace, sibling to FreeMat/)
  crates/
    fm-core       # numeric types, Array value (ndarray-backed), formatting
    fm-parser     # lexer + recursive-descent parser + AST + miette diagnostics
    fm-interp     # Context/Scope, tree-walking evaluator, builtin registry, .m loader
    fm-linalg     # faer adapters (mtimes, \ /, inv, det, lu, qr, svd, eig, chol, norm, ...)
    fm-builtins   # the ~70 libCore builtins, ported
    fm-graphics   # handle-graphics scene model + serde JSON wire protocol
    fm-io         # file I/O + MAT-file read/write + FFT/regex-backed builtins
    fm-cli        # bin `fm`: crossterm/rustyline REPL + axum webserver + websocket
    fm-dap        # (Stage 10) Debug Adapter Protocol server for VS Code/Neovim
    fm-lsp        # (Stage 10, optional) Language Server for .m language smarts
  web/            # static assets: index.html + Plotly.js + small websocket client (JS)
  toolbox/        # the 317 .m files, copied/symlinked from FreeMat/toolbox, reused unchanged
  tests/          # conformance harness + selected .m tests/.mat fixtures from FreeMat/tests
  docs/PLAN.md    # this plan, committed into the repo
  PROGRESS.md     # per-stage checklist + running notes (source of truth across sessions)
```

## Component mapping (original → Rust)

| Original (C++/Qt) | Rust replacement |
|---|---|
| `Array.hpp` tagged union + QSharedData COW | `enum`-of-element-type over **ndarray** (`IxDyn`, column-major), `Arc` + `Arc::make_mut` COW, inline scalar fast path; complex via **num-complex** |
| `Scanner.cpp` / `Parser.cpp` | hand-written lexer + recursive-descent parser; **miette/thiserror** errors with spans; port grammar, precedence, transpose-vs-string & command-syntax disambiguation |
| `Interpreter.cpp` tree-walker | `fm-interp` evaluator: `expression`, LHS-indexing assignment, multi-return, control flow, short-circuit, colon ranges, broadcasting & promotion |
| `Context`/`Scope` | `fm-interp` Context with scope stack + call stack; same global/persistent/local rules |
| `addFunction`/`addSpecialFunction` | builtin `trait`/enum registry; `.m` functions loaded via `fm-parser` and run by the same evaluator (this makes the 317 toolbox files work) |
| EigenDecompose/SVD/QR/LU/LinearEqSolver (LAPACK) | `fm-linalg` over **faer** |
| FFTW | **rustfft** |
| PCRE / regex | **regex** |
| zlib (MAT compression) | **flate2 / miniz_oxide** |
| RanLib PRNG | **rand** + **rand_distr** |
| Handle graphics + RenderEngine + Qt/GL backends | `fm-graphics` scene-graph + **serde JSON** semantic protocol → **Plotly.js** in browser. No Rust-side renderer. |
| QTTerm + InterpreterThread | `fm-cli`: `crossterm`/`rustyline` TTY REPL driving the interpreter |
| Interpreter debug engine (`bpStack`, `processBreakpoints`, `doDebugCycle`) + `db*` commands | `fm-interp` debug engine: breakpoint registry, per-statement check, debug-cycle re-entering the REPL at the paused scope; `db*` builtins (Stage 10) |
| Qt editor + breakpoint margins + Variables/Stack tools (Qt signals) | protocol-agnostic debug-event interface → **DAP server** (`fm-dap`) for VS Code/Neovim + optional **LSP** (`fm-lsp`); no built-in editor, bring-your-own `$EDITOR` (Stage 10) |
| MAT-file I/O (`MatIO.cpp`) | `fm-io` (matfile format) |

### Dropped / deferred
- **GUI toolkit (egui/Qt), WASM, in-browser Rust** — dropped entirely.
- **LLVM/Clang JIT** — drop (revisit with cranelift, native-only, only if profiling demands).
- **VTK / ITK** — drop; 3D via Plotly.
- **libffi / MEX / imported functions** — native-only feature flag, deprioritized.
- **UMFPACK / ARPACK (sparse solvers/eig)** — defer; later via sprs / faer-sparse.

---

# Stages

> Each stage below is a standalone work order. Format: **Goal / Prerequisites / Crates & files /
> Reference (original C++) / Build / Tests / Acceptance**. The Standing rules apply on top.

## Stage 0 — Workspace scaffold & conventions
- **Goal:** an empty but correctly-wired workspace that builds, lints, and runs CI.
- **Prerequisites:** none.
- **Crates & files:** create the workspace and all crate skeletons above; `web/`, `toolbox/`,
  `tests/`, `docs/PLAN.md` (copy this file), `PROGRESS.md` (checklist of all stages).
- **Build:** workspace `Cargo.toml`; empty lib crates + `fm-cli` bin printing a banner; root
  `clippy.toml`/lint config; `rustfmt.toml`; GitHub-Actions (or `justfile`/`Makefile.toml`) CI
  running build + `clippy -D warnings` + `fmt --check` + `test`. Copy `FreeMat/toolbox/*.m`
  into `toolbox/`.
- **Tests:** a trivial passing unit test in each crate so `cargo test --workspace` is wired.
- **Acceptance:** `cargo build`, `cargo clippy -- -D warnings`, `cargo fmt --check`,
  `cargo test` all green; `cargo run -p fm-cli` prints a banner; `PROGRESS.md` exists.

## Stage 1 — `fm-core`: types & Array
- **Goal:** the value type the whole interpreter operates on.
- **Prerequisites:** Stage 0.
- **Reference:** `libs/libFreeMat/Array.hpp/.cpp`, `BasicArray.hpp`, `Complex.hpp`,
  `Types.hpp`, and display logic in `libs/libCore/` printing code.
- **Build:** numeric classes (Bool, i8..i64, u8..u64, f32, f64, complex f32/f64 via
  `num-complex`); `Array` as an `enum` over element type backed by **ndarray** (`IxDyn`,
  column-major), `Arc` COW with `Arc::make_mut`, scalar fast path; char/string arrays, cell
  arrays, struct arrays; MATLAB-style display/formatting (`format short`/`long`, scaling).
- **Tests:** unit tests for construction, type promotion rules, COW semantics, and formatting
  golden strings (compare against FreeMat output samples).
- **Performance (mandatory):** FreeMat is *not* arena-based — it uses one contiguous
  column-major buffer per array (`BasicArray<T>`) + Qt QSharedData COW + a **union inline scalar
  fast path**. Match it: store the array column-major (ndarray F-order); get COW via `Arc` +
  `Arc::make_mut`; and **the `Array` enum MUST have an inline `Scalar` variant** (value stored
  directly, no 1×1 ndarray, no heap alloc). The dominant interpreter allocation pressure is
  scalar temporaries — without the inline scalar, scalar loops malloc per iteration. Keep the
  buffer layout exactly column-major so the Stage 5 faer boundary is ~zero-copy.
- **Acceptance:** Definition of Done; `Array` covers all FreeMat data classes with round-trip
  + display tests; scalar arithmetic allocates no heap buffers (assert via a tight-loop test).

## Stage 2 — `fm-parser`: lexer + parser + AST (with miette)
- **Goal:** turn source text into an AST, with pretty span-based error diagnostics.
- **Prerequisites:** Stage 0 (independent of `fm-core`).
- **Reference:** `libs/libFreeMat/Scanner.cpp/.hpp`, `Parser.cpp/.hpp`, `Token.cpp/.hpp`,
  `Tree.cpp/.hpp`.
- **Build:** hand-written lexer (numbers, strings, comments, line-continuations, bracket-nesting,
  reserved words, **transpose-vs-string disambiguation**, command syntax); recursive-descent
  parser with correct operator precedence, matrix/cell literals, ranges (`a:b:c`), function &
  anonymous-function defs, control-flow statements. AST as Rust enums; **every node carries a
  byte span**. Errors are `thiserror`/`miette` `Diagnostic`s with `#[source_code]` + `#[label]`.
- **Tests:** unit tests asserting parse trees for representative snippets; error-path tests
  asserting diagnostics point at the right span. Pull tricky syntax from `FreeMat/tests/` for
  coverage.
- **Acceptance:** Definition of Done; a malformed snippet renders an annotated miette error.

## Stage 3 — `fm-interp`: evaluator, scope, registry, `.m` loader
- **Goal:** a working tree-walking interpreter that can run `.m` functions.
- **Prerequisites:** Stages 1, 2.
- **Reference:** `libs/libFreeMat/Interpreter.cpp/.hpp`, `Context.cpp/.hpp`, `Scope.cpp/.hpp`,
  `FunctionDef.hpp`.
- **Build:** Context/Scope (global/persistent/local + call stack); evaluator (`expression`,
  assignment with LHS indexing, multi-return, for/while/if/switch/try-catch/break/continue/
  return, short-circuit `&&`/`||`, colon ranges); operator dispatch with broadcasting &
  promotion; full indexing (paren/brace/field, `end`, logical, linear & subscript,
  grow-on-assign); error/exception model as `miette` diagnostics (MException-like). Builtin
  **registry** (trait/enum + registration like `addFunction`/`addSpecialFunction`). **`.m`
  loader** that parses and runs toolbox files via the same evaluator.
- **Debug-readiness (mandatory, cheap now — enables Stage 10 without a retrofit):** route every
  statement through a single execution chokepoint (an `exec_statement` seam where a breakpoint
  hook can later be checked); thread the executing statement's source line/`span` into the active
  scope (FreeMat packs it into the token id; we use the AST span); and make the Context's active
  scope switchable (the basis for `dbup`/`dbdown`). No debugger yet — just the seams.
- **Tests:** unit tests for scoping, indexing, control flow, broadcasting; load and run a couple
  of simple `toolbox/*.m` functions end-to-end.
- **Acceptance:** Definition of Done; the REPL (even minimal) can evaluate expressions and call a
  `.m` function.

## Stage 4 — Conformance harness
- **Goal:** run FreeMat's own `.m` tests against the Rust interpreter; this is the project's
  primary correctness signal from here on.
- **Prerequisites:** Stage 3.
- **Reference:** `FreeMat/tests/` (inspect the `test_*.m` format and how pass/fail is signaled).
- **Build:** in `tests/`, a runner (a Rust integration test or `fm-cli --run-tests`) that loads
  selected `FreeMat/tests/*.m`, executes them, and reports pass/fail; copy in the `.m` tests
  (and `.mat` fixtures once `fm-io` exists) that current features support. Track pass-rate.
- **Tests:** the harness itself; start with the subset exercising Stages 1–3 features.
- **Acceptance:** Definition of Done; `cargo test` runs the conformance subset and reports a
  pass-rate; `PROGRESS.md` records which `tests/` dirs are covered.

## Stage 5 — `fm-linalg` + core math builtins  ·  **★ Milestone 1**
- **Goal:** real numeric work: arithmetic, elementary functions, and linear algebra in the REPL.
- **Prerequisites:** Stages 3, 4.
- **Reference:** `libs/libFreeMat/EigenDecompose.cpp`, `QRDecompose.cpp`, `LUDecompose.cpp`,
  `LinearEqSolver.cpp`, `MatrixMultiply.cpp`; `libs/libCore/` elementary-math & reduction files.
- **Build:** `fm-linalg` over **faer** (`*`, `\`, `/`, `inv`, `det`, `lu`, `qr`, `svd`, `eig`,
  `chol`, `norm`, `rank`, `pinv`); first tranche of `fm-builtins` (elementary math, trig,
  `sum`/`prod`/`min`/`max`/`mean`, `zeros`/`ones`/`eye`/`linspace`, relational/logical,
  `rand`/`randn` via `rand`/`rand_distr`).
- **Tests:** unit tests checking faer results against known matrices; port the `tests/` `.m`
  cases for math/matrix/trig/stat that these builtins cover.
- **Acceptance:** Definition of Done; `cargo run -p fm-cli` then `A=[1 2;3 4]; A*A'`, plus
  `eig`/`svd`/`lu`/`\` give correct results; the math/matrix conformance subset is green.

## Stage 6 — `fm-builtins`: remaining core functions
- **Goal:** finish the libCore builtin surface.
- **Prerequisites:** Stage 5.
- **Reference:** the rest of `libs/libCore/*.cpp` (~70 files total) and `libs/libFN/`.
- **Build:** array manipulation (`reshape`, `cat`/`[ ]`, `repmat`, `sort`, `find`, `unique`,
  `permute`, `squeeze`), string functions, set operations, type conversions, `isa`/`class`,
  cell/struct ops. Lean on `toolbox/*.m` for anything already written in FreeMat-language.
- **Tests:** port the matching `tests/` `.m` cases (array/string/util/general subdirs);
  pass-rate should jump.
- **Acceptance:** Definition of Done; the bulk of the non-graphics conformance suite is green.

## Stage 7 — `fm-graphics` + webserver + Plotly  ·  **★ Milestone 2**
- **Goal:** `plot(...)` from the terminal renders in a browser.
- **Prerequisites:** Stage 6.
- **Reference:** `libs/libGraphics/` handle objects (`HandleFigure`, `HandleAxis`,
  `HandleLineSeries`, `HandleSurface`, ...), `RenderEngine.hpp` (for the property model), and
  `toolbox/graph*` / plotting `.m` files.
- **Build:** `fm-graphics` retained scene-graph (figure→axes→series) + **serde JSON** semantic
  protocol (series x/y/style; surface Z+colormap; axes limits/scale/labels). Low-level handle
  builtins so the `toolbox` plotting `.m` files work mostly unchanged. In `fm-cli`, embed an
  **axum/tokio** webserver + **websocket**; serve `web/index.html` (loads Plotly + a small JS
  client). `drawnow` pushes the scene; Plotly renders; browser interactions stream back.
- **Tests:** unit tests for scene→JSON serialization; a smoke test that `plot` produces the
  expected JSON payload. (Visual check is manual.)
- **Acceptance:** Definition of Done; launch CLI, `x=0:.1:10; plot(x,sin(x))`, a figure appears
  in the browser (served URL) and a second `plot` updates it live over the websocket.

## Stage 7.5 — Graphics handle-property system (set/get, subplot, contour)
- **Goal:** real graphics handles + a `set`/`get` property model — unlocking `subplot`, `contour`,
  multiple axes per figure, and live property tweaking. This is the single biggest gap for
  realistic plotting (Stage 7 shipped single-axes-per-figure with no property system). **Scheduled
  after Stage 8.**
- **Prerequisites:** Stage 7.
- **Reference:** `libs/libGraphics/` — `HandleObject`/`HandleFigure`/`HandleAxis` property model,
  `HandleCommands.cpp` (`hcreate`/`hset`/`hget`/`hline`/…), `RenderEngine.hpp`; toolbox plotting
  `.m` (`subplot.m`, `contour`, `newplot.m`, `axes`, and `title`/`axis`/`xlabel` as property
  setters).
- **Build:**
  - **Handle registry + objects:** a handle-id → object map in interpreter graphics state; objects
    (figure/axes/line/surface/image/text) carry a **property bag**. Builtins: `set(h,'Prop',val)` /
    `get(h,'Prop')`, real-handle `gcf`/`gca`, `figure(n)`, `axes(h)`, `delete(h)`, `cla`/`clf`,
    `findobj`, `ishandle`.
  - **Multiple axes per figure:** extend the `fm-graphics` scene model to N axes with position
    rectangles; `subplot(m,n,p)` creates/selects a grid cell.
  - **Property → scene mapping:** serialize axes positions + properties so the Plotly frontend lays
    out **subplots** (`xaxis2`/`yaxis2`… domains) and honors live property changes; add `contour`
    (Plotly contour trace) and other handle-gated plot types.
  - The toolbox plotting `.m` files (`subplot`, `contour`, label/axis setters) then run mostly
    unchanged on top of the handle builtins.
- **Tests:** `set`/`get` round-trip per object; `subplot(2,1,1); plot(..); subplot(2,1,2); plot(..)`
  yields two axes in one figure (assert the scene JSON); enable the deferred `tests/handle` dir.
- **Acceptance:** Definition of Done; from the REPL a `subplot` grid renders multiple plots in one
  browser figure, `set`/`get` adjust properties live, and the `handle` conformance subset passes.

## Perf fix (before Stage 8) — in-place indexed assignment

### Context
The tight indexed-assignment loop `A = zeros(1000); for i=1:1000; for j=1:1000; A(i,j)=i+j; end; end`
runs in ~10 s in the Rust interpreter vs **0.78 s** in the original C++ (JIT off) — a >10×
regression. Root cause confirmed in code: `index::scatter` (`fm-interp/src/index.rs:434`) handles
**every** indexed write by materializing the whole array to a `Vec` via `to_f64_vec(base)`,
writing one element, then rebuilding a fresh `Arc` (`build_real`, line 439) — i.e. O(N) per
single-element write. The assignment path (`interp.rs:386–391`) also clones the array out of the
symbol table first (`lvalue_base` `.cloned()`, line 430), so the existing COW accessors are never
exercised. `gather` (`index.rs:276`) has the same whole-array materialization on reads.

**The core `Array` design is sound and does NOT change.** The enum + `Arc` COW + `make_mut_*`
accessors (`fm-core/src/array.rs:421–435`, generated by the `cow_accessor!` macro) are exactly
what's needed — the defect is purely the interpreter's scatter/gather hot path bypassing them.
That's why this is worth fixing before Stage 8: it implies **no core-data-structure rework**.

### Fix — mutate the symbol-table slot in place via the existing COW accessors
1. **Owning/mutable slot access** (`context.rs`, `scope.rs`): add `take_local(name) -> Option<Array>`
   (+ matching `set`/`lookup_mut`). Replace the indexed-assignment `clone → scatter → store-back`
   with **take → mutate → put-back**, so the owned `Array` has `Arc` strong-count 1 in the common
   (non-aliased) case and `make_mut` mutates in place rather than deep-copying.
2. **In-place scatter** (`index.rs`): make `scatter_into(target: &mut Array, plan, rhs)` that, for
   the hot case — same result class, in-bounds, no growth, not cell/struct/char/complex/deletion —
   calls `target.make_mut_<class>()` for `&mut ArrayD<T>`, takes `as_slice_memory_order_mut()`
   (column-major, matching how `plan.linear` is computed), and writes only the `plan.linear`
   positions. O(count), no whole-array copy. **COW is preserved automatically**: if the buffer is
   shared (`B = A`), `make_mut` clones once then writes. Fall back to the existing
   materialize+rebuild path (assigned into `*target`) for grow / type-promote / complex / char /
   cell / struct / deletion.
3. **In-place gather** (`index.rs` `gather`/`gather_unchecked`): index `as_slice_memory_order()` at
   `plan.linear` directly instead of `to_f64_vec` of the whole array. O(count) reads.
4. **(Optional, cheap)** swap the symbol-table `HashMap` for `rustc-hash`/`FxHashMap` to cut
   per-iteration variable-name hashing.

`assign_to` (`interp.rs`) Index branch becomes: read dims (cheap), build the plan (evaluates
`i`,`j`), `take` the slot, `scatter_into(&mut arr, …)`, `set` it back. Nested l-values (`a.b(2)`)
keep the current path (rare, not hot).

### Files
- `crates/fm-interp/src/index.rs` — `scatter` → in-place `scatter_into` + fallback; O(count) `gather`.
- `crates/fm-interp/src/interp.rs` — `assign_to` Index branch (take/plan/scatter_into/set).
- `crates/fm-interp/src/context.rs`, `scope.rs` — `take_local`/`set`/`lookup_mut` (+ optional FxHashMap).
- `crates/fm-core/src/array.rs` — expected unchanged (reuse `make_mut_*`); add a thin
  memory-order-mut helper only if the interpreter can't reach `as_slice_memory_order_mut()` cleanly.

### COW correctness (must-have tests — the fast path must not break copy-on-write)
- `B = A; A(i,j) = x` ⇒ **B unchanged** (aliasing), and the non-aliased loop mutates in place.
- Value-correctness for scalar / vector / logical / range / `:` scatter, unchanged vs today.
- Fallback paths still correct: growth (`A(2000)=1`), type-promote (`A(1)=int8(5)`), complex,
  char, cell/struct paren-assign, deletion `A(1)=[]`.

### Verification
- **Baseline + after:** time the benchmark loop before (~10 s) and after; target sub-second (aim
  at/below the 0.78 s C++ baseline). Add a dev-only `criterion` bench (or an `#[ignore]`d timing
  test) — not a gating wall-clock assert (flaky).
- `cargo build` / `clippy -D warnings` / `fmt --check` / `test` all green.
- Conformance reporter: **no regression** (indexing-heavy tests stay green; pass-rate ≥ current).
- Manual: run the loop in `cargo run -p fm-cli` and confirm it returns promptly.

## Stage 8 — `fm-io`: MAT files, file I/O, FFT, regex
- **Goal:** persistence and signal/string builtins; unlocks `.mat`-fixture conformance tests.
- **Prerequisites:** Stage 6 (Stage 7 independent).
- **Reference:** `libs/libCore/MatIO.cpp`, `FFT.cpp`; `Serialize.cpp`, `File.cpp`.
- **Build:** MAT-file read/write (matfile 7.x; `flate2` for compression), `save`/`load`, file
  I/O builtins, `fft`/`ifft` via **rustfft**, regex builtins via **regex**.
- **Tests:** round-trip MAT read/write; enable the `tests/` cases needing `.mat` fixtures; FFT
  against known transforms.
- **Acceptance:** Definition of Done; `.mat`-dependent conformance tests now run and pass.

## Stage 9 — Advanced / optional (native-gated)
- **Goal:** the long tail, prioritized by demand.
- **Prerequisites:** Stage 6+.
- **Build (pick per need):** sparse matrices (`sprs`/faer-sparse) + sparse solvers; special
  functions (`statrs`); optimization (`argmin` / port levmar); audio (`cpal`); interactive 3D
  plot polish; optional cranelift JIT for hot scalar loops.
- **Acceptance:** Definition of Done per item; conformance pass-rate continues to climb.

## Stage 10 — Debugging & editor integration (DAP + `db*` engine; optional LSP)
- **Goal:** FreeMat's editor+debugger experience — set breakpoints, step, inspect/modify
  variables, walk the call stack — driven from the user's own editor (VS Code/Neovim), plus the
  terminal `db*` commands. No GUI toolkit, no built-in editor.
- **Prerequisites:** Stage 6 (mature interpreter + scopes); intended after Stages 7–8. Relies on
  the Stage 3 debug seams (chokepoint + per-scope source line + switchable active scope).
- **Reference:** `libs/libFreeMat/Interpreter.cpp` — `bpStack`, `processBreakpoints` (~L1879),
  `doDebugCycle` (~L1592), `addBreakpoint`/`deleteBreakpoint`, `stackTrace` (~L2620),
  `dbup`/`dbdown` (L659-703); `libs/libCore/Debug.cpp` (`dbstop`/`dbdelete`/`dblist`);
  `Control.cpp` (`dbauto`); `libs/libXP/Editor.cpp`, `StackTool.cpp`, `VariablesTool.cpp` (the
  observer/event model to generalize).
- **Build:**
  - **Debug engine in `fm-interp`:** breakpoint registry keyed by (file/function, line); a check
    at the Stage 3 statement chokepoint; a debug-cycle that re-enters the REPL with the paused
    scope active (FreeMat's `keyboard` scope); step/trace traps; `dbstop if error` (autostop).
  - **`db*` builtins/statements:** `dbstop`, `dbclear`/`dbdelete`, `dblist`, `dbstep`, `dbcont`,
    `dbstack`, `dbup`, `dbdown`, `dbquit`, `keyboard` — matching FreeMat semantics. `edit`/`open`
    launches `$EDITOR` (bring-your-own).
  - **Protocol-agnostic debug-event interface:** replace FreeMat's Qt signals
    (`ShowActiveLine`/`updateVarView`/`updateStackView`/`RefreshBPLists`) with a Rust trait /
    channel the engine emits to. The terminal frontend prints; the DAP frontend translates.
  - **`fm-dap` (new crate) — a DAP server:** implement `setBreakpoints`, `launch`/`attach`,
    `stackTrace`, `scopes`, `variables`, `setVariable`, `continue`, `next`/`stepIn`/`stepOut`,
    `evaluate` (REPL in the paused frame) — mapping DAP onto the engine. Ship a minimal VS Code
    debugger contribution / launch config for `.m`.
  - **`fm-lsp` (new crate, optional) — an LSP server for `.m`:** diagnostics (reuse the
    `fm-parser` miette errors), document symbols, hover, completion, go-to-definition over the
    function table.
- **Tests:** engine unit tests (breakpoint hits the right line, step counts, scope switch for
  `dbup`/`dbdown`, variable read/modify at the paused frame); a DAP integration test driving a
  scripted session (set breakpoint → launch → hit → read variable → step → continue) over the
  protocol; port any FreeMat `tests/` debug cases.
- **Acceptance:** Definition of Done; from a terminal, `dbstop`/`dbstep`/`dbcont` work and the
  debug prompt inspects the paused scope; from VS Code (or a DAP test client), a breakpoint in a
  `.m` file pauses execution, shows the call stack and variables, and step/continue work.

---

## Help / documentation system — **done** (separate effort)

The Rust-native help system that replaces FreeMat's C++/Qt + Doxygen docs pipeline is
complete (phases P0–P8). Design contract: [`HELP_SYSTEM.md`](./HELP_SYSTEM.md); roadmap:
[`HELP_REGEN_PLAN.md`](./HELP_REGEN_PLAN.md); contributor guide:
[`WRITING_DOCS.md`](./WRITING_DOCS.md). In short: docs are embedded via `register_doc!`
next to builtins; `cargo xtask docgen` recaptures executed `fm-exec` fragment transcripts
into a compiled-in DB (CI-gated by `docgen --check`); `help <name>` renders terminal text
(+ a clickable OSC 8 browser URL) and `helpwin` opens the rich browser page (`/help`,
KaTeX + highlight.js + Plotly figures). ~558 legacy Doxygen pages were machine-migrated
(`cargo xtask migrate-docs` / `migrate-place`). See `PROGRESS.md` for status and the
deferred fragment-fidelity follow-up.

---

## Open follow-ups (decide as they surface, not blocking)
- 2D-only first vs early interactive 3D (recommend 2D Plotly traces first, 3D `surf`/`mesh`
  in Stage 9).
- Whether to keep bug-for-bug MATLAB quirks or normalize where the original deviates — decide
  per failing conformance test.
