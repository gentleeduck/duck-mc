# Phase 1 - baseline

First recorded bench. Pipeline is multi-pass (separate lex, parse,
transform, emit walks). Sidecar runs every JS remark / rehype plugin
listed in the config without any native short-circuit.

## What dmc was doing

- Native: lex + parse + native transformers + HTML emit. No syntect; no
  math engine; no plugin gate.
- Sidecar: one Node child per call (no daemon yet). For each `+gfm`,
  `+pretty-code`, `+kitchen-sink` variant, the entire JS plugin chain
  runs.
- Velite: shells out to the velite CLI as a parity reference.

## Numbers (median ms, lower is better)

| variant | 10 files | 100 files | 1000 files |
| --- | ---: | ---: | ---: |
| native                  |  0.36 |   1.24 |   10.65 |
| sidecar+remark-gfm      | 13.58 |  58.62 |  404.86 |
| sidecar+pretty-code     | 21.42 | 102.65 |  806.87 |
| sidecar+kitchen-sink    | 40.42 | 302.66 | 2652.43 |
| velite+remark-gfm       | 330.43 | 895.31 | 5984.50 |
| velite+kitchen-sink     | 260.12 | 380.61 | 1375.06 |

## Headline

`sidecar+kitchen-sink @ 1000` = **2652 ms**. The realistic
"markdown + GFM + math + pretty-code" workload. This is the number
later phases attack.

## Host

32 CPU, x86_64, linux. Same host across every phase.

## Plots

- `scale.svg` - log-log scaling
- `throughput.svg` - files / sec
- `table.svg` - the table above as an SVG
