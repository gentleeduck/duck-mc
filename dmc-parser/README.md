<p align="center">
  <img src="../public/logo-dark.svg" alt="dmc-parser" width="120"/>
</p>

<h1 align="center">dmc-parser</h1>

<p align="center">
  Typed AST parser for the dmc MDX compiler.
</p>

<p align="center">
  <a href="../LICENSE">MIT</a> -
  <a href="../CHANGELOG.md">Changelog</a> -
  <a href="../CONTRIBUTING.md">Contributing</a> -
  <a href="https://crates.io/crates/dmc-parser">crates.io</a> -
  <a href="https://docs.rs/dmc-parser">docs.rs</a>
</p>

<p align="center">
  <a href="https://crates.io/crates/dmc-parser"><img src="https://img.shields.io/crates/v/dmc-parser.svg" alt="crates.io"/></a>
  <a href="https://docs.rs/dmc-parser"><img src="https://docs.rs/dmc-parser/badge.svg" alt="docs.rs"/></a>
  <a href="../LICENSE"><img src="https://img.shields.io/crates/l/dmc-parser.svg" alt="MIT"/></a>
</p>

---

Typed Markdown/MDX parser for the `dmc` AST.

## Public API

- `dmc_parser::parse(&str) -> Document`
- `dmc_parser::parse_with(&str, ParseOptions) -> Document`
- `dmc_parser::parse_inline_str(&str) -> Vec<Node>`
- `dmc_parser::Parser` for manual token-stream parsing
- `dmc_parser::ast`, `refs`, `Slugger`, `github_slugify`

## Dialect flags

- `ParseOptions::cm_strict_html_blocks`
- `ParseOptions::gfm_autolinks`
- `ParseOptions::legacy_gfm_emphasis`

## Compliance

- CommonMark: `652/652`
- GFM: `670/670`

The parser resolves reference links and footnotes in a two-pass parse,
and the end-to-end spec status is checked through `dmc-parser` +
`dmc-codegen`.

## Spec suites

```sh
cargo test -p dmc-parser --test commonmark_spec commonmark_spec_no_regression -- --nocapture
cargo test -p dmc-parser --test gfm_spec gfm_spec_no_regression -- --nocapture
```
