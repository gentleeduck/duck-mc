# CopyLinkedFiles

Copies local files referenced via `src=` (images) or `href=` (links)
to an output asset directory. Rewrites the rendered URL to a public
prefix.

## Feature flag

`assets` (default on). Pulls `blake3` for content hashing.

## Input

`Node::Image { src, ... }` and `Node::Link { href, ... }` whose URL
is a relative path (not `http://`, `https://`, `data:`, `mailto:`,
nor an absolute path starting with `/`).

## Behaviour

1. Resolve URL relative to the source file's parent dir.
2. Hash the file contents (blake3).
3. Copy to `<assets_dir>/<base>-<hash:6>.<ext>`.
4. Rewrite the node's `src` / `href` to `<public_base>/<base>-<hash>.<ext>`.

## Configuration

```rust
pub struct CopyLinkedFilesOptions {
    pub source_dir: PathBuf,
    pub assets_dir: PathBuf,
    pub public_base: String,
}

pub struct CopyLinkedFiles { /* private */ }

impl CopyLinkedFiles {
    pub fn new(source_dir: PathBuf, assets_dir: PathBuf, public_base: String) -> Self;
}
```

Path: `dmc_transform::CopyLinkedFiles`. Configured via the
`PipelineConfig::copy_linked_files` field; dmc-core wires it from
`CompileConfig::output_assets` + `CompileConfig::output_base`.

## Failure modes

| failure | code | severity |
|---------|------|----------|
| write failed mid-publish | `T008 AssetCopyFailed` | error |
| referenced asset missing | `TW003 AssetSourceMissing` | warning |

## Config surface (TS)

```ts
defineConfig({
  output: {
    assets: "public/assets",
    base: "/assets/",
    name: "[name]-[hash:6].[ext]",
  },
  markdown: {
    copyLinkedFiles: true,
  },
});
```

## Example

Source MDX:

```mdx
![logo](./logo.png)

[whitepaper](./paper.pdf)
```

After pass:

```mdx
![logo](/assets/logo-7a8b3f.png)

[whitepaper](/assets/paper-2c4e91.pdf)
```

Files copied to `public/assets/`.

## Hashing

```rust
let hash = blake3::hash(content_bytes);
let name = format!("{stem}-{hex:6}.{ext}", hex = hash.to_hex());
```

Hash truncated to 6 hex chars (configurable via `name` template).
Collisions extremely unlikely at this length for typical asset
counts; bump if your repo has tens of thousands of assets.

## Idempotence

The hash key means the same source file always produces the same
output filename. Subsequent builds reuse the existing copy.

## Skipping

URLs starting with these schemes / patterns are not copied:
`http://`, `https://`, `data:`, `mailto:`, `javascript:`, `#anchor`,
`/absolute-path`.
