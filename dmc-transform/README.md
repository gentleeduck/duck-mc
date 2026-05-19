<p align="center">
  <img src="../public/logo-dark.svg" alt="dmc-transform" width="120"/>
</p>

<h1 align="center">dmc-transform</h1>

<p align="center">
  Transform pipeline and built-in transformers for the dmc compiler.
</p>

<p align="center">
  <a href="../LICENSE">MIT</a> -
  <a href="../CHANGELOG.md">Changelog</a> -
  <a href="../CONTRIBUTING.md">Contributing</a> -
  <a href="https://crates.io/crates/dmc-transform">crates.io</a> -
  <a href="https://docs.rs/dmc-transform">docs.rs</a>
</p>

<p align="center">
  <a href="https://crates.io/crates/dmc-transform"><img src="https://img.shields.io/crates/v/dmc-transform.svg" alt="crates.io"/></a>
  <a href="https://docs.rs/dmc-transform"><img src="https://docs.rs/dmc-transform/badge.svg" alt="docs.rs"/></a>
  <a href="../LICENSE"><img src="https://img.shields.io/crates/l/dmc-transform.svg" alt="MIT"/></a>
</p>

---

AST-to-AST transform pipeline for parsed Markdown/MDX documents.

## Public API

- `Pipeline::new()`
- `Pipeline::with_defaults()`
- `Pipeline::with_defaults_for(&PipelineConfig)`
- `Pipeline::run(...)` and `Pipeline::run_silent(...)`
- `Transformer`, `Visitor`, `walk_root`
- Built-ins such as `AssignHeadingIds`, `AutolinkHeadings`,
  `BareUrlAutolink`, `CodeImport`, and feature-gated transformers
  like `PrettyCode`, `Math`, `Emoji`, and `Mermaid`

## Compliance

- The transform crate runs on top of the parser AST produced by the
  CommonMark `652/652` and GFM `670/670` pipeline.
- Transformer behavior is validated in crate-local tests; syntax
  compliance itself is owned by the parser/codegen spec suites.

## Spec suites

```sh
cargo test -p dmc-parser --test commonmark_spec commonmark_spec_no_regression -- --nocapture
cargo test -p dmc-parser --test gfm_spec gfm_spec_no_regression -- --nocapture
```
