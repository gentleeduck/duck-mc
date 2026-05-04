# Persistent caches

Two on-disk caches live under `<output_dir>/.cache/`:

```
<output_dir>/.cache/
|- dmc/{16-hex blake3}.json   per-file compile output
`- math.json                  KaTeX/MathML render cache
```

## File cache

Implemented in `dmc::engine::cache::FileCache`.

### Key composition

```rust
pub fn key(source: &[u8], path: &Path, cfg_fingerprint: &[u8]) -> String {
    let mut h = Hasher::new();
    h.update(b"dmc/v1");
    h.update(VERSION.as_bytes());     // CARGO_PKG_VERSION
    h.update(b"\0src\0");
    h.update(source);
    h.update(b"\0path\0");
    h.update(path.to_string_lossy().as_bytes());
    h.update(b"\0cfg\0");
    h.update(cfg_fingerprint);
    h.finalize().to_hex().as_str()[..16].to_string()
}
```

Inputs:
- dmc version (any upgrade busts every entry)
- source bytes (any edit busts that file)
- file path (avoid identical-content collisions)
- caller-supplied config fingerprint

Output: 16 hex chars (blake3 truncated). One JSON file per record.

### `cfg_fingerprint`

```rust
pub fn fingerprint<T: Serialize>(cfg: &T) -> Vec<u8> {
    let json = serde_json::to_vec(cfg).unwrap_or_default();
    blake3::hash(&json).as_bytes().to_vec()
}
```

`Collection::process` wraps the relevant inputs:

```rust
let cfg_fp = fingerprint(&(
    &cfg.compile,
    &cfg.include_html,
    &self.name,
    &self.schema,
    &cfg.output_format,
));
```

Any config field that affects output goes into the tuple. Adding a new
field that influences emission means adding it here too.

### Hit path

```rust
let cache_key = cache.as_ref().map(|_| FileCache::key(source.as_bytes(), path, &cfg_fp));
if let (Some(c), Some(k)) = (cache.as_ref(), cache_key.as_ref())
    && let Some(hit) = c.get(k)
{
    return (Some(hit), local_diag_engine);
}
```

Skips lex + parse + transform + codegen + sidecar entirely. The cached
value is the final velite-shaped record.

### Miss path

After compile + sidecar + schema + record build:

```rust
if let (Some(c), Some(k)) = (cache.as_ref(), cache_key.as_ref()) {
    c.put(k, &rec);
}
```

Best effort. A write failure never breaks the build.

## Math cache

In-memory `HashMap<(String, bool, MathEngine), String>` inside
`dmc_transform::Math`. Persisted across builds.

### Lifecycle

```rust
// Engine::run (in)
let math_cache_path = cfg.output_dir.join(".cache").join("math.json");
#[cfg(feature = "math")]
if cfg.cache_enabled {
    dmc_transform::Math::load_cache(&math_cache_path);
}

// ... run collections ...

// Engine::run (out)
#[cfg(feature = "math")]
if cfg.cache_enabled {
    dmc_transform::Math::save_cache(&math_cache_path);
}
```

Loaded once per build; flushed at end. Repeated math expressions
(common in technical docs) hit memory in microseconds.

### Format

JSON array of rows: `[latex, display, engine, html]`. `engine` is the
`MathEngine` discriminant (0 = Katex, 1 = Mathml). Switching engines
keeps both sets of cached renders.

## Toggling

```toml
[engine_config]
cache_enabled = true   # default
```

Set `false` for clean-room builds. Wipe via `rm -rf <output>/.cache`.

## Bench

| build | time | speedup |
|-------|------|---------|
| cold (no cache) | 1187 ms | 1.0x |
| warm (cache hit) | 334 ms | **3.55x** |

Demo measured on `examples/nextjs` kitchen-sink content (2 records).
On larger collections the speedup is closer to "skip everything that
did not change", so a 1000-file rebuild with 1 changed file approaches
the cost of compiling 1 file + 999 cache reads.
