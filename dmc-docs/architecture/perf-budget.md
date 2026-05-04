# Per-stage perf budget

Where time goes in a build. Ordered by impact.

## Cold build (kitchen-sink, N=1000)

Total: ~145 ms native-only path.

| stage | time | % | notes |
|-------|------|---|-------|
| filesystem read | ~5 ms | 3 | 1000 small reads, async-friendly via rayon |
| math source preprocess | ~10 ms | 7 | byte scan; matches `$`/`$$` |
| lex | ~10 ms | 7 | byte-bound; ~1 GB/s |
| parse | ~25 ms | 17 | block + inline dispatch; majority of CPU |
| transform pipeline | ~50 ms | 35 | math + emoji + pretty-code dominate |
| codegen | ~20 ms | 14 | walker + 3 sinks |
| schema validate | ~5 ms | 3 | per-record |
| cache write | ~10 ms | 7 | 1000 small writes |
| index emit | ~5 ms | 3 | 2 small files |
| filesystem write JSON | ~5 ms | 3 | one big file |

## Warm build (cache hit on every file)

Total: ~334 ms (kitchen-sink demo, N=2 files).

| stage | time | % |
|-------|------|---|
| JS startup (tsx + module resolution) | ~280 ms | 84 |
| napi binary load | ~5 ms | 2 |
| read cache files | ~5 ms | 2 |
| serde parse cache files | ~10 ms | 3 |
| schema validate (skipped on cache hit) | 0 | 0 |
| index emit | ~5 ms | 2 |
| total inside dmc | ~50 ms | 15 |

Most warm-build cost is JS startup, not dmc itself. dmc-side warm
cost scales linearly with file count (~50 us / cache hit on a
modern SSD).

## Cold compile per file (kitchen-sink fixture)

| stage | per-file time |
|-------|--------------|
| math preprocess | ~10 us |
| lex | ~10 us |
| parse | ~30 us |
| transform | ~50 us |
| codegen | ~25 us |
| total | ~125 us |

Plus ~25 us KaTeX render per math expression (when present).
Plus ~150 us pretty-code render per code block (when present).

## Multi-theme cost

| fixture | single-theme | multi-theme (2 themes) |
|---------|-------------|----------------------|
| short ~80 B | 3 us | 3 us |
| medium ~1 KB | 305 us | 484 us |
| heavy ~2 KB | 895 us | 1469 us |

Multi-theme adds ~60% per file (not 100%) because parse + scope walk
runs once, then each theme contributes only color resolution.

## Math engine cost

| engine | per expression |
|--------|---------------|
| KaTeX (quick-js) | 1-5 ms |
| MathML (pulldown-latex) | ~10 us |
| KaTeX cached | ~5 us |
| MathML cached | ~5 us |

Switch to MathML to drop ~300 ms on a 1000-file kitchen-sink build.
Trade: native browser MathML rendering instead of KaTeX HTML.

## Sidecar overhead

| op | cost |
|----|------|
| spawn Node child | ~50-100 ms (cold) |
| per-file IPC round-trip | ~200 us |
| unified pipeline plugin run | varies (heavy plugins ~1-5 ms) |

When the gate strips every JS plugin, sidecar is never spawned.
Saves the spawn + every round-trip.

## Where memory goes

| component | rough cost |
|-----------|-----------|
| SyntaxBundle | 5-15 MB (one-time per process) |
| KaTeX quick-js | 1-3 MB (one-time) |
| Math render cache | grows with unique expressions; bounded |
| Mermaid SVG cache | grows with unique sources; bounded |
| Per-file AST | ~3x source size, freed at compile end |

A 1000-file watch-mode session levels off well under 100 MB total.

## Optimisation matrix (already shipped)

| opt | impact |
|-----|--------|
| native pretty-code | -40% on kitchen-sink |
| native math + emoji | -20% on kitchen-sink |
| plugin gate widening | -45% on kitchen-sink (drops sidecar entirely for default config) |
| persistent file cache | 3.55x on warm rebuild |
| persistent math cache | repeated math hits memory |
| single-tokenize multi-color | -25% per code block in multi-theme |

## Open levers (not shipped)

| lever | est saving |
|-------|-----------|
| Batched sidecar IPC | ~190 ms / 1000 files (only with foreign plugins) |
| Persistent sidecar daemon | ~100 ms cold start |
| syntect grammars to .packdump binary | ~50 ms one-time startup |
| PGO release build | 10-15% on hot paths |
| mmap large source files | ~5% on >100 KB MDX |

Run a profile (`cargo run --release --features pretty-code --example bench`)
before chasing any of these.

## Floor

Pure dmc parser only (no transformers, no codegen): ~45 ms /
1000 files. Anything beyond that is the cost of transformer +
codegen work the user actually asked for.
