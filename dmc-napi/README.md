<p align="center">
  <img src="./logo-dark.svg" alt="@gentleduck/md" width="120"/>
</p>

<h1 align="center">@gentleduck/md</h1>

<p align="center">
  Native Rust MDX compiler with a velite-shaped TypeScript API.
</p>

<p align="center">
  <a href="https://github.com/gentleeduck/duck-mc/blob/master/LICENSE">MIT</a> -
  <a href="https://github.com/gentleeduck/duck-mc/blob/master/CHANGELOG.md">Changelog</a> -
  <a href="https://github.com/gentleeduck/duck-mc/blob/master/CONTRIBUTING.md">Contributing</a> -
  <a href="https://github.com/gentleeduck/duck-mc/tree/master/dmc-docs">Docs</a> -
  <a href="https://github.com/gentleeduck/duck-mc/tree/master/duck-benchmarks">Benchmarks</a>
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/@gentleduck/md"><img src="https://img.shields.io/npm/v/@gentleduck/md.svg" alt="npm"/></a>
  <a href="https://github.com/gentleeduck/duck-mc/blob/master/LICENSE"><img src="https://img.shields.io/npm/l/@gentleduck/md.svg" alt="MIT"/></a>
</p>

---

## Install

```sh
pnpm add @gentleduck/md
```

Optional: `@gentleduck/md-sidecar` for foreign remark / rehype plugins.

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

## Watch mode

`duck-md dev` (alias `duck-md watch`) does an initial build, then
rebuilds on file change via chokidar. It seeds a `Map<absPath, sha256>`
of every `.md` / `.mdx` file under `root` after the first build and
re-hashes on each change event; saves that don't alter content log
`[duck-md] no-op (<rel> unchanged)` and skip the rebuild. The same
check applies to the config file. `add` / `unlink` events always
rebuild. This dedupe sits above the per-file blake3 cache in
`dmc-core`.

## Native features

Pretty code (syntect), KaTeX/MathML math, emoji, code imports,
npm-command tabs, mermaid, bare URL autolinks, heading autolinks,
asset copy.

JS plugins listed in config that have native equivalents
(`remark-gfm`, `rehype-pretty-code`, `rehype-katex`, `rehype-slug`,
etc) are stripped from the sidecar payload automatically.

## Docs

Repo: [github.com/gentleeduck/duck-mc](https://github.com/gentleeduck/duck-mc)

- Per-crate references, architecture, integration guides:
  [`dmc-docs/`](https://github.com/gentleeduck/duck-mc/tree/master/dmc-docs)
- duck-ui website cross-link:
  [github.com/gentleeduck/duck-ui](https://github.com/gentleeduck/duck-ui)
- Migration from velite:
  [`dmc-docs/guides/migrating-from-velite.md`](https://github.com/gentleeduck/duck-mc/blob/master/dmc-docs/guides/migrating-from-velite.md)

## Benchmarks

Five recorded phases; **9.5x velite** at the kitchen-sink workload.
Full numbers: [`duck-benchmarks/`](https://github.com/gentleeduck/duck-mc/tree/master/duck-benchmarks).

## Contributing

PR checklist + style notes:
[`CONTRIBUTING.md`](https://github.com/gentleeduck/duck-mc/blob/master/CONTRIBUTING.md).

## License

MIT
