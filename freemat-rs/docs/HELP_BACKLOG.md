# Help-completeness backlog

Tracks the remaining work to make every help page show its example **output**.

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

## Status (2026-06-21)

- Fragments kept **live: 250** · **downgraded: 79** (of 329 with fragments).
- Of the 79: **10 are non-deterministic (correct — leave as-is)**, **69 are fixable**.
- 0 crashes (all fixed — see "Done" below).

This session's progress: live fragments 224 → 259 (then 250 after a re-probe normalized counts);
8 interpreter crashes fixed; ~16 builtins added. History in git (`a7df1e6c1`, `bb843a36d`,
`a2b16b7cb`, graphics `7019d175e`/`3eea8febe`/`f8dcefab7`/`2f007d705`).

---

## Backlog — the 69 fixable downgrades, by category

Effort: **S** small (a few hrs) · **M** medium · **L** large.

### 1. Graphics — axes / handle properties (~10) · effort M
Need handle-graphics property plumbing (axes limits, color limits, view controls) and a few
extra plot types. Ties into the graphics track.

`xlim` · `ylim` · `zoom` · `clim` · `clabel` · `contour3` · `patch` · `tubeplot` · `winlev` ·
`pvalid`

- `xlim`/`ylim`/`clim`/`zoom`: read/set axes limit properties (the scene `Axes` already has a
  `limits` field — wire get/set builtins to it).
- `patch`: filled polygons from face/vertex data → a new series kind (close to `fill`).
- `contour3`: 3-D contour (contour lines lifted to z) → scene bound like `surf`.
- `clabel`/`tubeplot`/`winlev`/`pvalid`: niche; lower priority.

### 2. FreeMat `rand*` distributions (11) · effort M
FreeMat-specific RNG distribution generators (seeded via the shared RNG).

`randbeta` · `randbin` · `randchi` · `randexp` · `randf` · `randgamma` · `randmulti` ·
`randnbin` · `randnchi` · `randnf` · `randp`

- Implement on top of `rand_distr` where a distribution exists; otherwise the standard
  transforms. Must honor the seeded RNG (`rand('state',s)`) for deterministic fragments.

### 3. I/O & session (~16) · effort M–L
File I/O, persistence, and workspace/session introspection.

`load` · `save` · `fread` · `fseek` · `ftell` · `feof` · `format` · `getprintlimit` ·
`setprintlimit` · `exist` · `import` · `type` · `who` · `where` · `system` · `isset`

- `who`/`where`/`exist`/`isset`: workspace/scope introspection (query the current scope).
- `format`/`get/setprintlimit`: display-format state (the interpreter has a `format` field).
- `load`/`save`: MAT round-trip already exists in `fm-io`; the doc examples may need fixtures
  or write/read in the fragment's temp CWD.
- `fread`/`fseek`/`ftell`/`feof`: low-level file handles (need a file-handle table).
- `type`: print a function/file's source. `system`: shell-out (non-deterministic output —
  may stay display-only). `import`: FFI-adjacent — likely defer.

### 4. Language-feature / introspection docs (~11) · effort M
Examples that depend on defining helper functions across fragments, or on call-context
introspection that a top-level fragment can't exercise.

`function` · `keywords` · `nargin` · `nargout` · `script` · `return` · `feval` · `inline` ·
`symvar` · `lasterr` · `keyboard`

- `nargin`/`nargout`/`return`/`keywords`/`function`/`script`: these document language
  constructs; their examples need multi-fragment helper `.m` definitions (the ` ```fm-file `
  mechanism exists — the converter may need to thread helper files into these fragments).
- `feval`: should already work — re-check why its example errors.
- `inline`/`symvar`: inline-function objects + symbolic-var extraction (parser-level work).
- `lasterr`: last-error state. `keyboard`: interactive debug entry (Stage 10; display-only
  is acceptable).

### 5. Bigger math (~8) · effort L
`expm` · `betainc` · `legendre` · `ode45` · `cov` · `matrixpower` · `interplin1` · `teps`

- `expm` (matrix exponential), `matrixpower` (A^p for matrices), `ode45` (ODE solver),
  `legendre`, `betainc` (incomplete beta), `cov` (covariance), `interplin1` (linear interp).
  Each is a real numeric feature; size individually.

### 6. Sparse constructors (3) · effort S–M
`speye` · `sprand` · `sprandn` — sparse identity / random sparse (the sparse core exists in
`fm-core`/`fm-linalg`; add the constructors).

### 7. Misc (~3) · effort S
- `unique` — the `'rows'` option and multi-output `[u,i,j]` forms (base `unique` works).
- `lu` — float `[l,u,p] = lu(A)` factor-returning form (see also `REMAINING.md`).
- `bind` — deprecated standalone-exe builder (toolbox); likely leave display-only.

### 8. Threads (6) · **out of scope** (see `REMAINING.md` §C)
`threadcall` · `threadid` · `threadkill` · `threadstart` · `threadvalue` · `threadwait` —
parallel/threads are out of scope; their examples stay display-only.

### 9. Non-deterministic (10) · **leave as display-only (correct)**
`clock` · `clocktotime` · `etime` · `tic` · `toc` · `cd` · `pwd` · `ls` · `getpath` · `setpath`
— output is wall-clock- or working-directory-dependent, so capturing it would make
`docgen --check` non-reproducible. These are intentionally display-only.

---

## Graphics track — status

Plotting commands added this session (live view `web/index.html` + help-page figures):

- **Done:** `bar`/`barh`, `hist`, `stem`, `stairs`, `errorbar`, `plot3`, `peaks`; un-squashed
  3-D surfaces (`aspectmode`) + camera-aware `view`; `colormap` (named + RGB matrices) +
  named-map generators (`jet`/`hot`/…), `colorbar`, `pcolor`, `contourf`, `scatter`/`scatter3`;
  `area`, `fill`, `pie`, `polar`, `quiver`, `text` annotations. (Already present before:
  `plot`, `surf`/`mesh`, `contour`, `image`/`imagesc`, `loglog`/`semilogx`/`semilogy`.)
- **Remaining graphics builtins:** the axes-property set in §1 above (`xlim`/`ylim`/`clim`/
  `patch`/`contour3`/…), plus 3-D extras not yet started: `surfl`/`surfc`/`meshc`,
  `waterfall`, `sphere`/`cylinder`/`ellipsoid`.
- **Known simplification:** `quiver` draws shaft lines + tip markers, not rotated arrowheads.

---

## Done (this effort)

Crashes fixed (were aborting the whole `fm` session): cell/struct → numeric/char casts
(`int8({4})` etc.); `fliplr`/`flipud`/`circshift` on N-D arrays; `sort` on a cellstr;
`rand`/`randn('state',0)` SIGABRT. `eval(try,catch)` in the output path
(`b = eval('z','a+1')`). Builtins added: `flipdim`, `shiftdim`, `transpose`, `cellstr`,
`strstr`, `isalpha`/`isdigit`/`isspace`, `fullfile`, `getenv`, `erfinv`, `idiv`, `randperm`,
`version`, `verstring`, `norm(v,p)` p-norms, multi-arg/cellstr `char`.

See also [`REMAINING.md`](./REMAINING.md) (overall port backlog) and
[`WRITING_DOCS.md`](./WRITING_DOCS.md) (how the fragment/promote cycle works).
