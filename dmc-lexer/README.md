<p align="center">
  <img src="../public/logo-dark.svg" alt="dmc-lexer" width="120"/>
</p>

<h1 align="center">dmc-lexer</h1>

<p align="center">
  MDX, JSX, and GFM tokenizer for the dmc compiler.
</p>

<p align="center">
  <a href="../LICENSE">MIT</a> -
  <a href="../CHANGELOG.md">Changelog</a> -
  <a href="../CONTRIBUTING.md">Contributing</a> -
  <a href="https://crates.io/crates/dmc-lexer">crates.io</a> -
  <a href="https://docs.rs/dmc-lexer">docs.rs</a>
</p>

<p align="center">
  <a href="https://crates.io/crates/dmc-lexer"><img src="https://img.shields.io/crates/v/dmc-lexer.svg" alt="crates.io"/></a>
  <a href="https://docs.rs/dmc-lexer"><img src="https://docs.rs/dmc-lexer/badge.svg" alt="docs.rs"/></a>
  <a href="../LICENSE"><img src="https://img.shields.io/crates/l/dmc-lexer.svg" alt="MIT"/></a>
</p>

---

Flat-token lexer for Markdown, GFM, MDX, JSX, and frontmatter.

## Public API

- `dmc_lexer::Lexer::new(source, meta, &mut diag)`
- `Lexer::scan_tokens()`
- `dmc_lexer::token::{Token, TokenKind}`

The lexer owns byte offsets and logical columns; the parser owns
structure and higher-level disambiguation.

## Compliance

- The lexer feeds the parser/codegen path that passes CommonMark
  `652/652` and GFM `670/670`.
- Direct lexer regression coverage also lives under `cargo test -p dmc-lexer`.

## Spec suites

```sh
cargo test -p dmc-parser --test commonmark_spec commonmark_spec_no_regression -- --nocapture
cargo test -p dmc-parser --test gfm_spec gfm_spec_no_regression -- --nocapture
```
