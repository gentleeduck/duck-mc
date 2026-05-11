# dmc-lexer

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
