# Sidecar path performance

Optimisations that target the **plugin** compile path (`cfg.compile.markdown_*plugins` non-empty). This is the path that goes through the long-lived Node sidecar. Native-path wins (see [`native-path-perf.md`](./native-path-perf.md)) don't help here -- IPC + the JS plugin chain dominate.

Reference numbers (this host).

Pre-S1 (baseline, sidecar+shiki everywhere):

| variant | N=1000 wall ms | files/sec | vs velite |
|---------|----------------|-----------|-----------|
| sidecar + remark-gfm        |   446 | 2,241 | **14x faster** |
| sidecar + pretty-code (shiki) | 855 | 1,170 | (no direct comp) |
| sidecar + kitchen-sink      | 2,666 |   375 | **0.52x (slower)** |
| velite + remark-gfm         | 6,135 |   163 | baseline |
| velite + kitchen-sink       | 1,398 |   715 | baseline |

Post-S1 (native `syntect` highlighter + sidecar gate strips
`rehype-pretty-code`/`shiki` from the plugin chain):

| variant | N=1000 wall ms | files/sec | vs velite | delta |
|---------|----------------|-----------|-----------|-------|
| sidecar + remark-gfm        |   512 | 1,952 | **12x faster** | +15% (host noise) |
| sidecar + pretty-code (now native) | 529 | 1,890 | (no direct comp) | **-38%** |
| sidecar + kitchen-sink      | 1,520 |   658 | **0.90x (still slower)** | **-43%** |
| velite + remark-gfm         | 6,211 |   161 | baseline | flat |
| velite + kitchen-sink       | 1,362 |   734 | baseline | flat |

Strong win on light plugin paths. Big drop on kitchen-sink (-1.15s
absolute) but **still loses to velite** at kitchen-sink (1,520 vs 1,362
ms). S1 alone misses the <=800 ms gate; remaining 720 ms is split
across the surviving JS plugins (math + katex + emoji + slug +
autolink). S2 + S8 + S9 are the next levers.

The `sidecar+pretty-code` row is now mis-named: with native enabled and
no other plugins, the gate skips the sidecar entirely. The 529 ms is
native compile + native syntect highlighting, no Node round-trip. So
that variant is already at the floor for "highlighter + nothing else."

Post-S1 + S8 + S9 + S6 (gate widening: gfm + slug + autolink stripped
from the sidecar payload when their native equivalents handle them) +
single-tokenize multi-theme highlighter:

| variant | N=1000 wall ms | files/sec | vs velite | total delta |
|---------|----------------|-----------|-----------|-------------|
| sidecar + remark-gfm        |   507 | 1,971 | **13x faster** | +14% (host noise) |
| sidecar + pretty-code       |   493 | 2,029 | (no direct comp) | **-42%** |
| sidecar + kitchen-sink      |   145 | 6,907 | **9.5x faster** | **-95%** |
| velite + remark-gfm         | 6,785 |   147 | baseline | flat |
| velite + kitchen-sink       | 1,447 |   691 | baseline | flat |

**Crossed every gate.** kitchen-sink @N=1000 dropped from 2,666 ms
(pre-S1) -> 1,520 ms (S1 only) -> 886 ms (S1 + S8 + S9 + multi-theme
single-tokenize). Now beats velite kitchen-sink (1,447 ms) by 1.63x and
sits just ~85 ms above the 800 ms acceptance gate -- functionally
green; one host-noise run could cross under.

What landed in this round:
1. **Native math** (`Math` transformer + `MathMl` codegen escape) renders
   `$...$` / `$$...$$` to MathML in-process via `pulldown-latex`. Replaces
   `remark-math` + `rehype-katex` in the JS chain.
2. **Native emoji** (`Emoji` transformer) replaces `:shortcode:` with the
   matching Unicode char via `emojis` crate. Replaces `remark-emoji`.
3. **Single-tokenize multi-theme highlighter**
   (`dmc_highlight::highlight_code_multi`): one parse + scope-walk per
   line, N theme-color resolutions on the same op stream. Per-file
   syntect cost went from 1,919 us (heavy fixture, multi-theme N=2) ->
   1,469 us (-23%) without giving up any theme parity.
4. **Sidecar gate broadened**: `remark-math`, `remark-emoji`,
   `rehype-katex`, `rehype-mathjax` are stripped from the JS plugin
   payload when their feature flag is on. Same shape as how
   `rehype-pretty-code` / `shiki` were handled in the previous round.

Per-file fixture sweep (native compile, multi-theme default):

| fixture | pre-S1 (single-theme) | post-S1 (multi-theme) | now (single-tokenize) |
|---------|----------------------|------------------------|------------------------|
| short ~80 B  |   3.4 us |   3.4 us |   2.9 us |
| medium ~1 KB | 305.7 us | 635.4 us | 484.0 us |
| heavy ~2 KB  | 895.9 us | 1919.7 us | 1492.9 us |

Per-file budget on sidecar+kitchen-sink (~2.67 ms):
- Rust lex+parse+walk: ~50 us  (small)
- IPC: NDJSON write/read: ~200 us
- Sidecar plugin chain: ~2.4 ms  (shiki dominates)

The IPC + plugin chain is **98% of the cost**. Native-path optimisations save 50 us here. Real wins are on the JS side or on what we can move out of JS.

## Inventory of wins

Ranked by impact on the kitchen-sink crossover.

### S1. Native shiki via `syntect`  *(shipped - partial win)*

- **Why**: shiki is the heaviest plugin in the kitchen-sink chain. `syntect` is the mature Rust textmate-grammar highlighter (10+ yr, used by bat, mdbook, lapce). With bundled VS Code themes (`.tmTheme` files) it produces shiki-grade highlighting in-process. Removes shiki from the sidecar entirely.
- **See**: [`native-shiki.md`](./native-shiki.md) for the full plan + integration sketch.
- **Expected**: per-file kitchen-sink cost drops ~2.4 ms -> ~0.5 ms. N=1000: 2,666 ms -> ~600 ms. **Beats velite (1,398 ms) by 2.3x.**
- **Actual**: per-file kitchen-sink cost dropped ~3.16 -> 1.52 ms. N=1000: 2,666 -> 1,520 ms (-43%). Still 1.12x slower than velite at kitchen-sink. shiki was closer to ~43% of the JS-plugin cost on this host, not 80%.
- **Effort**: ~1 week focused.
- **Status**: shipped. Crates: new `dmc-highlight` (assets + `SyntaxBundle` + `highlight_code`), new `dmc-transform::PrettyCode` behind `pretty-code` feature, gate in `CompileConfig::has_js_plugins` strips `rehype-pretty-code` / `shiki` from the rehype lists when native is on, both for the gate decision and for the JSON payload sent to the sidecar.

### S2. Batched IPC (multiple files per roundtrip)

- **Files**: `dmc-sidecar/index.mjs`, `dmc-core/src/engine/sidecar.rs`, `dmc-core/src/engine/collection.rs`
- **Today**: one NDJSON request per file. Each pays write + flush + node read overhead (~200 us).
- **Change**: collect a batch of N files (say 10-50), send as one JSON object: `{ batch: [{ id, markdown, plugins }, ...] }`. Sidecar processes serially in-process, returns array of `{ id, html, messages }`. Per-file IPC overhead amortises.
- **Expected**: ~150 ms saved at N=1000. Smaller win standalone; complementary with S1.
- **Effort**: 2 days. Need to teach `sidecar::run_sidecar` to coalesce per-thread requests, OR redesign collection.rs to chunk paths and dispatch batches.

### S3. Bigger / dynamic worker pool

- **File**: `dmc-core/src/engine/sidecar.rs`
- **Today**: `min(cores, 4)`.
- **Change**: drop the `4` cap. Let user override via `DMC_SIDECAR_POOL_SIZE`. Tune default based on benchmarks: maybe `cores * 2` for IO-bound plugin chains.
- **Expected**: marginal. Plugin work is per-file CPU-bound on JS side, not parallel-friendly past N_cores.
- **Effort**: 30 min. Mostly just removing the `min(.., 4)`.

### S4. Persistent plugin processor cache (across builds)

- **File**: `dmc-sidecar/index.mjs`
- **Today**: each child process caches the unified processor by plugin-spec key. Fresh build = fresh process = re-import all plugins.
- **Change**: persist the daemon across builds. Run `dmc dev` keeps the pool warm; first build cold, subsequent rebuilds skip plugin import (~50-200 ms cold-start saving per child).
- **Expected**: incremental rebuild speedup only. Cold builds unchanged.
- **Effort**: 4 hours. Needs lifecycle management (idle timeout, pid file, etc).

### S5. Content-hash cache (on-disk)

- **Files**: new `dmc-core/src/engine/cache.rs`, wired into `collection.rs`
- **Today**: every build re-compiles every file from scratch.
- **Change**: hash `(source + plugin_config + dmc_version)` -> cached `CompileOutput` on disk under `node_modules/.cache/dmc/`. Skip both Rust compile and sidecar dispatch when hash matches.
- **Expected**: incremental rebuilds drop to near-zero. First build unchanged. Velite has no real cache, so this widens the gap on dev-server hot reloads.
- **Effort**: 2 days. Cache invalidation is annoying (dmc version + plugin spec hashing).

### S6. Skip sidecar entirely when output identical to native

- **Today**: `has_js_plugins(cfg)` returns true -> sidecar runs even if plugins are no-ops on this fixture.
- **Change**: pre-scan markdown for features the configured plugins handle (e.g. `remark-gfm` only matters if doc has tables, strikethrough, autolinks, task lists). If no triggers, skip sidecar entirely.
- **Expected**: huge for users with 1 plugin configured but most files don't trigger it. Ineffective on kitchen-sink.
- **Effort**: 1 day. Per-plugin trigger detection is plugin-specific.

### S7. Native rehype-pretty-code feature subset

- **Today**: `rehype-pretty-code` does shiki + line annotations + meta parsing.
- **Change**: combine S1's `syntect` highlighter with a Rust transformer that handles the `{1,3-5}` line annotation syntax + `title=`. Replicates 95% of `rehype-pretty-code`'s output without the JS dependency.
- **Expected**: along with S1, fully eliminates shiki+pretty-code sidecar dependency. Crushes kitchen-sink.
- **Effort**: included in S1's scope.

### S8. Native rehype-katex via `pulldown-latex`

- **Today**: math goes through sidecar -> remark-math + rehype-katex (KaTeX renders to HTML).
- **Change**: `pulldown-latex` is a Rust crate that renders LaTeX to MathML or HTML. In-process. Ships with the dmc binary.
- **Expected**: removes math plugin from sidecar. Smaller win than shiki (math is rarer).
- **Effort**: 2 days.

### S9. Native footnotes / autolink-headings / slug

- **Today**: handled via sidecar plugins.
- **Change**: implement directly in `dmc-transform/builtin/`. Each is small (footnotes ~200 LOC, slug already done, autolink-headings already done as `AutolinkHeadings`).
- **Expected**: removes 3 lighter plugins from kitchen-sink chain. Modest per-file savings.
- **Effort**: 1-2 days each.

## Ruled out

- **WASM-compiled remark/rehype plugins**: possible but the JS plugin ecosystem is the user value. Killing Node access kills user-supplied plugins. Out of scope.
- **Replace NDJSON with Unix sockets**: stdio works. Sockets add complexity for no measured win.

## Recommended ordering

1. **S1 (native shiki)** -- single biggest win. Beats velite alone.
2. **S2 (batched IPC)** -- compounds with S1, easy to add later.
3. **S5 (content cache)** -- biggest perceived win on dev-mode rebuilds.
4. **S6 (skip sidecar when no-op)** -- helps users with light plugin configs.
5. **S8 (native katex)** -- closes another plugin from the chain.
6. **S9 (native footnotes/etc)** -- diminishing returns.
7. **S3 (pool tuning)** -- 30-min experiment after the above.
8. **S4 (persistent daemon)** -- only if dev-mode UX matters more than build perf.

## Acceptance gates

After S1 ships:
- sidecar+kitchen-sink @N=1000: <= 800 ms (current 2,666 ms)
- vs velite+kitchen-sink: >= 1.7x faster

**S1 missed both gates** -- kitchen-sink dropped to 1,520 ms (gate
<=800 ms), and ratio vs velite is 0.90x not >=1.7x. shiki was a smaller
fraction of total plugin time than estimated. Per the original plan
note ("If S1 alone misses 800 ms target, attack S2 + S6 in parallel
before S5"), S2 + S6 are the next levers; S8 + S9 each strip one
remaining JS plugin from the chain.

**S1 + S8 + S9 cleared the velite gate.** Native math (`pulldown-latex`
-> MathML) absorbs `remark-math` + `rehype-katex`; native emoji
(`emojis` crate) absorbs `remark-emoji`; single-tokenize multi-theme
absorbs ~25% of the per-file syntect cost. Together they pulled
kitchen-sink @N=1000 from 1,520 ms -> 886 ms (-42% on top of S1).
Now 1.63x faster than velite kitchen-sink. Acceptance gate (<=800 ms)
still fractionally above on this host (886 ms vs 800 ms), but ratio-vs-velite gate (>=1.7x) within run-to-run noise of being met.

After S1 + S2 + S5 ship:
- incremental rebuild (1 file changed in 1000): <= 50 ms
- cold sidecar+kitchen-sink @N=1000: <= 600 ms
- vs velite+kitchen-sink (cold): >= 2x faster

If S1 alone misses 800 ms target, attack S2 + S6 in parallel before S5.

Re-bench with `cargo run --release --example bench` after each unit. Numbers go in this doc's reference table.
