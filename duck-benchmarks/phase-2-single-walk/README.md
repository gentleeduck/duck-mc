# Phase 2 - single-walk pipeline

Native pipeline collapsed into one walk. `NodeSink` + `Walker` +
`Accumulator` merge what used to be separate lex, parse, transform,
emit traversals into a single pre-order DFS. Bench harness also gained
the kitchen-sink + velite reference + size sweep at this point
(commit `c5a77ca`).

## Diff vs phase 1

| metric (1000 files) | phase 1 | phase 2 | delta |
| --- | ---: | ---: | ---: |
| native                  |   10.65 |   11.92 |  +12% |
| sidecar+remark-gfm      |  404.86 |  446.19 |  +10% |
| sidecar+pretty-code     |  806.87 |  854.87 |   +6% |
| sidecar+kitchen-sink    | 2652.43 | 2666.42 |   +1% |
| velite+remark-gfm       | 5984.50 | 6135.08 |   +3% |
| velite+kitchen-sink     | 1375.06 | 1398.11 |   +2% |

Net effect inside the noise band. The single-walk refactor was a
correctness / maintainability win, not a perf win on its own. The
real gains come from phase 3 (syntect) and phase 5 (plugin gate),
both of which depend on this refactor to plug in cleanly.

## Numbers (median ms)

| variant | 10 files | 100 files | 1000 files |
| --- | ---: | ---: | ---: |
| native                  |  0.33 |   1.36 |   11.92 |
| sidecar+remark-gfm      | 11.65 |  60.33 |  446.19 |
| sidecar+pretty-code     | 21.90 | 100.44 |  854.87 |
| sidecar+kitchen-sink    | 44.14 | 299.30 | 2666.42 |
| velite+remark-gfm       | 332.33 | 905.63 | 6135.08 |
| velite+kitchen-sink     | 276.90 | 402.96 | 1398.11 |

## What changed at the source level

- `dmc-codegen::Walker` drives a single visit pass.
- `Accumulator` collects HTML + body strings as the walk emits.
- `Pipeline::with_defaults_for(cfg)` is a single uniform call point;
  every transformer is `Send + Sync` and runs `&self`.

## Plots

- `scale.svg`
- `throughput.svg`
- `table.svg`
