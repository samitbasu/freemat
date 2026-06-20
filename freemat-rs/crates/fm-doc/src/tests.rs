//! Unit tests for the `fm-doc` P1 deliverable.

use crate::{
    BlockKind, DocEntry, Registry, Resolution, SectionEntry, SourceKind, damerau_levenshtein,
    fragment_script_from_body, parse_body,
};

// --- inventory collection (exercises register_doc!/register_section!) -------

crate::register_section! {
    id: "testsec",
    title: "Test Section",
    summary: "A section used only by fm-doc's own tests.",
}

crate::register_doc! {
    name: "zzz_test_alpha",
    section: "testsec",
    summary: "Alpha test entry (no aliases).",
    body: "## Usage\nalpha\n",
}

crate::register_doc! {
    name: "zzz_test_beta",
    aliases: ["zzz_test_b", "zzz_test_bb"],
    section: "testsec",
    summary: "Beta test entry (with aliases).",
    body: "## Usage\nbeta\n",
}

#[test]
fn global_registry_collects_inventory_entries() {
    let reg = Registry::global();
    let names: Vec<&str> = reg.in_section("testsec").map(|e| e.name).collect();
    assert!(names.contains(&"zzz_test_alpha"), "got {names:?}");
    assert!(names.contains(&"zzz_test_beta"), "got {names:?}");
    assert!(reg.section("testsec").is_some());
}

#[test]
fn global_registry_module_stem_is_file_stem() {
    let reg = Registry::global();
    let e = reg
        .iter()
        .find(|e| e.name == "zzz_test_alpha")
        .expect("entry present");
    match e.source {
        SourceKind::RustBuiltin { krate, module } => {
            assert_eq!(krate, "fm-doc");
            // module_path!() here is "fm_doc::tests" -> stem "tests".
            assert_eq!(module, "tests");
        }
        SourceKind::Toolbox { .. } => panic!("expected RustBuiltin"),
    }
}

#[test]
fn global_aliases_resolve() {
    let reg = Registry::global();
    match reg.resolve("zzz_test_b") {
        Resolution::Exact(e) => assert_eq!(e.name, "zzz_test_beta"),
        other => panic!("expected alias exact, got {other:?}"),
    }
}

// --- helpers ---------------------------------------------------------------

fn entry(name: &'static str, section: &'static str) -> DocEntry {
    DocEntry {
        name,
        aliases: &[],
        section,
        summary: "s",
        body_md: "",
        source: SourceKind::RustBuiltin {
            krate: "test",
            module: "m",
        },
    }
}

fn entry_aliased(
    name: &'static str,
    aliases: &'static [&'static str],
    section: &'static str,
) -> DocEntry {
    DocEntry {
        name,
        aliases,
        section,
        summary: "s",
        body_md: "",
        source: SourceKind::RustBuiltin {
            krate: "test",
            module: "m",
        },
    }
}

fn section(id: &'static str) -> SectionEntry {
    SectionEntry {
        id,
        title: "T",
        summary: "s",
    }
}

// --- deterministic order ---------------------------------------------------

#[test]
fn entries_sorted_by_section_then_name() {
    let entries = vec![
        entry("sin", "trig"),
        entry("abs", "math"),
        entry("cos", "trig"),
        entry("zeros", "math"),
    ];
    let reg = Registry::try_build(entries, vec![section("math"), section("trig")]).expect("no dup");
    let order: Vec<(&str, &str)> = reg.iter().map(|e| (e.section, e.name)).collect();
    assert_eq!(
        order,
        vec![
            ("math", "abs"),
            ("math", "zeros"),
            ("trig", "cos"),
            ("trig", "sin"),
        ]
    );
}

#[test]
fn building_twice_yields_identical_order() {
    let mk = || {
        vec![
            entry("cos", "trig"),
            entry("abs", "math"),
            entry("sin", "trig"),
        ]
    };
    let a = Registry::try_build(mk(), vec![]).unwrap();
    let b = Registry::try_build(mk(), vec![]).unwrap();
    let oa: Vec<_> = a.iter().map(|e| (e.section, e.name)).collect();
    let ob: Vec<_> = b.iter().map(|e| (e.section, e.name)).collect();
    assert_eq!(oa, ob);
}

#[test]
fn in_section_is_name_sorted() {
    let reg = Registry::try_build(
        vec![
            entry("sin", "trig"),
            entry("cos", "trig"),
            entry("tan", "trig"),
        ],
        vec![],
    )
    .unwrap();
    let names: Vec<&str> = reg.in_section("trig").map(|e| e.name).collect();
    assert_eq!(names, vec!["cos", "sin", "tan"]);
}

// --- duplicate-name detection ----------------------------------------------

#[test]
fn duplicate_name_is_error() {
    let entries = vec![entry("cos", "trig"), entry("cos", "math")];
    let err = Registry::try_build(entries, vec![]).unwrap_err();
    assert_eq!(err.name, "cos");
    // Both source locations are surfaced.
    let msg = err.to_string();
    assert!(msg.contains("cos"), "{msg}");
}

#[test]
fn distinct_names_ok() {
    assert!(Registry::try_build(vec![entry("a", "s"), entry("b", "s")], vec![]).is_ok());
}

// --- resolution ------------------------------------------------------------

#[test]
fn resolve_exact() {
    let reg = Registry::try_build(vec![entry("cos", "trig")], vec![]).unwrap();
    match reg.resolve("cos") {
        Resolution::Exact(e) => assert_eq!(e.name, "cos"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn resolve_alias() {
    let reg = Registry::try_build(vec![entry_aliased("angle", &["arg"], "trig")], vec![]).unwrap();
    match reg.resolve("arg") {
        Resolution::Exact(e) => assert_eq!(e.name, "angle"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn resolve_case_insensitive() {
    let reg = Registry::try_build(vec![entry("cos", "trig")], vec![]).unwrap();
    match reg.resolve("COS") {
        Resolution::Exact(e) => assert_eq!(e.name, "cos"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn resolve_did_you_mean_edit_distance() {
    let reg =
        Registry::try_build(vec![entry("cos", "trig"), entry("sin", "trig")], vec![]).unwrap();
    match reg.resolve("cs") {
        Resolution::Suggestions(s) => assert!(s.contains(&"cos"), "{s:?}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn resolve_did_you_mean_prefix() {
    let reg = Registry::try_build(
        vec![entry("linspace", "array"), entry("logspace", "array")],
        vec![],
    )
    .unwrap();
    match reg.resolve("lin") {
        Resolution::Suggestions(s) => assert!(s.contains(&"linspace"), "{s:?}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn resolve_none_when_nothing_close() {
    let reg = Registry::try_build(vec![entry("cos", "trig")], vec![]).unwrap();
    assert!(matches!(reg.resolve("qqqqqzzzzz"), Resolution::None));
}

#[test]
fn suggestions_capped_at_eight() {
    let names = [
        "aaa", "aab", "aac", "aad", "aae", "aaf", "aag", "aah", "aai", "aaj",
    ];
    let entries: Vec<DocEntry> = names.iter().map(|n| entry(n, "s")).collect();
    let reg = Registry::try_build(entries, vec![]).unwrap();
    match reg.resolve("aaz") {
        Resolution::Suggestions(s) => assert!(s.len() <= 8, "{}", s.len()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn damerau_levenshtein_transposition() {
    assert_eq!(damerau_levenshtein("ab", "ba"), 1);
    assert_eq!(damerau_levenshtein("cos", "cos"), 0);
    assert_eq!(damerau_levenshtein("", "abc"), 3);
    assert_eq!(damerau_levenshtein("kitten", "sitting"), 3);
}

// --- block classification --------------------------------------------------

#[test]
fn classify_all_block_kinds() {
    let body = r#"
## Heading

```text
verbatim here
```

```fm
y = cos(x)
```

```fm-exec
a = 1
```

```fm-exec:figure
plot(1)
```

```fm-file:hello.m
function hello
```

```rust
let x = 1;
```
"#;
    let parsed = parse_body(body);
    let kinds: Vec<&BlockKind> = parsed.blocks.iter().map(|b| &b.kind).collect();
    assert_eq!(parsed.blocks.len(), 6, "{kinds:?}");
    assert_eq!(kinds[0], &BlockKind::Text);
    assert_eq!(kinds[1], &BlockKind::Fm);
    assert_eq!(kinds[2], &BlockKind::FmExec { expect_errors: 0 });
    assert_eq!(kinds[3], &BlockKind::FmExecFigure { expect_errors: 0 });
    assert_eq!(
        kinds[4],
        &BlockKind::FmFile {
            name: "hello.m".to_string()
        }
    );
    assert_eq!(
        kinds[5],
        &BlockKind::Other {
            info: "rust".to_string()
        }
    );
}

#[test]
fn heading_extracted() {
    let parsed = parse_body("## Usage\ntext\n\n## Example\nmore\n");
    let texts: Vec<&str> = parsed.headings.iter().map(|h| h.text.as_str()).collect();
    assert_eq!(texts, vec!["Usage", "Example"]);
}

// --- # errors: parsing -----------------------------------------------------

#[test]
fn errors_line_parsed_and_stripped() {
    let body = "```fm-exec\n# errors: 2\nx = bogus\n```\n";
    let parsed = parse_body(body);
    assert_eq!(parsed.blocks.len(), 1);
    assert_eq!(
        parsed.blocks[0].kind,
        BlockKind::FmExec { expect_errors: 2 }
    );
    assert_eq!(parsed.blocks[0].body, "x = bogus\n");
}

#[test]
fn errors_line_defaults_to_zero() {
    let body = "```fm-exec\nx = 1\n```\n";
    let parsed = parse_body(body);
    assert_eq!(
        parsed.blocks[0].kind,
        BlockKind::FmExec { expect_errors: 0 }
    );
    assert_eq!(parsed.blocks[0].body, "x = 1\n");
}

#[test]
fn errors_line_on_figure_block() {
    let body = "```fm-exec:figure\n# errors: 1\nplot(bad)\n```\n";
    let parsed = parse_body(body);
    assert_eq!(
        parsed.blocks[0].kind,
        BlockKind::FmExecFigure { expect_errors: 1 }
    );
}

// --- fragment-script extraction --------------------------------------------

#[test]
fn fragment_script_multi_block_state() {
    let body = r#"
```fm-file:a.m
contents A
```

```fm-exec
# errors: 1
x = 1
```

```fm-file:b.m
contents B
```

```fm-exec:figure
plot(x)
```

```fm-exec
# errors: 2
y = 2
```
"#;
    let script = fragment_script_from_body(body).expect("has fragments");
    // fm-file ordering preserved.
    assert_eq!(
        script.files,
        vec![
            ("a.m".to_string(), "contents A\n".to_string()),
            ("b.m".to_string(), "contents B\n".to_string()),
        ]
    );
    // One input per exec block, in order.
    assert_eq!(script.inputs.len(), 3);
    assert_eq!(script.inputs[0], "x = 1\n");
    assert_eq!(script.inputs[1], "plot(x)\n");
    assert_eq!(script.inputs[2], "y = 2\n");
    // expect_errors summed across exec blocks (1 + 0 + 2).
    assert_eq!(script.expect_errors, 3);
    // want_figure set by the fm-exec:figure block.
    assert!(script.want_figure);
}

#[test]
fn fragment_script_none_when_no_executable() {
    let body = "## Usage\n```text\njust text\n```\n```fm\ndisplay only\n```\n";
    assert!(fragment_script_from_body(body).is_none());
}

#[test]
fn fragment_script_files_only() {
    let body = "```fm-file:x.m\nhi\n```\n";
    let script = fragment_script_from_body(body).expect("file present");
    assert_eq!(script.inputs.len(), 0);
    assert_eq!(script.files.len(), 1);
    assert!(!script.want_figure);
}

// --- math & cross-link recognition -----------------------------------------

#[test]
fn inline_and_display_math_recognized() {
    let body = "Some $a + b$ inline and $$\\int_0^1 x\\,dx$$ display.\n";
    let parsed = parse_body(body);
    assert_eq!(parsed.math.len(), 2);
    assert!(!parsed.math[0].display);
    assert_eq!(parsed.math[0].tex, "a + b");
    assert!(parsed.math[1].display);
    assert_eq!(parsed.math[1].tex, "\\int_0^1 x\\,dx");
}

#[test]
fn math_inside_code_is_ignored() {
    // `$` inside fenced and inline code must not be recognized as math.
    let body = "Real $x$ math.\n\n```text\ncost = $5 and $10\n```\n\nInline `$not$ math` here.\n";
    let parsed = parse_body(body);
    assert_eq!(parsed.math.len(), 1, "{:?}", parsed.math);
    assert_eq!(parsed.math[0].tex, "x");
}

#[test]
fn cross_links_recognized() {
    let body = "See [the sine](sin) and [external](https://example.com).\n";
    let parsed = parse_body(body);
    assert_eq!(parsed.links.len(), 1, "{:?}", parsed.links);
    assert_eq!(parsed.links[0].text, "the sine");
    assert_eq!(parsed.links[0].target, "sin");
}

#[test]
fn module_stem_helper() {
    assert_eq!(crate::module_stem("fm_builtins::elementary"), "elementary");
    assert_eq!(crate::module_stem("a::b::c"), "c");
    assert_eq!(crate::module_stem("single"), "single");
}
