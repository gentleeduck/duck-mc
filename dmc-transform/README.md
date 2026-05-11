# dmc-transform

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
