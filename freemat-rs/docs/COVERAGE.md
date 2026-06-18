# FreeMat-rs builtin coverage

> **Generated snapshot** regenerated after the builtin gap-fill pass (HEAD
> `d93455450`, rust-port branch; `--list-builtins` now reports **318**). This is a
> point-in-time diff of the **real freemat-rs registration table** against the
> FreeMat 4.2 builtin surface — *not* a grep of the source tree. A prior
> grep-based inventory was wrong (it falsely reported `pi`/`eps` as missing);
> this one is built from what is actually registered.
>
> **How to regenerate the freemat-rs side:**
> ```
> cargo run -q -p fm-cli -- --list-builtins
> ```
> That constructs an `Interpreter`, registers the full standard library
> (`fm_builtins::register_standard_library`, which pulls in `fm-io` and the
> graphics + time builtins), and prints every registered function name, sorted,
> one per line. That output is the **authoritative** freemat-rs builtin set used
> below.

## What counts as a "builtin"

Two subtleties make a naive count wrong, both handled here:

1. **Evaluator constants are not in the function table.** `pi`, `e`, `eps`,
   `Inf`/`inf`, `NaN`/`nan`, `i`/`j`/`I`/`J`, `true`, `false` are resolved
   directly in `fm-interp`'s `eval_ident` (interp.rs ~L624), so they do **not**
   appear in `--list-builtins`. **They ARE implemented** — the prior report's
   claim that `pi`/`eps` were missing was a false negative from grepping the
   registration table. This snapshot adds those 13 constants to the
   freemat-rs implemented set.
2. **freemat-rs implements many FreeMat toolbox `.m` functions as native Rust
   builtins** (e.g. `plot`, `linspace`, `repmat`, `all`, `any`). So the fair
   comparison is the **whole FreeMat name universe** (C++ builtins + toolbox
   `.m`) vs the freemat-rs registration table + constants.

The FreeMat 4.2 side is taken from:
- **C++ builtins** — the `//function` / `//sfunction` / `//sgfunction` /
  `//gfunction` `@@Signature` directives across
  `/home/samitbasu/Devel/freemat/FreeMat/libs/` (libCore, libGraphics, libFN,
  libFreeMat). **330 names** (of which **44** are the dropped ITK/VTK/GL image
  primitives, e.g. `cannyedgedetector`, `glshow`).
- **toolbox** — the `.m` basenames under
  `/home/samitbasu/Devel/freemat/FreeMat/toolbox/`. **249 names** (the 94
  `toolbox/help/*.m` doc stubs and the `tests/` harness stubs are excluded from
  the coverage tables).

## Summary

| metric | count |
|---|---:|
| freemat-rs registered builtins (`--list-builtins`) | **318** |
| freemat-rs evaluator constants (`pi`/`eps`/`i`/`true`/…) | 13 |
| **freemat-rs total implemented** | **~330** |
| FreeMat C++ builtins (all directive types) | 330 |
|  — of which dropped (ITK/VTK/GL image primitives) | 44 |
| FreeMat toolbox `.m` (excl. help/test stubs) | ~219 |
| **FreeMat name universe scored below** (C++ core + toolbox) | **505** |
| **FreeMat names implemented in freemat-rs** | **~265** |
| **headline coverage** | **~52%** |

> The builtin gap-fill pass added: **bit ops** (`bitand`/`bitor`/`bitxor`/
> `bitcmp`/`bitshift`), **base conversion** (`dec2hex`/`hex2dec`/`dec2bin`/
> `bin2dec`/`num2hex`/`hex2num`/`int2bin`/`bin2int`), **polynomial**
> (`polyval`/`polyfit`/`roots`/`poly`/`polyder`/`polyint`/`conv`/`deconv`),
> **linalg extras** (`cond`/`rcond`/`rref`/`kron`/`null`/`orth`/`tril`/`triu`),
> **trig gaps** (degree variants `sind`…`acscd`, hyperbolic reciprocals
> `sech`/`csch`/`coth`/`asech`/`acsch`/`acoth`, inverse reciprocals
> `acot`/`asec`/`acsc`), **special functions** (`erf`/`erfc`/`gamma`/`gammaln`),
> **misc numeric** (`vec`/`diff`/`dot`/`cross`/`meshgrid`/`ndgrid`/`deal`), and
> `eps` (as a callable function) + `seed` (deterministic RNG reseeding).

`pi`/`eps` status: **implemented** (evaluator constants).
`subplot` status: **NOT implemented** — it is a FreeMat *toolbox* function that
depends on the graphics handle / `set`/`get` property system and multiple axes
per figure, which are gated on **Stage 7.5** (not yet built). Same for
`contour`, `axes`, `cla`, `colorbar`, `colormap`, `set`/`get`.

## Coverage by category

Categorized by the originating FreeMat source (libCore file / libGraphics /
toolbox subdir) rather than by keyword, so the buckets are accurate. "FM" =
FreeMat names in that category (excluding dropped ITK/GL and help/test stubs);
"rs" = how many of those freemat-rs implements; "cov%" = rs/FM.

| category | FM | rs | cov% | notable missing |
|---|---:|---:|---:|---|
| math (elementary) | 29 | 25 | 86% | `betainc` `erfinv` `legendre` (`erf`/`erfc`/`gamma`/`gammaln` ✓) |
| trig | 33 | 33 | 100% | — (degree variants, hyperbolic + inverse reciprocals all added) |
| reductions / stat | 6 | 5 | 83% | `cov` |
| linear algebra | 18 | 16 | 89% | `eigs` `expm` (`cond`/`rcond`/`rref`/`tril`/`triu`/`kron`/`null`/`orth` ✓) |
| array construct / manip | 67 | 49 | 73% | `arrayfun` `shiftdim` `nonzeros` (`meshgrid`/`ndgrid`/`deal`/`vec` ✓) |
| logical / relational | 8 | 8 | 100% | — (`bitand`/`bitor`/`bitxor`/`bitcmp`/`bitshift`/`dec2bin`/`bin2dec`/`int2bin`/`bin2int` ✓) |
| strings | 23 | 20 | 87% | `cellstr` `strstr` |
| cell / struct | 7 | 1 | 14% | the `ctype*` C-struct interop (`ctypedefine`/`ctypesize`/…) — native-FFI, deprioritized |
| type / inspection | 44 | 28 | 64% | `computer` `version` `which`/`who`/`whos` `issparse` (`dec2hex`/`hex2dec`/`num2hex` ✓) |
| file I/O | 25 | 9 | 36% | `dlmread` `csvread`/`csvwrite` `fseek`/`ftell` `format` `input` `fflush` |
| FFT / signal | 8 | 2 | 25% | `conv` `fftn`/`ifftn` `fftshift`/`ifftshift` `hilbert` |
| random | 16 | 4 | 25% | `randperm`, and the distribution draws (`randbeta`/`randchi`/`randp`/…) (`seed` ✓) |
| graphics | 83 | 21 | 25% | `subplot` `axes` `set`/`get` `contour` `colorbar`/`colormap` `xlim`/`ylim` `patch` `plot3` (Stage 7.5) |
| time | 5 | 4 | 80% | `clocktotime` (`tic`/`toc`/`clock`/`etime` ✓; rs adds `cputime`/`pause`/`now`) |
| debug | 11 | 1 | 9% | `dbstop`/`dbstep`/`dblist`/`dbstack` `lasterr` `warning` (Stage 10) |
| sparse | 7 | 0 | 0% | `sparse` `full` `speye` `spones` `nnz` `spy` (Stage 9) |
| polynomial | 6 | 6 | 100% | — (`polyval`/`polyfit`/`roots`/`poly`/`polyder`/`polyint`/`conv`/`deconv` ✓) |
| ODE | 13 | 5 | 38% | `ode45` `odeset` `deval` `trapz`/`cumtrapz` |
| system / OS | 26 | 0 | 0% | `cd` `pwd` `dir`/`ls` `getenv` `system` `mkdir` `fileparts` `path` `help` |
| misc | 70 | 26 | 37% | `conv2` `interp2` `func2str`/`str2func` `fullfile` `getenv` (`diff`/`dot`/`cross`/`conv`/`rcond` ✓) |
| **TOTAL** | **505** | **~265** | **~52%** | |

Notes on the table:
- **math/trig** is high-value but partial: the elementary functions are all
  there; the degree-valued trig (`sind` etc.), inverse reciprocals
  (`acot`/`asec`/`acsc`), and special functions (`erf`/`gamma`) are the gap.
- **graphics 25%** reflects the Stage 7 single-axes-per-figure model with no
  property system: `plot`/`surf`/`mesh`/`image`/`title`/`axis`/`grid`/`hold`/
  `legend`/`semilog*`/`loglog` work, but everything that needs real handles +
  `set`/`get` (`subplot`, `axes`, `contour`, `colorbar`, limits) is **Stage 7.5**.
- **logical/relational 0%, sparse 0%, polynomial 0%, system/OS 0%** are the
  cleanest large wins still open: bit ops, the sparse type (Stage 9), polynomial
  functions, and OS/filesystem builtins.
- freemat-rs also ships ~50 MATLAB-standard names FreeMat lacks under the same
  spelling (e.g. `chol`, `gcd`/`lcm`, `mesh`, `union`/`intersect`/`setdiff`/
  `ismember`, `flip`, `mat2str`, `cputime`/`pause`/`now`) — these are net
  additions, not double-counted in the FM column above.

## Appendix — not-yet-implemented builtins (actionable backlog)

Every scored FreeMat name freemat-rs does not yet implement, grouped by
category. (Dropped ITK/VTK/GL primitives and help/test stubs are omitted.)

- **math** (3): `betainc`, `erfinv`, `legendre` (`erf`/`erfc`/`gamma`/`gammaln` now ✓)
- **trig** (0): all degree variants, hyperbolic reciprocals, and inverse reciprocals are now implemented
- **reductions / stat** (1): `cov`
- **linear algebra** (2): `eigs`, `expm` (`cond`/`rcond`/`rref`/`tril`/`triu`/`kron`/`null`/`orth` now ✓)
- **array construct / manip** (17): `arrayfun`, `cast`, `flipdim`, `isalpha`, `isdigit`, `ishandle`, `ishold`, `isinttype`, `isspace`, `issquare`, `isstr`, `maxdim`, `nnz`, `nonzeros`, `shiftdim`, `subsref`, `test` (`deal`/`meshgrid`/`ndgrid`/`vec` now ✓)
- **logical / relational** (0): `bitand`/`bitor`/`bitxor`/`bitcmp`/`bitshift`/`dec2bin`/`bin2dec`/`int2bin`/`bin2int` now ✓
- **strings** (3): `cellstr`, `regexprepdriver`, `strstr`
- **cell / struct** (6): `cenum`, `ctypedefine`, `ctypefreeze`, `ctypeprint`, `ctypesize`, `ctypethaw`
- **type / inspection** (17): `IsInf`, `IsNaN`, `computer`, `isequalwithequalnans`, `issparse`, `makehandleclass`, `mfilename`, `nargin`, `nargout`, `p_end`, `string`, `version`, `verstring`, `where`, `which`, `who`, `whos` (`dec2hex`/`hex2dec`/`num2hex`/`hex2num` now ✓)
- **file I/O** (16): `csvread`, `csvwrite`, `dlmread`, `fflush`, `format`, `fseek`, `ftell`, `getline`, `getprintlimit`, `input`, `rawread`, `rawwrite`, `setprintlimit`, `type`, `wavread`, `wavwrite`
- **FFT / signal** (6): `conv`, `fftn`, `fftshift`, `hilbert`, `ifftn`, `ifftshift`
- **random** (12): `randbeta`, `randbin`, `randchi`, `randexp`, `randf`, `randgamma`, `randmulti`, `randnbin`, `randnchi`, `randnf`, `randp`, `randperm` (`seed` now ✓)
- **graphics** (62): `axes`, `cla`, `clabel`, `clim`, `close`, `colorbar`, `colormap`, `colorset`, `completeprops`, `contour`, `contour3`, `copper`, `copy`, `datacursormanager`, `datacursormode`, `figlower`, `figraise`, `get`, `gray`, `hcontour`, `himage`, `hist`, `hline`, `hpatch`, `hpoint`, `hrawplot`, `htext`, `htextbitmap`, `imread`, `imwrite`, `is2dview`, `islinespec`, `markerset`, `matchit`, `newplot`, `parseit`, `patch`, `pcolor`, `plot3`, `point`, `print`, `pvalid`, `quiver`, `set`, `sizefig`, `stcmp`, `styleset`, `subplot`, `surface`, `testtube`, `text`, `tubeplot`, `uicontrol`, `view`, `volrender`, `vtkfigure`, `winlev`, `xlim`, `ylim`, `zlim`, `zoom`, `zplane`
- **time** (1): `clocktotime`
- **debug** (10): `dbauto`, `dbdelete`, `dblist`, `dbstop`, `errorcount`, `fdump`, `jitcontrol`, `jitstat`, `lasterr`, `warning`
- **sparse** (7): `full`, `sparse`, `speye`, `spones`, `sprand`, `sprandn`, `spy`
- **polynomial** (0): `poly`/`polyder`/`polyfit`/`polyint`/`polyval`/`roots` (+ `conv`/`deconv`) now ✓
- **ODE** (8): `cumtrapz`, `deval`, `idiv`, `mpower`, `ode45`, `odeset`, `teps`, `trapz`
- **system / OS** (26): `blaslib`, `cd`, `copyfile`, `delete`, `dir`, `dirsep`, `fileattrib`, `fileparts`, `getpath`, `help`, `helpwin`, `htmlread`, `import`, `loadlib`, `ls`, `mkdir`, `mkdir_core`, `pathtool`, `pwd`, `rmdir`, `setpath`, `urlwrite`, `wavplay`, `wavrecord`, `what`, `xmlread`
- **misc** (44): `addpath`, `bind`, `conv2`, `ctypecast`, `ctypenew`, `ctyperead`, `ctypewrite`, `diary`, `docli`, `exit`, `filesep`, `fitfun`, `fullfile`, `func2str`, `gausfit`, `getenv`, `gfitfun`, `inline`, `inline_evaluate`, `install`, `interp2`, `interplin1`, `license`, `path`, `pathsep`, `pcode`, `qtnew`, `quiet`, `rehash`, `rescan`, `simkeys`, `source`, `str2func`, `symvar`, `system`, `threadcall`, `wb_test`, `wbgentests`, `wbtest_exact`, `wbtest_near`, `wbtest_near_permute`, `wbtestcompare`, `wbtestinputs`, `wrap_jit_test`, `wrap_test` (`diff`/`dot`/`cross`/`rcond` now ✓)

## Toolbox caveat — runnability is the real signal

FreeMat ships **317 `.m` toolbox files** (the same files are present verbatim in
`freemat-rs/toolbox/`). They run **unchanged** on the interpreter — *but only
when the builtins they call exist*. A toolbox `.m` whose name is "not
implemented as a native builtin" above may still be **runnable** if freemat-rs
provides the same behavior natively, and conversely a toolbox `.m` that is
"present" is useless until its dependency builtins land. So this name-level diff
is a *capability inventory*, not a runnability score.

The authoritative runnability signal is the **conformance suite** (`cargo run
--release -q -p fm-conformance`), which actually executes the FreeMat `test_*.m`
cases. After the builtin gap-fill pass it stands at **309 / 640 ≈ 48.3%** — that is the honest
"how much actually works end to end" number; the table above is the "how much of
the API surface exists" number.
