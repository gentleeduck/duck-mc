# dmc-parser

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
