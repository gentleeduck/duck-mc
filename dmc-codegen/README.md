# dmc-codegen

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
