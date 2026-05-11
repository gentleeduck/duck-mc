<p align="center">
  <img src="./public/logo-dark.svg" alt="dmc" width="120"/>
</p>

<h1 align="center">@gentleduck/md</h1>

<p align="center">
  Native Rust MDX compiler with a velite-shaped TypeScript API.
</p>

<p align="center">
  <a href="./LICENSE">MIT</a> -
  <a href="./CHANGELOG.md">Changelog</a> -
  <a href="./CONTRIBUTING.md">Contributing</a> -
  <a href="./dmc-docs">Docs</a> -
  <a href="./duck-benchmarks">Benchmarks</a>
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/@gentleduck/md"><img src="https://img.shields.io/npm/v/@gentleduck/md.svg" alt="npm"/></a>
  <a href="https://crates.io/crates/dmc-core"><img src="https://img.shields.io/crates/v/dmc-core.svg" alt="crates.io"/></a>
  <a href="./LICENSE"><img src="https://img.shields.io/crates/l/dmc-core.svg" alt="MIT"/></a>
</p>

---

## Install

```sh
pnpm add @gentleduck/md
```

Optional: `@gentleduck/md-sidecar` for foreign remark/rehype plugins.

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

## Workspace

| Crate | Role |
| --- | --- |
| [`dmc-lexer`](dmc-lexer) | MDX / JSX / GFM tokenizer |
| [`dmc-parser`](dmc-parser) | Typed AST parser |
| [`dmc-highlight`](dmc-highlight) | Bundled syntect grammars + themes |
| [`dmc-transform`](dmc-transform) | Native pipeline + builtin transformers |
| [`dmc-codegen`](dmc-codegen) | HTML + MDX body emitters |
| [`dmc-schema`](dmc-schema) | Velite-style schema builders |
| [`dmc-diagnostic`](dmc-diagnostic) | Shared diagnostic codes + spans |
| [`dmc-core`](dmc-core) | Engine, CLI, watch, collections |
| [`dmc-napi`](dmc-napi) | NAPI bindings (`@gentleduck/md`) |

JS-only: [`dmc-sidecar`](dmc-sidecar) (`@gentleduck/md-sidecar`).

## Examples

| Path | Stack |
| --- | --- |
| [`examples/nextjs`](examples/nextjs) | Next.js App Router, `@gentleduck/md` |
| [`examples/nextjs-velite`](examples/nextjs-velite) | velite, parity check |
| [`examples/web`](examples/web) | Vite + React |
| [`examples/acme-docs`](examples/acme-docs) | Multi-collection template |

## Build

```sh
pnpm install
cargo build --release
cargo test  --workspace --features pretty-code
pnpm --filter @gentleduck/md run build
```

## Docs

- [`dmc-docs/`](dmc-docs) - per-crate references, architecture, integration guides
- [`duck-benchmarks/`](duck-benchmarks) - benchmark phases, methodology guide, optimization map
- [duck-ui website](https://github.com/gentleeduck/duck-ui) - cross-linked intro + benchmarks page

## Benchmarks

Seven recorded phases under [`duck-benchmarks/`](duck-benchmarks).
Headline: **~10x velite** at the kitchen-sink workload, **~100x** on plain markdown.

- [`duck-benchmarks/README.md`](duck-benchmarks/README.md) - phase-by-phase log + cross-phase table
- [`duck-benchmarks/BENCHMARKS.md`](duck-benchmarks/BENCHMARKS.md) - parser microbenchmarks + compile-pipeline numbers
- [`duck-benchmarks/GUIDE.md`](duck-benchmarks/GUIDE.md) - how to record a new bench phase and tell signal from host noise
- [`duck-benchmarks/OPTIMIZATIONS.md`](duck-benchmarks/OPTIMIZATIONS.md) - per-crate optimization opportunities + timeline debt (not being worked on; the map if you want to push further)

## Contributing

PR checklist + style notes in [`CONTRIBUTING.md`](CONTRIBUTING.md).
Security: [`SECURITY.md`](SECURITY.md). Behaviour: [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md).

## License

MIT. See [`LICENSE`](LICENSE).
