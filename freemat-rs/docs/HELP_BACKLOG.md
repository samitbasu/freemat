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

## Status (2026-06-21, session 2)

- Fragments kept **live: 282** · **downgraded: 47** (probe counts); **289 captured**.
- Of the downgrades: **10 are non-deterministic (correct — leave as-is)**, **6 are threads
  (out of scope)**, the rest fixable.

Session 2 progress: live 250 → 273. Implemented the **11 `rand*` distributions**
(`randbeta`/`randbin`/`randchi`/`randexp`/`randf`/`randgamma`/`randmulti`/`randnbin`/
`randnchi`/`randnf`/`randp`, all on the shared seeded RNG); routed **`sprand`/`sprandn`**
through the seeded RNG + accept a full matrix (single-arg form); added **`who`/`whos`/`where`**
and a proper **`lasterr`** (interpreter `last_error` state); extended **`unique`** with `'rows'`
and the multi-output `[y,m,n]` forms; fixed **`load`/`save`** (don't force-append `.mat` when an
extension is given; **command-syntax `nargout` passthrough** so `load file` injects into the
workspace instead of returning a struct — `crates/fm-interp/src/interp.rs` `call_function`).
Then the **axes-property graphics set**: `xlim`/`ylim`/`clim`/`patch`/`contour3`/`zoom` (+ the
supporting `sizefig`), with new `Axes.clim` and `Figure.size` scene fields. Then **`inline`** +
**`symvar`** (anonymous-handle-backed inline functions; symbolic-var extraction), which also
unblocked **`feval`**.

Prior session: live 224 → 250; 8 interpreter crashes fixed; ~16 builtins added. History in git
(`a7df1e6c1`, `bb843a36d`, `a2b16b7cb`, graphics `7019d175e`/`3eea8febe`/`f8dcefab7`/`2f007d705`).

---

## Backlog — the 69 fixable downgrades, by category

Effort: **S** small (a few hrs) · **M** medium · **L** large.

### 1. Graphics — axes / handle properties · mostly ✅ DONE (session 2)
- `xlim`/`ylim`: ✅ get/set, with `[lo,inf]`→auto and `'auto'`/`'manual'` strings. The no-arg
  query computes data bounds via `auto_xy_bounds`/`series_xy_bounds` (`graphics.rs`).
- `clim`: ✅ get/set color-axis limits (new `Axes.clim` field).
- `patch`: ✅ `patch(x,y,c)`/`patch(x,y,z,c)` rendered as a filled polygon (reuses `FillSeries`).
- `contour3`: ✅ `contour` + default 3-D view + grid.
- `zoom`: ✅ FreeMat semantics (`>0` resize via `sizefig`, `==0` `axis image`, `<0` `axis
  normal`); **`sizefig`** also added (new `Figure.size` field).
- **Still TODO (niche):** `clabel` · `tubeplot` · `winlev` · `pvalid`. `subplot`'s fragment is
  blocked only by `tubeplot` (its 3rd example block); `subplot` itself works.

### 2. FreeMat `rand*` distributions (11) · ✅ DONE (session 2)
All implemented in `crates/fm-builtins/src/random.rs` on top of `rand_distr`, drawing from the
shared seeded generator (`with_rng`/`draw_try`) so the fragments are reproducible. `randmulti`
uses sequential conditional-binomials; `randnbin` a Gamma–Poisson mixture; `randnchi`/`randnf`
the documented chi-square decompositions.

### 3. I/O & session (~16) · effort M–L
File I/O, persistence, and workspace/session introspection.

`load` · `save` · `fread` · `fseek` · `ftell` · `feof` · `format` · `getprintlimit` ·
`setprintlimit` · `exist` · `import` · `type` · `who` · `where` · `system` · `isset`

- `who`/`where`/`exist`/`isset`: ✅ DONE — `who`/`whos`/`where` added (`interp_ops.rs`);
  `exist`/`isset` already existed and their fragments promoted once `who` worked.
- `format`/`get/setprintlimit`: display-format state (the interpreter has a `format` field).
  Still TODO — needs print-limit/format plumbing into the value formatter.
- `load`/`save`: ✅ DONE — fixed the `.mat` auto-extension clobber and made command-syntax
  `load file` inject into the workspace (the `call_function` nargout passthrough fix).
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
- `feval`/`inline`/`symvar`: ✅ DONE (session 2). `inline` builds an anonymous function handle
  from the expression string (explicit args, or auto-detected symbolic vars); `symvar` returns
  the sorted symbolic variables (free identifiers that aren't functions/constants); both share
  `symbolic_vars`/`collect_free_vars` (now exported from `fm-interp`). `feval` promoted once
  `inline` worked (its doc shares a fragment with the `inline` example).
- `lasterr`: last-error state. `keyboard`: interactive debug entry (Stage 10; display-only
  is acceptable).

### 5. Bigger math (~8) · effort L
`expm` · `betainc` · `legendre` · `ode45` · `cov` · `matrixpower` · `interplin1` · `teps`

- `expm` (matrix exponential), `matrixpower` (A^p for matrices), `ode45` (ODE solver),
  `legendre`, `betainc` (incomplete beta), `cov` (covariance), `interplin1` (linear interp).
  Each is a real numeric feature; size individually.

### 6. Sparse constructors (3) · ✅ DONE (session 2)
`speye` (its fragment was blocked only by `who`, now implemented); `sprand`/`sprandn` now draw
from the shared seeded RNG and accept a full matrix in the single-argument form.

### 7. Misc (~3)
- `unique` — ✅ DONE: `'rows'` + multi-output `[y,m,n]` (`array_manip.rs`).
- `lu` — still blocked: its fragment also exercises the **sparse** `[l,u,p,q,r] = lu(A)` form
  (UMFPACK-style), so the dense `[l,u,p]` alone won't promote it. Needs sparse LU (effort L).
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
- **Axes-property set (session 2):** `xlim`/`ylim`/`clim`/`patch`/`contour3`/`zoom`/`sizefig`
  done (see §1). Renderer note: `Axes.clim` and `Figure.size` are serialized but `web/index.html`
  does not yet consume them (no effect on the captured fragments, which are text-only here).
- **Remaining graphics builtins:** niche `clabel`/`tubeplot`/`winlev`/`pvalid`, plus 3-D extras
  not yet started: `surfl`/`surfc`/`meshc`, `waterfall`, `sphere`/`cylinder`/`ellipsoid`.
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
