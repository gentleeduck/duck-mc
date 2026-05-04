# Testing

How to test transformers, the engine, and end-to-end output.

## Layers

| layer | test framework | location |
|-------|---------------|----------|
| lexer | `cargo test` | `dmc-lexer/tests/*.rs` |
| parser | `cargo test` | `dmc-parser/tests/*.rs` |
| transform | `cargo test` | `dmc-transform/tests/*.rs` |
| codegen | `cargo test` | `dmc-codegen/tests/*.rs` |
| core (engine) | `cargo test` (with `insta` for snapshots) | `dmc-core/tests/*.rs` |
| napi | manual + `pnpm test` if added | `dmc-napi/test-*.mts` |

## Run all

```bash
cargo test --workspace --features pretty-code
```

`--features pretty-code` opts in to the syntect transformer; without
it, transformers requiring it are skipped (cfg gate).

## Run one crate

```bash
cargo test -p dmc-parser
```

## Run one test

```bash
cargo test -p dmc-parser -- nested_blockquote
```

Substring match against test names.

## Lexer test pattern

```rust
use dmc_lexer::{Lexer, token::TokenKind};
use std::sync::Arc;
use dmc_diagnostic::metadata::{Origin, SourceMeta};
use duck_diagnostic::DiagnosticEngine;

#[test]
fn lexes_bold_marker() {
    let meta = Arc::from(SourceMeta {
        path: Arc::from("<t>"),
        version: 0,
        origin: Origin::Inline("<t>"),
    });
    let mut e = DiagnosticEngine::new();
    let mut l = Lexer::new("**hi**", meta, &mut e);
    let _ = l.scan_tokens();
    let kinds: Vec<_> = l.tokens.iter().map(|t| t.kind.clone()).collect();
    assert!(matches!(kinds[0], TokenKind::Bold(2)));
}
```

## Parser test pattern

```rust
use dmc_parser::{parse, ast::*};

#[test]
fn parses_nested_list() {
    let d = parse("- one\n  - two\n");
    let ul = match &d.children[0] {
        Node::List(l) if !l.ordered => l,
        _ => panic!("expected unordered list"),
    };
    let inner = ul.children[0].children().iter()
        .find_map(|n| match n { Node::List(l) => Some(l), _ => None })
        .expect("nested list");
    assert_eq!(inner.children.len(), 1);
}
```

## Transformer test pattern

```rust
use dmc_parser::parse;
use dmc_transform::Pipeline;

#[test]
fn passes_through_when_no_match() {
    let mut doc = parse("plain paragraph");
    Pipeline::new().add(MyPass).run_silent(&mut doc);
    // assert nothing changed
}
```

`run_silent` provides synthetic `Origin::Inline("<test>")` meta and
a throwaway `DiagnosticEngine`. Use this everywhere except when the
test explicitly inspects diagnostics.

## Codegen test pattern

```rust
use dmc_codegen::render_html;
use dmc_parser::parse;
use pretty_assertions::assert_eq;

#[test]
fn h1_wraps_with_id() {
    let html = render_html(&parse("# Hello"));
    assert_eq!(html, r#"<h1 id="hello">Hello</h1>"#);
}
```

`pretty_assertions::assert_eq` produces colourful diffs on failure.

## Snapshot tests

`dmc-core` uses `insta` for output snapshots:

```rust
use insta::assert_snapshot;

#[test]
fn kitchen_sink() {
    let html = render_html(&parse(KITCHEN_SINK_MDX));
    assert_snapshot!(html);
}
```

First run writes `snapshots/<name>.snap`. Future runs diff. To
review:

```bash
cargo insta review
```

Approve / reject; commit the `.snap` files.

## Diagnostic testing

```rust
use dmc_diagnostic::Code;
use dmc_parser::parse;

#[test]
fn warns_on_clamped_heading() {
    let _ = parse("####### too many");
    // engine is captured inside parse(); use the lexer/parser directly to inspect
}
```

For tests that need to read the diag engine, use the lower-level API
(see `dmc-parser/examples.md`).

## End-to-end (engine)

```rust
use std::path::Path;
use dmc::Engine;
use dmc::engine::config::EngineConfig;
use dmc::engine::collection::Collection;
use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;

#[test]
fn engine_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("content/docs")).unwrap();
    std::fs::write(tmp.path().join("content/docs/hi.mdx"), "---\ntitle: hi\n---\n\n# Hi\n").unwrap();

    let cfg = EngineConfig {
        root: tmp.path().join("content"),
        output_dir: tmp.path().join(".out"),
        cache_enabled: false,
        collections: vec![Collection {
            name: "doc".into(),
            pattern: "docs/**/*.mdx".into(),
            base_dir: tmp.path().join("content"),
            schema: None,
            single: false,
        }],
        ..Default::default()
    };

    let mut diag = DiagnosticEngine::<Code>::new();
    Engine::run(&cfg, None, &mut diag).unwrap();

    let json = std::fs::read_to_string(tmp.path().join(".out/doc.json")).unwrap();
    assert!(json.contains("\"title\": \"hi\""));
}
```

`tempfile::tempdir()` for isolation. Set `cache_enabled: false` so
the test does not pollute a shared cache dir.

## Bench (perf regression)

```bash
cargo run --release -p dmc-core --features pretty-code --example bench
```

Output: `dmc-core/tmp/bench.json`. Compare across PRs by checking
the file in.

## Coverage

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --workspace --features pretty-code -o lcov
```

LCOV report works with most coverage viewers (codecov, coveralls,
VS Code coverage extension).

## CI

`cargo test --workspace --features pretty-code` is the headline.
Add `cargo clippy --workspace --all-features -- -D warnings` and
`cargo fmt --check` to catch lint / format drift before merge.

## Common gotchas

- Don't call `cargo test --release` for parser tests; debug build is
  fast enough and catches more (unwrap panics, etc).
- Don't share `DiagnosticEngine` across threads in a test; each
  `par_iter` thread should have its own.
- Don't write absolute paths in fixtures; use `tempfile` so tests
  pass on every platform.
