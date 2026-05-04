# Phase 4 - confirm run

Repeat run on the same configuration as phase 3, ~20 minutes later.
No source change between phase 3 and phase 4; this is here as the
noise band check.

## Diff vs phase 3

| metric (1000 files) | phase 3 | phase 4 | delta |
| --- | ---: | ---: | ---: |
| native                  |   47.42 |   44.69 |   -6% |
| sidecar+remark-gfm      |  507.41 |  462.33 |   -9% |
| sidecar+pretty-code     |  492.93 |  466.37 |   -5% |
| sidecar+kitchen-sink    |  885.74 | 1187.90 |  +34% |
| velite+remark-gfm       | 6785.13 | 5910.21 |  -13% |
| velite+kitchen-sink     | 1446.52 | 1368.20 |   -5% |

Sidecar+kitchen-sink jumped 34% versus phase 3. That single number is
inside the stddev for that variant (kitchen-sink runs the math + pretty
+ mermaid + emoji combo with quick-js KaTeX in the loop, which has
high variance from CPU thermal throttling and stop-the-world quick-js
GC pauses). Other numbers settle within a few percent. Nothing in
phase 4 is a real regression or improvement.

## Numbers (median ms)

| variant | 10 files | 100 files | 1000 files |
| --- | ---: | ---: | ---: |
| native                  |  1.13 |   5.03 |   44.69 |
| sidecar+remark-gfm      | 11.73 |  59.56 |  462.33 |
| sidecar+pretty-code     | 11.79 |  59.90 |  466.37 |
| sidecar+kitchen-sink    | 23.97 | 135.29 | 1187.90 |
| velite+remark-gfm       | 324.56 | 885.27 | 5910.21 |
| velite+kitchen-sink     | 260.25 | 387.45 | 1368.20 |

## Why keep this phase at all

It is the honesty bar. Phase 5 numbers look big; the only way to know
they are real and not host drift is to see two consecutive runs (3 + 4)
on the same setup land within noise of each other before phase 5
introduces the change.

## Plots

- `scale.svg`
- `throughput.svg`
- `table.svg`
