<p align="center">
  <img src="../public/logo-dark.svg" alt="dmc-codegen" width="120"/>
</p>

<h1 align="center">dmc-codegen</h1>

<p align="center">
  HTML and MDX body emitters for the dmc compiler.
</p>

<p align="center">
  <a href="../LICENSE">MIT</a> -
  <a href="../CHANGELOG.md">Changelog</a> -
  <a href="../CONTRIBUTING.md">Contributing</a> -
  <a href="https://crates.io/crates/dmc-codegen">crates.io</a> -
  <a href="https://docs.rs/dmc-codegen">docs.rs</a>
</p>

<p align="center">
  <a href="https://crates.io/crates/dmc-codegen"><img src="https://img.shields.io/crates/v/dmc-codegen.svg" alt="crates.io"/></a>
  <a href="https://docs.rs/dmc-codegen"><img src="https://docs.rs/dmc-codegen/badge.svg" alt="docs.rs"/></a>
  <a href="../LICENSE"><img src="https://img.shields.io/crates/l/dmc-codegen.svg" alt="MIT"/></a>
</p>

---

HTML and MDX emitters for the `dmc` document AST.

## Public API

- `render_html(&Document) -> String`
- `render_html_with(&Document, RenderOptions) -> String`
- `render_mdx_body(&Document) -> String`
- `HtmlEmitter`, `MdxBodyEmitter`, `Walker`, `NodeSink`

## Dialect flags

- `RenderOptions::gfm_disallowed_raw_html`

## Compliance

- The parser + HTML renderer path passes CommonMark `652/652`.
- The parser + HTML renderer path passes GFM `670/670`.

## Spec suites

```sh
cargo test -p dmc-parser --test commonmark_spec commonmark_spec_no_regression -- --nocapture
cargo test -p dmc-parser --test gfm_spec gfm_spec_no_regression -- --nocapture
```
