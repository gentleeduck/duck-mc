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
|   |- overview.md            (engine + crate boundaries)
|   |- system-overview.md     (full pipeline tour, ex `docs/architecture.md`)
|   |- pipeline.md            (per-stage walkthrough)
|   |- data-flow.md
|   |- caching.md
|   |- compiler-gaps.md       (parity vs velite / mdx-js)
|   |- benchmarks.md          (raw numbers + reproductions)
|   |- native-path-perf.md    (in-process compile cost analysis)
|   |- optimizations.md       (notes on phase-3+ wins)
|   |- perf-budget.md         (where time goes, ranked)
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
|- dmc-sidecar/             Node helper for foreign remark/rehype plugins
|                              (incl. `perf.md` - sidecar wall-clock cost)
`- articles/                long-form write-ups
    `- rust-mdx-compiler-vs-velite.md
```

## Per-crate doc -> source map

Every per-crate folder under `dmc-docs/` mirrors the workspace
crate of the same name. To jump from a doc page to the code that
backs it:

| doc folder                           | source crate                             | quick links                             |
| ------------------------------------ | ---------------------------------------- | --------------------------------------- |
| `dmc-docs/dmc-lexer/`                | `dmc-lexer/src/`                         | `lib.rs`, `lexers/{code,jsx,...}.rs`    |
| `dmc-docs/dmc-parser/`               | `dmc-parser/src/`                        | `lib.rs`, `block.rs`, `inline.rs`, `jsx.rs` |
| `dmc-docs/dmc-highlight/`            | `dmc-highlight/src/`                     | `lib.rs` (syntect bundle, theme list)   |
| `dmc-docs/dmc-transform/`            | `dmc-transform/src/`                     | `pipeline.rs`, `builtin/{pretty_code,mermaid,...}.rs` |
| `dmc-docs/dmc-codegen/`              | `dmc-codegen/src/`                       | `mdx.rs`, `html.rs`                     |
| `dmc-docs/dmc-diagnostic/`           | `dmc-diagnostic/src/`                    | `lib.rs`, `metadata.rs`                 |
| `dmc-docs/dmc-schema/`               | `dmc-schema/src/`                        | `lib.rs`                                |
| `dmc-docs/dmc-core/`                 | `dmc-core/src/`                          | `engine/{mod,collection,compile,cache}.rs` |
| `dmc-docs/dmc-napi/`                 | `dmc-napi/`                              | `mod.ts`, `src/lib.rs`                  |
| `dmc-docs/dmc-sidecar/`              | `dmc-sidecar/src/`                       | `index.mjs`, plugin-protocol files      |

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
