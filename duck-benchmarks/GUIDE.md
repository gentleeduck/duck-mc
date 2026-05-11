# Producing & validating a benchmark phase

How to record a new `phase-N-<label>/` folder, and how to sanity-check
that the numbers mean something. Read this before adding a phase.

## 0. When to cut a phase

Cut a phase when a *named, landed* change could plausibly move the
compile pipeline — a perf rewrite, a correctness fix in a hot path, a
plugin-gate change. Don't cut one for docs-only or test-only commits.
Each phase is a historical snapshot at one commit; it is not
re-runnable later (the `examples/nextjs/content/` fixture set drifts).

## 1. Quiet the machine

Numbers on a busy laptop are noise. Before running:

- close other heavy processes; let the CPU cool / governor settle;
- plug in (no battery throttling);
- run each bench command twice, keep the second — the first warms
  caches and the cargo build.

## 2. Headline bench

```sh
cargo run --release -p dmc-core --features pretty-code --example bench
```

Writes to `dmc-core/tmp/` (gitignored):
`bench.json`, `scale.svg`, `table.svg`, `throughput.svg`.

`bench.json` has, per variant (`native`, three `sidecar+*`, two
`velite+*`): the 10 / 100 / 1000-file size sweep (10 samples each,
min/median/p95/max/mean/stddev) plus single-file fixture sample
arrays. The 1000-file `median` is the headline number.

## 3. Flamegraph + stage profile

```sh
# in-process pprof flamegraph of the native path on a realistic single fixture
cargo run --release -p dmc-core --features pretty-code --example flamegraph

# lex / parse / transform / codegen split (+ per-transformer split)
cargo run --release -p dmc-core --features pretty-code --example profile \
  > duck-benchmarks/phase-N-<label>/flamegraph/stage-profile.txt

# flamegraph over the real apps/duck corpus (~370 mdx)
cargo run --release -p dmc-core --features pretty-code --example flamegraph_consumer
```

`flamegraph.rs` and `flamegraph_consumer.rs` write **directly into the
latest `phase-N-<label>/flamegraph/` folder** — update the hard-coded
path in those two examples when you create the new folder. `profile.rs`
prints to stdout; redirect it yourself (strip the cargo `Running …`
line).

`flamegraph_consumer` prefers `apps/duck/content/.dmc-cache/preprocessed`
(the production-exact input — generate it with `bun run build:docs` in
the consumer) and falls back to the raw `apps/duck/content/` tree. The
raw fallback leaves more MDX/JSX for the native parser, so its wall-
clock in `duck-ui.txt` is inflated by `pprof`'s deep-stack unwinding —
treat that flamegraph as *shape only*, not timing. For per-file timing
use `dmc compile <file>` or the headline bench.

These three flamegraph artifacts are optional but expected from phase 6
on.

## 4. Record the folder

```sh
mkdir -p duck-benchmarks/phase-N-<label>/flamegraph
cp dmc-core/tmp/{bench.json,scale.svg,table.svg,throughput.svg} \
   duck-benchmarks/phase-N-<label>/
# flamegraph svgs/txt are already in place from step 3
```

Then write `phase-N-<label>/README.md` — copy the previous phase's
shape:

1. **What changed since phase N-1** — bullet the landed commits, by
   subsystem. End with the source-commit range.
2. **Diff vs phase N-1** — a table of the 1000-file medians for all
   six variants, with the % delta. Then *interpret it honestly*
   (see §5).
3. **Stage profile / flamegraph** — paste the lex/parse/transform/
   codegen table; point at `flamegraph/flame.svg`.
4. **Speedups vs velite** at 1000 files.
5. **How to reproduce** — the commands above.
6. **Files** — list every artifact.

Finally update **this folder's `README.md`**:

- add the phase row to the headline table;
- add a one-line entry to the "Optimisation track" table;
- bump the "N recorded bench runs" count in the intro.

## 5. Validate — does the number mean anything?

- **Velite is the control.** The two `velite+*` columns run an
  unchanged external CLI; nothing in this repo touches them. If they
  move >~5 % between phases, the *host* drifted that run — discount a
  similar slice of any native/sidecar movement as noise, not signal.
- **Check the stddev band** in `bench.json`. A median that moved less
  than ~1 stddev is noise; say so in the README rather than telling a
  story about it.
- **Watch for cross-direction moves.** If `sidecar+kitchen-sink`
  dropped while `native` rose and no commit touched the JS path, that
  swing is pure variance and a fair upper bound on this run's noise.
- **Single-file vs 1000-file can disagree** — per-token regressions
  (e.g. an `Arc` clone) barely show at one file and dominate at 1000.
  Report both if they diverge.
- **Don't claim a speedup the diff can't support.** If §1 wasn't done
  (busy machine), or velite moved a lot, the honest line is "within
  host noise".

## 6. Parser micro-bench regression gate

Separate from the phase folders: `cargo bench -p dmc-parser --bench
parse` (criterion) is the parser micro-bench, baselines in
[`BENCHMARKS.md`](BENCHMARKS.md), and the `bench-regression` CI
workflow runs it on PRs and fails past the noise band. Re-record
`BENCHMARKS.md` when you intentionally move parser perf; mention in the
commit that it was re-run and whether it regressed.
