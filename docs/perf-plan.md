# dmc performance optimization plan

## Context

Real benchmark numbers — measured against velite 0.3.1, @mdx-js/mdx 3.x, unified remark→rehype, and marked 18 on 3 fixtures (mdx.mdx 117 lines, skills.mdx 96 lines, whoiam.mdx 141 lines). All times are wall-clock medians on Linux. Full reproduction: see [`docs/benchmarks.md`](./benchmarks.md).

### In-process per-file (compile only)

| Engine                      | skills.mdx   |
| --------------------------- | ------------ |
| dmc (Rust, cargo bench) | **0.119 ms** |
| @mdx-js/mdx                 | 2.583 ms     |
| unified remark→rehype HTML  | 1.986 ms     |
| marked (md-only baseline)   | 0.136 ms     |

dmc is already faster than `marked`, the fastest-known JS markdown parser. **21.7× faster than @mdx-js/mdx** with full MDX semantics. Little single-file headroom left at the parse layer.

### Full build (3 fixtures, cold rebuild)

| Tool          | Median     | Per-file |
| ------------- | ---------- | -------- |
| velite build  | 250 ms     | 83 ms    |
| dmc build | **2.4 ms** | 0.8 ms   |

### Full build (999 fixtures, cold rebuild)

| Tool          | Median    | Per-file | Speedup             |
| ------------- | --------- | -------- | ------------------- |
| velite build  | 7330 ms   | 7.34 ms  | 1×                  |
| dmc build | **46 ms** | 0.05 ms  | **159× faster**     |

### Sidecar bottleneck

`dmc-core/src/engine.rs:519-552` spawns `node dmc-sidecar/index.mjs` once **per file** when JS plugins are configured.

Cold spawn cost measured: **115.5 ms median per file** (process startup + ESM import graph; before any actual work). At 999 files this would add ~115 s of pure spawn overhead, mitigated only by rayon-parallel cores. With 16 cores still ~7 s of cold-start wasted. **Single biggest real bottleneck once any user adds shiki / rehype-pretty-code.**

## Verified hot paths (from code reading)

| # | File:line                              | Issue |
|---|----------------------------------------|-------|
| 1 | `dmc-core/src/engine.rs:538`       | `Command::new("node")` per file in `run_sidecar` |
| 2 | `dmc-core/src/engine.rs:159`       | `Pipeline::with_defaults()` rebuilt inside rayon `par_iter` per file |
| 3 | `dmc-schema/src/primitives.rs:53`  | `fancy_regex::Regex::new(pat)` inside `parse()` — recompiled per call |
| 4 | `dmc-codegen/src/mdx.rs` (20+ `format!`) | Each AST node allocates several fresh `String`s |
| 5 | `dmc-codegen/src/html.rs`          | Same pattern |
| 6 | `dmc-core/src/engine.rs:223-227`   | `serde_json::to_string_pretty` then `fs::write` (large buffered string) |
| 7 | `dmc-schema/src/primitives.rs:36,46` | `s.chars().count()` for length checks (full O(n) walk) |

## Ruled out by data (do NOT pursue)

- **Lexer SIMD / `memchr`** — parser at 36 µs already beats `marked`. <5 % upside.
- **bumpalo arena allocator** — parse is dwarfed by codegen + serialization.
- **mmap source files** — per-file `read_to_string` of <10 KB is microseconds.
- **Streaming AST instead of `serde_json::Value`** — AST is already typed Rust enums.
- **AOT schema descriptor** — already AOT-compiled in `dmc-schema/src/compile.rs`.

## Work units

Sized by impact-per-effort. Each unit ships independently and merges on its own.

### U1 — Long-lived sidecar with NDJSON protocol  *(biggest win)*

- **Files**: `dmc-sidecar/index.mjs`, `dmc-core/src/engine.rs:503-552`.
- **Change**: Replace per-file `Command::new("node")` with one child `node` started once per build. NDJSON over stdin (one JSON request per line) → NDJSON over stdout (one response per line, correlation id). Sidecar keeps the unified pipeline + plugin imports loaded across requests.
- **Engine side**: lazy-init `SidecarHandle` (Mutex<Child + LineReader>), feed each file's markdown through it from inside `par_iter`.
- **Expected**: 999-file build with `remarkPlugins=[remark-gfm]` drops from ~115 s spawn cost to ~120 ms cold + ~2-3 ms / file ≈ **40-60× faster** with plugins.

### U2 — Hoist `Pipeline` construction out of `par_iter`

- **Files**: `dmc-core/src/engine.rs:158-170`.
- **Change**: Build a single `Pipeline` (with optional `DisableGfm` / `CopyLinkedFiles`) once before `paths.par_iter()`. `CopyLinkedFiles` currently captures `path.parent()` — refactor to take parent dir per call.
- **Expected**: shaves a few µs per file × 999 files ≈ ~2-5 ms at scale.

### U3 — Pre-compile schema regex + length-on-bytes

- **Files**: `dmc-schema/src/primitives.rs:5-63`.
- **Change**: Store `Option<fancy_regex::Regex>` (compiled once at builder time) instead of `Option<String>`. Validate the pattern in builder; surface compile errors at config-load time. Replace `s.chars().count()` with `s.len()` for `min/max/length` checks.
- **Expected**: removes per-file regex compile (~50-200 µs each when used). Free correctness fix moving error to config time.

### U4 — Codegen MDX writer refactor

- **Files**: `dmc-codegen/src/mdx.rs` (192 lines).
- **Change**: Replace `format!`-returns-`String` with `write!(out, ...)` against a single `&mut String`. Each Node currently allocates 1-5 short strings; this collapses to amortized zero alloc.
- **Expected**: codegen is ~83 µs of the 119 µs compile (skills.mdx). 30-50 % cut here = **~25-40 µs / file = 20-35 % overall single-file speedup**.

### U5 — Codegen HTML writer refactor

- **Files**: `dmc-codegen/src/html.rs` (261 lines).
- **Change**: Same refactor as U4. Mechanical port using the `&mut String` pattern.
- **Expected**: same magnitude as U4 for the HTML branch.

### U6 — Streaming JSON output writer

- **Files**: `dmc-core/src/engine.rs:223-227, 267, 291`.
- **Change**: `serde_json::to_writer_pretty(BufWriter::new(File::create(&out_path)?), &records)?`. Skips intermediate `String`. For 999 files `docs.json` is multi-MB; building full string before write wastes memory and copies twice.
- **Expected**: 999-file build memory drops; wall-clock saves ~5-15 ms at scale.

### U7 — Reuse buffer for per-file JS string-escape

- **Files**: `dmc-codegen/src/escape.rs`.
- **Change**: Add `js_string_into(&mut String, &str)` variant that appends rather than allocates. Call from U4's writer.
- **Expected**: amplifies U4 — full Node visitor produces zero short-lived `String`s.

## What is intentionally NOT in this plan

- **Multiple parallel sidecars (worker pool)** — measure after U1; one sidecar captures 95 % of the win.
- **Lexer/parser micro-opts** — measured gains <5 %.
- **Mmap, arena allocators, custom JSON serializer** — premature.

## Verification recipe (same for every unit)

Set up bench environment once per worktree:

```sh
cargo build --release -p dmc-core --bin dmc
ls /tmp/duck-bench-big/content/docs | wc -l   # → 999
```

Run **before** and **after** the change:

```sh
# 1. Per-file native (criterion)
cargo bench -p dmc-core --bench compile 2>&1 | grep -E "time:|compile|parse"

# 2. End-to-end build at scale (no JS plugins)
cd /tmp/duck-bench-big && rm -rf .gentleduck && \
  /usr/bin/time -v <repo>/target/release/dmc build --config dmc.toml 2>&1 | \
  grep -E "Elapsed|Maximum resident"

# 3. Compare against velite at the same scale
cd /tmp/duck-bench-big && node bench-scale.mjs
```

For **U1 specifically**, also measure the JS-plugin path with `markdown.remarkPlugins=[remarkGfm]` configured and rebuild 999 files; should drop 40-60×.

Acceptance per unit: no regression in (1) and (3); positive change in the unit's targeted metric.

## Critical files (most touched)

- `dmc-core/src/engine.rs` — U1, U2, U6
- `dmc-sidecar/index.mjs` — U1
- `dmc-schema/src/primitives.rs` — U3
- `dmc-codegen/src/mdx.rs` — U4
- `dmc-codegen/src/html.rs` — U5
- `dmc-codegen/src/escape.rs` — U7

## Execution

Every unit is independent. Per /batch protocol, spawn 7 worker agents in parallel, each in its own git worktree, each `run_in_background: true`.

## Status table (rendered after spawn)

| # | Unit                                  | Status  | PR |
| - | ------------------------------------- | ------- | -- |
| 1 | Long-lived sidecar (NDJSON)           | pending | —  |
| 2 | Hoist Pipeline out of par_iter        | pending | —  |
| 3 | Pre-compile schema regex              | pending | —  |
| 4 | Codegen MDX writer refactor           | pending | —  |
| 5 | Codegen HTML writer refactor          | pending | —  |
| 6 | Streaming JSON output writer          | pending | —  |
| 7 | Reusable JS-escape append variant     | pending | —  |
