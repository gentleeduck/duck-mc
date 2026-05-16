# dmc-napi

JS bindings for the dmc engine. Published as `@gentleduck/md` on npm.

## Architecture

```mermaid
flowchart LR
    User[TS/JS app] --> Pkg[@gentleduck/md mod.ts]
    Pkg --> Native[dmc.linux-x64-gnu.node<br/>napi-rs cdylib]
    Native --> Core[dmc-core Engine]
    Core --> Cargo[dmc-parser / dmc-transform / dmc-codegen]
    Pkg -.->|optional| Sidecar[dmc-sidecar Node helper<br/>foreign remark/rehype plugins]
```

## What this crate ships

- `dmc-napi/src/lib.rs` - napi-rs entry exposing Rust functions to JS
- `dmc-napi/mod.ts` - TypeScript wrapper, schema builder, config helpers
- `dmc-napi/index.js` / `index.d.ts` - napi-rs generated loader + types
- `*.node` - prebuilt platform binary

## Public surface

- `defineConfig(cfg)` - identity helper for type narrowing
- `s` - Zod-style schema builder for frontmatter (`s.object`, `s.string`, ...)
- `build(cfg)` - run the full engine
- `compile(source)` - one-shot compile string -> CompileOutput
- `compileMany(sources)` - batched compile
- `latexToHtml(latex, display)` - direct KaTeX render

See [`api.md`](api.md) for full TypeScript signatures.

## CLI: `duck-md dev`

`bin/duck-md.mjs` provides the `duck-md` CLI (`build` / `dev` / `watch` /
`info` / `clean`). `dev` (alias `watch`) runs an initial build, then
uses chokidar to rebuild on change. After the first build it seeds a
`Map<absPath, sha256>` of every `.md` / `.mdx` file under `root`; on a
change event it re-hashes, and if the hash matches the stored one it
logs `[duck-md] no-op (<rel> unchanged)` and skips the rebuild. The
same check applies to the config file. `add` / `unlink` events always
rebuild. This sits above the dmc-core per-file blake3 cache.

## Files

- [`api.md`](api.md) - exported types and functions
- [`js-api.md`](js-api.md) - usage patterns (defineConfig, hooks, loaders)
- [`typescript-config.md`](typescript-config.md) - how `.ts` configs load
- [`examples.md`](examples.md) - real configs end to end
