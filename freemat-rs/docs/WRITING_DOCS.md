# Writing & Maintaining Help Docs

This is the contributor guide for the Rust-native help system. The full design
contract is [`HELP_SYSTEM.md`](./HELP_SYSTEM.md); this is the day-to-day how-to.

## TL;DR workflow

```
edit a register_doc!{…}  →  cargo xtask docgen  →  cargo build  →  run `fm`
```

**The footgun:** the captured fragment transcripts live in a *generated, compiled-in*
file (`crates/fm-doc/src/generated/fragments.rs`, `include!`d by `fm-doc`). After you
edit a doc you must run **`cargo xtask docgen`** to recapture its fragment, and then
**rebuild** so the new transcript is linked into `fm`/the server. If you skip `docgen`,
CI fails (`docgen --check`); if you skip the rebuild, the running binary still shows the
old transcript. Editing the doc body alone changes nothing the user sees until both run.

## Adding or editing a doc

Put a `register_doc!` next to the builtin (or in the section module under
`crates/fm-builtins/src/docs/`). Minimal shape:

```rust
fm_doc::register_doc! {
    name: "myfunc",          // canonical topic key, lowercase, globally unique
    aliases: ["mf"],         // optional; resolve to this topic
    section: "mathfunctions",// must match a register_section! id
    summary: "One-line summary shown in indexes and the help header.",
    body: r#"
## Usage
What it does. Inline code is `like this`.

```fm-exec
myfunc(3)
```
"#,
}
```

Sections are declared once with `register_section!{ id, title, summary }`. A duplicate
topic `name` (across the whole registry) is a hard error at startup — keep names unique.

## The markdown dialect (body_md)

CommonMark, plus (see `HELP_SYSTEM.md` §3 for the full table):

| You write | Rendered as |
|-----------|-------------|
| `` `code` `` | inline code |
| `## Usage` | section heading |
| ```` ```text ```` | verbatim block |
| ```` ```fm ```` | FreeMat source, highlighted, **not** executed |
| ```` ```fm-exec ```` | executed; transcript captured & shown |
| ```` ```fm-exec:figure ```` | executed **and** the resulting plot is captured |
| ```` ```fm-file:NAME ```` | declares an aux file for later `fm-exec` blocks |
| `$ x^2 $` / `$$ … $$` | inline / display math (KaTeX in the browser) |
| `[text](othertopic)` | cross-link to another help topic |

`fm-exec` blocks may start with `# errors: N` to declare an expected error count
(default 0). `docgen` fails if the actual count differs.

## Fragments: keep them deterministic

`docgen` runs every `fm-exec` block through the interpreter and stores the exact REPL
transcript. For `--check` to stay green on every machine, fragments must be
**reproducible**:

- RNG is seeded (`seed(1,0)`) before each fragment, so `rand`/`randn` are fine.
- **Avoid** wall-clock/timing builtins (`tic`/`toc`/`clock`/`now`/`date`/…) and
  environment-dependent ones (`pwd`/`cd`/`dir`/`ls`/`tempname`/…) in executed blocks —
  their output varies per run/machine. Use a ```` ```fm ```` display block instead.
- Each fragment runs in a fresh temp working directory, so file-writing examples
  (`save`, `csvwrite`, …) are isolated and never pollute your checkout.

## Browser vs terminal

- `help <name>` → terminal text render (+ a `Browser:` URL, clickable via OSC 8 on a
  capable terminal). `help` lists sections; `help <section-id>` lists a section's topics.
- `helpwin <name>` → opens the rich browser page (`/help/<name>`): KaTeX math, highlighted
  code, and Plotly figures for `fm-exec:figure`.
- Math renders only in the browser; the terminal prints the TeX verbatim.

## The legacy migration (historical)

The bulk of today's corpus was machine-converted from FreeMat's Doxygen `.doc` pages by
`cargo xtask migrate-docs` (→ staging) and `cargo xtask migrate-place` (→ the
`fm-builtins/src/docs/` modules). `migrate-place` **probes** each fragment and keeps it
live only if it runs as declared in the current interpreter; otherwise it downgrades the
example to a display-only ```` ```fm ```` block.

**Re-enabling downgraded fragments:** as builtins mature, re-running `cargo xtask
migrate-place` auto-promotes any example that now runs cleanly back to a live `fm-exec`
block (then `docgen` captures its transcript). You can also hand-edit a specific topic's
`fm` block back to `fm-exec` and run `docgen` — if it runs deterministically, it stays.

## CI

`.github/workflows/ci.yml` runs fmt, clippy, build, test, **and `cargo xtask docgen
--check`**. The last one fails if the generated fragment DB is stale or any doc has an
unknown section, a dangling cross-link, or an `# errors:` mismatch. Run `cargo xtask
docgen` and commit the regenerated `fragments.rs` to fix.
