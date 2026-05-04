# Phase 3 - native syntect

`syntect` bundled inside the binary; the `pretty-code` transformer
runs in Rust. The shiki + rehype-pretty-code pair on the JS side no
longer has highlighting work to do for code blocks, so the
`sidecar+*` variants drop sharply on the highlighting-heavy
configurations.

## Diff vs phase 2

| metric (1000 files) | phase 2 | phase 3 | delta |
| --- | ---: | ---: | ---: |
| native                  |   11.92 |   47.42 | **+298%** (more native work) |
| sidecar+remark-gfm      |  446.19 |  507.41 |  +14% |
| sidecar+pretty-code     |  854.87 |  492.93 | **-42%** |
| sidecar+kitchen-sink    | 2666.42 |  885.74 | **-67%** |
| velite+remark-gfm       | 6135.08 | 6785.13 |  +11% (drift) |
| velite+kitchen-sink     | 1398.11 | 1446.52 |   +3% |

The native column gets slower because it now does the syntect parse +
multi-theme tokenisation per code block. That cost moved off the
sidecar, so the realistic `sidecar+kitchen-sink` workload drops 3x.

## Numbers (median ms)

| variant | 10 files | 100 files | 1000 files |
| --- | ---: | ---: | ---: |
| native                  |  1.09 |   5.28 |   47.42 |
| sidecar+remark-gfm      | 12.57 |  73.28 |  507.41 |
| sidecar+pretty-code     | 14.65 |  61.51 |  492.93 |
| sidecar+kitchen-sink    | 19.45 |  97.76 |  885.74 |
| velite+remark-gfm       | 367.38 | 939.41 | 6785.13 |
| velite+kitchen-sink     | 294.61 | 419.02 | 1446.52 |

## What changed at the source level

- `dmc-highlight` crate added. Holds the `SyntaxBundle` (grammars +
  themes) behind a `OnceLock`.
- `dmc-transform::PrettyCode` does its own multi-theme tokenisation
  (single tokenise, N colour resolutions) and emits the dmc-namespaced
  output attributes (`data-dmc-figure`, `--dmc-{mode}-bg` etc).
- The sidecar still runs but with `rehype-pretty-code` skipped, so its
  remaining work is just whatever foreign plugins remain.

## Plots

- `scale.svg`
- `throughput.svg`
- `table.svg`
