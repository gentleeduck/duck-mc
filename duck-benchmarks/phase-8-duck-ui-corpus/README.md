# phase 8 — duck-ui real corpus: velite vs dmc, cold vs warm

Real-world spot check, not a microbenchmark: how long the content-compile
step actually takes on the real `@duck-ui/apps/duck` docs site, on the
actual host, before and after the velite &rarr; `@gentleduck/md` (dmc)
migration. Raw numbers and full methodology are in [`bench.json`](bench.json).

Every other `duck-benchmarks/phase-*` folder times the Rust engine
directly against synthetic fixtures in `examples/nextjs/content/`. This
phase instead checks out the real `@duck-ui` repo at two points in time
and times the actual CLI a developer runs.

## Headline (median wall-clock, full CLI process)

| variant | median | vs velite |
| --- | ---: | ---: |
| velite (`a91c669b1`, pre-migration, 366 files) | 218.3 s | 1x (baseline) |
| dmc, cold cache (`2a4823bf5`, 516 files) | 5.48 s | **39.9x faster** |
| dmc, warm cache (`2a4823bf5`, 516 files) | 2.98 s | **73.3x faster** |

Cache alone cuts dmc's own build time by **45.6%** (5.48s &rarr; 2.98s).
dmc is doing 41% more work (516 vs 366 content files — the corpus grew
between the two commits) and is still 40-73x faster; the comparison is
not normalized per-file and if anything understates dmc's advantage.

## Why velite has one number, not cold/warm

velite has no persistent cache. Three consecutive `--clean` runs came
back at 215.4s / 218.3s / 225.3s — no warm-up trend, i.e. every velite
build is architecturally cold. There's nothing to "warm" (see the
`velite is the control` framing in [`../GUIDE.md`](../GUIDE.md)).

## What was actually timed

Content-compile only — not `next build`. Concretely:

- **old** (`a91c669b1`, the commit immediately before the migration
  commit `5a99f61d3`): `node node_modules/.bin/velite build --clean`
- **new** (`2a4823bf5`, current `master`): `node node_modules/@gentleduck/md/bin/duck-md.mjs build --quiet`,
  cold after `duck-md clean`, warm on immediate re-run

Both include full process startup (Node, TS config load, and for dmc
the shiki/syntect highlighter init) — this is the wall-clock time a
developer actually waits on, not an isolated parser benchmark.

## How this was produced

1. Checked out `@duck-ui` `master` (`2a4823bf5`), timed 3 cold + 5 warm
   `duck-md build` runs in `apps/duck`.
2. `git stash` the regenerated `.gentleduck/*.json` output (tracked
   files the build touches), `git checkout a91c669b1` (detached), `bun install`
   at the workspace root.
3. `packages/duck-docs` needed a rebuild (`bun run build`) before velite
   could resolve `@gentleduck/docs/dist/velite/index.js` — its `dist/`
   was stale for the old commit's import graph.
4. Timed 3 cold `velite build --clean` runs in `apps/duck`.
5. `git checkout -- apps/duck bun.lock`, `git checkout master`, dropped
   the benchmark stash, `bun install` again to restore `master`'s
   `node_modules` exactly (verified via `bun install` reporting "no
   changes" against the lockfile).

## Caveats

- Corpus size differs (366 vs 516 files) because content grew between
  the two commits — this is the real historical corpus at each point,
  not a controlled fixture (same caveat the other phases apply to
  `examples/nextjs/content/` drift, see [`../GUIDE.md`](../GUIDE.md)).
- Small sample: n=3 cold, n=5 warm. This is a spot check, not the
  200-sample statistical runs the other phases use.
- dmc's warm cache never reaches 100% hit rate on this corpus — every
  warm run reported `516 files: 510 hits, 6 misses`. Six files
  recompute every time; not investigated here.
- **Environment note (not a benchmark result):** running `duck-md
  build` via `bun` directly fails on this host —
  `Cannot find module './cjs/index.cjs' from ''`. Root cause: `duck-md.mjs`'s
  `loadConfig()` re-execs itself under tsx by spawning
  `process.execPath` to run tsx's CLI loader; under Bun,
  `process.execPath` is the `bun` binary, not `node`, and tsx's
  Node-oriented loader breaks when launched that way. Every
  measurement in this phase was taken by invoking the CLI with a real
  Node binary (`/usr/bin/node .../duck-md.mjs build`) instead, which
  works correctly. Worth fixing in `dmc-napi/bin/duck-md.mjs` if `bun
  duck-md build` is meant to be a supported entry point.
