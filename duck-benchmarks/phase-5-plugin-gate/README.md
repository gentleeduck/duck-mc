# Phase 5 - plugin gate widening

The big drop. The sidecar payload now strips every JS plugin whose
work is owned by a native transformer:

- `remark-gfm` (parser already does GFM)
- `remark-math` (Math transformer)
- `remark-emoji` (Emoji transformer)
- `rehype-pretty-code`, `shiki` (PrettyCode transformer)
- `rehype-katex`, `rehype-mathjax` (Math transformer)
- `rehype-slug`, `rehype-autolink-headings` (AutolinkHeadings transformer)

When the user's config lists only those plugins, the sidecar runs but
its plugin chain is empty, so it becomes a near no-op forwarder. When
the user lists a foreign plugin (e.g. `rehype-mermaidjs-bundled`), the
sidecar still runs that one but skips the native-owned ones.

## Diff vs phase 4

| metric (1000 files) | phase 4 | phase 5 | delta |
| --- | ---: | ---: | ---: |
| native                  |   44.69 |   44.73 |   ~0% |
| sidecar+remark-gfm      |  462.33 |   46.01 | **-90%** |
| sidecar+pretty-code     |  466.37 |   44.94 | **-90%** |
| sidecar+kitchen-sink    | 1187.90 |  144.77 | **-88%** |
| velite+remark-gfm       | 5910.21 | 5934.00 |   ~0% |
| velite+kitchen-sink     | 1368.20 | 1381.46 |   ~0% |

Velite numbers stay constant (sanity); native stays constant; the
sidecar variants drop ~10x because the JS work is now zero where the
native already did it.

## Numbers (median ms)

| variant | 10 files | 100 files | 1000 files |
| --- | ---: | ---: | ---: |
| native                  |  0.92 |   5.24 |   44.73 |
| sidecar+remark-gfm      |  1.16 |   5.29 |   46.01 |
| sidecar+pretty-code     |  1.08 |   5.31 |   44.94 |
| sidecar+kitchen-sink    |  3.12 |  15.16 |  144.77 |
| velite+remark-gfm       | 323.06 | 904.79 | 5933.99 |
| velite+kitchen-sink     | 267.19 | 394.11 | 1381.46 |

## Speedups vs velite at 1000 files

| compared | velite ms | dmc ms | speedup |
| --- | ---: | ---: | ---: |
| native vs velite+gfm                | 5934.00 | 44.73   | **132x** |
| native vs velite+kitchen-sink       | 1381.46 | 44.73   |  **30.9x** |
| sidecar+kitchen-sink vs velite+ks   | 1381.46 | 144.77  |   **9.5x** |
| sidecar+gfm vs velite+gfm           | 5934.00 | 46.01   | **129x** |

## What changed at the source level

- `dmc-core::engine::compile::is_native_owned_remark` and
  `is_native_owned_rehype` enumerate the JS plugin names whose work is
  done natively. Each is gated on the matching Cargo feature
  (`#[cfg(feature = "math")]` etc).
- `CompileConfig::effective_*_plugins()` returns the user's plugin list
  with native-owned names filtered out.
- `CompileConfig::has_js_plugins()` returns false when only
  native-owned names remain. The engine then skips the sidecar
  entirely; native HTML is final.

The `sidecar+*` variants in the bench still ship to the sidecar
(forced via the `+` label) so the shape of the plugin gate is visible;
in real builds with no foreign plugins the cost drops to native-only.

## Plots

- `scale.svg`
- `throughput.svg`
- `table.svg`
