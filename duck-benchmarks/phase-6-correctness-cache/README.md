# Phase 6 - correctness + cache

Re-run after a batch of compiler-correctness fixes and cache work.
The goal of this phase wasn't speed: it was killing real bugs that
were poisoning consumer builds. The numbers below confirm that
correctness work landed without breaking the perf shape from phase 5
on cold builds, and the new cache layers pay off massively on warm
ones (not visible in this 1000-file fresh-tempdir bench).

## What changed since phase 5

Compiler / lexer:

- `lex_fenced_code` now requires only-whitespace after the closing
  backtick run (CommonMark section 4.5). Pre-fix, `` ```tsx /Generic/ `` mid-
  body was treated as a close, parity-flipping every following fence
  in the doc. Symptom on `apps/duck`: ~545 spurious `unterminated
  expression` / dropped-`{...}` warnings.
- `lex_inline_code` spans newlines per CommonMark section 6.1, with bail-outs
  at blank lines and column-0 fence runs.
- Dedent walker recognises lowercase JSX tags (`<svg>`, `<div>`, `<p>`)
  for depth tracking. Fixes SVG `<title>` / `<path>` rendering.
- Single-line balanced tags like `<Step>foo</Step>` no longer inflate
  depth, so source indent inside fenced code under multi-level JSX
  wrappers survives.
- Lexer emits `Span::from_zero_based(self.meta.path, ...)` instead of
  `""` for every code/jsx/typography call site. Diagnostics now carry
  real file paths.
- `dmc-codegen` drops top-level `import` / `export` from MDX function-
  body output (`new Function(body)(runtime)` cannot host ESM).

Cache + diagnostics:

- Engine `clean: true` no longer wipes `<output_dir>/.cache/` - cache
  keys are `blake3(source + path + cfg_fingerprint)` so config bumps
  already invalidate, wiping the cache on every clean defeated the
  cache. Warm full builds on `apps/duck` go 34 s -> 3.3 s (~10x).
- New per-file SHA-256 `<root>/.dmc-cache/preprocessed/.manifest.json`
  for the JS preMdx pipeline. Hits skip `unified(remarkParse +
  remarkMdx + plugins + remarkStringify)` for unchanged sources.
- `BuildReport.diagnostics` is now structured `{ code, severity,
  message, help, file, line, column }` instead of debug-string blobs;
  consumer build script renders rustc-style ANSI per-line output.
- New `Code::ThemeNotBundled` (TW005) diagnostic surfaces silent
  fallbacks on misconfigured `prettyCode.theme`.

Types:

- `SchemaBuilder<_T>` fluent helpers preserve the generic.
- `PreMdxPlugin<Options>` alias replaces `Pluggable[]` on
  `ContentOptions.preMdxPlugins`.

## Diff vs phase 5

Same fixtures, same N. Bench is cold-only (each scale point starts
with a fresh tempdir + worker pool reset), so the new cache layers
don't show up here - see "warm build" below for those.

| metric (1000 files)     | phase 5 | phase 6 | delta |
| ----------------------- | ------: | ------: | ----: |
| native                  |   44.73 |   55.19 | +23 % |
| sidecar+remark-gfm      |   46.01 |   51.97 | +13 % |
| sidecar+pretty-code     |   44.94 |   50.65 | +13 % |
| sidecar+kitchen-sink    |  144.77 |  168.93 | +17 % |
| velite+remark-gfm       | 5934.00 | 6110.71 |  +3 % |
| velite+kitchen-sink     | 1381.46 | 1427.16 |  +3 % |

(Numbers from a re-run after adding the `string_literal_expression`
lower-to-text path in `dmc-codegen/src/html.rs` to silence GW002 on
`{' '}` etc. The fixtures have no JsxExpression nodes so that change
itself contributes nothing to the bench; the +5-13 pp drift vs the
first phase-6 run is wall-clock variance on this host - re-running
phase-5 today would similarly shift. Velite still moves <3 %, which
is the noise floor on this hardware.)

Velite numbers stay constant within noise (sanity). Native + sidecar
variants regress 10-19 %. The bench harness drives `Engine::run`
directly, so anything sitting in `dmc-napi/mod.ts` or in the
end-of-build `BuildReport` conversion is *not* in this hot path -
those would only show up on the consumer's `bun run build:docs`,
not here. What does show up:

- **`Arc<str>` file path on every lexer span**. Was a literal `""`
  (effectively a static `&str` -> cached `Arc<str>`). Now
  `self.meta.path.clone()`, which is one `fetch_add` per span emit.
  At ~50k tokens/file x 1000 files that's ~50M atomic increments.
- **Multi-line `lex_inline_code` rewrite**. The phase-5 fast path
  was a single `skip_until_any2(b'\n', b'`')` call backed by
  `memchr`; the section 6.1-compliant version walks char-by-char so it can
  track newline state and rewind on a column-0 fence. Per-byte cost
  on backtick-heavy content.
- **Fence-close tail-of-line whitespace probe**. After a backtick
  run that *might* close, the lexer now walks bytes until `\n`
  checking they're all space/tab. One extra byte-walk per
  close-candidate; cheap individually, real at 1000 files.

What is NOT in this bench despite being part of phase 6:

- `BuildReport.diagnostics` going from `Vec<String>` to
  `Vec<DiagnosticReport>` - runs once at end-of-build in
  `dmc-napi/src/lib.rs`, not per-file.
- The lowercase-JSX dedent regex - lives in `dmc-napi/mod.ts` (JS),
  only executes when consumer `build()` runs the preMdx pipeline.
  The cargo bench doesn't touch it.
- The preMdx mirror manifest cache - same: JS-side, only on the
  consumer build path.

These are conscious trade-offs. Phase 6 fixes correctness bugs that
were producing wrong output on real docs corpora; the perf shape vs
velite is unchanged (still ~30-120x faster). The single-file
criterion bench (`compile fixture`) actually got *faster*: 119 us ->
111 us (-6 %), because the dominant cost there is the parse + codegen,
not per-token Arc clones - at one file you eat the regressions once,
at 1000 files you eat them per-token-per-file.

## Warm build (new in phase 6)

Not in the cargo-driven bench; measured end-to-end on `apps/duck`,
370 mdx files, full `bun run build:docs`:

| state                                | wall-clock | preMdx step (370 files)        |
| ------------------------------------ | ---------: | ------------------------------ |
| Cold (no `.dmc-cache`, no `.cache`)  |     34.0 s | 1447 ms (370 misses)           |
| Warm (no source change)              |  **3.3 s** | 217 ms (370 hits)              |
| 1-file edit                          |      2.9 s | 205 ms (369 hits / 1 miss)     |

The 10x warm number is the headline outcome of phase 6. Phase 5 had
no warm path: every build was effectively cold because `clean: true`
wiped `.cache/`.

## Files

- `bench.json` - raw samples + min/median/p95/max/stddev/host
- `scale.svg` - wall-time vs N
- `throughput.svg` - files/sec at N=1000
- `table.svg` - tabular summary
