<p align="center">
  <img src="./public/logo-dark.svg" alt="dmc" width="120"/>
</p>

<h1 align="center">dmc</h1>

<p align="center">
  Native Rust MDX compiler with a velite-shaped TypeScript API.
</p>

<p align="center">
  <a href="./LICENSE">MIT</a> -
  <a href="./dmc-docs">Docs</a> -
  <a href="./duck-benchmarks">Benchmarks</a> -
  <a href="./docs/migrating-from-velite.md">Migrating from velite</a>
</p>

---

## Install

```sh
pnpm add @gentleduck/md
```

Optional: `@gentleduck/md-sidecar` if you still need JS remark/rehype plugins.

## Quick start

```ts
// dmc.config.ts
import { defineConfig, s } from "@gentleduck/md";

export default defineConfig({
  output: { data: ".gentleduck", html: true },
  collections: {
    posts: {
      name: "Post",
      pattern: "content/posts/**/*.mdx",
      schema: (s) => s.object({
        title: s.string(),
        date: s.isodate(),
        slug: s.path(),
      }),
    },
  },
});
```

```sh
dmc build
```

Outputs `.gentleduck/Post.json` + typed `index.d.ts`. Import from any framework.

## Workspace

| Crate | Role |
| --- | --- |
| `dmc-lexer` | MDX / JSX / GFM tokenizer |
| `dmc-parser` | Typed AST parser (block, inline, JSX, table) |
| `dmc-highlight` | Bundled syntect grammars + themes |
| `dmc-transform` | Native pipeline + builtin transformers |
| `dmc-codegen` | HTML + MDX body emitters |
| `dmc-schema` | Velite-style schema builders |
| `dmc-diagnostic` | Shared diagnostic codes + spans |
| `dmc-core` | Engine, CLI, watch, collections |
| `dmc-napi` | NAPI bindings (`@gentleduck/md`) |

JS-only: `dmc-sidecar` (`@gentleduck/md-sidecar`).

## Native features

Pretty code (syntect), KaTeX/MathML math, emoji, code imports, npm-command tabs, mermaid, bare URL autolinks, heading autolinks, asset copy. Each gated by a Cargo feature; unused ones compile out.

JS plugins listed in config that have native equivalents (`remark-gfm`, `rehype-pretty-code`, `rehype-katex`, `rehype-slug`, etc) are stripped from the sidecar payload automatically.

## Examples

| Path | Stack |
| --- | --- |
| [`examples/nextjs`](examples/nextjs) | Next.js App Router, `@gentleduck/md` |
| [`examples/nextjs-velite`](examples/nextjs-velite) | velite, same content for parity check |
| [`examples/web`](examples/web) | Vite + React |
| [`examples/acme-docs`](examples/acme-docs) | Docs-site template |

## Docs

- [`dmc-docs/`](dmc-docs) - per-crate references, architecture, integration guides
- [`docs/`](docs) - architecture notes, benchmarks, perf write-ups

## Build

```sh
pnpm install
cargo build --release
cargo test --workspace --features pretty-code
pnpm --filter @gentleduck/md run build
```

## CLI

```sh
dmc build --config dmc.config.ts
dmc dev   --config dmc.config.ts
dmc compile path/to/file.mdx
```

## Migrating from velite

```diff
- import { defineConfig, s } from 'velite'
+ import { defineConfig, s } from '@gentleduck/md'
```

See [`docs/migrating-from-velite.md`](docs/migrating-from-velite.md).

## License

MIT
