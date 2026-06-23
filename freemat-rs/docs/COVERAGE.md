# FreeMat-rs builtin coverage

> **Generated snapshot (2026-06-22, post handle-system pass)** — a point-in-time
> diff of the **real freemat-rs registration table** against the FreeMat 4.2 name
> universe (C++ builtins + toolbox `.m`), *not* a grep of the source tree.
>
> **Regenerate the freemat-rs side:**
> ```
> cargo run -q -p fm-cli -- --list-builtins | sort -u
> ```
> **Regenerate the FreeMat side:**
> ```
> grep -rhoE '^//(s?g?function|sgfunction) +[A-Za-z_][A-Za-z0-9_]*' ../FreeMat/libs | awk '{print $2}' | sort -u
> find ../FreeMat/toolbox -name '*.m' | grep -vE '/help/|/tests/' | xargs -n1 basename | sed 's/\.m$//' | sort -u
> ```
> ITK/VTK/GL image-processing primitives (`cannyedgedetector`, `glshow`, …) are
> dropped from the scored universe (out of scope).

## What counts as a "builtin"

1. **Evaluator constants** (`pi`, `e`, `eps`, `Inf`/`inf`, `NaN`/`nan`,
   `i`/`j`/`I`/`J`, `true`, `false`) resolve in `fm-interp`'s `eval_ident`, so
   they aren't in `--list-builtins` but ARE implemented (added to the set below).
2. freemat-rs implements many FreeMat toolbox `.m` functions as native Rust
   builtins, so the fair comparison is the **whole FreeMat name universe** (C++
   core + toolbox) vs the freemat-rs registration table + constants.

## Summary

| metric | count |
|---|---:|
| freemat-rs registered builtins (`--list-builtins`) | **512** |
| freemat-rs evaluator constants | 13 |
| **freemat-rs total implemented (names)** | **525** |
| FreeMat name universe scored (C++ core + toolbox, ITK/VTK/GL & help/test stubs excluded) | **503** |
| **FreeMat names covered by freemat-rs** | **433** |
| **headline name coverage** | **86.1%** |
| missing names | 70 |
| **— of which small / actionable** | **0** |

> Progression: 348 registered / ~52% (initial) → 452 / 74.4% (after the
> help-completeness backlog) → 500 / 83.9% (after the coverage-fill pass) →
> **512 / 86.1%** (after the graphics handle-system pass that added the root
> object, full parent/children/type navigation, a defaults-aware property
> catalogue for figure/axes/line/surface/image/contour/patch/text, the handle
> constructors `hline`/`hpatch`/`himage`/`hcontour`/`surface`/`htext`, and
> `copyobj`/`newplot`/`figraise`/`figlower`/`findobj`-filters/`get(h)`/`set(h)`).

## Runnability is the stronger signal

This is the **API-surface** number. The **runnability** number — the conformance
suite executing FreeMat's own `test_*.m` cases — is **672/677 ≈ 99.3%** (the 5
red are out of scope: a buggy corpus test, C-FFI, threads; see `REMAINING.md`).
A name absent below may still run via an equivalent (`mpower`↔`^`,
`isstr`↔`ischar`, `IsInf`/`IsNaN`↔`isinf`/`isnan`).

---

## The 70 missing names — all out-of-scope or gated on a deferred feature

There are **no remaining small/actionable builtins**: every missing name needs a
large deferred feature (the debugger), device / network access, a heavy
dependency, GUI/mouse/renderer support, or is FreeMat-internal / out-of-scope.
The graphics **handle-system core is now implemented** (root, parent/children,
property catalogue with defaults, handle constructors, `copyobj`/`newplot`/
`findobj`/`figraise`/`figlower`/`get(h)`/`set(h)`); what remains under graphics
is GUI widgets, interactive cursors, mouse picking, figure-image export, the UDD
class machinery, and toolbox-internal construction helpers.

- **Native C-FFI (13)** — `import`, `loadlib`, `bind`, `cenum`, `ctypecast`,
  `ctypedefine`, `ctypefreeze`, `ctypenew`, `ctypeprint`, `ctyperead`,
  `ctypesize`, `ctypethaw`, `ctypewrite`. (libffi/imported-C dropped from scope.)
- **Threads / JIT / perf internals (6)** — `threadcall`, `jitcontrol`,
  `jitstat`, `blaslib`, `pcode`, `wrap_jit_test`.
- **Debugger — Stage 10, not yet built (7)** — `dbstop`, `dbauto`, `dbdelete`,
  `dblist`, `errorcount`, `fdump`, `warning`.
- **FreeMat test / benchmark / GUI / installer internals (15)** — `test`,
  `testtube`, `wb_test`, `wbgentests`, `wbtestcompare`, `wbtest_exact`,
  `wbtestinputs`, `wbtest_near`, `wbtest_near_permute`, `wrap_test`, `simkeys`,
  `docli`, `quiet`, `qtnew`, `install`.
- **Graphics — beyond the handle-system core (19)** — GUI/interactive/export and
  toolbox-internal construction helpers:
  - GUI / interactive / mouse: `uicontrol`, `datacursormanager`,
    `datacursormode`, `point`, `hpoint`.
  - Figure-image export (needs a headless renderer; the frontend is Plotly):
    `print`, `copy`.
  - UDD class machinery: `makehandleclass`, `subsref`.
  - Signal pole-zero plot: `zplane`.
  - Toolbox-internal construction helpers (not needed now that plotting +
    property model are native): `colorset`, `completeprops`, `hrawplot`,
    `htextbitmap`, `markerset`, `matchit`, `parseit`, `stcmp`, `styleset`.
  > The MATLAB-standard 3-D plot types `surfl`/`surfc`/`meshc`/`waterfall`/
  > `sphere`/`cylinder`/`ellipsoid` are now **implemented** (Plotly-rendered;
  > the geometry generators return `[X,Y,Z]` grids). They have no FreeMat
  > directive name, so they aren't scored in the counts above.
- **Interactive / audio device (3)** — `input` (interactive stdin), `wavplay`,
  `wavrecord` (need an audio device). `wavread`/`wavwrite` ARE implemented.
- **Heavy dependency — network / XML / HTML (3)** — `urlwrite`, `xmlread`,
  `htmlread`.
- **Other internal helpers (4)** — `inline_evaluate`, `mkdir_core`, `p_end`,
  `regexprepdriver`.

---

## Quick map: where things live
- Interpreter / eval / indexing / scopes: `crates/fm-interp/`
- Values / `Array` (dense + sparse + function-handle), formatting: `crates/fm-core/`
- Lexer / parser / AST: `crates/fm-parser/`
- Dense + sparse linear algebra (faer): `crates/fm-linalg/`
- Builtins (math/strings/array/poly/bitops/baseconv/handles/graphics/random/time/…): `crates/fm-builtins/`
- MAT / file I/O / FFT / regex / image / WAV / OS: `crates/fm-io/`
- REPL + graphics webserver: `crates/fm-cli/`; browser frontend: `web/index.html`
- Conformance harness + corpus: `crates/fm-conformance/` (`-- --failures` lists red)
- Overall backlog: `docs/REMAINING.md`; help-example backlog: `docs/HELP_BACKLOG.md`
