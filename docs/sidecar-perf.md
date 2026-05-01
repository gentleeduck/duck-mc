# Sidecar Performance Plan

Focused plan for the Node sidecar path in `dmc-core`. Narrower than [`perf-plan.md`](./perf-plan.md), which covers the full engine. This document is the authoritative source for sidecar work; if it conflicts with `perf-plan.md` (which still references pre-refactor `engine.rs` line numbers), this one wins.

## Goal

Make the sidecar path fast enough that adding `remark-gfm` / `rehype-pretty-code` / `shiki` to a dmc project does not regress build times below velite parity. Native (no-plugin) path must stay 100×+ faster than velite. See [`benchmarks.md`](./benchmarks.md) for the current numbers.

## Findings (current code state, post engine refactor)

The engine module was split from `engine.rs` into `engine/{mod,collection,config,utils}.rs`. Several issues are visible by reading the current tree:

### F1 — Sidecar call site is commented out

`dmc-core/src/engine/collection.rs:76-80`:

```rust
// if has_js_plugins(cfg) {
//   if let Some(html) = run_sidecar(&compiled.content, cfg) {
//     compiled.html = html;
//   }
// }
```

`run_sidecar` itself still exists (`dmc-core/src/engine/utils.rs:338`) but nothing calls it. Any user with `markdown.remarkPlugins` configured today silently gets the native HTML, ignoring their plugin chain. This is a correctness bug before it is a perf bug.

### F2 — Double render when the sidecar runs

`dmc-core/src/compile.rs:108` always calls `dmc_codegen::render_html(&doc)` regardless of whether a sidecar will replace the result. Once F1 is re-enabled, plugin builds will render every file twice: once in Rust, then again in Node. Native render is ~30 µs / file; at 999 files that is ~30 ms wasted on every plugin build, every time.

### F3 — Debug `println!` in the hot loop

`dmc-core/src/engine/collection.rs:53` and `:74`:

```rust
println!("source: {:#?}", source);
...
println!("source: {:#?}", compiled.html);
```

These are inside `for path in &paths` in `process()`. Every file pays a synchronous stdout flush, plus the `{:#?}` pretty-print walk over the entire source string. Empirically this dominates perf on any directory > 50 files. Free win: delete.

### F4 — Per-file `node` spawn

`dmc-core/src/engine/utils.rs:338-370` (`run_sidecar`) shells out via `Command::new("node").spawn()` once per file. Cold spawn cost was previously measured at ~115 ms (process start + ESM import graph). At 999 files even with rayon parallelism on 16 cores, cold spawn alone burns ~7 s. This is the single biggest plugin-path cost.

### F5 — `has_js_plugins` returns true on `include_html` alone

`dmc-core/src/engine/utils.rs:325-327`:

```rust
if cfg.include_html {
  return true;
}
```

Setting `include_html = true` (no actual plugins configured) forces the sidecar path. Native render already produces HTML; the user gets nothing for the cost. This gate should require an actual plugin list to be non-empty, not just the HTML emit flag.

## Work units (sequential, same branch — per project convention)

Per saved feedback, these execute in order on the current branch with no per-unit PRs.

### S1 — Delete the hot-loop `println!`s

- **File**: `dmc-core/src/engine/collection.rs`
- **Change**: Remove the two `println!("source: ...")` calls (lines 53 and 74).
- **Why first**: free, instant, and the debug output is corrupting any honest measurement of the steps below.
- **Expected**: 5–20× faster on directories with > 50 files (entirely from removing stdout sync).

### S2 — Tighten the sidecar gate

- **File**: `dmc-core/src/engine/utils.rs:318-332` (`has_js_plugins`)
- **Change**: Drop the `if cfg.include_html { return true; }` short-circuit. Sidecar runs only when at least one plugin list is non-empty. `include_html` is satisfied by the native renderer.
- **Expected**: removes accidental sidecar invocation for users who only want HTML output.

### S3 — Skip native HTML render when the sidecar will replace it

- **Files**: `dmc-core/src/compile.rs`, `dmc-core/src/engine/collection.rs`
- **Change**:
  - Add `pub struct CompileOpts { pub emit_html: bool }` with `Default::default() = { emit_html: true }`.
  - Add `compile_with_pipeline_opts(source, pipeline, opts)`. Existing `compile_with_pipeline` becomes a thin wrapper passing `Default::default()`.
  - In `finalize`, gate line 108: `let html = if opts.emit_html { dmc_codegen::render_html(&doc) } else { String::new() };`.
  - In `collection.rs`, build opts as `CompileOpts { emit_html: !has_js_plugins(cfg) }` and use `compile_with_pipeline_opts`.
- **Expected**: zero native HTML waste on plugin builds. ~30 ms cut at 999 files; correctness improves because we stop overwriting one renderer's output with another.

### S4 — Re-enable the sidecar call

- **File**: `dmc-core/src/engine/collection.rs:76-80`
- **Change**: Uncomment the block. Combined with S3, it becomes:
  ```rust
  let use_sidecar = has_js_plugins(cfg);
  let opts = CompileOpts { emit_html: !use_sidecar };
  let mut compiled = compile_with_pipeline_opts(&source, &pipeline, opts);
  if use_sidecar {
      compiled.html = run_sidecar(&compiled.content, cfg).unwrap_or_default();
  }
  ```
- **Expected**: plugin chains actually run again. Correctness restored.

### S5 — Long-lived sidecar daemon (NDJSON over stdin/stdout)

- **Files**: `dmc-sidecar/index.mjs`, `dmc-core/src/engine/utils.rs`
- **Change**:
  - Sidecar reads NDJSON from stdin (one JSON request per line: `{ id, markdown, remarkPlugins, rehypePlugins }`), writes NDJSON to stdout (one response per line: `{ id, html }`). Plugin imports and the unified pipeline initialize once on startup.
  - Replace per-file `Command::new("node").spawn()` with a process-global `OnceCell<Mutex<SidecarHandle>>`. `SidecarHandle` owns the child, a buffered stdin writer, and a buffered stdout `Lines` reader. Each `run_sidecar` call writes one request line and reads one response line under the lock.
  - On first call, lazily spawn `node dmc-sidecar/index.mjs`. Tear down at process exit (Drop impl is sufficient).
- **Expected**: ~115 ms cold start once per build (not per file). Per-file overhead drops to ~2–3 ms. At 999 files with `remark-gfm`, plugin path goes from ~7 s spawn cost to ~120 ms + processing — **40–60× faster**.

### S6 — Worker pool for sidecar parallelism

- **Files**: `dmc-core/src/engine/utils.rs`
- **Change**: Replace the single `Mutex<SidecarHandle>` with a small pool sized to `min(num_cpus, 4)`. Round-robin or work-steal requests across pool workers so rayon's parallel file iteration is not serialized through one Node process.
- **Defer**: only do this if S5's mutex contention shows up in flamegraphs. Single-worker likely captures 90 % of the win.
- **Expected**: incremental gain on top of S5 for very large dirs with expensive plugin chains.

### S7 — Content-hash cache for incremental rebuilds

- **Files**: new `dmc-core/src/engine/cache.rs`, wired into `collection.rs`
- **Change**: Hash `blake3(source ++ plugins_config_serialized ++ dmc_version)`. Cache key → `CompileOutput` on disk under `node_modules/.cache/dmc/` (matches velite's cache dir convention). On rebuild, return cached `CompileOutput` if hash matches; else compile and store.
- **Expected**: incremental builds become near-instant on unchanged files. Initial build unchanged. Velite has no real cache, so this widens the gap on dev-server hot reloads.

### S8 — Bench harness for the plugin path

- **Files**: new `dmc-core/benches/plugin_path.rs`, fixture under `/tmp/duck-bench-big/` (already exists per `perf-plan.md`)
- **Change**: Criterion bench that compiles the 999-file fixture twice — once with no plugins (baseline) and once with `remark-gfm` configured. Compare wall clock against velite running the same fixture with the same plugin.
- **Acceptance gates** (CI fail if not met):
  - No-plugin path: ≥ 100× faster than velite (baseline guard, currently ~159× per `benchmarks.md`).
  - Plugin path: ≥ 5× faster than velite after S5.
  - Plugin path: ≥ 1.2× faster than velite even before S5 (just S1–S4 should cross this).

## What is intentionally NOT in this plan

- **Replacing the sidecar with WASM-compiled remark/rehype** — possible but the JS plugin ecosystem is the value; killing Node access means killing user plugins. Out of scope.
- **Native Rust shiki / pretty-code transformer** — covered separately; not part of the sidecar perf story.
- **Streaming markdown chunks to the sidecar** — premature; per-file is already the right granularity.
- **Multi-process IPC over Unix sockets** — stdin/stdout is fine; sockets add complexity for no measured win.

## Verification

Run before each unit and after the last:

```sh
cargo build --release -p dmc-core --bin dmc

# Per-file (criterion, native path)
cargo bench -p dmc-core --bench compile 2>&1 | grep "time:"

# Full build, no plugins (S1 visibility)
cd /tmp/duck-bench-big && rm -rf .gentleduck && \
  /usr/bin/time -v <repo>/target/release/dmc build 2>&1 | grep Elapsed

# Full build, with remark-gfm (S2–S6 visibility)
cd /tmp/duck-bench-big && rm -rf .gentleduck && \
  DMC_PLUGINS=remark-gfm /usr/bin/time -v <repo>/target/release/dmc build 2>&1 | grep Elapsed

# Velite parity check
cd /tmp/duck-bench-big && node bench-scale.mjs
```

Per-unit acceptance: targeted metric improves, no regression in the no-plugin path.

## Execution order

`S1 → S2 → S3 → S4 → S5 → S7 → S8`. S6 deferred until S5 mutex contention is observed. Each commit on the current branch; no per-unit PRs.

## Critical files

- `dmc-core/src/engine/collection.rs` — S1, S3, S4
- `dmc-core/src/engine/utils.rs` — S2, S5, S6
- `dmc-core/src/compile.rs` — S3
- `dmc-sidecar/index.mjs` — S5
- `dmc-core/src/engine/cache.rs` (new) — S7
- `dmc-core/benches/plugin_path.rs` (new) — S8
