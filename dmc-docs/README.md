<p align="center">
  <img src="../public/logo-dark.svg" alt="dmc" width="96"/>
</p>

# dmc Documentation

Per-crate reference + architecture + user guides for `@gentleduck/md`.

## Layout

```text
dmc-docs/
|- README.md                root index (this file)
|- architecture/            cross-crate diagrams + data flow
|   |- overview.md
|   |- pipeline.md
|   |- data-flow.md
|   |- caching.md
|   |- error-handling.md
|   |- threading-model.md
|   |- feature-gates.md
|   `- output-formats.md
|- guides/                  user-facing walkthroughs
|   |- getting-started.md
|   |- configuration.md
|   |- plugins.md
|   |- caching.md
|   |- performance.md
|   |- migrating-from-velite.md
|   |- nextjs.md
|   |- cli-reference.md
|   |- api-cheatsheet.md
|   |- contributing.md
|   `- troubleshooting.md
|- dmc-lexer/               tokeniser
|- dmc-parser/              AST builder
|- dmc-highlight/           syntect bundle (themes + grammars)
|- dmc-transform/           AST transformers (gfm, math, emoji, ...)
|- dmc-codegen/             AST to HTML / MDX body emitters
|- dmc-diagnostic/          shared diagnostic codes + spans
|- dmc-schema/              frontmatter validation
|- dmc-core/                engine (compile + cache + sidecar)
|- dmc-napi/                JS bindings (@gentleduck/md npm package)
`- dmc-sidecar/             Node helper for foreign remark/rehype plugins
```

## Read order

1. [`architecture/overview.md`](architecture/overview.md) - what every
   piece does and how they fit.
2. [`guides/getting-started.md`](guides/getting-started.md) - install
   and first build.
3. The per-crate folder you care about. Every crate folder has at
   least `README.md`, `api.md`, often `examples.md`.

## Conventions

- Code blocks use the language hint (`rust`, `ts`, `mdx`, `bash`).
- Diagrams use Mermaid when a flowchart helps; ASCII when smaller.
- Comments and prose are terse. No filler.
- Every public type / function in `api.md` is annotated with its
  location (`crate::module::Item`).
- ASCII only. No em-dash, en-dash, ellipsis glyph, curly quotes,
  arrows, or comparison glyphs in unicode form. Use `-`, `'`, `"`,
  `...`, `->`, `<-`, `>=`, `<=`, `!=`, `*`, `.`.

## Status

Workspace version `0.1.0`. Schema may break across minor versions
until 1.0. Cache format keyed on the dmc version, so version bumps
auto-invalidate.
