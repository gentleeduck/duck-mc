<p align="center">
  <img src="./logo-dark.svg" alt="@gentleduck/md" width="120"/>
</p>

<h1 align="center">@gentleduck/md</h1>

<p align="center">
  Native Rust MDX compiler with a velite-shaped TypeScript API.
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

## Native features

Pretty code (syntect), KaTeX/MathML math, emoji, code imports, npm-command tabs, mermaid, bare URL autolinks, heading autolinks, asset copy.

JS plugins listed in config that have native equivalents (`remark-gfm`, `rehype-pretty-code`, `rehype-katex`, `rehype-slug`, etc) are stripped from the sidecar payload automatically.

## Docs

Repo: [github.com/gentleeduck/duck-mc](https://github.com/gentleeduck/duck-mc)

- Per-crate references, architecture, integration guides: [`dmc-docs/`](https://github.com/gentleeduck/duck-mc/tree/master/dmc-docs)
- Migration from velite: [`docs/migrating-from-velite.md`](https://github.com/gentleeduck/duck-mc/blob/master/docs/migrating-from-velite.md)

## License

MIT
