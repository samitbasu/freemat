# FreeMat-rs builtin coverage

> **Generated snapshot (2026-06-22)** — regenerated after the help-completeness
> backlog was finished (sessions 2–3). This is a point-in-time diff of the
> **real freemat-rs registration table** against the FreeMat 4.2 name universe
> (C++ builtins + toolbox `.m`), *not* a grep of the source tree.
>
> **How to regenerate the freemat-rs side:**
> ```
> cargo run -q -p fm-cli -- --list-builtins | sort -u
> ```
> That constructs an `Interpreter`, registers the full standard library
> (`fm_builtins::register_standard_library`, which pulls in `fm-io` and the
> graphics + time builtins), and prints every registered function name. That
> output is the **authoritative** freemat-rs builtin set used below.
>
> **How to regenerate the FreeMat side:**
> ```
> # C++ builtins (all @@Signature directive types):
> grep -rhoE '^//(s?g?function|sgfunction) +[A-Za-z_][A-Za-z0-9_]*' ../FreeMat/libs | awk '{print $2}' | sort -u
> # toolbox .m basenames (excl. help/ doc stubs and tests/ harness):
> find ../FreeMat/toolbox -name '*.m' | grep -vE '/help/|/tests/' | xargs -n1 basename | sed 's/\.m$//' | sort -u
> ```
> The ITK/VTK/GL image-processing primitives (e.g. `cannyedgedetector`,
> `glshow`) are **dropped** from the scored universe (deliberately out of scope).

## What counts as a "builtin"

1. **Evaluator constants are not in the function table.** `pi`, `e`, `eps`,
   `Inf`/`inf`, `NaN`/`nan`, `i`/`j`/`I`/`J`, `true`, `false` are resolved in
   `fm-interp`'s `eval_ident`, so they are not in `--list-builtins` but ARE
   implemented (added to the freemat-rs set below).
2. **freemat-rs implements many FreeMat toolbox `.m` functions as native Rust
   builtins** (`plot`, `linspace`, `repmat`, `all`, …). So the fair comparison
   is the **whole FreeMat name universe** (C++ core + toolbox) vs the freemat-rs
   registration table + constants.

## Summary

| metric | count |
|---|---:|
| freemat-rs registered builtins (`--list-builtins`) | **452** |
| freemat-rs evaluator constants (`pi`/`eps`/`i`/`true`/…) | 13 |
| **freemat-rs total implemented (names)** | **465** |
| FreeMat name universe scored (C++ core + toolbox, ITK/VTK/GL & help/test stubs excluded) | **503** |
| **FreeMat names covered by freemat-rs** | **374** |
| **headline name coverage** | **74.4%** |
| missing names | 129 |
|  — of which out-of-scope / FreeMat-internal / debugger | 60 |
|  — **genuinely actionable** | **69** |

> Up from the prior snapshot (348 registered / ~52% coverage). Sessions 2–3 added
> the `rand*` distributions, `cov`/`betainc`/`legendre`/`expm`/`matrixpower`/
> `ode45`/`interplin1`/`teps`, sparse `lu`, `who`/`whos`/`where`/`lasterr`/`type`/
> `inline`/`symvar`/`nargin`/`nargout`, `format`/`get`/`setprintlimit`/`fseek`/
> `ftell`/`feof`, the axes-property + niche graphics set
> (`xlim`/`ylim`/`clim`/`patch`/`contour3`/`zoom`/`sizefig`/`tubeplot`/`clabel`/
> `winlev`/`pvalid`/`subplot`), and keyword-argument call syntax.

## Runnability is the stronger signal

This table is the **API-surface** number ("how many names exist"). The
**runnability** number ("how much actually works end to end") is the conformance
suite, which executes FreeMat's own `test_*.m` cases: **672/677 ≈ 99.3%** (the 5
red are all out of scope — a buggy corpus test, C-FFI, threads; see
`REMAINING.md`). A name absent below may still run via an equivalent (e.g.
`mpower`↔`^`, `isstr`↔`ischar`); conversely a present name is useless until its
dependency builtins exist. Treat this as a *capability inventory*.

---

## The 60 out-of-scope / internal missing names

These are not planned (or are FreeMat-internal and never user-facing):

- **Native C-FFI (13)** — `import`, `loadlib`, `bind`, `cenum`, `ctypecast`,
  `ctypedefine`, `ctypefreeze`, `ctypenew`, `ctypeprint`, `ctyperead`,
  `ctypesize`, `ctypethaw`, `ctypewrite`. (libffi/imported-C dropped from scope.)
- **Threads (1)** — `threadcall` (parallel/threads out of scope).
- **JIT / perf internals (5)** — `jitcontrol`, `jitstat`, `blaslib`, `pcode`,
  `wrap_jit_test`.
- **Debugger — Stage 10 (7)** — `dbstop`, `dbauto`, `dbdelete`, `dblist`,
  `errorcount`, `fdump`, `warning` (the planned-but-unbuilt interactive debugger).
- **FreeMat test / benchmark / GUI / installer internals (13)** — `test`,
  `testtube`, `wb_test`, `wbgentests`, `wbtestcompare`, `wbtest_exact`,
  `wbtestinputs`, `wbtest_near`, `wbtest_near_permute`, `wrap_test`, `simkeys`,
  `docli`, `quiet`, `qtnew`, `install`.
- **Graphics / parser internal helpers (21)** — `completeprops`, `parseit`,
  `matchit`, `stcmp`, `styleset`, `markerset`, `makehandleclass`, `hrawplot`,
  `htextbitmap`, `p_end`, `is2dview`, `islinespec`, `newplot`, `colorset`,
  `datacursormanager`, `datacursormode`, `inline_evaluate`, `mkdir_core`,
  `regexprepdriver` (toolbox-internal helpers, not called directly by users).

---

## The 69 actionable missing names, by category

### Graphics — handle system / 3-D / interactive (Stage 7.5) (15)
`copy` · `print` · `surface` · `uicontrol` · `zlim` · `zplane` · `figlower` ·
`figraise` · `hcontour` · `himage` · `hline` · `hpatch` · `htext` · `point` ·
`hpoint`
> Need the full handle/`set`/`get` property system, text-object handles, and —
> for `point`/`hpoint` — frontend mouse-event plumbing. `zlim` is the 3-D analog
> of the already-done `xlim`/`ylim`. Not-yet-started 3-D plot types
> (`surfl`/`surfc`/`meshc`/`waterfall`/`sphere`/`cylinder`/`ellipsoid`) have no
> failing help fragment so aren't in this name diff.

### OS / filesystem (22)
`addpath` · `path` · `pathsep` · `pathtool` · `rehash` · `rescan` · `mkdir` ·
`rmdir` · `copyfile` · `fileattrib` · `dirsep` · `filesep` · `diary` · `exit` ·
`license` · `urlwrite` · `xmlread` · `htmlread` · `system` · `getpath` ·
`setpath` · `what`
> `getpath`/`setpath`/`what` are environment-dependent (non-deterministic);
> `system` is shell-out. The rest are straightforward filesystem/path builtins —
> the cleanest large win still open.

### File I/O & audio (9)
`input` · `fflush` · `getline` · `rawread` · `rawwrite` · `wavread` ·
`wavwrite` · `wavplay` · `wavrecord`

### Signal / FFT (5)
`fftn` · `ifftn` · `fftshift` · `ifftshift` · `hilbert`

### Numerics (6)
`trapz` · `cumtrapz` · `interp2` · `odeset` · `deval` · `mpower`
> `mpower` already works as the `^` operator; only the named-builtin form is
> absent. `odeset`/`deval` would round out the now-implemented `ode45`.

### Type / inspection (12)
`computer` · `mfilename` · `isinttype` · `issquare` · `isstr` · `ishold` ·
`isequalwithequalnans` · `maxdim` · `subsref` · `IsInf` · `IsNaN` ·
`clocktotime`
> Several are aliases of existing functionality: `isstr`↔`ischar`,
> `IsInf`/`IsNaN`↔`isinf`/`isnan` (capitalized internal spellings). `clocktotime`
> is time-based (non-deterministic). `subsref` ties into the class/handle system.

---

## Quick map: where things live
- Interpreter / eval / indexing / scopes: `crates/fm-interp/`
- Values / `Array` (dense + sparse + function-handle), formatting: `crates/fm-core/`
- Lexer / parser / AST: `crates/fm-parser/`
- Dense + sparse linear algebra (faer): `crates/fm-linalg/`
- Builtins (math/strings/array/poly/bitops/baseconv/handles/graphics/random/…): `crates/fm-builtins/`
- MAT / file I/O / FFT / regex / image: `crates/fm-io/`
- REPL + graphics webserver: `crates/fm-cli/`; browser frontend: `web/index.html`
- Conformance harness + corpus: `crates/fm-conformance/` (`-- --failures` lists red)
- Overall backlog: `docs/REMAINING.md`; help-example backlog: `docs/HELP_BACKLOG.md`
