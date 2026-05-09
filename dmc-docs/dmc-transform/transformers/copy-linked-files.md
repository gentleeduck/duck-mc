# `copy-linked-files`

Copies relative `src=` / `href=` assets referenced from MDX content into
a public output directory, rewriting the URL in-place so the build
output is self-contained.

- **Source:** `dmc-transform/src/builtin/copy_linked_files.rs`
- **Feature flag:** `assets`
- **Config struct:** [`CopyLinkedFilesOptions`](../src/config.rs)
- **TS slot:** `markdown.copyLinkedFiles` + `output.assets` + `output.base`

## Configuration

```ts
import { defineConfig } from '@gentleduck/md/config'

export default defineConfig({
  output: {
    assets: 'public/assets',          // copies land here
    base:   '/assets',                // URL prefix written into MDX
  },
  markdown: {
    copyLinkedFiles: true,            // toggle the transformer
  },
})
```

## Behavior

For an MDX node like `<img src="./diagram.png" />` next to a source
file at `content/post.mdx`:

1. Resolves `./diagram.png` relative to the source dir.
2. Copies the file to `<assets_dir>/<hash>.png`.
3. Rewrites the JSX/Markdown URL to `<public_base>/<hash>.png`.

Hashing keeps duplicate filenames from colliding and lets caches
cache-bust on content change.

## Knob reference

| Knob (Rust) | TS | Effect |
|---|---|---|
| `source_dir` | (auto) | Source `.mdx` parent dir. |
| `assets_dir` | `output.assets` | Where copied files land. |
| `public_base` | `output.base` | URL prefix written into rewritten links. |
