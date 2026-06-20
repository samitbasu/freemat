# Help System Regeneration — Plan & Contract

Status: **P0 complete** (design spec finalized). This document is the shared roadmap/DAG for
the multi-agent effort to rebuild FreeMat's documentation/help system in the Rust port. Every
delegated agent should read **`docs/HELP_SYSTEM.md` first** (the authoritative data-model /
dialect / macro / artifact / runtime contract), then this file for the phase plan.
Cross-session tracker lives in the task list; high-level status rolls up into `PROGRESS.md`.

## Goal

Replace the original C++/Qt + Python (`helpgen.py` → Doxygen → HTML/LaTeX/PDF + `.mdc` text)
documentation pipeline with a Rust-native one:

- **Doc sources are embedded in Rust source** via a `register_doc!` macro next to each builtin.
  Toolbox `.m` functions keep their doc blocks in-place (translated to the new dialect).
- **`cargo xtask docgen`** regenerates all help artifacts.
- **Fragment generation** re-runs the `fm` interpreter to capture REPL transcripts and
  re-injects them into the rendered docs (replaces `fraggen.m` + `@<...@>`).
- **Help is shown in the browser** (reuse the embedded axum server) and **as terminal text**:
  `help <name>` prints concise text + opens/links the browser page; `helpwin <name>` opens
  the browser page.
- **Full migration** of the ~500 legacy `@Module` doc blocks (C++ `FreeMat/src/**`, libs, and
  toolbox `.m`) into the new system.

## Background (original system, for reference)

> **Corrected after inspecting `../FreeMat` (2026-06-20).** The version we port from has
> already migrated off the old inline `@Module`/`@<...@>` comment blocks to a **Doxygen**
> system. See `docs/HELP_SYSTEM.md` §1 for the authoritative legacy map. In brief:
- C++ builtins carry only a `//@@Signature … //DOCBLOCK <id>` stub; the body lives in a
  separate Doxygen page `../FreeMat/doc/<section>/<name>.doc` (`\page`, `\section`,
  `\verbatim`, `\f[…\f]`/`\f$…\f$`, `\if FRAGMENT … \endif`, `\verbinclude`, `\image`).
- **Real migration surface: ~597 `.doc` pages across ~35 sections, 334 with executable
  fragments.** Only **2** toolbox `.m` files still use the old inline `%!`/`@Module` style.
- Original `helpgen.py`/`fraggen.m`/`mergefragments.py` + Doxygen produced HTML/LaTeX/PDF;
  `\if FRAGMENT` blocks were run by a live FreeMat capturing `--> input\n<output>` into
  `*.m.out` and re-injected via `\verbinclude`.
- Sources to migrate from: `../FreeMat/doc/**/*.doc` (primary), the `//DOCBLOCK` stubs in
  `../FreeMat/libs/**/*.cpp` (to map id→builtin), the 2 inline-`@Module` `.m` files, and
  section descriptions in `../FreeMat/doc/sections/sec_*.doc` /
  `../FreeMat/toolbox/help/section_descriptors.txt`.

## Target architecture

### Markdown dialect (the contract every doc body uses)

Doc bodies are CommonMark with these conventions (mapping from legacy markup):

| Legacy            | New (markdown)                                  |
|-------------------|-------------------------------------------------|
| `@|code|`         | `` `code` `` (inline code)                      |
| `@[ ... @]`       | ```` ```text ```` fenced block (verbatim)       |
| `@< ... @>`       | ```` ```fm-exec ```` fenced block (executed)    |
| `@{name ... @}`   | ```` ```fm-file:name ```` fenced block          |
| `@figure name`    | ```` ```fm-exec:figure ```` (captures a plot)   |
| `\[ ... \]`       | `$$ ... $$` (display math, KaTeX)               |
| `\f$ ... \f$`     | `$ ... $` (inline math)                         |
| `\begin{itemize}` | `- ` markdown lists                             |
| `@@Usage` etc.    | `## Usage` markdown headings                    |

`fm-exec` blocks: each is a sequence of REPL input lines. docgen runs them through the
capture engine and renders the captured transcript (input prefixed with `--> `, followed by
output) in place of / beneath the source. A leading `# errors: N` line declares expected
error count (replaces the old first-line error count).

### DocEntry schema (the in-memory + serialized model)

```
DocEntry {
  name: String,           // primary topic name, e.g. "sin"
  aliases: Vec<String>,   // alternate names that resolve here
  section: String,        // section id, e.g. "elementary"
  summary: String,        // one-line, used in indexes & `help` header
  body_md: String,        // markdown body in the dialect above
  source: SourceKind,     // RustBuiltin{crate,module} | Toolbox{path} 
}
```

### `register_doc!` macro

Lives in a new `fm-doc` crate. Registers a `DocEntry` near each builtin. Collected into a
registry at startup (recommended impl: `inventory`-style distributed collection, or explicit
`register_docs(&mut Registry)` per module — implementer chooses; must be deterministic when
sorted by `(section, name)`). The runtime registry holds `body_md` (compiled in); captured
fragment transcripts are a separate generated artifact (see below).

### Generated artifacts

`cargo xtask docgen` produces:
1. **Fragment DB** — captured transcripts keyed by fragment content hash, embedded into the
   binary (generated Rust or an embedded file via `include_*`). Cached: unchanged fragments
   are not re-run.
2. **Browser help assets** — served at `/help` and `/help/<name>` by the axum server
   (markdown→HTML with KaTeX + code highlighting + Plotly for figures). May render at runtime
   from the registry + fragment DB, or be pre-rendered — implementer chooses, document it.
3. **Terminal text rendering** — concise text for `help <name>` (the `.mdc` equivalent),
   derived at runtime from the registry + fragment DB.

### Runtime help

- `help <name>`: print terminal text rendering; show the browser URL (and open it per the
  "terminal + browser" decision). `help` with no args lists sections.
- `helpwin <name>`: open the browser help page.
- Resolution: exact name → alias → case-insensitive → "did you mean".

## Phased decomposition (delegation units)

Each phase is an agent-sized task with a crisp DoD. DAG dependencies noted. Standing gates for
every code phase: `cargo build`, `cargo clippy -- -D warnings`, `cargo fmt --check`, relevant
tests; main agent re-verifies gates independently (agents have over-claimed before).

- **P0 — Design spec finalization** ✅ **done** — `docs/HELP_SYSTEM.md` authored (dialect,
  macro contract, artifact format, runtime contract); open choices locked (§0); legacy map
  corrected (§1). Authored centrally. *Blocks all.*
- **P1 — `fm-doc` crate**: DocEntry, Registry, `register_doc!` macro, markdown-dialect parser
  (extract `fm-exec`/`fm-file`/`figure` fences, math, headings), `.m` `%!`-block parser.
  Unit tests. *Depends: P0.*
- **P2 — Fragment capture engine**: scripted-session capture in `fm-cli` (feed input lines →
  exact REPL transcript incl. `ans =` displays + errors). `fm --capture-fragment` and/or a lib
  fn. Tests vs. live REPL formatting. *Depends: P0 (can run parallel to P1).*
- **P3 — `xtask` crate + `cargo xtask docgen`**: `.cargo/config.toml` alias; collect docs (P1)
  + run/cache fragments (P2) → generated artifacts. *Depends: P1, P2.*
- **P4 — Browser help serving + rendering**: axum `/help`, `/help/<name>`, search; md→HTML +
  KaTeX + highlight + Plotly figures. *Depends: P3 (artifact format; may stub early).*
- **P5 — `help`/`helpwin` builtins + terminal rendering**: register builtins, terminal text
  render from registry, browser launch via existing `webbrowser`. *Depends: P1, P4.*
- **P6 — Legacy migration converter**: `cargo xtask migrate-docs` — parse legacy `@Module`
  blocks (C++ + `.m`) → emit `register_doc!` bodies / translated `.m` blocks into a staging
  area for review. *Depends: P1.*
- **P7 — Bulk migration (fan-out by section)**: run the converter per section/crate, place
  `register_doc!` entries next to builtins, translate toolbox `.m` blocks in place, fix
  conversion artifacts, regenerate fragments, spot-check. One agent per section
  (elementary, trig, reductions, logical, constructors, random, linalg, inspection,
  array, strings, setops, cellstruct, time, interp_ops, bitops, baseconv, polynomial, io,
  graphics, sparse, misc, …). *Depends: P3, P6.*
- **P8 — Integration & polish**: end-to-end docgen clean; `help`/`helpwin` verified; browser
  index/search; CI wiring for `docgen --check`; update `PLAN.md`/`PROGRESS.md`; author a
  "writing docs" contributor note. *Depends: all.*

## Open implementation choices — **resolved in P0**

All locked in `docs/HELP_SYSTEM.md` §0. Summary: `inventory` distributed registration ·
generated-Rust fragment DB checked in · runtime `pulldown-cmark` md→HTML · Plotly scene-JSON
figures (browser only) · KaTeX math · `help` = text + URL (no auto-open), `helpwin` = open.
