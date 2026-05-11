# Benchmarks

Parser microbenchmarks for `dmc-parser`, collected on 2026-05-11 with:

- OS: Linux 6.19.11-arch1-1 x86_64
- CPU: AMD Ryzen 9 9955HX (16C/32T, max 5.46 GHz)
- Rust: `rustc 1.95.0`
- Command: `CARGO_NET_OFFLINE=true cargo bench -p dmc-parser --bench parse`

Notes:

- Inputs are synthetic markdown corpora built from a repeated mixed-feature template.
- Sizes target roughly 1 KiB, 100 KiB, and 5 MiB.
- `criterion` reports throughput in MiB/s; the MB/s column below is the same midpoint converted to decimal MB/s.
- The optional `pulldown-cmark` comparison arm was skipped in this environment because adding a new lockfile dependency required a crates.io index refresh, which is unavailable offline.

## Results

| Benchmark | Input | Time / iter | Time / iter (ns) | Throughput (MiB/s) | Throughput (MB/s) |
| --- | --- | ---: | ---: | ---: | ---: |
| `dmc_parser::parse` | small (~1 KiB) | 23.075 µs | 23,075 | 50.091 | 52.5 |
| `dmc_parser::parse` | medium (~100 KiB) | 2.1458 ms | 2,145,800 | 45.607 | 47.8 |
| `dmc_parser::parse` | large (~5 MiB) | 160.62 ms | 160,620,000 | 31.132 | 32.6 |
| `parse + dmc_codegen::render_html` | small (~1 KiB) | 30.047 µs | 30,047 | 38.468 | 40.3 |
| `parse + dmc_codegen::render_html` | medium (~100 KiB) | 2.5543 ms | 2,554,300 | 38.313 | 40.2 |
| `parse + dmc_codegen::render_html` | large (~5 MiB) | 215.59 ms | 215,590,000 | 23.193 | 24.3 |

## Claim audit

No `500M tok/s` claim was present in the repo docs, README files, or source comments at the time of this run, so no wording change was needed.
