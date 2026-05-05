<p align="center">
  <img src="../public/logo-dark.svg" alt="@gentleduck/md-sidecar" width="120"/>
</p>

<h1 align="center">@gentleduck/md-sidecar</h1>

<p align="center">
  Optional Node helper for foreign remark / rehype plugins, paired with @gentleduck/md.
</p>

<p align="center">
  <a href="../LICENSE">MIT</a> -
  <a href="../CHANGELOG.md">Changelog</a> -
  <a href="../CONTRIBUTING.md">Contributing</a> -
  <a href="https://www.npmjs.com/package/@gentleduck/md-sidecar">npm</a>
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/@gentleduck/md-sidecar"><img src="https://img.shields.io/npm/v/@gentleduck/md-sidecar.svg" alt="npm"/></a>
  <a href="../LICENSE"><img src="https://img.shields.io/npm/l/@gentleduck/md-sidecar.svg" alt="MIT"/></a>
</p>

---

## Install

```sh
pnpm add @gentleduck/md-sidecar
```

Pair it with `@gentleduck/md` when your config lists JS plugins that
have no native equivalent (e.g. `rehype-mermaidjs-bundled`,
`remark-toc-tree`).

## Quick start

The sidecar runs as a long-lived NDJSON daemon spawned by the main
crate. You don't import it directly. List the JS plugins in your
`dmc.config.ts`:

```ts
import { defineConfig } from "@gentleduck/md";

export default defineConfig({
  collections: { /* ... */ },
  compileOptions: {
    markdownRehypePlugins: [
      "rehype-mermaidjs-bundled",
    ],
  },
});
```

`@gentleduck/md` auto-spawns the sidecar when at least one
non-native plugin is listed. Native-owned plugins (`remark-gfm`,
`rehype-pretty-code`, `rehype-katex`, etc) are stripped from the
sidecar payload by the plugin gate; the sidecar runs only the
foreign ones.

## Docs

- [npm](https://www.npmjs.com/package/@gentleduck/md-sidecar) -
  [main pkg](https://www.npmjs.com/package/@gentleduck/md) -
  [duck-ui website](https://github.com/gentleeduck/duck-ui)
- Sidecar protocol: [`dmc-docs/dmc-sidecar/`](../dmc-docs/dmc-sidecar)

## Contributing

See [`../CONTRIBUTING.md`](../CONTRIBUTING.md).

## License

MIT. See [`../LICENSE`](../LICENSE).
