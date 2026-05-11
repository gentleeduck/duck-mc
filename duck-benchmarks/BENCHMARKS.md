# Benchmarks

Parser microbenchmarks for `dmc-parser`, collected on 2026-05-11 with:

- OS: Linux 6.19.11-arch1-1 x86_64
- CPU: AMD Ryzen 9 9955HX (16C/32T, max 5.46 GHz)
- Rust: `rustc 1.95.0`
- Command: `cargo bench -p dmc-parser --bench parse`

Notes:

- Inputs are synthetic markdown corpora built from a repeated mixed-feature template (`dmc-parser/benches/parse.rs`).
- Sizes target roughly 1 KiB, 100 KiB, and 5 MiB.
- `criterion` reports throughput in MiB/s; the MB/s column below is the same midpoint converted to decimal MB/s (MB/s = MiB/s x 1.048576).
- The optional `pulldown-cmark` comparison arm is skipped in this environment because adding a new lockfile dependency requires a crates.io index refresh, which is unavailable offline.
- This run reflects the post-G2/G8 tree: the miri-clean `raw_source_for_token_range` reslice and the three fuzz DoS fixes (nested-`[` link-label recursion bound, list-item code-block tab loop, bare-url slice) are all in place. Within run-to-run noise of the prior numbers - no regression.

## Results

| Benchmark | Input | Time / iter | Time / iter (ns) | Throughput (MiB/s) | Throughput (MB/s) |
| --- | --- | ---: | ---: | ---: | ---: |
| `dmc_parser::parse` | small (~1 KiB) | 23.199 us | 23,199 | 49.82 | 52.2 |
| `dmc_parser::parse` | medium (~100 KiB) | 2.1607 ms | 2,160,700 | 45.29 | 47.5 |
| `dmc_parser::parse` | large (~5 MiB) | 160.98 ms | 160,980,000 | 31.06 | 32.6 |
| `parse + dmc_codegen::render_html` | small (~1 KiB) | 30.156 us | 30,156 | 38.33 | 40.2 |
| `parse + dmc_codegen::render_html` | medium (~100 KiB) | 2.5831 ms | 2,583,100 | 37.89 | 39.7 |
| `parse + dmc_codegen::render_html` | large (~5 MiB) | 217.85 ms | 217,850,000 | 22.95 | 24.1 |

### Previous run (for reference)

| Benchmark | Input | Time / iter | Throughput (MB/s) |
| --- | --- | ---: | ---: |
| `dmc_parser::parse` | small | 23.075 us | 52.5 |
| `dmc_parser::parse` | medium | 2.1458 ms | 47.8 |
| `dmc_parser::parse` | large | 160.62 ms | 32.6 |
| `parse + render_html` | small | 30.047 us | 40.3 |
| `parse + render_html` | medium | 2.5543 ms | 40.2 |
| `parse + render_html` | large | 215.59 ms | 24.3 |

## Full `dmc-core` compile pipeline

`cargo bench -p dmc-core --bench compile` - drives `Compiler::compile`
(lex -> parse -> default transform pipeline -> HTML + MDX codegen + metadata
/ TOC / excerpt extraction), plus a parse-only baseline on the same input.

| Benchmark | Input | Time / iter |
| --- | --- | ---: |
| `compile fixture` (frontmatter + headings + lists + fenced code, ~260 B) | full pipeline | 121.63 us |
| `compile simple` (`# Hello\n\nworld\n`, 15 B) | full pipeline | 5.163 us |
| `parse fixture` (same fixture as above) | parse only | 3.079 us |

So end-to-end compile is ~40x the bare parse on the fixture: the cost is
the transform-pipeline setup (incl. syntax-highlighter load), the passes
themselves, and rendering the document twice (HTML + MDX) on top of
deriving metadata/TOC/excerpt. Throughput at this fixture size is ~2 MB/s
for full compile vs ~85 MB/s for parse-only - for bulk parsing use
`dmc_parser::parse` directly; `Compiler::compile` is the one-shot
"render this MDX file" entry point.

## Claim audit

No `500M tok/s` claim is present in the repo docs, README files, or source comments, so no wording change was needed.

## Where the time goes (cost centers, unoptimized)

- Token `Vec<Token>` heap allocation in the lexer.
- Owned `String` `Text` node values + reallocating `unescape_markdown` / `decode_entities_in` (they early-return-then-`to_string()`, so even no-op cases allocate once).
- `Span` carrying an `Arc<str>` path (`.clone()` is an atomic refcount bump).
- `Vec<Node>` AST plus drain/insert churn in the emphasis resolver.
- In-place token mutation (`try_promote_text_*`) on list / blockquote recovery paths.

Open perf work (G9, not yet done): `Cow<'src, str>` for `Text` values (needs `'src` threaded through the AST - invasive), `smallvec` for the hottest inline child vectors, then re-bench.

> We are not going to optimize more thus we already solved the problem, if you want to do it, i can
> help you by pointing to the right direction.

The full per-crate catalogue of remaining optimizations (token streaming, alloc-free text, path interning, syntect output caching) and the "done wrong due to timeline" debt list live in [`OPTIMIZATIONS.md`](OPTIMIZATIONS.md). To record or validate a new bench run, see [`GUIDE.md`](GUIDE.md).
