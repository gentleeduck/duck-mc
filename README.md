# dmc

`dmc` is a Rust MDX compiler with a velite-shaped TypeScript API. This repo currently contains the native Rust pipeline, the `@duck/md` NAPI package, an optional Node sidecar for JS remark/rehype plugins, and multiple example apps.

## Current state

- Native Rust pipeline for lexing, parsing, transforms, code generation, schema validation, and collection builds.
- Velite-style TypeScript helpers in `@duck/md`: `defineConfig`, `defineCollection`, `defineSchema`, `definePlugin`, and `s.*`.
- Native default features for pretty code, math, emoji, bare URL autolinks, heading autolinks, code imports, and npm command tabs.
- Optional `@duck/md-sidecar` package for configs that still need JS plugin execution.

## Workspace crates

| Crate | Role |
| --- | --- |
| `dmc-diagnostic` | Shared diagnostic codes and source metadata. |
| `dmc-lexer` | Tokenizer for MDX, JSX, and GFM-style syntax. |
| `dmc-parser` | Typed AST parser for block, inline, JSX, and table nodes. |
| `dmc-highlight` | Bundled syntect-based syntax highlighting assets and helpers. |
| `dmc-transform` | Native transform pipeline and built-in transformers. |
| `dmc-codegen` | HTML and MDX body emitters. |
| `dmc-schema` | Velite-style schema builders and markdown-aware fields. |
| `dmc-core` | Compile/build engine, CLI, watch mode, and collection output. |
| `dmc-napi` | NAPI bindings and the `@duck/md` package surface. |

## JS packages in this repo

- `dmc-napi/` -> `@duck/md`
- `dmc-sidecar/` -> `@duck/md-sidecar`

`dmc-sidecar` is in the repo, but it is not part of the Rust workspace in `Cargo.toml`.

## Examples

- [`examples/web/`](examples/web/) - Vite + React demo that renders the compiled `body` string at runtime.
- [`examples/nextjs/`](examples/nextjs/) - Next.js App Router demo that builds content with `@duck/md`.
- [`examples/nextjs-velite/`](examples/nextjs-velite/) - velite version of the same style of demo for side-by-side comparison.
- [`examples/acme-docs/`](examples/acme-docs/) - larger docs-site-style Next.js example.
- [`examples/samples/`](examples/samples/) - shared MDX fixtures and architecture samples.
- [`examples/COMPARISON.md`](examples/COMPARISON.md) - notes for the dmc vs velite demo setup.

## Docs

- [`docs/`](docs/) - architecture notes, benchmarks, migration notes, and performance writeups.
- [`dmc-docs/`](dmc-docs/) - crate-level documentation.

## Build and test

```sh
pnpm install
cargo build
cargo test --workspace
pnpm --filter @duck/md run build
```

## Run examples

```sh
pnpm --filter dmc-web dev
pnpm --filter dmc-nextjs dev
pnpm --filter velite-nextjs dev
pnpm --filter acme-docs dev
```

## CLI

```sh
cargo run -p dmc-core --bin dmc -- build --config dmc.config.ts
cargo run -p dmc-core --bin dmc -- dev --config dmc.config.ts
cargo run -p dmc-core --bin dmc -- compile path/to/file.mdx
```

## Migrating from velite

```diff
- import { defineConfig, s } from 'velite'
+ import { defineConfig, s } from '@duck/md'
```

See [`docs/migrating-from-velite.md`](docs/migrating-from-velite.md) for the current compatibility notes.

## License

MIT.
