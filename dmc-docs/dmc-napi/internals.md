# dmc-napi internals

Bridge between the JS world (`@gentleduck/md` npm package) and Rust
(`dmc-core` engine). Built with napi-rs.

## Layout

```
dmc-napi/
|- src/lib.rs              napi-rs entry; exposes Rust fns to JS
|- mod.ts                  TypeScript wrapper; schema builder + helpers
|- index.js                napi-rs generated loader (binary lookup)
|- index.d.ts              napi-rs generated TS declarations
|- package.json            npm metadata (name = @gentleduck/md)
|- *.node                  prebuilt platform binary
|- build.rs                napi-rs build helper
|- Cargo.toml              cdylib crate
`- scripts/                helper scripts (build, sync types, etc)
```

## napi-rs entry

```rust
#[napi]
pub fn compile(source: String) -> Result<Value> {
    let mut diag = DiagnosticEngine::<Code>::new();
    let out = Compiler::compile(&source, &mut diag);
    serde_json::to_value(&out).map_err(|e| Error::from_reason(e.to_string()))
}

#[napi]
pub fn build(input: BuildInput) -> Result<BuildReport> {
    // ...
}

#[napi]
pub fn latex_to_html(latex: String, display: bool) -> Result<String> {
    // ...
}
```

`#[napi]` generates the JS-callable shim. Return `Result` for any
fallible op; `Error::from_reason(s)` wraps a string into a JS
exception.

## Build pipeline

```bash
pnpm --filter @gentleduck/md run build
# runs:
napi build --platform --release
```

napi build:

1. Compiles `dmc-napi` as `cdylib` for the host platform.
2. Renames the artefact to `dmc.<platform-triple>.node`.
3. Regenerates `index.d.ts` from the `#[napi]` annotations.

## Platform binary lookup

`index.js` (generated) tries each known platform triple in order;
the first that exists loads. Falls back to npm scoped packages
(`@gentleduck/md-linux-x64-gnu`, etc) when the binary is in a separate
package (npm-distribution path).

## TypeScript wrapper

`mod.ts` re-exports the napi shims with friendlier types:

```ts
import { build as nativeBuild } from "./index.js";

export interface UserConfig { /* ... */ }

export async function build(cfg: UserConfig): Promise<BuildReport> {
    // 1. Map UserConfig to BuildInput (the napi-shaped struct)
    // 2. Strip TS-only refs (Pluggable plugin functions stay JS-side)
    // 3. Call nativeBuild
    // 4. If config has remark/rehype plugins, run unified pipeline JS-side
    //    against compile output (sidecar replacement on the napi path)
    // 5. Run prepare / complete hooks
    // 6. Apply collection callbacks
}
```

`build` is async because the post-processing (unified pipeline,
hooks) may be async.

## Schema builder

`mod.ts` ships an `s` builder that mirrors velite's surface:

```ts
export const s = {
    object: (shape) => /* ... */,
    string: () => /* ... */,
    // ...
};
```

Each method returns a chain object with `.optional()`, `.default()`,
`.transform()`, `.refine()`, `.parse()`. `.toDescriptor()` serialises
to the JSON shape `dmc_schema::compile_descriptor` consumes.

The Rust engine receives the descriptor JSON and validates per-record
during `Collection::process`.

## Hook plumbing

Hooks (`prepare`, `complete`, `onRecord`) live JS-side. The wrapper
in `mod.ts` runs them after the Rust engine returns:

```ts
const report = await nativeBuild(buildInput);

if (cfg.prepare || cfg.complete || hasCallbacks) {
    const data = await readEveryCollectionJson(report);
    await cfg.prepare?.(data);
    await runCallbacks(data, callbacks);
    await cfg.complete?.(data);
    await writeBackEveryCollectionJson(data, report);
}
```

Slow-ish (rereads + rewrites each collection's JSON) but transparent.

## Loaders

```ts
defineLoader({
    test: /\.yaml$/,
    load({ path, value }) { return { data: parse(value) }; },
});
```

Loaders run JS-side BEFORE the Rust engine. Files matching a
loader's `test` regex get parsed by the loader; the result feeds
into schema validation. Non-MDX files only; MDX always goes through
the dmc pipeline.

## Error path

```rust
serde_json::to_value(&out).map_err(|e| Error::from_reason(e.to_string()))
```

`Error::from_reason` produces a JS `Error` with the given message.
napi-rs converts at the FFI boundary; consumers see standard
exceptions:

```ts
try { await build(cfg); } catch (e) {
    console.error(e.message);
}
```

## Memory + GC

napi-rs wraps Rust types in `JsObject`. The N-API runtime tracks
references; Rust frees when the JS handle goes out of scope. dmc-napi
returns `Value` (serde JSON) from compile / build, so no long-lived
Rust objects cross the boundary.

## Async functions

`#[napi]` on an `async fn` produces a JS function returning a
Promise. Used internally by `build`:

```rust
#[napi]
pub async fn some_async(input: String) -> Result<String> { /* ... */ }
```

JS sees a real Promise; awaiting it yields the resolved value or
throws on Err.

## Generation

```bash
napi build --platform --release   # runs Rust compile + sync index.{js,d.ts}
```

The generated `index.d.ts` reflects the latest `#[napi]` types. Do
not edit by hand; regenerate after changing Rust signatures.

## Publishing

CI matrix builds the binary on every supported platform; npm
package ships the prebuilt `*.node` files via optional
platform-specific subpackages (`@gentleduck/md-linux-x64-gnu`, etc) so
consumers download only their platform.

## Local dev

```bash
pnpm --filter @gentleduck/md run build    # rebuild after Rust edits
pnpm --filter dmc-nextjs dev         # demo app picks up new binary
```

The example apps depend on `@gentleduck/md: workspace:*`, so a fresh build
flows through immediately.
