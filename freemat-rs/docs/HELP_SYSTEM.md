# Help System — Design Spec & Contract (P0)

Status: **finalized**. This is the authoritative contract for the FreeMat-rs help/docs
rebuild. The phased roadmap and DAG live in [`HELP_REGEN_PLAN.md`](./HELP_REGEN_PLAN.md);
**this** document pins down the data model, the markdown dialect, the macro API, the
generated-artifact formats, and the runtime behavior that P1–P8 must implement *exactly*.
If an implementation detail here proves wrong, change it **here first**, then in code.

Companion docs: [`PLAN.md`](./PLAN.md) (overall port), [`PROGRESS.md`](../PROGRESS.md)
(status of record).

---

## 0. Decisions locked in P0

These resolve the "Open implementation choices" from the plan. They are deliberate
engineering defaults, each reversible, but **not to be relitigated** without updating this
section.

| # | Choice | **Decision** | Rationale |
|---|--------|-------------|-----------|
| 1 | Registry collection | **`inventory` crate** (distributed static registration) | Zero per-module boilerplate; a `register_doc!` next to each builtin is collected automatically. Sorted by `(section, name)` at load for determinism. |
| 2 | Artifact embedding | **Generated Rust source, checked in**, under `crates/fm-doc/src/generated/` | Reviewable git diffs; no startup parse; `cargo build` stays hermetic (no network, no codegen build-script surprise). |
| 3 | Browser rendering | **Runtime md→HTML** via `pulldown-cmark` | Single source of truth (the in-binary registry); no pre-render duplication; `/help` reflects the running binary exactly. |
| 4 | Figure capture | **Plotly scene JSON** captured through the graphics sink, embedded in the fragment DB; rendered only in the browser. Terminal omits figures (prints a `[figure: name]` placeholder). | Reuses the existing `fm-graphics` `Scene` + Plotly browser stack. |
| 5 | Browser math | **KaTeX** (self-hosted assets) | Faster, no layout reflow, smaller than MathJax, offline-friendly. |
| 6 | `help` browser behavior | `help <name>` **prints text and shows the browser URL but does NOT auto-open** a browser. When stdout is a TTY that supports OSC 8, the URL is rendered as a **clickable terminal hyperlink** (graceful fallback to a bare URL otherwise). `helpwin <name>` opens the browser. | Matches original FreeMat (`help` = text, `helpwin` = window); auto-opening on every `help` in a terminal REPL is hostile, but a clickable link makes the page one click away. |

New workspace dependencies these introduce (add to root `Cargo.toml`
`[workspace.dependencies]` as each phase lands):

```toml
inventory = "0.3"          # P1: distributed doc registration
pulldown-cmark = "0.12"    # P1/P4: markdown dialect parsing + HTML render
supports-hyperlinks = "3"  # P5: detect OSC 8 hyperlink support for `help`
# (KaTeX + highlight.js shipped as self-hosted static assets under web/, not crates)
```

---

## 1. Legacy source map (corrected)

> **Correction to the plan's "Background" section.** The plan described inline
> `@Module`/`@<...@>` comment blocks as the migration source. That is the *old* FreeMat
> markup. The version we are porting from (`../FreeMat`) has **already migrated** to a
> Doxygen-based system. The real migration surface is:

- **C++ builtins** declare a signature + a doc-page id, *not* an inline body:
  ```cpp
  //@@Signature
  //function cos CosFunction jitsafe
  //inputs x
  //outputs y
  //DOCBLOCK mathfunctions_cos
  ```
  The body lives in a separate **Doxygen `.doc` page**:
  `../FreeMat/doc/<section>/<name>.doc`, e.g. `doc/mathfunctions/cos.doc`:
  ```
  /*!
  \page mathfunctions_cos COS Trigonometric Cosine Function
  Section: \ref sec_mathfunctions "Mathematical Functions"
  \section Usage
  ... <tt>inline code</tt> ... \verbatim block \endverbatim ...
  \f[ display math \f]   \f$ inline math \f$
  \section Example
  \if FRAGMENT
  frag_mathfunctions_cos_000.m
  0
  x = linspace(0,1);
  plot(x,cos(2*pi*x))
  mprint('cosplot');
  \endif
  \verbinclude frag_mathfunctions_cos_000.m.out
  \image html cosplot.png
  */
  ```
- **Counts (non-vtk):** **597 `.doc` pages** across **~35 sections**, **334** containing
  executable `\if FRAGMENT` blocks. Section descriptions: `doc/sections/sec_<id>.doc` and
  `../FreeMat/toolbox/help/section_descriptors.txt`.
- **Toolbox `.m`:** only **2** files still carry the old inline `%!`/`@Module` style
  (`toolbox/deprecated/bind.m` and one other). Everything else was moved to `.doc`. So the
  P6 converter is **primarily a Doxygen-`.doc` parser**, with the old `@Module`/`%!` grammar
  as a small secondary path.

### Legacy → new dialect mapping (the P6 converter's job)

| Doxygen `.doc`                  | New (markdown dialect)                          |
|---------------------------------|-------------------------------------------------|
| `\page <id> <NAME> <summary>`   | `name` + `summary` + section from `<id>` prefix |
| `Section: \ref sec_x "..."`     | `section` field                                 |
| `\section Usage` / `\section Function Internals` / `\section Example` | `## Usage` / `## Function Internals` / `## Example` |
| `<tt>code</tt>`                 | `` `code` ``                                    |
| `\verbatim … \endverbatim`      | ```` ```text ```` fenced block                  |
| `\if FRAGMENT … \endif`         | ```` ```fm-exec ```` (see §3; drop the leading filename line, keep the error-count line) |
| `\verbinclude frag_*.m.out`     | *dropped* — the transcript is regenerated, not pasted |
| `\f[ … \f]`                     | `$$ … $$` (display math)                        |
| `\f$ … \f$`                     | `$ … $` (inline math)                           |
| `\image html name.png` + `mprint('name')` in fragment | ```` ```fm-exec:figure ```` (capture the plot) |
| `\ref other "text"` / `\ref other` | `[text](other)` cross-link (resolved by name)|
| `\[ … \]`, `\begin{itemize}`    | `$$…$$`, `- ` markdown lists (old `@Module` path)|

Legacy `@Module`/`@@Section`/`@|code|`/`@[…@]`/`@<…@>`/`@{name…@}` (the 2 remaining `.m`
files + any stragglers) map per the table in `HELP_REGEN_PLAN.md` §"Markdown dialect".

---

## 2. Data model

### 2.1 `DocEntry` (in-memory + serialized)

```rust
/// One help topic. The compiled-in registry holds these; fragment transcripts
/// are stored separately (see §4) and joined at render time by content hash.
pub struct DocEntry {
    /// Primary topic name, e.g. "cos". Lowercase, the canonical key.
    pub name: &'static str,
    /// Alternate names resolving here, e.g. ["arg"] for "angle".
    pub aliases: &'static [&'static str],
    /// Section id, e.g. "mathfunctions". Must match a SectionEntry.
    pub section: &'static str,
    /// One-line summary, used in indexes and the `help` header.
    pub summary: &'static str,
    /// Markdown body in the dialect of §3.
    pub body_md: &'static str,
    /// Provenance, for tooling and the "writing docs" workflow.
    pub source: SourceKind,
}

pub enum SourceKind {
    /// Native Rust builtin. `module` is the file-stem, e.g. "trig".
    RustBuiltin { krate: &'static str, module: &'static str },
    /// Embedded/toolbox `.m` function. `path` is repo-relative.
    Toolbox { path: &'static str },
}
```

All fields are `&'static` because entries are produced by the `register_doc!` macro at
compile time. The registry is a `BTreeMap`-like view sorted by `(section, name)`.

### 2.2 `SectionEntry`

```rust
pub struct SectionEntry {
    pub id: &'static str,        // "mathfunctions"
    pub title: &'static str,     // "Mathematical Functions"
    pub summary: &'static str,   // one paragraph, from sec_<id>.doc
}
```

Sections are registered the same way (`register_section!`) and seeded from
`section_descriptors.txt` during migration.

### 2.3 Registry API

```rust
pub struct Registry { /* sorted views built from inventory at first access */ }

impl Registry {
    pub fn global() -> &'static Registry;             // lazily built, deterministic
    pub fn get(&self, name: &str) -> Option<&DocEntry>;   // exact → alias
    pub fn resolve(&self, query: &str) -> Resolution;     // §6 resolution policy
    pub fn section(&self, id: &str) -> Option<&SectionEntry>;
    pub fn sections(&self) -> impl Iterator<Item = &SectionEntry>;
    pub fn in_section(&self, id: &str) -> impl Iterator<Item = &DocEntry>;
    pub fn iter(&self) -> impl Iterator<Item = &DocEntry>;
}

pub enum Resolution<'a> {
    Exact(&'a DocEntry),
    Suggestions(Vec<&'a str>),   // "did you mean" — names within edit distance
    None,
}
```

Determinism requirement: building the registry twice in one process yields identical order
(sort by `(section, name)`), and a duplicate `name` across two `register_doc!` sites is a
**hard error at registry build** (panic with both source locations) — caught by a unit test
and by `docgen --check`.

---

## 3. Markdown dialect (the doc-body contract)

Bodies are CommonMark (rendered by `pulldown-cmark`) plus the conventions below. Every doc
author and the P6 converter target this exactly.

- **Inline code:** standard `` `code` ``.
- **Headings:** `## Usage`, `## Function Internals`, `## Example`, etc. (`#` is reserved for
  the auto-generated page title; bodies start at `##`).
- **Lists, emphasis, links, tables:** standard CommonMark / GFM tables.
- **Cross-links:** `[text](name)` where `name` is a topic `name`/alias resolves to that help
  page (browser: `/help/name`; terminal: rendered as `text` with `(see: name)`).
- **Math:** `$ inline $` and `$$ display $$`, rendered by KaTeX in the browser; in the
  terminal, math is printed verbatim (dollar signs stripped) — no terminal TeX rendering.

### 3.1 Fenced blocks (the special ones)

| Fence info string | Meaning |
|-------------------|---------|
| ```` ```text ```` | Verbatim block. Rendered as-is, monospaced. Not executed. |
| ```` ```fm ```` | FreeMat source for *display only* (syntax-highlighted, not run). |
| ```` ```fm-exec ```` | **Executed.** Lines are fed to the capture engine (§4); the captured transcript replaces/augments the block at render time. |
| ```` ```fm-exec:figure ```` | Executed like `fm-exec`, **and** the resulting graphics `Scene` is captured and embedded as a Plotly figure (browser only). |
| ```` ```fm-file:NAME ```` | Declares an auxiliary file `NAME` (e.g. `hello.m`) made available on disk to subsequent `fm-exec` blocks in the **same DocEntry**, in declaration order. Rendered as a labeled code block. |

**`fm-exec` block grammar:**

```
```fm-exec
# errors: 0
x = linspace(0,1);
y = cos(2*pi*x)
```
```

- An **optional first line** `# errors: N` declares the number of errors the script is
  expected to raise (replaces the legacy fragment's bare error-count line). Default `0`.
  docgen fails if the actual error count differs (unless `N` matches).
- Remaining lines are REPL input, executed in order **in a fresh scope per DocEntry**
  (state persists across multiple `fm-exec` blocks within one DocEntry, like a session;
  resets between DocEntries).
- The rendered output is the transcript: each input line prefixed `--> `, followed by the
  captured output (including `ans = …` displays and error text). See §4.2 for exact format.

A DocEntry's set of `fm-exec`/`fm-file` blocks, concatenated in order, forms its
**fragment script**; its content hash is the fragment DB key (§4.3).

---

## 4. Fragment capture engine (P2)

### 4.1 Interface

Two entry points, same core:

- **Library:** `fm_cli::capture::run_fragment(script: &FragmentScript) -> CapturedFragment`
  (used by xtask in-process — fastest, no subprocess).
- **CLI:** `fm --capture-fragment <script.json>` writing `CapturedFragment` JSON to stdout
  (fallback / isolation if a fragment can crash the interpreter).

```rust
pub struct FragmentScript {
    pub files: Vec<(String, String)>,  // fm-file blocks: (name, contents)
    pub inputs: Vec<String>,           // one entry per fm-exec block (multi-line)
    pub expect_errors: usize,          // sum of `# errors:` declarations
    pub want_figure: bool,             // any fm-exec:figure present
}

pub struct CapturedFragment {
    pub transcript: String,    // exact terminal-format transcript (§4.2)
    pub error_count: usize,    // actual errors raised
    pub figure: Option<String>,// Plotly Scene JSON, if want_figure
}
```

### 4.2 Transcript format (must match the live REPL byte-for-byte)

The capture must reproduce what a user sees typing the same lines into `fm`:

- Each input line is echoed as `--> <line>` (the REPL prompt is `--> `).
- Suppressed lines (trailing `;`) produce no value display; unsuppressed expressions display
  `ans = …` or `<var> = …` using the **exact** interpreter formatting (same code path as the
  REPL — reuse the existing display formatter, do not re-implement).
- Errors render exactly as the REPL prints them (miette/plain), counted into `error_count`.
- Trailing whitespace per line is trimmed; the transcript ends with a single newline.

P2's tests assert capture output equals a live REPL session for a corpus of representative
scripts (scalars, matrices, complex, strings, errors, multi-output).

### 4.3 Caching & determinism

- Key = `blake3(canonicalized fragment script)` (lib already pulls no hasher — add `blake3`
  or reuse `rustc-hash` over a stable serialization; **blake3 recommended** for collision
  safety across the on-disk DB). Pin the choice in P3.
- Unchanged fragments are **not** re-run: docgen reads the existing DB, recaptures only
  missing/changed keys.
- Fragments must be **deterministic**: `rand`/`randn` seeded to a fixed value at capture
  start; `tic`/`toc`/`clock`/`now` either avoided in docs or stubbed. docgen sets a fixed
  RNG seed before each fragment script.

---

## 5. Generated artifacts (P3 — `cargo xtask docgen`)

`.cargo/config.toml` provides the alias:

```toml
[alias]
xtask = "run --package xtask --"
```

`cargo xtask docgen` (run from the workspace root):

1. **Collects** all `DocEntry`/`SectionEntry` from the registry (built into a small
   collector binary, or via `inventory` from a lib target — P3 decides the mechanics).
2. **Extracts** each entry's fragment script (P1 parser) and **runs/caches** fragments (P2).
3. **Emits** the fragment DB as checked-in generated Rust:
   ```
   crates/fm-doc/src/generated/fragments.rs   // pub static FRAGMENTS: &[(&str, Fragment)]
   ```
   keyed by hash, holding `transcript` and optional `figure` JSON. `fm-doc` `include!`s it.
4. **`cargo xtask docgen --check`** (CI): regenerates into a temp dir and fails if it differs
   from the checked-in file, or if any `# errors:` mismatch / duplicate name / dangling
   cross-link / unknown section is found. This is the CI gate (P8).

The **registry itself is not generated** — `body_md` is compiled in via `register_doc!`.
Only the *captured fragment transcripts* (which require running the interpreter) are
generated. This keeps `cargo build` hermetic; only `docgen` runs the interpreter.

---

## 6. Runtime help contract (P5)

### 6.1 Resolution policy (used by `help`, `helpwin`, and the browser)

1. Exact `name` match (case-sensitive) → that entry.
2. Alias match → target entry.
3. Case-insensitive `name`/alias match → that entry.
4. Otherwise → "did you mean": names within Damerau-Levenshtein distance ≤ 2, plus prefix
   matches, capped at ~8 suggestions.

### 6.2 `help`

- `help` (no args): print the section list — each `SectionEntry.title` + a one-line summary;
  hint to run `help <section-id>` or `help <name>`.
- `help <section-id>`: list the topics in that section (name + summary).
- `help <name>`: print the **terminal text rendering** (§6.4) of the entry, then a final
  `Browser:` line pointing at `http://127.0.0.1:<port>/help/<name>`. Link rendering:
  - stdout is a TTY **and** `supports_hyperlinks::on(Stream::Stdout)` → render the URL as a
    clickable **OSC 8** hyperlink (`ESC ]8;;<url>ESC \<label>ESC ]8;;ESC \`, label = the
    topic name or `open ↗`).
  - TTY without OSC 8 support → print the bare URL.
  - not a TTY (piped/redirected) → print the bare URL (no escapes).

  If the graphics server isn't running yet, print `(run helpwin <name> for the rich browser
  page)` instead of a URL — `help` itself never starts the server or opens a browser.
- Unresolved: print the "did you mean" suggestions.

### 6.3 `helpwin <name>`

- Ensures the embedded server is running (start it if not — same path as first plot), then
  opens `http://127.0.0.1:<port>/help/<name>` via the existing `webbrowser` crate.
- `helpwin` (no args): open `/help` (the index).

### 6.4 Terminal text rendering (the `.mdc` equivalent)

Derived at runtime from `body_md` + fragment DB. A small markdown→plain-text pass:

- Title line: `NAME — summary` (uppercase name), then a blank line.
- `## Heading` → the heading text, underlined with `-`.
- Inline `` `code` `` → code text (optionally dimmed); `**bold**`/`*italic*` → stripped or
  ANSI if a TTY.
- `text`/`fm` fenced blocks → indented 2 spaces.
- `fm-exec` blocks → the captured transcript (already terminal-formatted), indented 2 spaces.
- `fm-exec:figure` → the transcript plus a `[figure: <name>] (see helpwin <name>)` line.
- Math `$…$`/`$$…$$` → inner TeX printed verbatim (dollar signs removed).
- Cross-links `[text](name)` → on an OSC 8-capable TTY with the server running, a clickable
  link to `/help/name` labeled `text`; otherwise `text (see: name)`.
- Width-wrapped to the terminal width (fallback 80). OSC 8 escapes have zero display width, so
  wrap on the visible label, not the escape bytes.

### 6.5 Browser rendering (P4)

- Routes on the existing axum server: `GET /help` (index: sections → topics, plus a search
  box), `GET /help/<name>` (one page), `GET /help/search?q=…` (JSON, name+summary substring +
  resolution).
- Page render: `body_md` → HTML via `pulldown-cmark`; KaTeX renders `$`/`$$`; highlight.js
  styles `fm`/`text`/`fm-exec` blocks; `fm-exec` shows the captured transcript; `fm-exec:figure`
  embeds the Plotly `Scene` JSON into a `<div>` rendered by the same Plotly client code the
  graphics view uses. Cross-links become `<a href="/help/name">`. KaTeX + highlight.js assets
  are self-hosted under `web/` (no CDN), consistent with the "self-contained binary" goal.

---

## 7. `register_doc!` macro (P1)

Placed next to each builtin registration. Example for `cos`:

```rust
register_doc! {
    name: "cos",
    aliases: [],
    section: "mathfunctions",
    summary: "Trigonometric cosine of the argument.",
    body: r#"
## Usage
Computes `cos(x)` for an n-dimensional numeric array `x`. Integer inputs are
promoted to `double`. Output has the same size and type as `x`.

```text
y = cos(x)
```

## Function Internals
$$ \cos x \equiv \sum_{n=0}^{\infty} \frac{(-1)^n x^{2n}}{(2n)!} $$

## Example
```fm-exec:figure
x = linspace(0,1);
plot(x,cos(2*pi*x))
```
"#,
}
```

Contract:

- Expands to an `inventory::submit!` of a `DocEntry` with `source: RustBuiltin { krate, module }`
  filled from `env!("CARGO_PKG_NAME")` and `module_path!()` (file stem).
- `aliases` defaults to `[]`, may be omitted.
- The macro does **not** parse `body` at compile time (kept cheap); the dialect parser runs
  in `docgen` and in `--check` (so malformed bodies fail CI, not every build).
- A parallel `register_section!{ id, title, summary }` for sections.
- Toolbox `.m` doc blocks are registered by a small generated table (P6/P7 emit it) with
  `source: Toolbox { path }`, since `.m` files cannot host a Rust macro.

Open P1 mechanics (decide & document in P1, within this contract): whether `register_doc!`
lives in `fm-doc` and is re-exported, and how the collector binary in P3 forces linkage of
all `inventory::submit!` sites (typically a `register_all()` that references each builtin
crate, or building docgen against the full `fm` binary).

---

## 8. Standing gates (every code phase)

`cargo build` · `cargo clippy -- -D warnings` · `cargo fmt --check` · phase tests ·
(where relevant) the conformance suite. The main agent **re-verifies** these independently —
sub-agents have over-claimed before (see `PROGRESS.md` process note). `docgen --check` joins
the gate set once P3 lands.

---

## 9. Phase → contract crosswalk

| Phase | Builds | Must satisfy |
|-------|--------|--------------|
| P1 | `fm-doc`: model, `register_doc!`, dialect parser, `.doc`/`.m` parse helpers | §2, §3, §7 |
| P2 | fragment capture in `fm-cli` | §4 |
| P3 | `xtask` + `docgen` | §5 |
| P4 | browser `/help` | §6.5 |
| P5 | `help`/`helpwin` builtins + terminal render | §6.1–6.4 |
| P6 | `migrate-docs` converter | §1 mapping → §3 dialect |
| P7 | bulk migration, per section | §1, §3, §7; regen fragments (§4) |
| P8 | integration, CI `--check`, contributor note | §5, §8 |
