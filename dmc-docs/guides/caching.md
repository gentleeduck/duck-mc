# Caching

dmc has two persistent caches and several in-process caches. Cold and
warm builds differ a lot; this is the user-side guide.

## Where

```
<output_dir>/.cache/
|- dmc/{16-hex}.json   per-file compile output
`- math.json           KaTeX/MathML render cache
```

Default `<output_dir>` is `.gentleduck`. So:

```
.gentleduck/.cache/dmc/86ee354a0cc1dfce.json
.gentleduck/.cache/math.json
```

## Toggle

```ts
defineConfig({
  cacheEnabled: true,  // default
});
```

`false` disables both. Useful for clean-room CI runs.

## Wipe

```bash
rm -rf .gentleduck/.cache
```

Or:

```bash
rm -rf .gentleduck   # nuke output too
```

## Invalidation

Auto-invalidates on:

- dmc version bump (key includes `CARGO_PKG_VERSION`)
- source byte change (any edit to the file)
- config change (the cfg fingerprint covers `compile`, `include_html`,
  collection name, schema, output format)

Does not auto-invalidate on:

- TS-only plugin function-body edits (function refs do not appear in
  the serde-json fingerprint)

For plugin code iteration, wipe manually or set `cacheEnabled: false`
during the iteration.

## .gitignore

```
.gentleduck/
```

Cache lives inside the output dir, so the same line covers both.

## CI persistence

GitHub Actions:

```yaml
- uses: actions/cache@v4
  with:
    path: .gentleduck/.cache
    key: dmc-${{ hashFiles('content/**/*.mdx', 'duck-md.config.ts') }}
```

Vercel: build cache picks up `.gentleduck/.cache` automatically when
left in `output_dir` (provided the dir is not in `.vercelignore`).

## Numbers

Demo: `examples/nextjs` kitchen-sink (2 records, ~12 KB output each):

```
=== COLD ===
pnpm content   1.187 s

=== WARM ===
pnpm content   0.334 s
```

3.55x. Most of the warm cost is JS startup (tsx + module resolution).

For 1000 files where 1 changed:

- cold: ~1200 ms (every file recompiled)
- warm: ~50 ms (1 recompile, 999 cache reads)

## What does not cache

- Index emission (tiny; not worth caching).
- Sidecar output cached implicitly via the file cache (the sidecar's
  HTML is part of the cached velite record).
- Mermaid SVGs cached separately by `dmc_transform::Mermaid`.

## Compared to velite

velite has a `Map<string, any>` per process. Dies when the process
exits. Every CLI invocation is a cold build.

dmc keeps the file + math cache on disk. Every build sees the
cumulative cache. Watch mode wins extra: same warm cache reused
across edits.
