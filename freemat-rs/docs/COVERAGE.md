# FreeMat-rs builtin coverage

> **Generated snapshot** at commit `56d46cc92` (rust-port branch). This is a
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
| freemat-rs registered builtins (`--list-builtins`) | **244** |
| freemat-rs evaluator constants (`pi`/`eps`/`i`/`true`/…) | 13 |
| **freemat-rs total implemented** | **~255** |
| FreeMat C++ builtins (all directive types) | 330 |
|  — of which dropped (ITK/VTK/GL image primitives) | 44 |
| FreeMat toolbox `.m` (excl. help/test stubs) | ~219 |
| **FreeMat name universe scored below** (C++ core + toolbox) | **505** |
| **FreeMat names implemented in freemat-rs** | **206** |
| **headline coverage** | **~41%** |

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
| math (elementary) | 29 | 21 | 72% | `erf` `erfc` `gamma` `gammaln` `betainc` |
| trig | 33 | 13 | 39% | degree variants `sind`/`cosd`/`tand`, `acot`/`asec`/`acsc`, `sech`/`coth` |
| reductions / stat | 6 | 5 | 83% | `cov` |
| linear algebra | 18 | 10 | 56% | `cond` `eigs` `expm` `rref` `tril` `triu` |
| array construct / manip | 67 | 46 | 69% | `meshgrid` `ndgrid` `deal` `arrayfun` `shiftdim` `nonzeros` |
| logical / relational | 8 | 0 | 0% | `bitand` `bitor` `bitxor` `bitcmp` `dec2bin` `bin2dec` |
| strings | 23 | 20 | 87% | `cellstr` `strstr` |
| cell / struct | 7 | 1 | 14% | the `ctype*` C-struct interop (`ctypedefine`/`ctypesize`/…) — native-FFI, deprioritized |
| type / inspection | 44 | 24 | 55% | `computer` `version` `which`/`who`/`whos` `dec2hex`/`hex2dec` `issparse` |
| file I/O | 25 | 9 | 36% | `dlmread` `csvread`/`csvwrite` `fseek`/`ftell` `format` `input` `fflush` |
| FFT / signal | 8 | 2 | 25% | `conv` `fftn`/`ifftn` `fftshift`/`ifftshift` `hilbert` |
| random | 16 | 3 | 19% | `randperm` `seed`, and the distribution draws (`randbeta`/`randchi`/`randp`/…) |
| graphics | 83 | 21 | 25% | `subplot` `axes` `set`/`get` `contour` `colorbar`/`colormap` `xlim`/`ylim` `patch` `plot3` (Stage 7.5) |
| time | 5 | 4 | 80% | `clocktotime` (`tic`/`toc`/`clock`/`etime` ✓; rs adds `cputime`/`pause`/`now`) |
| debug | 11 | 1 | 9% | `dbstop`/`dbstep`/`dblist`/`dbstack` `lasterr` `warning` (Stage 10) |
| sparse | 7 | 0 | 0% | `sparse` `full` `speye` `spones` `nnz` `spy` (Stage 9) |
| polynomial | 6 | 0 | 0% | `polyval` `polyfit` `roots` `poly` `polyder` `polyint` |
| ODE | 13 | 5 | 38% | `ode45` `odeset` `deval` `trapz`/`cumtrapz` |
| system / OS | 26 | 0 | 0% | `cd` `pwd` `dir`/`ls` `getenv` `system` `mkdir` `fileparts` `path` `help` |
| misc | 70 | 21 | 30% | `diff` `dot` `cross` `conv2` `interp2` `func2str`/`str2func` `fullfile` `getenv` |
| **TOTAL** | **505** | **206** | **~41%** | |

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

- **math** (8): `asech`, `betainc`, `erf`, `erfc`, `erfinv`, `gamma`, `gammaln`, `legendre`
- **trig** (20): `acosd`, `acot`, `acotd`, `acoth`, `acsc`, `acscd`, `acsch`, `asec`, `asecd`, `asind`, `atand`, `cosd`, `cotd`, `coth`, `cscd`, `csch`, `secd`, `sech`, `sind`, `tand`
- **reductions / stat** (1): `cov`
- **linear algebra** (8): `cond`, `eigs`, `expm`, `rref`, `transpose`, `tril`, `triu`, `xnrm2`
- **array construct / manip** (21): `arrayfun`, `cast`, `deal`, `flipdim`, `isalpha`, `isdigit`, `ishandle`, `ishold`, `isinttype`, `isspace`, `issquare`, `isstr`, `maxdim`, `meshgrid`, `ndgrid`, `nnz`, `nonzeros`, `shiftdim`, `subsref`, `test`, `vec`
- **logical / relational** (8): `bin2dec`, `bin2int`, `bitand`, `bitcmp`, `bitor`, `bitxor`, `dec2bin`, `int2bin`
- **strings** (3): `cellstr`, `regexprepdriver`, `strstr`
- **cell / struct** (6): `cenum`, `ctypedefine`, `ctypefreeze`, `ctypeprint`, `ctypesize`, `ctypethaw`
- **type / inspection** (20): `IsInf`, `IsNaN`, `computer`, `dec2hex`, `hex2dec`, `isequalwithequalnans`, `issparse`, `makehandleclass`, `mfilename`, `nargin`, `nargout`, `num2hex`, `p_end`, `string`, `version`, `verstring`, `where`, `which`, `who`, `whos`
- **file I/O** (16): `csvread`, `csvwrite`, `dlmread`, `fflush`, `format`, `fseek`, `ftell`, `getline`, `getprintlimit`, `input`, `rawread`, `rawwrite`, `setprintlimit`, `type`, `wavread`, `wavwrite`
- **FFT / signal** (6): `conv`, `fftn`, `fftshift`, `hilbert`, `ifftn`, `ifftshift`
- **random** (13): `randbeta`, `randbin`, `randchi`, `randexp`, `randf`, `randgamma`, `randmulti`, `randnbin`, `randnchi`, `randnf`, `randp`, `randperm`, `seed`
- **graphics** (62): `axes`, `cla`, `clabel`, `clim`, `close`, `colorbar`, `colormap`, `colorset`, `completeprops`, `contour`, `contour3`, `copper`, `copy`, `datacursormanager`, `datacursormode`, `figlower`, `figraise`, `get`, `gray`, `hcontour`, `himage`, `hist`, `hline`, `hpatch`, `hpoint`, `hrawplot`, `htext`, `htextbitmap`, `imread`, `imwrite`, `is2dview`, `islinespec`, `markerset`, `matchit`, `newplot`, `parseit`, `patch`, `pcolor`, `plot3`, `point`, `print`, `pvalid`, `quiver`, `set`, `sizefig`, `stcmp`, `styleset`, `subplot`, `surface`, `testtube`, `text`, `tubeplot`, `uicontrol`, `view`, `volrender`, `vtkfigure`, `winlev`, `xlim`, `ylim`, `zlim`, `zoom`, `zplane`
- **time** (1): `clocktotime`
- **debug** (10): `dbauto`, `dbdelete`, `dblist`, `dbstop`, `errorcount`, `fdump`, `jitcontrol`, `jitstat`, `lasterr`, `warning`
- **sparse** (7): `full`, `sparse`, `speye`, `spones`, `sprand`, `sprandn`, `spy`
- **polynomial** (6): `poly`, `polyder`, `polyfit`, `polyint`, `polyval`, `roots`
- **ODE** (8): `cumtrapz`, `deval`, `idiv`, `mpower`, `ode45`, `odeset`, `teps`, `trapz`
- **system / OS** (26): `blaslib`, `cd`, `copyfile`, `delete`, `dir`, `dirsep`, `fileattrib`, `fileparts`, `getpath`, `help`, `helpwin`, `htmlread`, `import`, `loadlib`, `ls`, `mkdir`, `mkdir_core`, `pathtool`, `pwd`, `rmdir`, `setpath`, `urlwrite`, `wavplay`, `wavrecord`, `what`, `xmlread`
- **misc** (49): `addpath`, `bind`, `conv2`, `cross`, `ctypecast`, `ctypenew`, `ctyperead`, `ctypewrite`, `diary`, `diff`, `docli`, `dot`, `exit`, `filesep`, `fitfun`, `fullfile`, `func2str`, `gausfit`, `getenv`, `gfitfun`, `inline`, `inline_evaluate`, `install`, `interp2`, `interplin1`, `license`, `path`, `pathsep`, `pcode`, `qtnew`, `quiet`, `rcond`, `rehash`, `rescan`, `simkeys`, `source`, `str2func`, `symvar`, `system`, `threadcall`, `wb_test`, `wbgentests`, `wbtest_exact`, `wbtest_near`, `wbtest_near_permute`, `wbtestcompare`, `wbtestinputs`, `wrap_jit_test`, `wrap_test`

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
cases. At this commit it stands at **284 / 637 ≈ 44.6%** — that is the honest
"how much actually works end to end" number; the table above is the "how much of
the API surface exists" number.
