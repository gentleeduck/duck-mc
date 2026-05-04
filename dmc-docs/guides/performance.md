# Performance

Numbers and levers, terse.

## Headline

Kitchen sink, N=1000 files:

| variant | time | files/sec | vs velite |
|---------|------|-----------|-----------|
| dmc full native (gate strips all named plugins) | 145 ms | 6907 | **9.5x faster** |
| dmc warm cache hit | 334 ms | varies | 1.55x velite |
| velite kitchen-sink | 1380 ms | 724 | baseline |
| dmc gfm only | 462 ms | 2163 | **12.8x faster** |
| velite gfm only | 5910 ms | 169 | baseline |

## Per-stage budget at N=1000 (1188 ms run)

| stage | time | % |
|-------|------|---|
| native lex/parse/transform/codegen | 45 ms | 4 |
| KaTeX via quickjs | 400 ms | 34 |
| sidecar IPC | 200 ms | 17 |
| sidecar JS plugins | 500 ms | 42 |
| disk write | 40 ms | 3 |

After plugin gate widening (this version) sidecar is skipped entirely
when listed plugins are all native-owned, so the 700 ms goes to zero
and the build drops to ~145 ms.

## Knobs

| knob | default | when to change |
|------|---------|----------------|
| `mathEngine: "mathml"` | `"katex"` | drop ~300 ms for 1000 math-heavy files; tradeoff visual fidelity |
| `prettyCode.theme: "..."` (single string) | multi `{light, dark}` | save ~25% per code block; loses dark-mode CSS vars |
| `cacheEnabled: false` | true | force fresh build (CI smoke tests) |
| feature flag drop (`--no-default-features`) | all on | smaller binary; loses corresponding transformer |
| `output.html: false` | true | skip HtmlEmitter; consumer renders MDX runtime-side |

## Caches

Two on disk, several in process. See
[`../architecture/caching.md`](../architecture/caching.md).

| cache | persistence | speedup |
|-------|-------------|---------|
| File compile cache | disk | 3.55x warm rebuild |
| Math render cache | disk | repeated math: ms -> us |
| SyntaxBundle | process | one parse/run |
| Mermaid SVG | disk + process | mmdc seconds -> ms |

## When to optimise more

- Build feels slow in CI: check that cache is warm; CI agents often
  start fresh. Persist `<output>/.cache/` between runs.
- Hot reload feels sluggish: dev mode shares the math cache and file
  cache; usually the bottleneck is Next.js / Vite, not dmc.
- Single huge MDX (>100 KB): the lexer is byte-bound. Pre-split into
  smaller files if you can.

## When NOT to optimise

- Production builds <500 ms: invisible to users.
- 1-100 docs in dev mode: already <100 ms warm.

The 145 ms / 9.5x kitchen sink is the headline. Beyond that, ship
features instead.

## Open levers

| lever | est saving |
|-------|-----------|
| Batched sidecar IPC | ~190 ms per 1000 files (only matters with foreign plugins) |
| Persistent sidecar daemon | ~100 ms cold start |
| Compile syntect grammars to `.packdump` | ~50 ms one-time |
| PGO release build | 10-15% on hot paths |

None landed yet. Bench first; optimise only when a user reports a
slow build.
