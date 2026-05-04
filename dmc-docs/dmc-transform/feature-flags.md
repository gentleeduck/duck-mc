# Feature flags

Every transformer is feature-gated. Defaults pull a sensible chain
for a typical docs site. Slim builds drop bundles via
`--no-default-features`.

## Cargo.toml

```toml
[features]
default = ["mermaid", "assets", "npm-command", "math", "emoji"]
mermaid = []
assets = ["dep:blake3"]
npm-command = []
pretty-code = ["dep:dmc-highlight"]
math = ["dep:katex", "dep:pulldown-latex"]
emoji = ["dep:emojis"]
```

## Per-flag impact

| flag | default | binary cost | drops |
|------|---------|-------------|-------|
| `pretty-code` | off (must opt in via dmc-core/pretty-code) | ~5 MB syntect bundle | code highlighting |
| `math` | on | quickjs + pulldown-latex (~2 MB) | math rendering |
| `emoji` | on | unicode table (~5 MB) | shortcode replacement |
| `mermaid` | on | nothing native (uses external `mmdc`) | mermaid diagram support |
| `npm-command` | on | nothing | npm tab generation |
| `assets` | on | blake3 dep | copy-linked-files transformer |

## What stays always on

These transformers are gated only by config, not Cargo features:

- `CodeImport`
- `BareUrlAutolink`
- `AutolinkHeadings`
- `DisableGfm`

They have no heavy deps and are part of every default chain.

## Slim build example

```bash
cargo build --release \
  -p dmc-core \
  --no-default-features \
  --features cli,watch
```

Drops syntect, KaTeX, pulldown-latex, the emojis crate, and blake3.
Result: a markdown -> HTML core that handles GFM + bare URLs +
heading anchors and nothing else. Useful when the consumer has its
own highlighter, math, etc.

## Forwarding

`dmc-core/Cargo.toml` mirrors transform features so the CLI / napi
binary can opt in:

```toml
[features]
default = ["cli", "watch", "pretty-code", "math", "emoji"]
pretty-code = ["dmc-transform/pretty-code"]
math = ["dmc-transform/math"]
emoji = ["dmc-transform/emoji"]
```

The transformer features must match across crates (else the gate
strip in `compile.rs` becomes inconsistent with the actual pipeline).

## Runtime gate vs compile gate

| | compile gate | runtime gate |
|-|--------------|--------------|
| pretty-code | `cfg(feature = "pretty-code")` | `cfg.pretty_code` |
| math | `cfg(feature = "math")` | `cfg.math_engine` |
| emoji | `cfg(feature = "emoji")` | always on once feature on |
| mermaid | `cfg(feature = "mermaid")` | `mmdc` on PATH check |

Compile gate decides whether the symbol exists. Runtime gate decides
whether the configured behaviour is active.

## Adding a new transformer

1. Implement `Transformer` for your struct in `dmc-transform/src/builtin/<name>.rs`.
2. Add a Cargo feature in `dmc-transform/Cargo.toml` if it has heavy deps.
3. Mirror the feature in `dmc-core/Cargo.toml` for forwarding.
4. Register in `Pipeline::with_defaults_for(cfg)` in
   `dmc-transform/src/pipeline.rs`.
5. Add stripped JS plugin name(s) to
   `dmc-core::engine::compile::is_native_owned_*` so the sidecar
   does not duplicate work.
6. Doc it under `dmc-docs/dmc-transform/transformers/`.

See `writing-a-transformer.md` for the trait + visitor walkthrough.

## How features interact with the sidecar gate

`CompileConfig::is_native_owned_remark` and `is_native_owned_rehype`
gate the sidecar plugin payload by feature flag:

```rust
fn is_native_owned_remark(plugin: &Value) -> bool {
    let Some(name) = plugin_name(plugin) else { return false };
    match name {
        "remark-gfm" => true,
        "remark-math" => cfg!(feature = "math"),
        "remark-emoji" => cfg!(feature = "emoji"),
        _ => false,
    }
}
```

When the feature is off, the JS plugin runs in the sidecar; when on,
the dmc transformer takes over.

## Bench impact

| build | kitchen-sink @N=1000 |
|-------|---------------------|
| all features on (default) | 145 ms |
| `--no-default-features --features cli,watch` | n/a (no transformers; just markdown -> HTML core) |

Slim builds avoid plugin code paths entirely; bench numbers do not
apply because the comparison content cannot render.
