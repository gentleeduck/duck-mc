# Engine

`Engine::run` is the top-level entry. Drives one full build.

## Signature

```rust
pub fn run(
    cfg: &EngineConfig,
    config_path: Option<&Path>,
    diag_engine: &mut DiagnosticEngine<Code>,
) -> std::io::Result<()>;
```

`config_path` is the source path of the user's `.ts`/`.js` config when
applicable. Used by `index::write_index` to emit `typeof import(...)`
references in `index.d.ts` so the user's TS config types flow through.

## Sequence

```rust
pub fn run(
    cfg: &EngineConfig,
    config_path: Option<&Path>,
    diag_engine: &mut DiagnosticEngine<Code>,
) -> std::io::Result<()> {
    if cfg.clean && cfg.output_dir.exists() {
        std::fs::remove_dir_all(&cfg.output_dir)?;
    }
    std::fs::create_dir_all(&cfg.output_dir)?;

    let math_cache_path = cfg.output_dir.join(".cache").join("math.json");
    #[cfg(feature = "math")]
    if cfg.cache_enabled {
        dmc_transform::Math::load_cache(&math_cache_path);
    }

    for c in &cfg.collections {
        let _ = c.process(cfg, diag_engine);
    }

    #[cfg(feature = "math")]
    if cfg.cache_enabled {
        dmc_transform::Math::save_cache(&math_cache_path);
    }

    let format = cfg.output_format.as_deref().unwrap_or("esm");
    index::write_index(&cfg.output_dir, &cfg.collections, format, config_path)?;

    Ok(())
}
```

## Stages

| stage | purpose |
|-------|---------|
| clean | wipe `output_dir` if `cfg.clean` is true |
| ensure dir | mkdir `output_dir` |
| load math cache | warm `dmc_transform::Math` from disk |
| process collections | run each `Collection::process` in sequence (parallel inside) |
| save math cache | persist new math entries |
| write index | emit `index.js` + `index.d.ts` re-exports |

## Diagnostics

The shared `DiagnosticEngine<Code>` is mutated through. Each
`Collection::process` merges its per-thread engines back into this
one. Caller decides what to do with them after `run` returns (print,
fail-on-error, etc).

## `index::write_index`

```rust
pub fn write_index(
    out_dir: &Path,
    collections: &[Collection],
    format: &str,
    config_path: Option<&Path>,
) -> std::io::Result<()>;
```

Path: `dmc::engine::index::write_index`. `format` = `"esm"` or
`"cjs"`. Generates a top-level `index.js` re-exporting every
collection's JSON file:

```js
// index.js (esm)
import doc from "./doc.json" assert { type: "json" };
import post from "./post.json" assert { type: "json" };
export { doc, post };
```

Plus `index.d.ts` typed via `typeof import(config)["collections"]` so
`@gentleduck/md`'s `defineConfig` types flow through.
