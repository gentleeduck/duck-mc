# phase 9 - duck-ui with every diagram drawn: one browser, not 558

The real `@duck-ui/apps/duck` docs build again, as in
[phase 8](../phase-8-duck-ui-corpus), this time on a machine where mermaid
draws, so every run renders every diagram. The corpus has 279 diagrams, each
drawn in a light and a dark theme: 558 renders. Up to 0.6.2, dmc ran `mmdc`
for each render, launching node and headless Chrome every time; 0.6.3 draws
them all in one long-lived browser
([#148](https://github.com/gentleeduck/duck-mc/pull/148)). The SVGs are
byte-for-byte the same. Raw runs and the method are in [`bench.json`](bench.json).

## Slow to fast: duck-ui today (`43af295be`, 554 files)

| build | median | vs 0.6.2 cold |
| --- | ---: | ---: |
| 0.6.2, cold: `mmdc` for every diagram and theme | 285 s | 1x |
| 0.6.3, cold: one browser | 88 s | **3.2x** |
| 0.6.3, compile caches wiped, SVGs kept in `mermaid.outputDir` | 10 s | **28x** |
| 0.6.3, warm | 3.4 s | **84x** |

The 10 s row compiles everything and draws nothing, so about 78 s of the 88 s
cold build is mermaid. It is also the most a dmc upgrade costs: an upgrade
invalidates the per-doc cache but not `mermaid.outputDir`, whose SVGs are
keyed by theme and diagram source. With `outputDir` set, only a fresh clone
or `duck-md clean` pays the cold price.

## Against velite: the pre-migration commit (`a91c669b1`, 366 files)

Same files on the same machine. velite drew its diagrams with one headless
Chrome per page.

| build | median | vs velite |
| --- | ---: | ---: |
| velite 0.3.1, `build --clean` (it has no cache) | 414 s | 1x |
| dmc 0.6.2, cold | 168 s | 2.5x |
| dmc 0.6.3, cold | 74 s | **5.6x** |
| dmc 0.6.3, warm | 3.1 s | **134x** |

## Setup

- Apple M4 (4 performance + 6 efficiency cores), 16 GB, macOS 27, Node 25.
  dmc draws with mermaid-cli 11.15 on chrome-headless-shell 148; velite with
  Google Chrome 154.
- dmc runs use duck-ui's own `duck-md.config.ts`, imported by a bench config
  that points `root` at a frozen copy of the content and moves the output and
  `mermaid.outputDir` to a scratch directory. Each run's cache state is
  whatever the harness left there.
- velite runs in a `git archive` export of `a91c669b1`, installed from its own
  lockfile.
- Timed: the whole CLI process, wall-clock.

## Caveats

- Not a quiet machine. macOS storage indexing and other dev tools were busy
  throughout, and every run's load average is in `bench.json`. One 0.6.3 cold
  run took 130 s under that load; the median hides it and the range shows it.
- Small samples: n = 2 to 5 per row.
- Phase 8 ran on a 32-thread Linux desktop and does not record whether `mmdc`
  was installed. Without it dmc draws nothing (`MmdcUnavailable`), so phase 8's
  numbers do not compare with these.
