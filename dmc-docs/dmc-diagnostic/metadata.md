# Source metadata

`SourceMeta` carries source location across layer boundaries. Every
diagnostic span ties back to one. Same instance shared via `Arc` so
clones are cheap.

## `SourceMeta`

```rust
pub struct SourceMeta {
    pub path: Arc<str>,
    pub version: u64,
    pub origin: Origin,
}
```

Path: `dmc_diagnostic::metadata::SourceMeta`.

| field | meaning |
|-------|---------|
| `path` | display path for diagnostics. `Arc<str>` for cheap clone |
| `version` | logical version counter (bumped on dev-mode rebuild). Reserved for incremental work |
| `origin` | where the source came from (file, stdin, inline, memory) |

## `Origin`

```rust
pub enum Origin {
    File(PathBuf),
    Stdin,
    Inline(&'static str),
    Memory,
}
```

Path: `dmc_diagnostic::metadata::Origin`.

| variant | use |
|---------|-----|
| `File(p)` | normal disk-backed compile. Used by `code-import`, `copy-linked-files` to resolve relative paths |
| `Stdin` | one-shot CLI run reading from pipe |
| `Inline(s)` | tests / programmatic use; static-lifetime label |
| `Memory` | in-memory buffer with no disk anchor |

## How transformers consume it

```rust
use dmc_diagnostic::metadata::{Origin, SourceMeta};

fn resolve_relative(meta: &SourceMeta, rel: &str) -> Option<PathBuf> {
    if let Origin::File(p) = &meta.origin
        && let Some(dir) = p.parent()
    {
        return Some(dir.join(rel));
    }
    None
}
```

`code-import` and `copy-linked-files` use this pattern. When `origin`
is anything else (stdin / inline / memory), they emit `T012
BaseDirNotFound` rather than guessing.

## Construction

```rust
use std::sync::Arc;
use dmc_diagnostic::metadata::{Origin, SourceMeta};

let meta = Arc::from(SourceMeta {
    path: Arc::from("posts/hello.mdx"),
    version: 0,
    origin: Origin::File("posts/hello.mdx".into()),
});
```

`Compiler::compile_with_pipeline` wraps the meta in an `Arc<SourceMeta>`
internally; outer crates pass `&Path` and let compile handle the boxing.
