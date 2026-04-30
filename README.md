# dmc

Rust MDX compiler. Drop-in replacement for [velite](https://github.com/zce/velite).

- velite-shape JSON output (camelCase, frontmatter hoisted, typed `index.d.ts`)
- velite-parity `s.*` schema builder (string/number/boolean/array/object/enum/literal/union + raw/markdown/mdx/toc/metadata/excerpt/path/slug/unique/isodate/file/image)
- `defineConfig` accepts the same shape velite users already wrote
- TypeScript config support via `bun` or `node --import tsx`
- `dmc dev` watch mode
- `subheading-anchor` autolink class on every heading
- Native plugin pipeline (no Node child process for the common case)
- Optional Node sidecar (`@duck/md-sidecar`) for community remark/rehype plugins

## Migrating from velite

```diff
- import { defineConfig, s } from 'velite'
+ import { defineConfig, s } from '@duck/md'
```

Full migration guide: [`docs/migrating-from-velite.md`](docs/migrating-from-velite.md).

## Run

```sh
# build the whole workspace
cargo build

# build the napi binding for Node
cd dmc-napi && pnpm install && pnpm build

# run the React/Vite example
cd examples/web && pnpm install && pnpm dev
```

## CLI

```sh
dmc build --config dmc.config.ts          # one-shot
dmc dev   --config dmc.config.ts          # watch + rebuild
dmc build --strict --clean                    # fail on schema error, wipe output
dmc compile path/to/file.mdx                  # single-file dump to stdout
dmc init                                      # scaffold dmc.toml
```

## Workspace layout

| Crate               | Role                                                                                                                                                         |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `dmc-lexer`     | tokens (JSX boundary heuristic, GFM tables / strikethrough / tasklist, `<url>` autolinks)                                                                    |
| `dmc-parser`    | parser + AST (`pub mod ast`) — AST nodes live with the parser                                                                                                |
| `dmc-codegen`   | HTML emitter + MDX body emitter (`_createMdxContent` factory string)                                                                                         |
| `dmc-transform` | visitor + 5 native transformers: code_import (w/ `{1,3-5}` ranges), npm_command, bare_url, autolink_headings (`subheading-anchor`), pretty_code (line marks) |
| `dmc-schema`    | velite-parity schema builder + JSON descriptor compiler                                                                                                      |
| `dmc-core`      | engine + loaders (matter/yaml/json) + CLI (`build`/`init`/`compile`/`dev`)                                                                                   |
| `dmc-napi`      | `@duck/md` npm package — Node FFI                                                                                                                            |
| `dmc-sidecar`   | `@duck/md-sidecar` — Node-side runner for community JS plugins                                                                                               |

## Examples

- [`examples/web/`](examples/web/) — Vite + React, `MdxContent` runtime that strips `import`s and binds components via `new Function(body)(jsxRuntime, components)`
- [`examples/nextjs/`](examples/nextjs/) — Next.js App Router + server-rendered HTML

## License

MIT.
