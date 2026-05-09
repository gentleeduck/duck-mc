# Optimizations

Compact log of every perf knob landed so far. One bullet per change:
WHAT, WHY, COST/SAVING. No filler.

## Native transformers replacing sidecar JS plugins

- `pretty-code` (syntect). Replaces `rehype-pretty-code` + `shiki`. Why:
  shiki was ~80% of plugin time per file. Saving: ~40% on kitchen-sink.
- `math` (KaTeX via quick-js, with `MathEngine::Mathml` opt-out via
  pulldown-latex). Replaces `remark-math` + `rehype-katex`. Why: same JS
  plugin chain in process. Saving: ~150 ms per 1000 files in MathML mode;
  break-even in KaTeX mode (visual parity priority).
- `emoji` (`emojis` crate, unicode lookup). Replaces `remark-emoji`. Why:
  shortcode -> char is a 1-line table lookup. Saving: ~50 ms.
- `autolink-headings` (built-in transformer). Replaces `rehype-slug` +
  `rehype-autolink-headings`. Why: heading slug + anchor is one walk over
  the AST. Saving: ~150 ms.
- `gfm` (built into the dmc parser). Replaces `remark-gfm`. Why: tables,
  task lists, autolinks, strikethrough, all parsed natively. Saving:
  ~200 ms.

## Sidecar gate

- `CompileConfig::has_js_plugins` strips every plugin whose work is now
  done natively (`remark-gfm`, `remark-math`, `remark-emoji`,
  `rehype-pretty-code`, `shiki`, `rehype-katex`, `rehype-mathjax`,
  `rehype-slug`, `rehype-autolink-headings`) before deciding whether to
  spawn the sidecar. Why: a config that listed only those plugins now
  routes 100% native. Saving: full sidecar avoidance for typical configs
  (~600 ms per 1000 files).
- `effective_*_remark_plugins` / `effective_*_rehype_plugins` strip the
  same names from the JSON payload sent to the sidecar. Why: sidecar
  never re-does work. Saving: implicit (no double-render).

## Single-tokenize, multi-color highlighter

- `dmc_highlight::highlight_code_multi`. Why: multi-theme rendering used
  to call `highlight_code` once per theme; each call re-parsed the same
  scope stack. Now: one parse, N theme color resolutions. Saving:
  per-file syntect cost dropped 23% on the heavy fixture (1919 us ->
  1469 us) without giving up multi-theme parity.
- Adjacent same-style token merge in `highlight_code_multi`. Why: shiki
  coalesces adjacent same-color tokens; syntect emits one span per
  scope-change boundary. Merge step closes ~50% of the span-count gap.
  Saving: smaller HTML payload, downstream less to render.

## Caches

- `SyntaxBundle` (themes + grammars). Loaded once per process via
  `OnceLock`. Why: 25-100 ms one-time parse cost amortised across every
  build. Saving: ~50 ms per cold start after first.
- KaTeX `Opts` (display + inline). Cached once per process. Why: opts
  builder allocates JS handles; no need per math expression.
- Math render cache (in-memory + persisted). Keyed by
  `(latex, display, engine)`; persisted to
  `<output_dir>/.cache/math.json` between builds. Why: KaTeX via
  quickjs is 1-5 ms per expression, but most docs reuse the same
  formulae. Saving: subsequent renders of repeated math hit memory in
  microseconds.
- Mermaid SVG cache (in-memory + on-disk when `output_dir` set). Why:
  `mmdc` is multi-second per render. Cache key is `blake3(source)`.
- Per-file persistent compile cache. Path:
  `<output_dir>/.cache/dmc/{16-hex}.json`. Key:
  `blake3(dmc_version + source + path + cfg_fingerprint)`. Why: warm
  rebuilds skip lex + parse + transform + codegen + sidecar entirely.
  Saving: 3.55x speedup on warm rebuild (1187 ms -> 334 ms on the
  kitchen-sink demo). Disable via `cache_enabled: false`.

## Source preprocessing

- `Math::preprocess_source`. Rewrites `$...$` and `$$...$$` to
  `<MathMl/>` JSX before the lexer. Why: dmc parser interpreted `_` and
  `^` inside math as Markdown emphasis markers, shredding LaTeX. Now
  the parser sees an opaque tag. Cost: one extra string scan per file
  (~10 us/KB).

## Architectural / structural

- `Pipeline::with_defaults_for(cfg)`. Single uniform place where every
  feature-gated transformer registers. Why: avoids `cfg!(feature)`
  blocks scattered across consumers; no two-place wiring drift.
- `dmc-highlight` extracted as its own crate. Why: dmc-codegen needed
  the highlighter, dmc-transform needed it for `pretty-code`, and
  package-level cycles are forbidden in Cargo. Leaf crate breaks the
  cycle without duplicating the asset bundle.
- Whitespace tokens preserved by the lexer. Previously dropped as
  trivia. Why: text-after-link patterns like `[x](url) y` were losing
  the space, producing `xy` without break. Cost: JSX parser added a
  `skip_jsx_ws` helper; inline parser now handles `Whitespace` tokens.

## Open levers (not yet shipped)

- Batched sidecar IPC. Currently one Node round-trip per file. Batch
  50-100 in one request. Estimated saving: ~190 ms per 1000 files.
- Persistent sidecar daemon. Keep Node alive across builds. Estimated
  saving: ~100 ms cold start.
- Compile syntect grammars to `.packdump` binary blobs. Estimated
  saving: ~50 ms one-time startup.
- Rust-port autolink-headings, footnotes, etc. as more native passes
  whenever a user reports a JS plugin still in their chain.

## Bench numbers (kitchen-sink @ N=1000)

| build | dmc | velite | dmc faster |
|-------|-----|--------|------------|
| pre any of this        | 2666 ms | 1398 ms | **0.52x (slower)** |
| native shiki only      | 1520 ms | 1362 ms | 0.90x |
| + native math + emoji  |  886 ms | 1447 ms | 1.63x |
| + KaTeX (visual parity)| 1188 ms | 1368 ms | 1.15x |
| + gfm/slug/autolink gated | **144.77 ms** | 1381 ms | **9.5x** |
| warm cache hit         |  334 ms |  517 ms | 1.55x |

For the trivial `remark-gfm` case (single plugin), dmc is **12.8x
faster** than velite at 1000 files (462 ms vs 5910 ms).

Throughput: dmc kitchen-sink 6907 files/sec vs velite 724 files/sec.
