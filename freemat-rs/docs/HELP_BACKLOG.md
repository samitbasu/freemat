# Help-completeness backlog

Tracks the work to make every help page show its example **output**.

## Background

Each help example is a fenced ` ```fm-exec ` block whose REPL transcript is captured by
`cargo xtask docgen` (see [`WRITING_DOCS.md`](./WRITING_DOCS.md)). An example only shows its
output if its fragment **runs cleanly**. `cargo xtask migrate-place` probes every fragment and
**downgrades** any that error, are non-deterministic, or crash to a plain display-only
` ```fm ` block (code shown, no output).

**To promote an example:** implement the missing builtin / behavior, then re-run
`cargo xtask migrate-place` → `cargo build -p fm-cli` → `cargo xtask docgen`. The tool
auto-promotes any fragment that now runs as declared. (`migrate-place` prints the live /
downgraded counts and per-topic downgrade reasons.)

## Status (2026-06-22) — **every fixable fragment is now live**

- Fragments kept **live: 308** · **downgraded: 21** · **310 captured**.
- **All 21 remaining downgrades are correct-as-display-only** (see "Intentionally display-only"
  below): 10 non-deterministic (wall-clock / cwd), 6 threads (out of scope), and 5 that are
  inherently non-reproducible (`system`/`import`/`bind`/`keyboard`/`return`).
- **There is no remaining fixable work in this backlog.**

History: live **224 → 250** (session 1) → **250 → 308** (sessions 2–3). The function-coverage
gap (functions with no impl at all, e.g. `point`) is a *separate* tracker —
[`COVERAGE.md`](./COVERAGE.md), not this file.

---

## Completed (sessions 2–3)

### Random distributions (11) — `crates/fm-builtins/src/random.rs`
`randbeta` `randbin` `randchi` `randexp` `randf` `randgamma` `randmulti` `randnbin` `randnchi`
`randnf` `randp`, all on the shared seeded RNG (`with_rng`/`draw_try`) so fragments are
reproducible. `randmulti` = sequential conditional-binomials; `randnbin` = Gamma–Poisson mixture;
`randnchi`/`randnf` = the documented chi-square decompositions.

### Sparse constructors — `crates/fm-builtins/src/sparse.rs`
`speye` (was blocked only by `who`); `sprand`/`sprandn` now draw from the shared seeded RNG and
accept a full matrix in the single-argument form.

### Introspection & session — `crates/fm-builtins/src/interp_ops.rs`, `fm-io`
`who`/`whos`/`where`; proper `lasterr` (interpreter `last_error` state); `exist`/`isset`
(pre-existing, promoted once `who` landed); `type` (print a function's source / built-in notice);
`nargin`/`nargout` introspection forms (`nargin(name|@handle)`), with a scripts table so a bare
name resolving to a `.m` script runs in the caller's workspace, filename-stem function
registration, and a literal-by-reference error; `inline`/`symvar` (anonymous-handle-backed inline
functions + symbolic-var extraction, sharing `collect_free_vars`), which unblocked `feval`.

### I/O — `crates/fm-io`
`load`/`save` (no forced `.mat` extension; command-syntax `nargout` passthrough in
`call_function` so `load file` injects into the workspace); `fseek`/`ftell`/`feof` + a rewritten
`fread` (2-D size vector with a single `inf`, proper precision parser, count output, tolerant of
closed handles); `format`/`getprintlimit`/`setprintlimit` (display-format state — extended
`FormatMode` with `short e`/`long e`/`short g`/`long g`; `Interpreter.print_limit` with a
deterministic truncation notice; `Short`/`Long` defaults unchanged so existing transcripts are
byte-stable).

### Array / misc
`unique` `'rows'` + multi-output `[y,m,n]` (`array_manip.rs`).

### Bigger math
`expm` + matrix power (`matrixpower`/`^`) via `matrix_function = V diag(f(d)) V^-1`
(`fm-linalg`); `cov`/`betainc`/`legendre`/`interplin1`/`teps` (`fm-builtins/src/misc.rs`);
`ode45` (DOPRI5 adaptive RK, `interp_ops.rs`); sparse `lu` 5-output `[l,u,p,q,r]`
(left-looking Gilbert–Peierls sparse LU with partial pivoting, `linalg.rs`).

### Graphics
Axes-property set `xlim`/`ylim`/`clim`/`patch`/`contour3`/`zoom` (+ `sizefig`); niche
`tubeplot`/`clabel`/`winlev`/`pvalid` (+ `subplot`, which was blocked only by `tubeplot`). New
*additive* scene fields `Axes.clim`, `Figure.size`, `SurfaceSeries.xmat/ymat`,
`ContourSeries.labels` (all `serde(default, skip_serializing_if)` so existing figure JSON is
unchanged). `web/index.html` consumes `xmat`/`ymat` and contour `showlabels`.

### Language feature
Keyword-argument call syntax (`/name`, `/name=expr`) — new `ExprKind::KeywordCall` produced only
when a call has keywords (ordinary calls/indexing untouched), bound in `eval_keyword_call`.

---

## Intentionally display-only (the 21 remaining — correct, not a backlog)

These cannot have a deterministic captured transcript; they stay ` ```fm ` display blocks.

### Non-deterministic (10) — wall-clock / working-directory
`clock` · `clocktotime` · `etime` · `tic` · `toc` · `cd` · `pwd` · `ls` · `getpath` · `setpath`.
Output varies per run/machine; capturing it would make `docgen --check` non-reproducible.
(`migrate-place`'s `NONDETERMINISTIC_BUILTINS` list force-downgrades these — `system` was added
to it.)

### Shell / FFI / build / interactive (5)
- `system` — shell-out; output depends on the host environment/filesystem.
- `import` — FFI: compiles a C file with `gcc` and loads a `.so`; non-portable.
- `bind` — deprecated standalone-executable builder; needs build infra and runs the artifact.
- `keyboard` — interactive debugger entry (Stage 10); no capturable output.
- `return` — its example uses `keyboard` to inspect a *paused* function's locals (interactive).

### Threads (6) — out of scope (see `REMAINING.md` §C)
`threadcall` · `threadid` · `threadkill` · `threadstart` · `threadvalue` · `threadwait`.

---

## Graphics track — status

- **Plot types done:** `plot`, `surf`/`mesh`, `contour`/`contourf`/`contour3`, `image`/`imagesc`,
  `loglog`/`semilogx`/`semilogy`, `bar`/`barh`, `hist`, `stem`, `stairs`, `errorbar`, `plot3`,
  `peaks`, `pcolor`, `scatter`/`scatter3`, `area`, `fill`, `patch`, `pie`, `polar`, `quiver`,
  `tubeplot`, `text`; `colormap` (+ named generators), `colorbar`.
- **Axes/handle properties done:** `xlim`/`ylim`/`clim`/`zoom`/`sizefig`/`view`/`axis`/`grid`/
  `subplot`/`clabel`/`winlev`/`pvalid`.
- **Not yet started (no failing help fragment):** 3-D extras `surfl`/`surfc`/`meshc`,
  `waterfall`, `sphere`/`cylinder`/`ellipsoid`; mouse-interactive `point`/`hpoint` (need
  frontend event plumbing — tracked in `COVERAGE.md`).
- **Known simplification:** `quiver` draws shaft lines + tip markers, not rotated arrowheads.

See also [`REMAINING.md`](./REMAINING.md) (overall port backlog), [`COVERAGE.md`](./COVERAGE.md)
(function-coverage tracker), and [`WRITING_DOCS.md`](./WRITING_DOCS.md) (the fragment/promote
cycle).
