# dmc Documentation

Reference docs for the crates currently documented under `dmc-docs`.

## Layout

```text
dmc-docs/
|- dmc-codegen/    AST to HTML / MDX body emitters
|- dmc-lexer/      tokeniser
|- dmc-parser/     AST builder
|- dmc-schema/     frontmatter validation
`- dmc-transform/  AST transformers (gfm, math, emoji, ...)
```

More crate folders can be added as they land.

## Read order

1. Open the crate folder you need.
2. Start with its `README.md`.
3. Use `api.md`, `examples.md`, and topic files when present.

## Conventions

- Code blocks use the language hint (`rust`, `ts`, `mdx`, `bash`).
- Diagrams use Mermaid when a flowchart helps; ASCII when smaller.
- Comments and prose are terse. No filler.
- Every public type / function in `api.md` is annotated with its
  location (`crate::module::Item`).

## Status

Workspace version `0.1.0`. Schema may break across minor versions
until 1.0. Cache format is keyed on the dmc version, so version bumps
auto-invalidate.
