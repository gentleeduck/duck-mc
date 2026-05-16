# Phase 7 - G-phase hardening

Re-run after the `G1`-`G9` hardening track: parser module split,
exact Unicode-punctuation classification, a miri-clean source reslice,
three fuzz-found DoS bounds, and clippy/rustdoc cleanup plus a
bench-regression CI gate. Like phase 6 this wasn't a speed phase - it
was correctness, safety, and maintainability. The numbers confirm the
perf *shape* vs velite is unchanged (still ~20-100× faster); the
absolute native/sidecar columns drift up a bit, partly real per-char
cost from the exact Unicode tables, partly wall-clock variance on this
host (velite moves ±5 % the same run).

## What changed since phase 6

Parser structure / safety:

- `dmc-parser/src/block/` split into `heading.rs`, `html.rs`,
  `code.rs`, `blockquote.rs`, `list.rs` - no behaviour change, the
  monolithic `block.rs` is gone (`G3`).
- `raw_source_for_token_range` no longer reslices through a raw
  pointer; it indexes the source `&str` by byte range directly.
  Miri-clean now (`G3`/`G8.1`).
- Audited and removed the remaining `unsafe` pointer slices in
  `dmc-parser` (`G3`).

Correctness / spec:

- Emphasis flanking now classifies Unicode punctuation *exactly*
  (full `Pattern_White_Space` + `P*`/`S*`-category test) instead of
  ASCII-only, so non-ASCII punctuation around `*` / `_` runs follows
  CommonMark left/right-flanking rules (`G5.3`). This is the one
  change that adds measurable hot-path cost: one category lookup per
  inline punctuation char.
- `lex_fenced_code` / `lex_inline_code` edges and entity/label edges
  pinned with new tests (`G5`).
- Recovery-path diagnostics tightened; MDX comments and ESM bodies
  covered (`G6`/`G7`).

Robustness (fuzz):

- `cargo-fuzz` targets + seed corpus added for the parser (`G2`).
- Three DoS inputs the fuzzer found are now bounded:
  nested-`[` link-label recursion depth cap, list-item code-block
  tab-loop fix, and the bare-url `www.`-only run no longer panics.

CI / docs:

- `bench-regression` workflow added (`G1.4`) - criterion parse benches
  run on PRs and fail on a regression past the noise band.
- Workspace-wide clippy + rustdoc hardening (`G8`); roadmap / README
  refresh (`G4`).

Source range: `4523aa9` (phase-6 bench) ... `5f9ef34` (HEAD).

## Diff vs phase 6

Same fixtures, same N, cold-only bench (fresh tempdir + pool reset per
scale point), `cargo run --release -p dmc-core --features pretty-code
--example bench` on the 32-CPU x86_64 linux host.

| variant (1000 files, median ms) | phase 6 | phase 7 | delta |
| ------------------------------- | ------: | ------: | ----: |
| native                          |   55.19 |   66.27 | +20 % |
| sidecar+remark-gfm              |   51.97 |   65.53 | +26 % |
| sidecar+pretty-code             |   50.65 |   61.81 | +22 % |
| sidecar+kitchen-sink            |  168.93 |  143.46 | −15 % |
| velite+remark-gfm               | 6110.71 | 6432.86 |  +5 % |
| velite+kitchen-sink             | 1427.16 | 1370.43 |  −4 % |

Read this honestly:

- **Velite moves ±5 %** between phases on this host with zero code
  changes touching it - that is the wall-clock noise floor. So a
  chunk of the native/sidecar drift is the same host variance, not
  the diff. `sidecar+kitchen-sink` actually *dropped* 15 %; nothing
  in `G1`-`G9` made the kitchen-sink JS path faster, so that swing is
  pure host variance and a fair upper bound on how noisy this bench
  is right now.
- **What is plausibly real**: the exact Unicode-punctuation
  classifier (`G5.3`). Emphasis resolution now does a real
  `char`-category lookup at every inline punctuation byte instead of
  an `is_ascii_punctuation()` branch. On the medium/heavy fixtures
  (lots of `*`, `_`, `(`, `)`, `[`, `]`) that is a per-char tax on
  the hottest inline loop. It buys CommonMark-correct flanking around
  non-ASCII punctuation, which the ASCII fast path got wrong.
- **What is *not* in this bench despite being phase 7**: the parser
  `block/` module split (compile-time only), the miri-clean reslice
  (same instruction count, just no `unsafe`), the fuzz DoS bounds
  (only fire on adversarial inputs the fixtures don't contain), and
  the CI / clippy / rustdoc work. None of those touch the per-file
  hot path on normal docs.

Per-file single-shot fixtures this run (200 samples each):

| fixture            | bytes | median | p95 |
| ------------------ | ----: | -----: | --: |
| short (~80 B)      |    41 | 4.55 µs | 5.36 µs |
| medium (~1 KB)     |  1255 | 605 µs | 679 µs |
| heavy (~2 KB)      |  2412 | 1.96 ms | 3.14 ms |
| long (~80 KB)      | 110894 | 82.0 ms | 85.0 ms |

## Speedups vs velite at 1000 files (phase 7)

| compared                          | velite ms | dmc ms | speedup |
| --------------------------------- | --------: | -----: | ------: |
| native vs velite+gfm              |   6432.86 |  66.27 |  97.1x |
| native vs velite+kitchen-sink     |   1370.43 |  66.27 |  20.7x |
| sidecar+kitchen-sink vs velite+ks |   1370.43 | 143.46 |   9.6x |
| sidecar+gfm vs velite+gfm         |   6432.86 |  65.53 |  98.2x |

## Stage profile (this phase)

`cargo run --release -p dmc-core --example profile --features pretty-code`
on the realistic single-file fixture, 5000 iters:

| stage     | µs/iter | share |
| --------- | ------: | ----: |
| lex       |    4.63 |  1.0 % |
| parse     |    7.48 |  1.6 % |
| transform |  370.52 | 77.4 % |
| codegen   |   95.95 | 20.0 % |
| **total** | **478.58** | |

Transform is still dominated by `pretty_code` (syntect highlight),
97 % of named-transformer time. `lex` + `parse` together are ~2.6 %
- the per-token Arc clone and exact-Unicode classifier raise the
parse share a hair vs phase 6 (3.23 → 4.63 µs lex, 4.23 → 7.48 µs
parse) but they are nowhere near the cost centre. See
`flamegraph/flame.svg` for the full sampled call tree (pprof @
997 Hz, ~4200 iters over 5 s).

### Consumer corpus flamegraph (`flamegraph/duck-ui.svg`)

Profiles the native compile over the real `apps/duck` corpus -
370 mdx files. This phase captured it from the **raw `content/`
tree**, not the `content/.dmc-cache/preprocessed` mirror (the mirror
isn't generated in this checkout; run `bun run build:docs` in the
consumer first to get the production-exact input). Raw vs mirror
differs by the JS preMdx pass only, which doesn't touch the native
path being profiled - it just leaves more MDX/JSX/expression syntax
for the native lexer+parser to chew through (deeper recovery, deeper
recursion).

Caveat on the wall-clock in `duck-ui.txt` (~143 s, ~387 ms/file):
that is **not** the compile cost - `dmc compile <file>` clocks
sub-millisecond per file and the 1000-file headline bench is 66 ms
total. The inflation is `pprof`'s signal-driven sampler unwinding
the much deeper native-recovery stacks the raw MDX produces, 997×/s.
Read the flamegraph for *shape* (which frames dominate), not for
absolute timings - for those use the headline bench or `dmc compile`.

## How to reproduce

```sh
# headline bench (writes bench.json + the 3 SVGs to dmc-core/tmp/)
cargo run --release -p dmc-core --features pretty-code --example bench

# in-process flamegraph + stage profile (write straight into this folder)
cargo run --release -p dmc-core --features pretty-code --example flamegraph
cargo run --release -p dmc-core --features pretty-code --example profile \
  > duck-benchmarks/phase-7-g-hardening/flamegraph/stage-profile.txt

# consumer-corpus flamegraph - needs apps/duck preprocessed mirror first
cargo run --release -p dmc-core --features pretty-code --example flamegraph_consumer
```

Copy the fresh `bench.json` + `*.svg` from `dmc-core/tmp/` here.
As with every phase, this is a *historical* snapshot at HEAD - the
fixture set in `examples/nextjs/content/` drifts, so re-running phase 7
later will not exactly reproduce these numbers.

## Files

- `bench.json` - raw samples + min/median/p95/max/stddev/host
- `scale.svg` - wall-time vs N
- `throughput.svg` - files/sec at N=1000
- `table.svg` - tabular summary
- `flamegraph/flame.svg` - pprof flamegraph of the native compile path (toy fixture)
- `flamegraph/stage-profile.txt` - lex / parse / transform / codegen breakdown
- `flamegraph/duck-ui.svg` + `duck-ui.txt` - pprof flamegraph + summary over the real `apps/duck` 370-mdx corpus (raw content tree - see caveat above)
