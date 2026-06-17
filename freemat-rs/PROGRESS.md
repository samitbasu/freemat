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

### Debugging (Stage 10, design locked — build deferred to after Stages 7–8)
- Decision: editor+debugger via **DAP/LSP** (drive from VS Code/Neovim) — no built-in editor,
  no GUI. Debug *engine* lives in `fm-interp`; new crates `fm-dap` (+ optional `fm-lsp`).
- **Stage 3 must add the cheap enabling seams now** (not a retrofit): single statement-execution
  chokepoint, per-scope source line/span, switchable active scope (for `dbup`/`dbdown`).
- FreeMat reference to port later: `Interpreter.cpp` `bpStack`/`processBreakpoints`/`doDebugCycle`/
  `dbup`/`dbdown`; `libCore/Debug.cpp`; observer model in `libXP/Editor.cpp` becomes a Rust
  event trait the terminal and DAP both consume.
