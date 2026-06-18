# FreeMat-rs — progress tracker

Source of truth for what is done across sessions. To work on the project, read
`docs/PLAN.md` (the full plan), pick a stage, implement it to the Definition of Done,
then tick it here and commit. Leave notes for the next session under each stage.

## Stage checklist

- [x] **Stage 0 — Workspace scaffold & conventions**
- [x] **Stage 1 — `fm-core`: types & Array**
- [x] **Stage 2 — `fm-parser`: lexer + parser + AST (miette)**
- [x] **Stage 3 — `fm-interp`: evaluator, scope, registry, `.m` loader**
- [x] **Stage 4 — Conformance harness**
- [x] **Stage 5 — `fm-linalg` + core math builtins  ·  ★ Milestone 1**
- [x] **Stage 6 — `fm-builtins`: remaining core functions**
- [x] **Stage 7 — `fm-graphics` + webserver + Plotly  ·  ★ Milestone 2**
- [ ] **Stage 7.5 — Graphics handle-property system (set/get, subplot, contour)** — scheduled after Stage 8
- [x] **Stage 8 — `fm-io`: MAT files, file I/O, FFT, regex**
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

### Stage 2 — done (`fm-parser`)
- **Deps:** `miette = "7.6"` (resolved 7.6.0) and `thiserror = "2.0"` (2.0.18). Added to
  `[workspace.dependencies]`; `fm-parser` opts in. The library defines `Diagnostic`s only; the
  graphical renderer is the `fancy` miette feature, enabled by the consumer — `fm-parser`'s
  **dev-dependency** turns `fancy` on for the error-rendering test, and the CLI will enable it in
  a later stage. `fm-parser` deliberately does **not** depend on `fm-core` (parser is independent).
- **Module layout (`crates/fm-parser/src/`):**
  - `span.rs` — `Span { start, end }` byte range; `From<Span> for miette::SourceSpan` + `Range`,
    plus `merge`/`empty`/`len`.
  - `token.rs` — `Token { kind, span }` and `TokenKind` enum (reserved words as variants,
    operators as explicit variants). `reserved()` lookup, `is_binary_operator`/`is_unary_operator`,
    `describe()` for diagnostics.
  - `lexer.rs` — hand-written, pull-based `Lexer`. Handles numbers (int/float/scientific/`f`
    single/`i`,`j` imaginary + **hex `0x` extension**), `''`-escaped strings, `%` line and
    `%{ %}` block comments, `...` continuations, bracket-depth tracking, the transpose-vs-string
    rule, and **blob mode** for command syntax. `tokenize()` helper for tests. The numeric
    literal's `text` payload holds the **value only** — the `f`/`d`/`i`/`j` suffix is stripped
    from the text (so `2.0f` → `RealF("2.0")`, parseable by the interpreter) while the token
    **span still covers the whole literal** including the suffix.
  - `error.rs` — `ParseError` (`thiserror::Error` + `miette::Diagnostic`) carrying
    `#[source_code]` (owned source `String`) + one `#[label]` span + optional `#[help]`. `span()`
    accessor; `clone_shallow()` (SourceSpan isn't trivially `Clone` through derive).
  - `ast.rs` — the AST enums (below). Every node carries a `Span`.
  - `parser.rs` — recursive-descent / precedence-climbing `Parser`. Public entry points
    `parse_program` / `parse_statements` / `parse_expression`.
  - `lib.rs` — re-exports + free `parse_program`/`parse_statements`/`parse_expression`/`tokenize`.
- **AST shape (top-level):** `Program::{Script(Vec<Stmt>), Functions(Vec<FunctionDef>)}`.
  `Stmt { kind: StmtKind, terminator: Terminator, span }` where `StmtKind` covers
  `Expr / Assign / MultiAssign / Command / For / While / If / Switch / Try / Global / Persistent /
  Keyword / FunctionDef`. `Expr { kind: ExprKind, span }` where `ExprKind` covers
  `Real / Imag / Str / Ident / End / Colon / Unary / Binary / Range / Transpose / Index /
  CellIndex / Field / DynField / Matrix / Cell / FuncHandle / AnonFunc`. `BinaryOp`/`UnaryOp`/
  `KeywordStmt` are flat enums; `Terminator::{Quiet,Display}` records `;` vs `,`/newline.
- **Precedence:** reproduces FreeMat's `precedence()` table exactly (||=1, &&=2, |=3, &=4,
  relational=5, `:`=6, +/-=7, `*`/`/`/`\`/`.*`/`./`/`.\`=8, unary=9, `^`/`.^`=10). Only `^`/`.^`
  are right-associative; precedence climbing uses `q = prec` (right) vs `prec+1` (left). Unary
  prefix parses its operand at bp 9 so `-2^2 == -(2^2)`.
- **Whitespace model:** the lexer emits `Space`/`Newline` tokens and tracks bracket depth; the
  parser keeps a `ws_significant` counter (FreeMat's `m_ignorews` stack). Outside `[ ]`/`{ }`
  spaces are skipped; inside, a dedicated matrix-element parser makes a space a column separator
  **unless** it sits between a binary operator and its operand — so `[1 -2]` → 2 cols,
  `[1 - 2]` → 1 col, `[1 -2 - 3 -4]` → 3 cols (matches FreeMat's documented cases).
- **Disambiguation / backtracking:** statements starting with an identifier or `[` are parsed
  tentatively (assignment → command syntax → bare expression; multi-assign → bare expression) by
  snapshotting the `Clone`-able lexer. Command syntax (`hold on` → `Command{"hold",["on"]}`)
  uses a non-blob lookahead probe (FreeMat's `t_lex`) then toggles lexer blob mode. A
  `furthest`-error tracker (FreeMat's `lastpos`/`lasterr`) records the deepest error — including
  **lexer** errors — across backtracks so the user sees the most relevant diagnostic rather than a
  shallow "expected terminator".
- **Diagnostics:** with the `fancy` feature a malformed snippet renders an annotated,
  source-highlighted message (verified by `tests/errors.rs::diagnostic_renders_annotated_snippet`).
- **Tests (62):** `tests/lexer.rs` (15) — numbers incl. hex, `''` strings, transpose-vs-string,
  multi-char & element-wise operators, line/block comments, `...` continuation, reserved words,
  span offsets, blob mode; `tests/parser.rs` (38) — precedence/associativity, ranges, matrix/cell
  literals + unary disambiguation, indexing/field/brace/dyn-field chains, magic colon, `end`,
  transpose, anon funcs & handles, assignments incl. multi-LHS, command syntax, every control-flow
  construct, function defs (outputs/inputs/by-ref/nested/multiple), a real `linspace.m` snippet;
  `tests/errors.rs` (9) — span-precise error-path assertions + fancy render. Plus 1 lib doctest.
  The Stage-0 `scaffold_builds` placeholder was removed.
- **Design decisions / deferrals:**
  - **`%{ %}` block comments** and **hex `0x` literals** are MATLAB-compatible **additions** — the
    C++ `Scanner.cpp` lacks both (its `fetchComment` only runs to EOL; no hex path). Block comments
    require `%{`/`%}` alone on their line (only blanks around them) and nest.
  - **`end` after a single non-nested function is optional** (FreeMat/MATLAB script-function form):
    `function y = f(x)\n y = x+1;` parses. `end` is consumed when present; nested functions inside
    a body are collected into `FunctionDef::nested`.
  - The Octave-compatibility paths in `Parser.cpp` (`octCompat`: `++`/`--`, `+=`/`-=`, `A()`
    blank-ref, re-indexing) are **not** ported — FreeMat defaults to MATLAB mode and the plan
    targets MATLAB compatibility. Revisit only if a conformance test needs Octave syntax.
  - The bare `:` magic-colon is recognized only as a standalone index argument (`A(:)`,
    `A(:, 1)`); elsewhere `:` builds a `Range`. `for (i = expr)` parenthesized headers are accepted.
  - `Command` carries the parsed string args directly (the interpreter will call
    `name('arg1','arg2')` in Stage 3); we don't synthesize a call node at parse time.
  - **Number-suffix text:** the `f`/`d` single/double markers and `i`/`j` imaginary markers are
    stripped from the literal's text payload (Stage 3 parses `text` straight to `f64`/`f32`); the
    token/AST span deliberately still spans the suffix so diagnostics highlight the full literal.
- **`tests/` syntax pulled from `FreeMat/tests/`:** the error-path snippets were modelled on
  `FreeMat/tests/parse/bad*.m` (e.g. `bad18.m`'s `a(]`, dangling-operator and unterminated
  constructs); the `switch` string-label and control-flow shapes follow `tests/flow/test_switch1.m`;
  the `linspace.m` function-def snippet is adapted from `toolbox/array/linspace.m`.

### Stage 3 — done (`fm-interp`)
- **Deps:** reuses `fm-core` + `fm-parser` (path deps) and `ndarray` (0.17, for the
  column-major rebuild/cast helpers), `miette` (7.6) + `thiserror` (2.0) for the runtime
  diagnostic. A dev-dep enables miette's `fancy` feature for error rendering. All workspace deps
  already pinned in the root `Cargo.toml`; nothing new added there.
- **Module layout (`crates/fm-interp/src/`):**
  - `error.rs` — `InterpError` (`miette::Diagnostic`, MException-like: message + optional MATLAB
    `identifier` + `#[source_code]`/`#[label]` span) and the control-flow `Signal` enum
    (`Break`/`Continue`/`Return`/`Error`) threaded through `Flow<T> = Result<T, Signal>`. No
    panics for control flow.
  - `value.rs` — flat-data bridge: `to_f64_vec`/`to_c64_vec` read any class in **column-major
    memory order** (see the ndarray gotcha below); `build_real`/`build_complex`/`char_matrix`
    rebuild typed arrays (saturating integer casts); `truth` (MATLAB all-nonzero condition),
    `to_index` (1-based→0-based).
  - `ops.rs` — operator dispatch: element-wise `+ - .* ./ .\ .^`, relational, `& |`, unary
    `+ - ~`, with **scalar↔array and singleton-dimension broadcasting** + `fm_core::promote`
    type promotion (relational/logical always yield `logical`); naive column-major `*` matmul;
    scalar `^`/`/`; matrix solve/`\`/matrix-power return a clear "not yet (Stage 5)" error.
  - `index.rs` — indexing engine over column-major linear positions: `plan_index` (linear +
    N-D subscript, `:`, logical masks), `gather`/`gather_cell_contents` (reads), `scatter`/
    `scatter_cell`/`scatter_cell_contents` (writes with **grow-on-assign**), `field_read`/
    `field_write` (struct fields, grow-from-`[]`).
  - `scope.rs` — `Scope`: locals + global/persistent **name** declarations + the **current
    statement span/line** (debug seam).
  - `context.rs` — `Context`: scope stack + call stack, shared `globals` table, per-function
    `persistents` table (keyed `function\0name`), and a **switchable active scope**
    (`set_active`/`active_index`) for future `dbup`/`dbdown`; `lookup`/`assign` honour the top
    scope's global/persistent declarations; `stack_trace` for `dbstack`.
  - `function.rs` — `Function` enum (`Builtin { fn(&mut Interpreter, &[Array], nargout) }` vs
    `Interpreted { Arc<FunctionDef>, Arc<String> src }`) + `FunctionTable` with
    `add_builtin`/`add_interpreted` (the `addFunction`/`addSpecialFunction` analogue — every
    builtin gets the interpreter, so "special" needs nothing extra).
  - `interp.rs` — the evaluator (`Interpreter`): expression eval (`eval`/`eval_multi`),
    statement execution, control flow, assignment + LHS indexing + multi-return, function calls,
    matrix/cell literals, ranges, transpose, concatenation (`hcat`/`vcat`/cell concat).
  - `builtins.rs` — the minimal builtin set + type-cast builtins.
  - `loader.rs` — `load_file`/`define_source`: parse a `.m` via `fm_parser` and register its
    functions through the same evaluator.
- **Debug-readiness seams (where they live):**
  1. **Single statement chokepoint** — `Interpreter::exec_statement` (`interp.rs`). *Every*
     statement (top-level, loop body, function body) routes through it; a comment marks the exact
     spot where Stage 10 inserts `self.check_breakpoint(stmt)?`.
  2. **Per-scope current line/span** — `exec_statement` calls `Context::set_current_span`, which
     writes `Scope::current_span`/`current_line` on the executing (top) scope before running it.
     `Context::stack_trace()` reads these for `dbstack`.
  3. **Switchable active scope** — `Context::{active, set_active, active_index, active_mut}`
     (`context.rs`): the inspected scope is decoupled from the executing scope, the basis for
     `dbup`/`dbdown`. Exercised by `scoping.rs::debug_seam_active_scope_switchable`.
- **Operator / indexing / control-flow coverage:**
  - Operators: all element-wise arithmetic + relational + logical + unary, broadcasting
    (scalar + singleton-dim), promotion (double-dominant, integer-keeps-class, single), complex
    lane (`+ - .* ./ .^`, `*`, `==`), naive matmul, transpose `'`/`.'`.
  - Indexing: linear & subscript reads, `:` magic colon, `end` (incl. inside ranges/arith),
    logical masks (linear + per-dim), ranges; assignment in place, grow-on-assign (vector + N-D),
    grow-from-nothing, logical-mask assign; cell `{}` read/write + grow; struct `.f` and dynamic
    `.(expr)` read/write + grow-from-`[]`.
  - Control flow: `for` (iterates **columns**, MATLAB semantics), `while`, `if/elseif/else`,
    `switch/case/otherwise` (numeric, string, cell-of-alternatives), `try/catch` (binds the
    message to `lasterr`), `break`/`continue`/`return`, short-circuit `&&`/`||`.
  - Functions: positional inputs, `varargin`/`varargout`, multi-return `[a,b]=f(...)`, `nargin`/
    `nargout`, recursion, `ans` echo. Built-in constants `pi e Inf NaN eps i j true false`.
- **Builtins added (minimal, per the plan):** `disp`/`display`, `error` (incl. `id:comp` form),
  `size` (incl. `size(x,dim)` and multi-return), `numel`, `length`, `ndims`, `isempty`, `prod`,
  `sum`, `class`, `zeros`, `ones`, `isa`, `ischar`, `isnumeric`, `iscell`, `isreal`, `mod`,
  `rem`, `abs`, `floor`, `ceil`, `round`, `num2str`, and the type casts `double single logical
  char int8..uint64`. The bulk is deferred to Stage 5/6.
- **`.m` end-to-end:** `tests/toolbox.rs` loads and runs the real `toolbox/array/isvector.m`
  (exercises `size`, `prod`, paren-index, `==`, `*`, `&&`, `||`) and `toolbox/array/isscalar.m`
  (`numel`, `==`) unchanged through the loader + evaluator.
- **Tests (50 integration + 1 doctest):** `operators.rs` (13), `indexing.rs` (14),
  `control_flow.rs` (12), `scoping.rs` (9 — local/global/persistent, multi-return, nargin,
  recursion, active-scope switch), `toolbox.rs` (2). Stage-0 `scaffold_builds` placeholder
  removed. `cargo test --workspace` green; `clippy --workspace --all-targets -D warnings` clean;
  `fmt --all --check` passes.
- **Critical decision — column-major reads:** `fm-core` stores dense buffers F-order, but
  ndarray's `.iter()` walks **logical (row-major)** order. All flat reads therefore go through
  `value::mem_order` (= `as_slice_memory_order`, with a `.t().iter()` fallback) so linear/COW
  index positions line up with the column-major model. This was the one real footgun; not a
  `fm-core` bug, just an ndarray API subtlety (documented here so the next session doesn't trip).
- **Deferrals / decisions (MATLAB-compatible choices made where ambiguous):**
  - **Stage 5:** matrix `\`/`/` solve, matrix `^`, and faer-backed `*` — `*` is a naive
    column-major matmul for now; solves error with an explicit "(Stage 5)" message.
  - **Function handles / anonymous functions** (`@f`, `@(x)...`) parse but error at eval — they
    arrive with the closure/dispatch work in Stage 5/6.
  - `try/catch` binds the error message to `lasterr`; binding a full **MException object** to the
    catch identifier waits for Stage 6 (the `InterpError.identifier` field is already carried).
  - `~` as a multi-LHS placeholder: the **parser** doesn't yet treat `~` as a discard token
    (`[~, y] = f()` fails to parse). The evaluator already discards an `Ident("~")` target, so
    this lights up for free once the parser supports it — left as a parser TODO (did not modify
    `fm-parser` per the stage constraints).
  - `prod`/`sum` implement the vector + 2-D column-reduction cases needed now (full N-D reductions
    are Stage 6).
  - Nested functions are registered as siblings in the flat function table (simple Stage 3
    model; proper nested-scope capture is a later refinement).

### Stage 4 — done (`fm-conformance`)
- **New crate `crates/fm-conformance`** (lib + `fm-conformance` bin), no new external deps — it
  drives `fm-interp` directly. Picked up by the `members = ["crates/*"]` glob (no root edit).
- **FreeMat's pass/fail convention (reproduced):** FreeMat runs each test in *test mode* (`-t`,
  `src/main.cpp`) which seeds the process exit code to **1 = fail**. A test is a function file
  `test_NAME.m` = `function test_val = test_NAME` (one output; the var name varies). The CMake
  wrapper calls `wrap_test('test_NAME')` — run the function in `try`/`catch`, only flip the exit
  code to 0 on success — or `~test_NAME` in `suite/`. A test **passes** iff the function runs
  without raising **and** returns an all-nonzero (true) value. The `test()`/`testeq()` helpers
  (`toolbox/array/test.m`, `tests/flow/testeq.m`) just reduce to a scalar logical; they do **not**
  throw, so the truthiness of the returned value is what matters.
  Our harness reproduces this in `run_test_file`: spin up a fresh `Interpreter` per test (FreeMat's
  process-per-test isolation), `load_file` every `.m` in the test's directory + a shared
  `_helpers/` dir (so the test fn and its `.m` helpers all register), then
  `call_function(stem, &[], 1, …)` and classify via `fm_interp::value::truth`:
  `Pass` (true) / `Fail` (false/empty/wrong) / `Error` (interpreter raised — missing builtin or
  unsupported feature, **or** a caught panic). An `Error` is **never** counted as a pass — the
  pass-rate is honest. Interpreter panics are caught with `catch_unwind` and a silenced panic hook
  so one tree-walker bug can't abort the run.
- **Self-contained corpus:** the targeted `.m` tests are copied into
  `crates/fm-conformance/data/tests/<dir>/` (+ `_helpers/test.m`, `_helpers/testeq.m`). The harness
  reads only from there — it does **not** touch `../FreeMat` at test time (asserted by
  `corpus_is_self_contained`). 632 `.m` files copied; `test_files_in` runs only the `test_*.m`
  stems (helpers are loaded but not invoked as tests), giving **603** runnable test functions.
- **Two-tier reporting:**
  - **Curated must-pass subset (gates `cargo test`)** — `tests/curated.rs::curated_subset_passes`
    asserts **50** named, currently-passing tests across array / flow / functions / operators /
    elementary / typecast / suite / variables (assignment, if/switch, continue, error, ranges,
    nargin, persistents, struct/cell subsetting, matrix concat, uint64 round-trip). Plus
    `full_suite_pass_count_does_not_regress` asserts the aggregate pass count ≥ a floor of **62**
    (raise the floor as later stages improve it). These run in normal `cargo test`.
  - **Non-gating reporter** — the `fm-conformance` **binary** (`cargo run -p fm-conformance`) and an
    `#[ignore]`d `tests/curated.rs::full_suite_report` print totals + a per-dir pass/fail/error
    breakdown. Many tests fail today (expected); this never breaks the build.
  - Run the reporter: `cargo run -p fm-conformance` (whole covered corpus),
    `cargo run -p fm-conformance -- flow` (one dir), add `--failures` / `--passes` to list tests, or
    `cargo test -p fm-conformance -- --ignored --nocapture full_suite_report`.
- **Current full-suite pass-rate: 62 / 603 = 10.3%.** Per covered directory (total / pass / fail /
  error):

  | dir | total | pass | fail | error |
  |---|---:|---:|---:|---:|
  | array | 68 | 7 | 6 | 55 |
  | binary | 3 | 0 | 0 | 3 |
  | constants | 1 | 0 | 0 | 1 |
  | elementary | 7 | 1 | 0 | 6 |
  | flow | 22 | 12 | 2 | 8 |
  | freemat | 11 | 0 | 0 | 11 |
  | functions | 9 | 2 | 2 | 5 |
  | inspection | 20 | 0 | 0 | 20 |
  | io | 6 | 0 | 0 | 6 |
  | operators | 66 | 1 | 0 | 65 |
  | random | 1 | 0 | 0 | 1 |
  | signal | 1 | 0 | 0 | 1 |
  | string | 3 | 0 | 0 | 3 |
  | suite | 337 | 32 | 10 | 295 |
  | typecast | 5 | 1 | 0 | 4 |
  | variables | 43 | 6 | 1 | 36 |
  | **TOTAL** | **603** | **62** | **21** | **520** |

  The 520 `Error`s are dominated by **missing builtins** (honest "unsupported", not inflated):
  `all` (80 — used by the `test()` helper, so landing it alone flips many), `rand`/`randn`/`sprandn`
  (134), `sparse`/`sparse_test_mat` (82), `typeof` (26), `struct` (20), `repmat`/`diag`/`strcmp`/
  `qr`/`eval`/`save`/… — all Stage 5/6/8 surface.
- **Covered vs deferred directories.** *Covered (run here):* array, binary, constants, elementary,
  flow, freemat, functions, inspection, io, operators, random, signal, string, suite, typecast,
  variables. *Deferred (recorded, not run, don't fail the build):* `reference` & `matcompat`
  (367+ `.mat` fixtures — need Stage 8 `fm-io`), `transforms`/`curvefit`/`signal`-wb (`.mat`
  reference + linalg/fft — Stage 5/8), `sparse` (no sparse type — Stage 9), `class` (user OOP),
  `handle`/`glwin`/`vtk*` (graphics — Stage 7 / dropped), `jit`/`itk` (dropped per plan), `parse`
  (negative parse tests — opposite convention), `debug` (Stage 10), `thread`/`os`/`external`/`num`/
  `mathfunctions` (empty or native-only). The `wb_test(...)` whitebox cases inside otherwise-covered
  dirs are skipped automatically — they don't follow the `test_*.m` naming and need `.mat`
  references.
- **Interpreter bugs found (documented for later, NOT fixed here per stage constraints):**
  1. **Struct-array concatenation `[a,b]` panics** in `fm-core/src/array.rs:75`
     ("element count must match dims") — `suite/test_struct1`, `suite/test_struct2` (and the
     `variables/` copies). The struct-array `hcat`/`vcat` path builds a struct array with mismatched
     element/dim counts. Address in the Stage 6 struct/cell ops work.
  2. **Element-deletion assignment `x(i) = []` unimplemented** — reports
     "assignment size mismatch: 0 elements into 1 positions" instead of deleting the indexed
     element/row/col. Tests: `suite/test_assign19` (`g(1) = []`), `suite/test_subset19`,
     `suite/test_struct3` (+ `array/`, `variables/` copies). This is the empty-RHS delete case of
     LHS indexing in `fm-interp/src/index.rs` (scatter); slot it into the Stage 6 indexing work.
  - Neither blocks the harness (both surface as honest `Error`s); they will flip to `Pass` once the
    relevant stage lands.

### Stage 5 — done (`fm-linalg` + first builtins tranche + REPL · ★ Milestone 1)

- **Deps added (all pinned in root `[workspace.dependencies]`):** `faer = "0.24"`
  (0.24.0), `rand = "0.9"` (0.9.x), `rand_distr = "0.5"`, `rustyline = "15"`. `fm-linalg`
  opts into `faer`/`ndarray`/`num-complex`; `fm-builtins` into `fm-core`/`fm-interp`/`fm-linalg`/
  `rand`/`rand_distr`; `fm-cli` into `fm-interp`/`fm-builtins`/`rustyline`/`miette` (`fancy`).

- **`fm-linalg` (already on disk from the prior session; this session added the missing
  `src/tests.rs`, fixed `qr`, kept the rest).** Operates on `fm_core::Array` via a `MatData`
  capture that reads any class into a **column-major `c64`** buffer (records `complex` so real
  results narrow back). Public surface: `mtimes`, `mldivide` (`\`), `mrdivide` (`/`), `mpower`
  (`A^p`, integer p, neg via `inv`), `inv`, `det`, `lu(nargout)` (`[L,U]` folds P into L so
  `L*U==A`; `[L,U,P]` gives `P*A==L*U`), `qr(nargout)` (full `m×m` Q + full `m×n` R — **fix:** pad
  faer's `min(m,n)×n` R with zero rows), `svd(nargout)` (`s` / `[U,S,V]`), `eig(nargout)`
  (`new_from_real` for real input so real-symmetric stays real; `[V,D]` for 2 outputs), `chol`
  (upper `R`, `R'*R==A`), `norm(NormKind::{Two,One,Inf,Fro})`, `rank`, `pinv`. **faer 0.24 entry
  points:** `MatRef::from_column_major_slice`, `&a * &b` matmul, `partial_piv_lu().solve_in_place`,
  `qr().{compute_Q, R, solve_lstsq_in_place}`, `Svd::new` + `.pseudoinverse()`, `Eigen::new` /
  `Eigen::new_from_real`, `Llt::new(_, Side::Lower)`, `MatRef::{determinant, norm_l2, norm_l1,
  norm_max}`. 18 unit tests vs known matrices, all green.

- **Operator rewiring (`fm-interp/src/ops.rs`; added `fm-interp → fm-linalg` dep, no cycle):**
  `mul` routes non-scalar `*` to `fm_linalg::mtimes`; `div` routes matrix `\`/`/` to
  `mldivide`/`mrdivide`; `pow` routes matrix `A^p` to `mpower`. Scalar and element-wise
  (`.* ./ .\ .^`) forms stay in `ops.rs`. The naive column-major matmul and the "(Stage 5)"
  deferral errors are gone. The Stage-3 `matrix_solve_deferred_to_stage5` test was rewritten to
  assert the real solve.

- **`fm-builtins` (filled in; was a Stage-0 scaffold).** Exposes
  `register_standard_library(&mut Interpreter)` (and `register_into(&mut FunctionTable)`), layered
  on top of the interpreter's minimal Stage-3 defaults. Modules + builtins added:
  - **elementary:** `sqrt`/`log`/`log2`/`log10` (negative-real → complex), `exp`, `log1p`/`expm1`,
    `power` (`.^`), `sign`, `fix`, `conj`/`real`/`imag`/`angle`/`arg`, `hypot`, `gcd`/`lcm`,
    `factorial`, `isnan`/`isinf`/`isfinite`, `deg2rad`/`rad2deg`.
  - **trig:** `sin`/`cos`/`tan`/`asin`/`acos`/`atan`, `sinh`…`atanh`, `sec`/`csc`/`cot`, `atan2`.
  - **reductions** (first-non-singleton dim, or explicit `dim`): `sum`/`prod`/`mean`/`cumsum`/
    `cumprod`/`median`/`var`/`std`; `min`/`max` return `[val, idx]` (and the `min(a,b)` two-arg
    element-wise form).
  - **logical:** `all`/`any` (**the high-priority unblocker** — `test()` is `all(x(:))`),
    `xor`/`not`/`isequal`/`find`, plus full-size `true`/`false`.
  - **constructors:** `zeros`/`ones` (full size-vector forms), `eye`, `linspace`/`logspace`,
    `repmat`, `diag` (build-from-vector / extract-diagonal).
  - **random:** `rand`/`randn`/`randi` via `rand::rng()` + `rand_distr::StandardNormal`.
  - **linalg wrappers:** `inv`/`det`/`eig`/`svd`/`lu`/`qr`/`chol`/`norm`/`rank`/`pinv`/`trace`.
  - **inspection/type/string:** `typeof` (complex → `complex`/`dcomplex`), `float`, `complex`/
    `dcomplex`, `feps`/`realmax`/`realmin`, `strcmp`/`strcmpi`, `iscellstr`, `issame`,
    `iscomplex`/`isfloat`/`isinteger`/`islogical`/`isvector`/`isscalar`/`ismatrix`/`isrow`/
    `iscolumn`/`isstruct`. 27 builtins integration tests (drive the interpreter end-to-end).

- **`fm-cli` REPL (`cargo run -p fm-cli`):** a lean `rustyline::DefaultEditor` loop — builds an
  `Interpreter`, registers the standard library, evaluates each line via `interp.run`, prints
  buffered output (`take_output`; trailing `;` suppresses the echo per the interpreter), and
  renders runtime/parse errors through `miette`'s `GraphicalReportHandler` (unicode theme).
  Ctrl-C abandons the line, Ctrl-D / `quit` / `exit` leaves. Graphics webserver is Stage 7.

- **Milestone 1 acceptance (evidence):** `crates/fm-conformance/tests/curated.rs::
  milestone1_acceptance` + `fm-builtins` tests verify `A=[1 2;3 4]; A*A' = [5 11;11 25]`,
  `eig([2 0;0 3]) = {2,3}`, `svd([3 0;0 4]) = {4,3}`, `lu([4 3;6 3])` reconstructs, and
  `[2 0;0 4]\[2;8] = [1;2]`. The live REPL prints the same.

- **Conformance: 62/603 (10.3%) → 198/603 (32.8%), Δ +136 tests.** Harness now registers the
  full standard library (`run_test_file`). Per-dir (total / pass / fail / error):

  | dir | total | pass | fail | error |
  |---|---:|---:|---:|---:|
  | array | 68 | 23 | 8 | 37 |
  | binary | 3 | 0 | 0 | 3 |
  | constants | 1 | 0 | 0 | 1 |
  | elementary | 7 | 5 | 0 | 2 |
  | flow | 22 | 20 | 2 | 0 |
  | freemat | 11 | 0 | 0 | 11 |
  | functions | 9 | 5 | 3 | 1 |
  | inspection | 20 | 9 | 2 | 9 |
  | io | 6 | 0 | 0 | 6 |
  | operators | 66 | 9 | 1 | 56 |
  | random | 1 | 0 | 0 | 1 |
  | signal | 1 | 0 | 0 | 1 |
  | string | 3 | 0 | 0 | 3 |
  | suite | 337 | 104 | 24 | 209 |
  | typecast | 5 | 2 | 0 | 3 |
  | variables | 43 | 21 | 7 | 15 |
  | **TOTAL** | **603** | **198** | **47** | **358** |

  Remaining `Error`s are dominated by Stage-6/8 surface: `sparse`/`sprandn`/`sparse_test_mat`
  (~169), `struct` (20), `eval`/`save`/`reshape`/`permute`/`sort`/`strcmp`-on-cells, etc.
- **Pass-floor guard raised 62 → 195** (`curated.rs::PASS_FLOOR`; live is 198, the small margin
  absorbs the few PRNG-dependent `rand`/`randn` tests). The curated must-pass list gained the
  elementary `all`-backed tests and array `det`/`diag`/`repmat`/`ones`/`isfloat`/`isinteger`.

- **Decisions / deferrals (MATLAB-compatible choices made where ambiguous):**
  - `eig` on a real input uses faer's `new_from_real` so a real-symmetric matrix returns real
    eigenvalues (not complex with tiny imaginary noise); ordering is faer's (not re-sorted —
    MATLAB itself does not guarantee eig order).
  - `qr` returns the **full** `[Q (m×m), R (m×n)]` (MATLAB default), padding faer's economy R.
  - `var`/`std` use the N-1 (sample) normalization, MATLAB's default.
  - `min`/`max` `[val,idx]` pick the **first** extreme index (MATLAB semantics); NaNs are skipped.
  - `complex`/`dcomplex` always produce a complex array; `typeof` reports `complex`/`dcomplex`
    because fm-core folds complex into the Float/Double classes (no separate complex `DataClass`).
  - `power`/`hypot`/`atan2`/`xor`/`min(a,b)` do scalar-or-equal-length broadcasting only (full
    singleton broadcasting for these stays in the operator path; Stage 6 can widen if needed).
  - The two documented `fm-core` bugs (struct-array concat, `x(i)=[]` delete) are **Stage 6** and
    were worked around, not fixed (they still surface as honest `Error`s).

### Stage 6 — done (`fm-builtins` core surface + the two documented bug fixes)

- **Deps added:** `fm-builtins` now also depends on `fm-parser` (for `eval`/`feval` parsing)
  and `ndarray` (column-major cell/char flattening helper). No new external crates.

- **Builtins added (grouped), in new `fm-builtins` modules:**
  - **`array_manip.rs` — array manipulation:** `reshape` (incl. a single `[]` placeholder
    dimension, inferred), `sort` (stable, ascending/`'descend'`, `[s,idx]`, first-non-singleton
    dim), `unique` (sorted+dedup; cell-of-strings path), `permute`/`ipermute` (N-D), `squeeze`,
    `fliplr`/`flipud`/`flip`/`rot90`, `circshift`, `sub2ind`/`ind2sub`, and `cat`/`horzcat`/
    `vertcat` (reuse the matrix-literal concat via the new `Interpreter::concat_values`).
    (`cumsum`/`cumprod` already lived in `reductions`; `repmat`/`diag` already existed —
    `repmat` was generalised to tile cells/char/complex, not just `double`, via a permutation.)
  - **`strings.rs` — string functions:** `strncmp`/`strncmpi`, `upper`/`lower` (`toupper`/
    `tolower`), `strtrim`, `deblank`, `blanks`, `strrep`, `strfind`, `strsplit`/`strjoin`,
    `str2num`/`str2double`, `mat2str`, a rewritten `num2str`, and **`sprintf`/`printf`** — a
    hand-written `printf` formatter covering `%d/%i/%u/%f/%e/%g/%s/%c/%x/%X/%o` with
    flags/width/precision, `\n`/`\t` escapes, C-style `e+NN` exponent normalisation, and MATLAB
    **argument recycling** (the format repeats over the flattened argument list).
  - **`setops.rs` — set operations:** `union`, `intersect`, `setdiff`, `ismember` (logical mask
    + optional location output), each with a numeric path and a cell-of-strings path; `unique`
    is shared from `array_manip`.
  - **`cellstruct.rs` — cell/struct ops:** `cell`, `struct` (scalar + cell-valued → struct
    array), `fieldnames`, `isfield`, `rmfield`, `getfield`/`setfield`, `orderfields`, `cell2mat`,
    `num2cell`, `struct2cell`/`cell2struct`, and the interpreter-aware `cellfun`/`structfun`
    (they re-enter `Interpreter::call_function` per element; `UniformOutput` packs scalars).
  - **`interp_ops.rs` — interpreter-aware:** `eval`/`evalin` (run a string in the current scope;
    `eval('expr')` with `nargout≥1` returns the expression's values, so `[U,S,V]=eval('svd(a)')`
    works), `feval`/`builtin`, `exist`, `isset`, `clear` (incl. `clear('all')`), `assignin`.
  - A `cell_mem_order` helper in `util.rs` flattens cell arrays in **column-major (memory)
    order** — `ndarray`'s `.iter()` is row-major, the documented footgun.

- **Bug fix 1 — struct-array concatenation `[a,b]` (was panicking in `fm-core/array.rs`).**
  Root cause: `hcat`/`vcat` had no struct path, so two structs fell through to the numeric lane
  (`result_class`→`Double`, `to_f64_vec`→empty), building a `[rows,cols]` real array with zero
  data → `from_shape_vec` panic. Fix: a new `concat_structs` in `fm-interp/interp.rs` that
  **unions field names** (first operand's order, then any new ones; missing fields filled with
  `[]`, matching FreeMat's "different fields when valid") and lays out elements column-major.
  `fm-core::StructArray` gained `from_fields`/`field_name_strings`/`field_pairs` to support it.
  Flips `suite/test_struct1`,`test_struct2` (and `variables/` copies).
- **Bug fix 2 — element-deletion `x(i) = []` (was "size mismatch", unimplemented).** Fix in
  `fm-interp/index.rs`: `scatter` now detects an empty non-cell/non-struct RHS and routes to a
  new `scatter_delete` that removes the indexed linear positions (keeping source orientation),
  or — for `x(i,:)` / `x(:,j)` — drops whole rows/columns. This needed a `deleted_axis` field on
  `IndexPlan` (set in `plan_subscript` when exactly one of two axes is non-colon). Also added
  struct-array **gather** (`s(idx)` → sub-struct array) and **`scatter_struct`** (`s(i)=struct`
  grow/overwrite with field union), so `g(3).foo=3; g(1)=[]` and `c(1)=a; c(2)=a; c(1).hoo=6`
  work. Flips `suite/test_assign19`,`test_subset19`,`test_struct3` (+ `array/`,`variables/`
  copies); also removed a separate interpreter panic in `array/test_assign9` (cell `repmat`).

- **`~` discard placeholder (item 3): not needed.** The parser already accepts `~` (it lexes as
  an identifier `"~"` and the evaluator's multi-assign already skips an `Ident("~")` target); no
  parser change was required, so `fm-parser` was left untouched.

- **Conformance: 198/603 (32.8%) → 258/603 (42.8%), Δ +60 from Stage 5 / +59 from the 33.0%
  baseline (+9.8 pp).** Per-dir (total / pass / fail / error), with the Stage-5 pass in parens:

  | dir | total | pass | (was) | fail | error |
  |---|---:|---:|---:|---:|---:|
  | array | 68 | 32 | (23) | 9 | 27 |
  | binary | 3 | 0 | (0) | 0 | 3 |
  | constants | 1 | 0 | (0) | 0 | 1 |
  | elementary | 7 | 6 | (5) | 0 | 1 |
  | flow | 22 | 20 | (20) | 2 | 0 |
  | freemat | 11 | 4 | (0) | 4 | 3 |
  | functions | 9 | 5 | (5) | 3 | 1 |
  | inspection | 20 | 15 | (9) | 3 | 2 |
  | io | 6 | 0 | (0) | 0 | 6 |
  | operators | 66 | 9 | (9) | 1 | 56 |
  | random | 1 | 0 | (0) | 0 | 1 |
  | signal | 1 | 0 | (0) | 0 | 1 |
  | string | 3 | 1 | (0) | 0 | 2 |
  | suite | 337 | 134 | (104) | 37 | 166 |
  | typecast | 5 | 2 | (2) | 0 | 3 |
  | variables | 43 | 30 | (21) | 8 | 5 |
  | **TOTAL** | **603** | **258** | **(198)** | **67** | **278** |

  Remaining `Error`s are dominated by **sparse** (`sparse`/`sprandn`/`sparse_test_mat`, ~170,
  Stage 9) — that alone is most of the `operators`/`array`/`suite` error count. Other notable
  gaps: `save`/`load`/`fopen`/`sscanf` (Stage 8 `fm-io`), `bitand`/`bitor`/`bitxor` (binary),
  `conv2`/`imwrite` (signal/io), and function-handle `@f` evaluation (deferred).

- **Pass-floor guard raised 195 → 246** (`curated.rs::PASS_FLOOR`; live is 258, margin absorbs
  the PRNG-dependent `rand`/`randn` tests and `eval2`'s `rand` matrix). The curated must-pass list
  gained the two bug-fix tests (`struct1/2/3`, `assign19`, `subset19`) plus `reshape1/2`, `sort`,
  `cell1`, `permute1/2`, `fieldnames1`, `isfield1`, `getfield1`, `eval1/2`, `feval1`.

- **Tests added:** 24 new `fm-builtins` integration tests (reshape/sort/unique/flip/circshift/
  cat/sub2ind/strings/sprintf/setops/struct/cell/cellfun/eval/deletion) and 6 new `fm-interp`
  indexing tests (vector/logical/row/column deletion, struct-array concat, struct grow+delete).

- **Decisions / deferrals (MATLAB-compatible choices where ambiguous):**
  - `struct` with differing fields across concatenation **unions** field names (FreeMat allows
    this), rather than erroring as stock MATLAB does.
  - `sprintf %g` uses Rust's shortest `f64` formatting (not C's exact `%g` precision rules);
    close enough for the corpus, revisit if a test needs strict `%g`.
  - `evalin('caller', …)` and `builtin('abs', …)` resolve in/against the **current** scope/table
    (no separate caller frame or builtin-vs-user shadowing yet) — so `freemat/test_evalin*` and
    `test_builtin1` still fail; full scope-aware `evalin` is deferred.
  - `cellfun`/`structfun` only implement the common single-output, UniformOutput cases.
  - `isset('a')` after `a=[]` returns true here (we track the binding), but FreeMat treats an
    empty assignment as unset — `inspection/test_isset1` consequently still fails; left as-is.
  - `permute`/`reshape`/`circshift`/`flip` materialise via a column-major position permutation,
    dispatching on element type (numeric/char/complex/cell) so they work for all dense classes.

### Stage 7 — done (`fm-graphics` scene-graph + webserver + Plotly · ★ Milestone 2)

- **Deps added** (pinned in root `[workspace.dependencies]`): `serde = "1"` (+ derive),
  `serde_json = "1"`, `tokio = "1"`, `axum = "0.8"` (`ws` feature), `futures-util = "0.3"`,
  `webbrowser = "1"`, and `tokio-tungstenite = "0.24"` (websocket *client*, fm-cli dev-dep only —
  drives the Milestone-2 test). `fm-graphics` opts into serde/serde_json; `fm-interp` and
  `fm-builtins` gain a path dep on `fm-graphics`; `fm-cli` opts into tokio/axum/futures-util/
  webbrowser (+ the dev-deps).

- **`fm-graphics` — the retained, renderer-agnostic scene graph + JSON wire protocol**
  (`crates/fm-graphics/src/`):
  - `scene.rs` — the semantic model: `Scene { figures: Vec<Figure> }` → `Figure { id, axes }` →
    `Axes { series, title, xlabel/ylabel/zlabel, limits, xscale/yscale, grid, legend, hold,
    equal }` → `Series` enum (`Line(LineSeries{x,y,line_style,marker,color,name})` /
    `Surface(SurfaceSeries{z,x,y,colormap,wireframe})` / `Image(ImageSeries{data,colormap})`).
    All `serde::{Serialize,Deserialize}` (round-trippable) with `skip_serializing_if` on
    defaulted fields so the wire payload stays lean. `Series`/`Scale` use `#[serde(tag=...)]` so
    the frontend dispatches on `"kind"`/string. `Scene::to_message()` wraps the scene in a tagged
    `WireMessage::Scene` envelope → `{"type":"scene","scene":{...}}` (the only message type so
    far; serialize-only because it borrows `&Scene`). `default_color(i)` reproduces FreeMat's
    `HandleAxis.cpp` default color order (blue/green/red/cyan/magenta/yellow/gray) for cycling.
  - `linespec.rs` — MATLAB linespec parser (`"r--o"` → color `rgb(...)` + line style + marker),
    matching `toolbox/graph` `islinespec`; `valid` flag lets `plot` reject non-linespec strings.
  - `lib.rs` — re-exports + the **`GraphicsSink` trait** (`fn publish(&self, &Scene)`, `Send +
    Sync`, dyn-compatible) the interpreter pushes through without depending on axum/tokio.
  - 7 unit tests: line→Plotly-shaped JSON, scene JSON round-trip, defaulted-field omission,
    figure lookup/insert, linespec parsing (incl. `-.` vs `-` vs `--`), color cycling.

- **Interpreter graphics state** (`fm-interp/src/graphics.rs`, `GraphicsState`): the retained
  `Scene` + `current_figure` id + a `dirty` flag + an optional `Box<dyn GraphicsSink>`. Added a
  `graphics: GraphicsState` field to `Interpreter` and `set_graphics_sink(...)`. **The sink is
  optional** — library/conformance tests update the scene but never publish, so nothing pulls in
  the webserver. `Interpreter::run` does an **implicit draw**: after a top-level unit, if
  `dirty`, it `flush()`es the scene through the sink (a trailing `;` suppresses value echo, NOT
  the plot — MATLAB semantics). `drawnow` flushes explicitly.

- **Graphics builtins** (`fm-builtins/src/graphics.rs`), all building the scene directly:
  `figure` (new/select by number), `plot` (x/y groups, implicit `1:n` x, trailing linespec,
  `newplot` clear-unless-hold, color cycling), `line` (always-add), `title`, `xlabel`/`ylabel`/
  `zlabel`, `legend` (names + on/off), `hold` (on/off/toggle), `axis` (`[xmin xmax ymin ymax]` /
  `equal` / `normal`/`auto` / accepted-noop `tight`/`square`), `grid` (on/off/toggle), `clf`,
  `gcf`/`gca` (return the figure handle; single-axes model), `drawnow`, `surf`/`mesh`
  (Z or X,Y,Z; wireframe flag), `image`/`imagesc`, and `semilogx`/`semilogy`/`loglog` (set log
  scale then delegate to `plot`). 7 new integration tests (line series shape/x/y/color/style,
  replace-without-hold, hold-on-append, title/label/grid/linespec, new-figure, sink flush).
  - **Deferred handle-graphics fidelity (pragmatic Milestone-2 scope):** no FreeMat
    handle-property `set`/`get` system, no real handle objects (`gcf`/`gca` return the figure
    *number* as a stand-in handle), one axes per figure (no `subplot`/multiple axes), no
    `ishandle`/`findobj`/`copyobj`, no property-value pairs on `plot` beyond a linespec, no
    `colorbar`/`colormap` selection, no text/annotation objects, no `print`/export. The
    `toolbox/graph/*.m` files (which lean on the property system) are **not** wired in; `plot`
    etc. are native builtins. These are recorded here and revisited in Stage 9 (3-D polish).

- **Webserver + websocket** (`fm-cli/src/server.rs`, exposed via a new `fm-cli` `lib.rs`):
  - `start(port)` binds a **std** `TcpListener` synchronously (so the real port is known when
    `port == 0`, and so it can be called from inside another tokio runtime — the test does),
    then spawns a background OS thread running a **multi-threaded tokio runtime** with an `axum`
    app: `GET /` serves `web/index.html` (via `include_str!`, so the binary is self-contained)
    and `GET /ws` upgrades a websocket. Returns a `ServerHandle { tx, latest, addr }`.
  - **REPL ⇄ server coexistence:** the REPL is the blocking `rustyline` loop on the main thread;
    the server runs on its own runtime/thread, so neither blocks the other. They communicate via
    a `tokio::sync::broadcast::Sender<String>` (serialized scene JSON) plus a shared
    `Arc<Mutex<Scene>>` (the latest snapshot). `ServerHandle` **implements `GraphicsSink`**: on
    `publish` it stores the snapshot and broadcasts the JSON (non-blocking; ignores "no
    subscribers"). On a new websocket connection the handler first sends the current snapshot
    (so a freshly-opened tab shows existing figures), then forwards every broadcast update.
  - `fm-cli/src/main.rs` starts the server on `127.0.0.1:0`, prints
    `Graphics server: http://127.0.0.1:<port>`, best-effort `webbrowser::open`s it, and installs
    the handle as the interpreter's graphics sink. A server-start failure is non-fatal (REPL
    still works; plotting just won't display).

- **Frontend** (`web/index.html`): plain JS, no build step. Loads **Plotly.js via CDN**
  (`cdn.plot.ly/plotly-2.35.2.min.js` — needs internet; a comment documents vendoring
  `plotly.min.js` next to the file to run offline). Opens a websocket to `/ws` (auto-reconnect on
  close), parses each `{"type":"scene",...}` message, and renders **one Plotly plot per
  `Figure`**, mapping line/surface/image series → scatter/surface/heatmap traces and axes
  fields → layout (title, axis labels, grid, log scale, limits, legend, `axis equal`). Uses
  `Plotly.react` so updates redraw **in place**; removes figures that vanish from the scene.

- **Milestone 2 — automated proof** (`fm-cli/tests/milestone2.rs`,
  `plot_streams_expected_scene_over_websocket`): starts the server on an OS port, installs it as
  the interpreter's sink, connects a `tokio-tungstenite` websocket client (the browser
  stand-in), asserts the on-connect snapshot is an empty scene, runs `x = 0:.1:10;` then
  `plot(x, sin(x));`, and asserts the client receives a `scene` message whose figure-1 axes has a
  **line trace** with `x.len()==101`, `x[0]==0`, `x[100]==10`, `y[0]==sin(0)`, `y[10]==sin(1)`,
  `color=="rgb(0,0,255)"`, `line_style=="-"`. Then `plot(x, cos(x));` and asserts the **live
  update** carries `cos` data. This exercises the exact `/ws` path a real browser uses.
  - **Manual check:** `cargo run -p fm-cli`, note the printed `Graphics server:` URL (a browser
    tab auto-opens if a browser is available — needs internet for the Plotly CDN). At the `-->`
    prompt: `x = 0:.1:10;` then `plot(x, sin(x))` — a blue sine curve appears in the browser.
    Run `plot(x, cos(x))` — the same figure updates live to a cosine. `hold on; plot(x, sin(x))`
    overlays a second (green) curve; `title('demo'); xlabel('x'); grid on` annotate it;
    `figure; plot(1:10, (1:10).^2)` opens a second Plotly figure in the page.

- **Conformance: unchanged at 258/603 (42.8%).** Stage 7 adds no `.m` graphics tests to the gate
  (graphics tests stay deferred per the plan); the graphics builtins are additive and don't touch
  the non-graphics surface. The pass-floor guard (`curated.rs::PASS_FLOOR = 246`) is untouched.

- **Verification:** `cargo build --workspace` ✓; `cargo clippy --workspace --all-targets -D
  warnings` clean ✓; `cargo fmt --all --check` ✓; `cargo test --workspace` green ✓ (incl. the
  7 fm-graphics, 7 new fm-builtins graphics, and the Milestone-2 websocket integration tests).

### Perf fix (before Stage 8) — in-place indexed assignment

- **Symptom / root cause.** `A=zeros(1000); for i=1:1000; for j=1:1000; A(i,j)=i+j; end; end`
  (1e6 single-element writes into a 1e6-element array) was catastrophically slow. Cause:
  `index::scatter` **materialised the whole array** to a `Vec<f64>` and **rebuilt a fresh `Arc`
  for every single write** — O(N) per element, O(N²) for the loop. Worse, the assignment path
  (`interp.rs` `assign_to` / `lvalue_base`) `lookup().cloned()`'d the array *out* of the symbol
  table first, so the existing COW `make_mut_*` accessors were never reached. `gather` had the same
  whole-array materialisation on reads.

- **Fix (interpreter hot path only — the `fm-core` `Array` design was NOT changed).**
  - `scope.rs` / `context.rs`: added `Scope::take_local` / `get_local_mut` and
    `Context::take` (remove & own, honouring global/persistent) + `set` (alias of `assign`). The
    indexed-assignment path now does **take → mutate → put-back** so the backing `Arc` has
    strong-count 1 in the non-aliased case and `make_mut_*` mutates in place.
  - `index.rs`: new `scatter_into(target: &mut Array, plan, rhs)` with an **in-place fast path**
    for same-class, in-bounds, no-growth, non-complex/char/cell/struct/deletion writes — it calls
    the COW `target.make_mut_<class>()` → `as_slice_memory_order_mut()` (column-major, matching
    `plan.linear`) and writes only the indexed positions (O(count), zero rebuild, deep-copies only
    when the `Arc` is shared). All other cases (growth, type-promote, complex, char, cell, struct,
    deletion) fall back to the existing materialise+rebuild `scatter` into `*target` — byte-identical
    behaviour. Integer saturation reuses `value::sat_i`/`sat_u` so in-place == rebuild.
  - `index.rs`: `gather_unchecked` now indexes `as_slice_memory_order()` directly at `plan.linear`
    (O(count)) instead of cloning the whole buffer via `mem_order`, for the contiguous case.
  - `interp.rs`: the `assign_to` `ExprKind::Index` branch (plain-variable base) now reads the
    target's dims (no data clone), builds the plan **while the variable is still bound** (so
    `v(v>2)=0` / `A(end)=…` still resolve), then `take` → `scatter_into` → `set`. Nested l-values
    (`a.b(2)`, `a(1).b`) keep the existing evaluate-base → scatter → store-back path. Added
    `plan_for_dims` so the plan can be built from dims directly.
  - **Optional cheap win:** symbol tables (`Scope::locals`, `Context::globals`/`persistents`) now
    use `FxHashMap` (`rustc-hash`) instead of the SipHash `HashMap` — variable lookup is a
    tree-walker hot path that doesn't need collision-DoS resistance.
  - **CLI:** added a `--no-gfx` flag (skips the embedded axum/tokio graphics server + browser
    auto-open) for headless / scripted / benchmark runs.

- **Benchmark (release, `fm --no-gfx`, wall-clock).**
  - The target `1000×1000` loop (1e6 writes): **before** = did not finish within 120s (O(N²),
    effectively unbounded — far worse than the original ~10s estimate); **after** = **~0.98s**
    (result `A(1000,1000)=2000` correct). Startup + `zeros(1000)` alone is ~1ms, so the time is
    essentially the loop. Reference C++ FreeMat is ~0.78s — same ballpark.
  - A measurable `300×300` baseline (90k writes): **before 1.489s → after 0.100s** (~15×).

- **COW-correctness tests (new `crates/fm-interp/tests/inplace_assign.rs`, 16 + 1 ignored).** The
  key guard `cow_alias_not_disturbed_by_indexed_assign`: `B = A; A(i,j) = x` ⇒ **B unchanged** (the
  shared `Arc` deep-copies on first write); plus `cow_alias_holds_across_many_writes`. Value
  correctness for scalar / vector / logical-mask / range / `:` / int8 scatter; fallback correctness
  for growth / type-promote / complex / char / cell-paren / deletion; and read-after-gather
  correctness. An `#[ignore]`d `inplace_bench` times the 1e6-write loop (informational, **no flaky
  wall-clock assert**).

- **Conformance: unchanged-to-slightly-up at 260/603 (43.1%)** (was 258; +2, no regression — the
  pass-floor guard `curated.rs::PASS_FLOOR = 246` is untouched). Verification: `cargo build
  --workspace` ✓, `cargo clippy --workspace --all-targets -D warnings` clean ✓, `cargo fmt --all
  --check` ✓, `cargo test --workspace` green ✓ (incl. the new COW tests + the slow curated
  conformance test).

- **New dep:** `rustc-hash = "2"` (workspace; `fm-interp` opts in). The core `fm-core::Array`
  enum, its `Arc` COW model, and the `make_mut_*` accessors were **not** modified — this was purely
  an interpreter hot-path change that finally *uses* the COW accessors that already existed.

### Perf round 2 (before Stage 8) — kill per-iteration heap traffic

- **Symptom / root cause.** A callgrind profile of the same `A(i,j)=i+j` loop showed it was
  **allocator-bound**: ~35 heap allocations *per iteration*, ~38% of instructions in `malloc`/`free`.
  The in-place fast path from round 1 removed the O(N) rebuild, but every iteration still churned
  short `Vec`s (shape reads, index plans, scalar buffers) and — the big one — the `for` loop
  variable and every 1-element index read were materialised as **1×1 dense `ArrayD`s** instead of
  inline scalars, so the scalar fast paths in arithmetic and indexing never fired.

- **What changed (items 1–6; `fm-core::Array` enum / `Arc` COW / `make_mut_*` untouched — only
  additive zero-alloc accessors).**
  1. **Zero-alloc shape reads.** Added `Array::shape() -> &[usize]` (scalars borrow a static
     `[1,1]`), `Array::dims_smallvec() -> Dims`, and a `Dims = SmallVec<[usize;4]>` alias.
     `dims()` now just `shape().to_vec()`; `numel()` products `shape()` (no Vec). Hot callers
     (`plan_for`/`plan_for_dims`, `columns_of`, `elementwise_arith`/`relational`/`equality`/
     `logical`/`pow`, `unary`, `mul`) read `shape()` instead of allocating a `dims()` Vec.
  2. **SmallVec index plans + scalar subscript fast path.** `IndexPlan.linear`/`result_dims`/
     `needed_dims` and the internal `strides`/`per_dim`/`needed`/`eff` are now `SmallVec`. New
     `plan_scalar`: when every subscript is a single in-bounds integer (`A(i)` / `A(i,j)`), it
     computes the one linear offset with **no per-dimension vectors at all**.
  3. **In-place scatter with no map mutation.** Added `Context::get_mut(name) -> Option<&mut Array>`
     (honours global/persistent). `assign_to`'s plain-variable index branch now builds the plan
     FIRST (it needs `&mut self` to evaluate `i`,`j`), THEN borrows the slot mutably and
     `scatter_into(slot, …)` — **zero map remove/insert, zero key realloc**. Brand-new variables
     fall back to insert-via-`set`; grow/retype reassigns `*slot` through the `&mut`. COW is
     automatic (`make_mut_*` clones once when the `Arc` is shared).
  4. **Scalar ⊕ scalar fast path.** `elementwise_arith`/`relational`/`equality`/`logical` and unary
     minus/not short-circuit when both operands are inline `Array::Scalar`: compute via
     `ScalarValue` + `cast_scalar` and return an `Array::Scalar` — **zero allocs**, speeding *all*
     scalar arithmetic. (`add_scalar` left as-is; the promotion-correct path reuses `cast_scalar`.)
  5. **Scalar RHS read without a buffer.** `scatter_into` reads a scalar rhs via `as_f64()` (no
     `to_f64_vec` Vec); only a multi-element rhs materialises.
  6. **`eval` no longer wraps in `vec![..]`.** `eval` handles the single-value expr kinds directly
     (returning `Array` by value); only the genuinely multi-valued kinds (bare ident, paren-index,
     brace-index — all possible function calls) delegate to `eval_multi`. `plan_for_dims`'s resolved
     subscripts are a `SmallVec<[IndexArg;2]>`.
  - **The decisive fix:** `gather_unchecked` now returns an inline `Array::Scalar` (new
    `scalar_at`) when the result is a single element — so `for i=1:N` loop variables and 1×1
    subexpressions stay scalar, and items 2 & 4's fast paths actually fire. This is also correct
    MATLAB semantics (a single-element index yields a scalar).

- **Benchmark (release, `fm --no-gfx`, wall-clock, 1e6 writes).** Round-1 baseline **~0.98s** →
  **~0.43s** (result `A(1000,1000)=2000` correct) — **beats the C++ FreeMat reference (~0.78s)** and
  the ~0.5s target. **Allocs/iter** (callgrind, N=200): the round-1 profile was ~35/iter (38% in
  malloc/free); after, malloc/free is ~18% of instructions and `to_f64_vec` dropped from 4×/iter to
  ~0 (the scalar paths bypass it). The remaining traffic is the unavoidable single COW write.

- **Conformance: 262/603 (43.4%)** — up from 260, **no regression**. COW/aliasing tests
  (`inplace_assign.rs`, incl. `cow_alias_not_disturbed_by_indexed_assign`) still pass.

- **New dep:** `smallvec = "1"` (workspace; `fm-core` + `fm-interp` opt in). The core
  `fm-core::Array` enum, its `Arc` COW model, and the `make_mut_*` accessors were **not** modified —
  the new `shape()`/`dims_smallvec()`/`Dims` are purely additive accessors.

### Stage 8 — done (`fm-io`: MAT files, file I/O, FFT, regex + conformance-speed fix)

- **Deps added** (pinned in root `[workspace.dependencies]`): `flate2 = "1"` (zlib for compressed
  MAT elements), `byteorder = "1"` (endian-aware MAT/file reads), `rustfft = "6"` (`fft`/`ifft`),
  `regex = "1"` (`regexp`/`regexprep`). `fm-io` opts into `fm-core`/`fm-interp`/`fm-parser`/
  `ndarray`/`num-complex` + the four new crates. `fm-builtins` gained a path dep on `fm-io` so the
  io builtins register everywhere `register_standard_library` is called (CLI + conformance).

- **`fm-io` module layout (`crates/fm-io/src/`):**
  - `matfile.rs` — **MAT v7 / Level-5 read AND write** (ported byte-for-byte from
    `libCore/MatIO.cpp`): 128-byte header, regular **and** packed "small-element" tags, the
    `miMATRIX` sub-element sequence (array-flags → dims → name → payload), array-flags
    complex/logical bits + class byte, and **zlib-compressed (`miCOMPRESSED`) elements on write**
    (matching FreeMat) with transparent inflate on read. Round-trips **double/single, complex,
    every integer class, logical (int32+logical-flag), char (UTF-16), struct (field-name-length +
    null-padded names + field-major fields), and cell (nested matrices)**. `read_mat` is tolerant
    (skips a variable it can't parse rather than failing the whole `load`); `read_first` reads just
    the leading variable. **Sparse MAT is deferred to Stage 9** (no sparse type in `fm-core`; a
    sparse element on read returns a clear error).
  - `fileio.rs` — `fopen`/`fclose`/`fread`/`fwrite`/`frewind`/`feof`/`fgetl`/`fgets`(+`fgetline`)
    over a **thread-local open-file table** (fids start at 3; 0/1/2 reserved). `fwrite`/`fread`
    honor a precision/class string; `fprintf_to` routes a formatted string to a file or back to the
    console.
  - `scanf.rs` — `sscanf`/`fscanf`: a pragmatic format parser covering `%d %i %u %f %e %g %s %c`
    with MATLAB format recycling, returning a numeric column vector (or char for `%s`/`%c`).
  - `fft.rs` — `fft`/`ifft`/`fft2`/`ifft2` via **rustfft**: transform along the first
    non-singleton (or given) dimension with optional length truncation/zero-pad, `ifft` normalized
    by `n`, column-major segment gather/scatter over arbitrary N-D shapes.
  - `regexp.rs` — `regexp`/`regexpi`/`regexprep` via the **regex** crate: default returns 1-based
    `start` indices; option keywords `start`/`end`/`match`/`tokens`/`split`/`once`; multi-output
    order (start,end,match,tokens,split); `regexprep` translates MATLAB `$N` → regex `${N}`.
  - `builtins.rs` — registration + `save`/`load`/`fprintf`/`fscanf`/`sscanf`/`fileread`/`exist`.
    `save f x y` (command syntax) writes named (or all) locals to a `.mat` (or `-ascii` text);
    `load f` injects variables into the current scope, `S = load('f')` (nargout≥1) returns a struct;
    `-ascii` load parses a whitespace/`,`/`;`-delimited numeric matrix. `fprintf` reuses the
    existing `sprintf` printf engine via `call_function`. `exist` extends the interp_ops one with a
    **file-on-disk → 2** check (registered last so it shadows it).

- **MAT read/write coverage + deferrals.** Write+read round-trip verified for all core classes
  (`crates/fm-io/src/tests.rs`, 20 tests). Reading **real MATLAB/FreeMat-written files** is proven
  against checked-in fixtures (`crates/fm-io/tests/fixtures/`): an uncompressed FreeMat v6 file
  reads fully; a zlib-compressed MATLAB v7 file's first variable reads cleanly (its trailing
  elements are a MATLAB object-reference cell dump we don't fully decode — a documented limitation,
  not needed by any conformance test). **Deferred:** sparse MAT (Stage 9), MAT v7.3/HDF5,
  big-endian MAT files, MATLAB object/function-handle classes.

- **`.mat`-fixture / new conformance dirs.** `reference` and `matcompat` were **inspected and kept
  deferred** — they contain only `.mat` *fixtures* + whitebox driver scripts (`mcompat.m`,
  `wbinputs.mat`, `wbtest_*_ref.mat`) with **no `test_*.m`** functions, so the harness has nothing
  runnable there (the MAT reader is exercised by the fm-io unit tests against real fixtures + the
  now-passing `io` save/load tests instead; the DEFERRED note was updated to say so honestly).
  **`transforms` was moved into the covered set** — its 34 `test_*.m` (eig/lu/qr/svd/fft, all
  self-contained) exercise the Stage-5 linalg + the new fft; **18/34 pass**. The `io` dir (already
  covered) went **0 → 3** (`save1`, `load1`, `sscanf1`). `curvefit` stays deferred (needs
  `fitfun`/`gausfit` optimization → Stage 9).

- **Conformance-test-speed restructure.** The old `full_suite_pass_count_does_not_regress` ran the
  **whole** covered corpus on every `cargo test` (~7–8 min, dragging `cargo test --workspace`). It
  is now `#[ignore]`d (run it via `cargo test -p fm-conformance -- --ignored
  full_suite_pass_count_does_not_regress` or the `cargo run -p fm-conformance` reporter). A new fast
  `curated_floor_is_met` asserts the curated must-pass subset's count instead. **The asserted
  conformance path now runs in ~0.7s** (full `cargo test --workspace` ≈ **1.9s** when compiled, vs
  ~7–8 min before). The curated subset gained Stage-8 anchors: `io/test_save1`, `io/test_load1`,
  `io/test_sscanf1`, `transforms/test_eig3`, `transforms/test_svd1`.

- **Conformance: 262/603 (43.4%) → 285/637 (44.7%), Δ +23 passes (+1.3 pp on the absolute pass
  count, with the denominator grown by the 34 newly-enabled `transforms` tests).** Per-dir
  (total / pass / fail / error):

  | dir | total | pass | fail | error |
  |---|---:|---:|---:|---:|
  | array | 68 | 33 | 7 | 28 |
  | binary | 3 | 0 | 0 | 3 |
  | constants | 1 | 0 | 0 | 1 |
  | elementary | 7 | 6 | 0 | 1 |
  | flow | 22 | 20 | 2 | 0 |
  | freemat | 11 | 4 | 4 | 3 |
  | functions | 9 | 5 | 3 | 1 |
  | inspection | 20 | 15 | 3 | 2 |
  | io | 6 | 3 | 0 | 3 |
  | operators | 66 | 9 | 1 | 56 |
  | random | 1 | 0 | 0 | 1 |
  | signal | 1 | 0 | 0 | 1 |
  | string | 3 | 1 | 0 | 2 |
  | suite | 337 | 139 | 35 | 163 |
  | transforms | 34 | 18 | 6 | 10 |
  | typecast | 5 | 2 | 0 | 3 |
  | variables | 43 | 30 | 7 | 6 |
  | **TOTAL** | **637** | **285** | **68** | **284** |

  The +23 breaks down as +18 transforms (eig/svd/qr/lu over faer), +3 io (save/load/sscanf), and +5
  in `suite` (fft/save/regexp-using tests) net of a small PRNG wobble. Remaining io misses:
  `dlmread`/`imwrite` (not implemented — image/CSV I/O) and `io/test_file1` (uses FreeMat's
  `return (expr)` value-syntax the parser doesn't yet accept — pre-existing parser gap, not an
  io-builtin gap).

- **Pass-floor guard raised 246 → 266** (`curated.rs::PASS_FLOOR`; live full-suite is 285, margin
  absorbs the PRNG-dependent `rand`/`randn`/`eig` transforms tests).

- **Verification:** `cargo build --workspace` ✓; `cargo clippy --workspace --all-targets -D
  warnings` clean ✓ (no `#[allow]`s added); `cargo fmt --all --check` ✓; `cargo test --workspace`
  green AND fast ✓ (the slow full-corpus assertion is `#[ignore]`d). 20 new `fm-io` tests (MAT
  round-trip per class, real-fixture reads, fft vs known transforms, regex match/replace, sscanf).

- **`fm-interp` was not modified** (only used: `context.lookup/assign`, `top().local_names()`,
  `call_function`, `value::{to_f64_vec,to_c64_vec,build_real,build_complex,char_matrix}`, `emit`) —
  the open-file table lives entirely in `fm-io` as a thread-local, so no interpreter state was
  added. The MAT reader's tolerance to unparseable trailing variables is the only behavioural
  nuance worth noting for next session.

- **Decisions / deferrals (MATLAB-compatible choices where ambiguous):**
  - MAT files are written **little-endian + zlib-compressed** (FreeMat's default); the reader
    accepts both compressed and uncompressed, both tag forms, but rejects big-endian and sparse.
  - `load` of a bare name tries `<name>.mat`; an ASCII-looking non-`.mat` file falls back to the
    `-ascii` numeric-matrix parser and binds to the file-stem variable.
  - `fread` returns a column vector in the requested precision's class (integers preserve class,
    float/double → double); `feof` is set after a short/EOF read (MATLAB semantics).
  - `regexp` default output is the `start` index vector; `sscanf %g` etc. all collect into one
    numeric column (recycling the format over the input).
  - `fft2`/`ifft2` are implemented as two 1-D passes over the first two dims.

### Debugging (Stage 10, design locked — build deferred to after Stages 7–8)
- Decision: editor+debugger via **DAP/LSP** (drive from VS Code/Neovim) — no built-in editor,
  no GUI. Debug *engine* lives in `fm-interp`; new crates `fm-dap` (+ optional `fm-lsp`).
- **Stage 3 must add the cheap enabling seams now** (not a retrofit): single statement-execution
  chokepoint, per-scope source line/span, switchable active scope (for `dbup`/`dbdown`).
- FreeMat reference to port later: `Interpreter.cpp` `bpStack`/`processBreakpoints`/`doDebugCycle`/
  `dbup`/`dbdown`; `libCore/Debug.cpp`; observer model in `libXP/Editor.cpp` becomes a Rust
  event trait the terminal and DAP both consume.
