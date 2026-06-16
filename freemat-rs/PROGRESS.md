# FreeMat-rs — progress tracker

Source of truth for what is done across sessions. To work on the project, read
`docs/PLAN.md` (the full plan), pick a stage, implement it to the Definition of Done,
then tick it here and commit. Leave notes for the next session under each stage.

## Stage checklist

- [x] **Stage 0 — Workspace scaffold & conventions**
- [x] **Stage 1 — `fm-core`: types & Array**
- [x] **Stage 2 — `fm-parser`: lexer + parser + AST (miette)**
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

### Debugging (Stage 10, design locked — build deferred to after Stages 7–8)
- Decision: editor+debugger via **DAP/LSP** (drive from VS Code/Neovim) — no built-in editor,
  no GUI. Debug *engine* lives in `fm-interp`; new crates `fm-dap` (+ optional `fm-lsp`).
- **Stage 3 must add the cheap enabling seams now** (not a retrofit): single statement-execution
  chokepoint, per-scope source line/span, switchable active scope (for `dbup`/`dbdown`).
- FreeMat reference to port later: `Interpreter.cpp` `bpStack`/`processBreakpoints`/`doDebugCycle`/
  `dbup`/`dbdown`; `libCore/Debug.cpp`; observer model in `libXP/Editor.cpp` becomes a Rust
  event trait the terminal and DAP both consume.
