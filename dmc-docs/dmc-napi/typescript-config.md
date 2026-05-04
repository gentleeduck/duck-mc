# TypeScript config loading

dmc accepts `.ts`, `.js`, and `.mjs` configs. Loader spawns a JS host
to import the user file and serialise the resolved config to JSON.

## Discovery order

```rust
fn load_ts(config: &PathBuf) -> std::io::Result<EngineConfig> {
    let attempts: &[(&str, &[&str])] = &[
        ("bun", &[]),
        ("node", &["--import", "tsx"]),
    ];
    // ...
}
```

| host | requires | speed |
|------|----------|-------|
| `bun` | bun on PATH | fastest, native TS |
| `node` + `tsx` | `node` + `tsx` install | slower; tsx loader |

First attempt that exits zero wins. Both produce the same JSON.

## Helper script

```js
// dmc-core/scripts/load-config.mjs (excerpt)
const path = process.argv[2];
const mod = await import(pathToFileURL(path).href);
const cfg = mod.default ?? mod;
process.stdout.write(JSON.stringify(cfg));
```

Stamped to a temp file at runtime; spawned with the user config path
as argv. Output captured, parsed as `EngineConfig`.

## Pluggable references

User configs can ship Pluggable refs (functions / arrays) for
`remarkPlugins` / `rehypePlugins`. The TS host imports them, then the
dmc engine fingerprints by name only when computing cache keys (refs
do not survive JSON round-trip; the ref-bearing plugins live entirely
in the in-process unified pipeline run by `mod.ts`, not the Rust
engine).

## Caching considerations

The cache fingerprint includes a serialisable view of `CompileConfig`.
TS-only references (functions) do not show up in the fingerprint. So
edits to plugin behaviour that do not change the config object will
not auto-bust the cache. Bump `cfg.cacheEnabled = false` for one
build, or wipe `<output>/.cache/`, when iterating on plugin code.

## TS errors

Errors in the user config surface as:

```
ts config: <message>
--- output ---
<stderr from ts host>
```

Returned via `std::io::Error::InvalidData`. Caller (CLI / napi) prints
and exits non-zero.
