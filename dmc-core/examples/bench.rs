//! End-to-end benchmark + plotter for dmc-core.
//!
//! Scenarios:
//!   1. Per-file native compile          - microseconds per call.
//!   2. Full build, native (cold)        - wall time vs N files.
//!   3. Full build, sidecar light        - + remark-gfm.
//!   4. Full build, sidecar heavy        - + remark-gfm + rehype-pretty-code (shiki).
//!   5. Full build, sidecar kitchen-sink - gfm + math + katex + emoji +
//!                                         pretty-code + slug + autolink-headings.
//!   6. Full build, velite light         - reference (gfm only).
//!   7. Full build, velite kitchen-sink  - reference (same plugin chain).
//!
//! Output (in `dmc-core/tmp/`):
//!   - bench.json        all samples + min / median / p95 / max / stddev / host
//!   - scale.svg         line chart, wall time vs N files (all variants)
//!   - throughput.svg    bar chart of files/second at the largest N
//!   - table.svg         tabular summary across all variants + scales
//!
//! Run:  cargo run --release --example bench
//!
//! No glaze:
//!   - All sidecar variants reset the worker pool between scale points so
//!     each measurement starts cold (no inter-scale plugin cache carryover).
//!   - First run installs npm deps into `dmc-core/tmp/bench-deps/` once.
//!     Subsequent runs reuse it. Plugin variants + velite skip silently
//!     if `npm` is unavailable or the install fails.
//!   - Mermaid intentionally NOT included in the kitchen-sink chain. The
//!     pure-JS renderers all wrap puppeteer/playwright + a real browser,
//!     which would dominate the wall time and obscure the parser+pipeline
//!     comparison. Math is via katex (pure JS, no headless browser).
//!   - Numbers come from one process on this host; variance is reported.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use dmc::Engine;
use dmc::engine::collection::Collection;
use dmc::engine::compile::{CompileConfig, Compiler};
use dmc::engine::config::EngineConfig;
use dmc::engine::sidecar::shutdown_pool;
use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;
use plotters::prelude::*;
use serde::Serialize;
use serde_json::json;
use tempfile::TempDir;

/// Representative MDX fixture. Inlined so the bench is self-contained.
const FIXTURE: &str = r#"---
title: "Sample Doc"
description: "A reasonably realistic mdx file used as the bench fixture."
date: "2026-05-02"
tags: ["mdx", "bench", "dmc"]
draft: false
---

import { Callout } from "../components/callout.tsx"
import { Tabs, Tab } from "../components/tabs.tsx"

# Heading One

A first paragraph with **bold**, *italic*, and ~~strikethrough~~ markers,
plus an [autolink](https://example.com) and an `inline code` span.

## Heading Two

> Block quote with **emphasis** inside, followed by a second line.

- first item with `code`
- second item with [a link](#heading-two)
  - nested item one
  - nested item two with ~~strike~~
- third item

- [x] done thing
- [ ] pending thing

| Column A | Column B | Column C |
| -------- | -------- | -------- |
| 1        | one      | first    |
| 2        | two      | second   |
| 3        | three    | third    |

```ts title="example.ts" {2-4}
export function add(a: number, b: number): number {
  if (a < 0 || b < 0) {
    throw new Error("negatives not supported")
  }
  return a + b
}
```

```sh
pnpm install
pnpm dev
```

<Callout type="info">
  Inline JSX block with a child paragraph and **markdown** inside.
</Callout>

## Heading Three

Final paragraph plus a terminating thematic break.

---

End.
"#;

/// Heavy fixture used by the kitchen-sink scenario. Adds math, emoji,
/// footnotes, more code blocks, more headings, more paragraphs - so each
/// added plugin actually has work to do.
const HEAVY_FIXTURE: &str = r#"---
title: "Heavy Doc"
description: "Exercises the full plugin chain: gfm, math, emoji, shiki, slug, autolink."
date: "2026-05-02"
tags: ["mdx", "bench", "dmc", "kitchen-sink"]
draft: false
---

import { Callout } from "../components/callout.tsx"

# Heavy Heading :rocket:

Inline math: $E = mc^2$ and $\sigma = \sqrt{\sum (x_i - \mu)^2 / N}$.

Display math:

$$
\int_{-\infty}^{\infty} e^{-x^2} \, dx = \sqrt{\pi}
$$

## Sub :sparkles:

Mixed inline content: **bold**, *italic*, ~~strike~~, `code`, an
[autolink](https://example.com), and an emoji :tada:.

> Quote with `inline code` and a footnote reference[^a].

[^a]: Footnote body with **emphasis** and a [link](https://example.org).

### Code blocks (multiple langs)

```ts title="lib/util.ts" {1,3-5}
export function clamp(v: number, lo: number, hi: number) {
  if (v < lo) return lo
  if (v > hi) return hi
  if (Number.isNaN(v)) throw new Error("nan")
  return v
}
```

```rust
fn fib(n: u32) -> u64 {
    let mut a: u64 = 0;
    let mut b: u64 = 1;
    for _ in 0..n {
        let t = a + b;
        a = b;
        b = t;
    }
    a
}
```

```python
def merge_sort(xs):
    if len(xs) <= 1:
        return xs
    mid = len(xs) // 2
    return merge(merge_sort(xs[:mid]), merge_sort(xs[mid:]))
```

```sh
docker run --rm -it -p 8080:8080 -v $(pwd):/app node:20 sh -c "cd /app && npm i && npm run build"
```

### Tables

| feature      | dmc native | sidecar light | sidecar heavy |
| ------------ | ---------- | ------------- | ------------- |
| gfm          | yes        | yes           | yes           |
| shiki        | no         | no            | yes           |
| math         | no         | no            | yes (katex)   |
| pure rust    | yes        | partial       | partial       |

### Task list

- [x] write parser
- [x] write transformer pipeline
- [ ] add jsx codegen
- [ ] add mdx-module wrap

### Long paragraph

A long paragraph repeated to give the parser a non-trivial inline pass.
A long paragraph repeated to give the parser a non-trivial inline pass.
A long paragraph repeated to give the parser a non-trivial inline pass.
A long paragraph repeated to give the parser a non-trivial inline pass.

<Callout type="warning">
  Component with **markdown** body and a [link](#heavy-heading).
</Callout>

## Final :checkered_flag:

End paragraph with a math span $\alpha + \beta = \gamma$ and an autolink
<https://example.org/end>.

---
"#;

const PER_FILE_ITERS: usize = 200;
const PER_FILE_WARMUP: usize = 20;
const SCALE_ITERS: usize = 10;
const SCALE_WARMUP: usize = 2;
const SCALES: &[usize] = &[10, 100, 1000];

#[derive(Serialize, Debug, Clone)]
struct Stats {
  samples: Vec<f64>,
  min: f64,
  median: f64,
  p95: f64,
  max: f64,
  mean: f64,
  stddev: f64,
}

impl Stats {
  fn from_samples(mut samples: Vec<f64>) -> Self {
    if samples.is_empty() {
      return Self { samples, min: 0.0, median: 0.0, p95: 0.0, max: 0.0, mean: 0.0, stddev: 0.0 };
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = samples.len();
    let min = samples[0];
    let max = samples[n - 1];
    let median = samples[n / 2];
    let p95_idx = ((n as f64) * 0.95).ceil() as usize;
    let p95 = samples[p95_idx.min(n - 1)];
    let mean = samples.iter().sum::<f64>() / n as f64;
    let var = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
    let stddev = var.sqrt();
    Self { samples, min, median, p95, max, mean, stddev }
  }
}

#[derive(Serialize, Debug)]
struct ScalePoint {
  files: usize,
  stats: Stats,
}

#[derive(Serialize, Debug)]
struct Variant {
  label: String,
  points: Vec<ScalePoint>,
}

#[derive(Serialize, Debug)]
struct PerFileEntry {
  fixture: String,
  bytes: usize,
  stats: Stats,
}

#[derive(Serialize, Debug)]
struct Report {
  per_file: Vec<PerFileEntry>,
  variants: Vec<Variant>,
  skipped: Vec<SkipNote>,
  host: HostInfo,
}

#[derive(Serialize, Debug)]
struct SkipNote {
  variant: String,
  reason: String,
}

#[derive(Serialize, Debug)]
struct HostInfo {
  cpus: usize,
  os: String,
  arch: String,
}

fn host_info() -> HostInfo {
  HostInfo {
    cpus: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
    os: std::env::consts::OS.to_string(),
    arch: std::env::consts::ARCH.to_string(),
  }
}

fn out_dir() -> PathBuf {
  let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo");
  let p = PathBuf::from(manifest).join("tmp");
  fs::create_dir_all(&p).ok();
  p
}

fn sidecar_path() -> PathBuf {
  let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo");
  PathBuf::from(manifest).join("..").join("dmc-sidecar").join("index.mjs")
}

fn check_node() -> Result<(), String> {
  Command::new("node").arg("--version").output().map_err(|e| format!("node not on PATH: {e}"))?;
  Ok(())
}

fn check_sidecar_ready() -> Result<PathBuf, String> {
  check_node()?;
  let path = sidecar_path();
  if !path.exists() {
    return Err(format!("sidecar entry not found at {}", path.display()));
  }
  let node_modules = path.parent().unwrap().join("node_modules");
  if !node_modules.exists() {
    return Err("sidecar deps missing; run `cd dmc-sidecar && npm i`".into());
  }
  Ok(path)
}

/// Persistent npm install for the heavy plugin variant + velite. One-time
/// cost on first run. Cached at `dmc-core/tmp/bench-deps/`.
fn ensure_bench_deps() -> Result<PathBuf, String> {
  check_node()?;
  let dir = out_dir().join("bench-deps");
  fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
  let pkg = dir.join("package.json");
  if !pkg.exists() {
    fs::write(
      &pkg,
      r#"{
  "name": "dmc-bench-deps",
  "version": "0.0.0",
  "type": "module",
  "private": true,
  "dependencies": {
    "velite": "^0.3",
    "remark-gfm": "^4",
    "remark-math": "^6",
    "remark-emoji": "^5",
    "remark-frontmatter": "^5",
    "rehype-katex": "^7",
    "rehype-pretty-code": "^0.13",
    "rehype-slug": "^6",
    "rehype-autolink-headings": "^7",
    "shiki": "^1"
  }
}
"#,
    )
    .map_err(|e| e.to_string())?;
  }
  let nm = dir.join("node_modules");
  if !nm.exists() {
    eprintln!("      installing bench deps in {} (one-time, ~30s)...", dir.display());
    // --ignore-scripts skips native postinstall builds (e.g. velite -> sharp
    // -> node-gyp) that fail without a system toolchain. Sharp's image
    // processing is not exercised by plain-mdx fixtures.
    let status = Command::new("npm")
      .args(["install", "--no-fund", "--no-audit", "--silent", "--ignore-scripts"])
      .current_dir(&dir)
      .stdout(Stdio::null())
      .stderr(Stdio::null())
      .status()
      .map_err(|e| format!("npm: {e}"))?;
    if !status.success() {
      return Err("npm install failed (try `npm install` manually inside tmp/bench-deps)".into());
    }
  }
  Ok(dir)
}

fn measure_per_file_with(body: &str) -> Stats {
  let mut samples = Vec::with_capacity(PER_FILE_ITERS);
  for i in 0..(PER_FILE_ITERS + PER_FILE_WARMUP) {
    let mut diag = DiagnosticEngine::<Code>::new();
    let started = Instant::now();
    let _ = Compiler::compile(body, &mut diag);
    let elapsed_us = started.elapsed().as_secs_f64() * 1_000_000.0;
    if i >= PER_FILE_WARMUP {
      samples.push(elapsed_us);
    }
  }
  Stats::from_samples(samples)
}

/// Tiny fixture: 4 lines, ~80 bytes. Lower bound on per-file native cost.
const SHORT_FIXTURE: &str = r#"---
title: "Tiny"
---

Hello, **world**.
"#;

/// Long fixture: heavy content repeated 50x to push the parser past 5k
/// lines / ~250 KB. Stresses the lexer + per-node allocator + codegen
/// writer at a single-file scale.
fn long_fixture() -> String {
  let body = HEAVY_FIXTURE;
  let chunks = body.split("---\n").collect::<Vec<_>>();
  // chunks[0] = "" (before first ---), chunks[1] = frontmatter, chunks[2] = body.
  let frontmatter = chunks.get(1).copied().unwrap_or("");
  let body_only = chunks.get(2).copied().unwrap_or(body);
  let mut out = String::with_capacity(body.len() * 50);
  out.push_str("---\n");
  out.push_str(frontmatter);
  out.push_str("---\n");
  for _ in 0..50 {
    out.push_str(body_only);
  }
  out
}

fn write_fixtures_with(root: &Path, n: usize, body: &str) {
  let docs = root.join("docs");
  fs::create_dir_all(&docs).expect("mkdir docs");
  for i in 0..n {
    let mut variant = String::with_capacity(body.len() + 32);
    variant.push_str(&format!("---\ntitle: \"Doc {i}\"\n---\n\n"));
    variant.push_str(body);
    fs::write(docs.join(format!("doc-{i}.mdx")), variant).expect("write fixture");
  }
}

fn make_cfg(
  root: &Path,
  output_dir: &Path,
  remark: &[serde_json::Value],
  rehype: &[serde_json::Value],
) -> EngineConfig {
  let compile = CompileConfig {
    markdown_remark_plugins: remark.to_vec(),
    markdown_rehype_plugins: rehype.to_vec(),
    ..CompileConfig::default()
  };
  EngineConfig {
    root: root.to_path_buf(),
    output_dir: output_dir.to_path_buf(),
    output_name: None,
    output_format: None,
    clean: true,
    strict: false,
    collections: vec![Collection {
      name: "docs".into(),
      pattern: "docs/**/*.mdx".into(),
      base_dir: root.to_path_buf(),
      schema: None,
      single: false,
    }],
    include_html: false,
    compile,
  }
}

fn run_engine_once(cfg: &EngineConfig) -> Duration {
  let mut diag = DiagnosticEngine::<Code>::new();
  let started = Instant::now();
  let _ = Engine::run(cfg, None, &mut diag);
  started.elapsed()
}

fn measure_native_scale(n: usize, fixture: &str) -> Stats {
  let tmp = TempDir::new().expect("tempdir");
  write_fixtures_with(tmp.path(), n, fixture);
  let output_dir = tmp.path().join(".dmc");
  let cfg = make_cfg(tmp.path(), &output_dir, &[], &[]);

  let mut samples = Vec::with_capacity(SCALE_ITERS);
  for i in 0..(SCALE_ITERS + SCALE_WARMUP) {
    if output_dir.exists() {
      fs::remove_dir_all(&output_dir).ok();
    }
    let elapsed = run_engine_once(&cfg);
    if i >= SCALE_WARMUP {
      samples.push(elapsed.as_secs_f64() * 1000.0);
    }
  }
  Stats::from_samples(samples)
}

fn measure_sidecar_scale(
  n: usize,
  fixture: &str,
  remark: &[serde_json::Value],
  rehype: &[serde_json::Value],
  cwd_for_plugin_resolution: Option<&Path>,
) -> Stats {
  let tmp = TempDir::new().expect("tempdir");
  write_fixtures_with(tmp.path(), n, fixture);
  let output_dir = tmp.path().join(".dmc");
  let cfg = make_cfg(tmp.path(), &output_dir, remark, rehype);

  // Reset the sidecar pool so each scale point starts cold (no plugin
  // cache carryover from the previous N).
  shutdown_pool();

  // Sidecar's userRequire resolves plugins from process.cwd()/package.json.
  // For variants that need third-party plugins, chdir into bench-deps so
  // the spawned node child inherits the right cwd.
  let prev_cwd = std::env::current_dir().ok();
  if let Some(p) = cwd_for_plugin_resolution {
    std::env::set_current_dir(p).expect("chdir bench-deps");
  }

  let mut samples = Vec::with_capacity(SCALE_ITERS);
  for i in 0..(SCALE_ITERS + SCALE_WARMUP) {
    if output_dir.exists() {
      fs::remove_dir_all(&output_dir).ok();
    }
    let elapsed = run_engine_once(&cfg);
    if i >= SCALE_WARMUP {
      samples.push(elapsed.as_secs_f64() * 1000.0);
    }
  }

  if let Some(p) = prev_cwd {
    std::env::set_current_dir(p).ok();
  }

  Stats::from_samples(samples)
}

/// Run velite against the same fixtures. Sets up a tempdir with a
/// velite.config.ts, symlinks node_modules from bench-deps, then times
/// `node node_modules/velite/dist/cli.js build` per scale point.
fn measure_velite_scale(n: usize, fixture: &str, deps: &Path, velite_config: &str) -> Result<Stats, String> {
  let tmp = TempDir::new().map_err(|e| e.to_string())?;
  write_fixtures_with(tmp.path(), n, fixture);

  // Symlink node_modules so velite finds its own internals.
  #[cfg(unix)]
  std::os::unix::fs::symlink(deps.join("node_modules"), tmp.path().join("node_modules")).map_err(|e| e.to_string())?;
  #[cfg(not(unix))]
  return Err("velite bench currently unix-only (symlink)".into());

  fs::write(tmp.path().join("velite.config.ts"), velite_config).map_err(|e| e.to_string())?;

  // Locate velite's CLI entry once.
  let cli_candidates = [
    deps.join("node_modules").join("velite").join("dist").join("cli.js"),
    deps.join("node_modules").join("velite").join("bin").join("velite.js"),
    deps.join("node_modules").join(".bin").join("velite"),
  ];
  let cli = cli_candidates
    .iter()
    .find(|p| p.exists())
    .ok_or_else(|| "velite CLI entry not found in node_modules".to_string())?
    .clone();

  let mut samples = Vec::with_capacity(SCALE_ITERS);
  for i in 0..(SCALE_ITERS + SCALE_WARMUP) {
    let dot = tmp.path().join(".velite");
    if dot.exists() {
      fs::remove_dir_all(&dot).ok();
    }
    let started = Instant::now();
    let status = Command::new("node")
      .arg(&cli)
      .arg("build")
      .current_dir(tmp.path())
      .stdout(Stdio::null())
      .stderr(Stdio::null())
      .status()
      .map_err(|e| e.to_string())?;
    if !status.success() {
      return Err("velite build returned non-zero".into());
    }
    if i >= SCALE_WARMUP {
      samples.push(started.elapsed().as_secs_f64() * 1000.0);
    }
  }
  Ok(Stats::from_samples(samples))
}

fn fmt_stats_ms(s: &Stats, n: usize) -> String {
  format!(
    "median {:>8.2} ms  ({:>6.2} ms/file)  | p95 {:>8.2}  | stddev {:>6.2}",
    s.median,
    s.median / n.max(1) as f64,
    s.p95,
    s.stddev,
  )
}

fn variant_color(idx: usize) -> RGBColor {
  match idx % 4 {
    0 => RGBColor(30, 110, 200), // blue   - native
    1 => RGBColor(220, 80, 60),  // red    - sidecar light
    2 => RGBColor(200, 140, 40), // orange - sidecar heavy
    _ => RGBColor(80, 160, 80),  // green  - velite
  }
}

fn plot_scale(out: &Path, variants: &[Variant]) -> Result<(), Box<dyn std::error::Error>> {
  let path = out.join("scale.svg");
  let root = SVGBackend::new(&path, (980, 600)).into_drawing_area();
  root.fill(&WHITE)?;

  let max_x = SCALES.last().copied().unwrap_or(1) as f64;
  let max_y = variants.iter().flat_map(|v| v.points.iter().map(|p| p.stats.median)).fold(0.0_f64, f64::max) * 1.15;

  let mut chart = ChartBuilder::on(&root)
    .margin(20)
    .caption("dmc full build wall time vs file count", ("sans-serif", 22))
    .x_label_area_size(45)
    .y_label_area_size(70)
    .build_cartesian_2d(0.0_f64..max_x, 0.0_f64..max_y)?;

  chart
    .configure_mesh()
    .x_desc("files in collection")
    .y_desc("wall time (ms, median)")
    .x_label_formatter(&|v| format!("{}", *v as u64))
    .y_label_formatter(&|v| format!("{:.0}", v))
    .draw()?;

  for (i, v) in variants.iter().enumerate() {
    let color = variant_color(i);
    let label = v.label.clone();
    chart
      .draw_series(LineSeries::new(v.points.iter().map(|p| (p.files as f64, p.stats.median)), color.stroke_width(3)))?
      .label(label)
      .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color.stroke_width(3)));
    for p in &v.points {
      chart.draw_series(std::iter::once(Circle::new((p.files as f64, p.stats.median), 4, color.filled())))?;
    }
  }

  chart.configure_series_labels().border_style(BLACK).background_style(WHITE.mix(0.85)).draw()?;
  root.present()?;
  Ok(())
}

fn plot_throughput(out: &Path, variants: &[Variant], target_n: usize) -> Result<(), Box<dyn std::error::Error>> {
  let path = out.join("throughput.svg");
  let root = SVGBackend::new(&path, (900, 520)).into_drawing_area();
  root.fill(&WHITE)?;

  let mut bars: Vec<(String, f64, RGBColor)> = Vec::new();
  for (i, v) in variants.iter().enumerate() {
    if let Some(p) = v.points.iter().find(|p| p.files == target_n) {
      let fps = (p.files as f64) / (p.stats.median / 1000.0);
      bars.push((v.label.clone(), fps, variant_color(i)));
    }
  }

  let max_y = bars.iter().map(|(_, fps, _)| *fps).fold(0.0_f64, f64::max) * 1.2;
  let n_bars = bars.len();
  if n_bars == 0 {
    return Ok(());
  }

  let mut chart = ChartBuilder::on(&root)
    .margin(20)
    .caption(format!("files / second at N = {}", target_n), ("sans-serif", 22))
    .x_label_area_size(60)
    .y_label_area_size(80)
    .build_cartesian_2d(0..n_bars, 0.0_f64..max_y)?;

  chart
    .configure_mesh()
    .disable_x_mesh()
    .y_desc("files / second (median)")
    .x_label_formatter(&|i| bars.get(*i).map(|(l, _, _)| l.clone()).unwrap_or_default())
    .draw()?;

  for (i, (_, fps, color)) in bars.iter().enumerate() {
    chart.draw_series(std::iter::once(Rectangle::new([(i, 0.0), (i + 1, *fps)], color.filled())))?;
  }

  root.present()?;
  Ok(())
}

fn print_summary(report: &Report) {
  println!("\n=== summary ===");
  let target_n = *SCALES.last().unwrap();
  let baseline = report.variants.iter().find(|v| v.label == "native");
  for v in &report.variants {
    let p = match v.points.iter().find(|p| p.files == target_n) {
      Some(x) => x,
      None => continue,
    };
    let fps = (p.files as f64) / (p.stats.median / 1000.0);
    let ratio_str = match baseline {
      Some(base) => match base.points.iter().find(|x| x.files == target_n) {
        Some(b) if v.label != "native" => {
          format!("  ({:.1}x slower than native)", p.stats.median / b.stats.median)
        },
        _ => String::new(),
      },
      None => String::new(),
    };
    println!(
      "  {:<22} N={:>5}  median {:>8.2} ms  ({:.0} files/sec){}",
      v.label, p.files, p.stats.median, fps, ratio_str
    );
  }
  if !report.skipped.is_empty() {
    println!("\nskipped variants:");
    for s in &report.skipped {
      println!("  {} - {}", s.variant, s.reason);
    }
  }
}

const VELITE_LIGHT_CFG: &str = r#"import { defineConfig, s } from 'velite'
import remarkGfm from 'remark-gfm'

export default defineConfig({
  root: '.',
  mdx: { remarkPlugins: [remarkGfm] },
  collections: {
    docs: {
      name: 'Doc',
      pattern: 'docs/**/*.mdx',
      schema: s.object({
        title: s.string(),
        code: s.mdx(),
      }).passthrough(),
    },
  },
})
"#;

const VELITE_KITCHEN_CFG: &str = r#"import { defineConfig, s } from 'velite'
import remarkGfm from 'remark-gfm'
import remarkMath from 'remark-math'
import remarkEmoji from 'remark-emoji'
import rehypeKatex from 'rehype-katex'
import rehypePrettyCode from 'rehype-pretty-code'
import rehypeSlug from 'rehype-slug'
import rehypeAutolinkHeadings from 'rehype-autolink-headings'

export default defineConfig({
  root: '.',
  mdx: {
    remarkPlugins: [remarkGfm, remarkMath, remarkEmoji],
    rehypePlugins: [
      [rehypePrettyCode, { theme: 'github-dark' }],
      rehypeKatex,
      rehypeSlug,
      [rehypeAutolinkHeadings, { behavior: 'append' }],
    ],
  },
  collections: {
    docs: {
      name: 'Doc',
      pattern: 'docs/**/*.mdx',
      schema: s.object({
        title: s.string(),
        code: s.mdx(),
      }).passthrough(),
    },
  },
})
"#;

fn kitchen_remark() -> Vec<serde_json::Value> {
  vec![json!("remark-gfm"), json!("remark-math"), json!("remark-emoji")]
}

fn kitchen_rehype() -> Vec<serde_json::Value> {
  vec![
    json!(["rehype-pretty-code", { "theme": "github-dark" }]),
    json!("rehype-katex"),
    json!("rehype-slug"),
    json!(["rehype-autolink-headings", { "behavior": "append" }]),
  ]
}

fn run_per_file_sweep() -> Vec<PerFileEntry> {
  let long = long_fixture();
  let cases: Vec<(&str, &str)> = vec![
    ("short  (~80 B)", SHORT_FIXTURE),
    ("medium (~1 KB)", FIXTURE),
    ("heavy  (~2 KB)", HEAVY_FIXTURE),
    ("long   (~80 KB)", long.as_str()),
  ];
  let mut out = Vec::new();
  for (label, body) in cases {
    print!("      {label:<18} ... ");
    std::io::stdout().flush().ok();
    let stats = measure_per_file_with(body);
    println!(
      "median {:>6.1} us  (min {:>5.1}, p95 {:>6.1}, stddev {:>5.2})",
      stats.median, stats.min, stats.p95, stats.stddev,
    );
    out.push(PerFileEntry { fixture: label.to_string(), bytes: body.len(), stats });
  }
  out
}

fn run_scale_variants(variants: &mut Vec<Variant>, skipped: &mut Vec<SkipNote>) {
  // 1. Native ---------------------------------------------------------------
  println!("\n[native, no plugins]  {} iters after {} warmup, cold rebuild", SCALE_ITERS, SCALE_WARMUP);
  let mut native_points = Vec::new();
  for &n in SCALES {
    print!("      N={n:>5} ... ");
    std::io::stdout().flush().ok();
    let stats = measure_native_scale(n, FIXTURE);
    println!("{}", fmt_stats_ms(&stats, n));
    native_points.push(ScalePoint { files: n, stats });
  }
  variants.push(Variant { label: "native".into(), points: native_points });

  // 2-7. JS-side scenarios require sidecar + bench-deps. ---------------------
  let path = match check_sidecar_ready() {
    Ok(p) => p,
    Err(reason) => {
      println!("\n      SKIPPED all sidecar/velite variants: {reason}");
      for v in [
        "sidecar+remark-gfm",
        "sidecar+pretty-code",
        "sidecar+kitchen-sink",
        "velite+remark-gfm",
        "velite+kitchen-sink",
      ] {
        skipped.push(SkipNote { variant: v.into(), reason: reason.clone() });
      }
      return;
    },
  };
  unsafe { std::env::set_var("dmc_SIDECAR", path.to_string_lossy().into_owned()) };

  let deps = match ensure_bench_deps() {
    Ok(d) => d,
    Err(reason) => {
      println!("\n      SKIPPED all sidecar/velite variants (npm): {reason}");
      for v in [
        "sidecar+remark-gfm",
        "sidecar+pretty-code",
        "sidecar+kitchen-sink",
        "velite+remark-gfm",
        "velite+kitchen-sink",
      ] {
        skipped.push(SkipNote { variant: v.into(), reason: reason.clone() });
      }
      return;
    },
  };

  // 2. Sidecar + remark-gfm ------------------------------------------------
  println!("\n[sidecar + remark-gfm]  medium fixture, pool reset between scales");
  let mut points = Vec::new();
  for &n in SCALES {
    print!("      N={n:>5} ... ");
    std::io::stdout().flush().ok();
    let stats = measure_sidecar_scale(n, FIXTURE, &[json!("remark-gfm")], &[], Some(&deps));
    println!("{}", fmt_stats_ms(&stats, n));
    points.push(ScalePoint { files: n, stats });
  }
  variants.push(Variant { label: "sidecar+remark-gfm".into(), points });

  // 3. Sidecar + pretty-code ------------------------------------------------
  println!("\n[sidecar + remark-gfm + rehype-pretty-code (shiki)]  medium fixture");
  let mut points = Vec::new();
  for &n in SCALES {
    print!("      N={n:>5} ... ");
    std::io::stdout().flush().ok();
    let stats = measure_sidecar_scale(
      n,
      FIXTURE,
      &[json!("remark-gfm")],
      &[json!(["rehype-pretty-code", { "theme": "github-dark" }])],
      Some(&deps),
    );
    println!("{}", fmt_stats_ms(&stats, n));
    points.push(ScalePoint { files: n, stats });
  }
  variants.push(Variant { label: "sidecar+pretty-code".into(), points });

  // 4. Sidecar + kitchen-sink ----------------------------------------------
  println!("\n[sidecar + kitchen-sink: gfm + math + katex + emoji + shiki + slug + autolink]  heavy fixture");
  let mut points = Vec::new();
  for &n in SCALES {
    print!("      N={n:>5} ... ");
    std::io::stdout().flush().ok();
    let stats = measure_sidecar_scale(n, HEAVY_FIXTURE, &kitchen_remark(), &kitchen_rehype(), Some(&deps));
    println!("{}", fmt_stats_ms(&stats, n));
    points.push(ScalePoint { files: n, stats });
  }
  variants.push(Variant { label: "sidecar+kitchen-sink".into(), points });

  // 5. Velite light --------------------------------------------------------
  println!("\n[velite + remark-gfm]  reference, medium fixture");
  let mut points = Vec::new();
  let mut light_fail: Option<String> = None;
  for &n in SCALES {
    print!("      N={n:>5} ... ");
    std::io::stdout().flush().ok();
    match measure_velite_scale(n, FIXTURE, &deps, VELITE_LIGHT_CFG) {
      Ok(stats) => {
        println!("{}", fmt_stats_ms(&stats, n));
        points.push(ScalePoint { files: n, stats });
      },
      Err(e) => {
        println!("FAILED: {e}");
        light_fail = Some(e);
        break;
      },
    }
  }
  if !points.is_empty() {
    variants.push(Variant { label: "velite+remark-gfm".into(), points });
  }
  if let Some(r) = light_fail {
    skipped.push(SkipNote { variant: "velite+remark-gfm".into(), reason: r });
  }

  // 6. Velite kitchen-sink -------------------------------------------------
  println!("\n[velite + kitchen-sink]  reference, heavy fixture");
  let mut points = Vec::new();
  let mut heavy_fail: Option<String> = None;
  for &n in SCALES {
    print!("      N={n:>5} ... ");
    std::io::stdout().flush().ok();
    match measure_velite_scale(n, HEAVY_FIXTURE, &deps, VELITE_KITCHEN_CFG) {
      Ok(stats) => {
        println!("{}", fmt_stats_ms(&stats, n));
        points.push(ScalePoint { files: n, stats });
      },
      Err(e) => {
        println!("FAILED: {e}");
        heavy_fail = Some(e);
        break;
      },
    }
  }
  if !points.is_empty() {
    variants.push(Variant { label: "velite+kitchen-sink".into(), points });
  }
  if let Some(r) = heavy_fail {
    skipped.push(SkipNote { variant: "velite+kitchen-sink".into(), reason: r });
  }
}

fn plot_table(out: &Path, report: &Report) -> Result<(), Box<dyn std::error::Error>> {
  // Header column + one column per scale + ms/file at largest + files/sec.
  let target_n = *SCALES.last().unwrap();
  let mut headers: Vec<String> = vec!["variant".into()];
  for n in SCALES {
    headers.push(format!("N={n} (ms)"));
  }
  headers.push(format!("ms/file @ N={target_n}"));
  headers.push(format!("files/sec @ N={target_n}"));
  headers.push("vs velite (kitchen)".into());

  // Reference row for ratios = velite kitchen-sink at target_n if available.
  let velite_kitchen = report
    .variants
    .iter()
    .find(|v| v.label == "velite+kitchen-sink")
    .and_then(|v| v.points.iter().find(|p| p.files == target_n))
    .map(|p| p.stats.median);

  let mut rows: Vec<Vec<String>> = Vec::new();
  for v in &report.variants {
    let mut row = Vec::with_capacity(headers.len());
    row.push(v.label.clone());
    for &n in SCALES {
      let cell = match v.points.iter().find(|p| p.files == n) {
        Some(p) => format!("{:.1}", p.stats.median),
        None => "-".into(),
      };
      row.push(cell);
    }
    let target = v.points.iter().find(|p| p.files == target_n);
    match target {
      Some(p) => {
        row.push(format!("{:.3}", p.stats.median / target_n as f64));
        let fps = (target_n as f64) / (p.stats.median / 1000.0);
        row.push(format!("{:.0}", fps));
        match velite_kitchen {
          Some(vref) if v.label != "velite+kitchen-sink" => {
            let ratio = vref / p.stats.median;
            row.push(format!("{:.1}x", ratio));
          },
          _ => row.push("-".into()),
        }
      },
      None => {
        row.push("-".into());
        row.push("-".into());
        row.push("-".into());
      },
    }
    rows.push(row);
  }

  let total_rows = rows.len() + 1; // + header
  let row_h = 30;
  let cell_pad = 12;
  // Compute column widths from text length in row content.
  let char_w = 9;
  let mut col_w: Vec<u32> = headers.iter().map(|h| (h.len() as u32) * char_w as u32 + cell_pad as u32 * 2).collect();
  for r in &rows {
    for (i, cell) in r.iter().enumerate() {
      let want = (cell.len() as u32) * char_w as u32 + cell_pad as u32 * 2;
      if want > col_w[i] {
        col_w[i] = want;
      }
    }
  }
  let total_w: u32 = col_w.iter().sum();
  let title_h = 50;
  let total_h = title_h + (total_rows as u32) * row_h as u32 + 30;

  let path = out.join("table.svg");
  let backend = SVGBackend::new(&path, (total_w + 40, total_h));
  let root = backend.into_drawing_area();
  root.fill(&WHITE)?;

  let title_style = ("sans-serif", 22, FontStyle::Bold).into_text_style(&root);
  root.draw_text(
    &format!("dmc bench summary - host {}/{} {} cores", report.host.os, report.host.arch, report.host.cpus),
    &title_style,
    (20, 14),
  )?;
  let subtitle = ("sans-serif", 13, &RGBColor(110, 110, 110)).into_text_style(&root);
  root.draw_text("median wall time at each scale; lower is better", &subtitle, (20, 36))?;

  // Header bg.
  let mut x = 20i32;
  let y0 = title_h as i32;
  let header_bg = RGBColor(235, 240, 248);
  root.draw(&Rectangle::new([(x, y0), (x + total_w as i32, y0 + row_h)], header_bg.filled()))?;

  // Header text + cell borders.
  let header_font = ("sans-serif", 14, FontStyle::Bold).into_text_style(&root);
  let cell_font = ("sans-serif", 13).into_text_style(&root);
  let zebra = RGBColor(248, 248, 250);
  let border = RGBColor(210, 210, 215);

  for (ci, h) in headers.iter().enumerate() {
    let cw = col_w[ci] as i32;
    root.draw(&Rectangle::new([(x, y0), (x + cw, y0 + row_h)], border.stroke_width(1)))?;
    root.draw_text(h, &header_font, (x + cell_pad as i32, y0 + 9))?;
    x += cw;
  }

  // Body rows.
  for (ri, row) in rows.iter().enumerate() {
    let yy = y0 + row_h + (ri as i32) * row_h;
    let mut x = 20i32;
    if ri % 2 == 0 {
      root.draw(&Rectangle::new([(x, yy), (x + total_w as i32, yy + row_h)], zebra.filled()))?;
    }
    let label_color = match row[0].as_str() {
      "native" => RGBColor(30, 110, 200),
      s if s.starts_with("sidecar") => RGBColor(220, 80, 60),
      s if s.starts_with("velite") => RGBColor(80, 160, 80),
      _ => RGBColor(60, 60, 60),
    };
    for (ci, cell) in row.iter().enumerate() {
      let cw = col_w[ci] as i32;
      root.draw(&Rectangle::new([(x, yy), (x + cw, yy + row_h)], border.stroke_width(1)))?;
      let style = if ci == 0 {
        ("sans-serif", 13, FontStyle::Bold).into_text_style(&root).color(&label_color)
      } else {
        cell_font.clone()
      };
      root.draw_text(cell, &style, (x + cell_pad as i32, yy + 9))?;
      x += cw;
    }
  }

  root.present()?;
  Ok(())
}

fn main() {
  let out = out_dir();
  println!("output: {}", out.display());

  println!(
    "\n[per-file native compile, fixture-size sweep]  {} iters after {} warmup",
    PER_FILE_ITERS, PER_FILE_WARMUP
  );
  let per_file = run_per_file_sweep();

  let mut variants: Vec<Variant> = Vec::new();
  let mut skipped: Vec<SkipNote> = Vec::new();
  run_scale_variants(&mut variants, &mut skipped);

  let report = Report { per_file, variants, skipped, host: host_info() };

  let json_path = out.join("bench.json");
  fs::write(&json_path, serde_json::to_string_pretty(&report).unwrap()).expect("write bench.json");
  println!("\nwrote {}", json_path.display());

  if let Err(e) = plot_scale(&out, &report.variants) {
    eprintln!("plot scale failed: {e}");
  } else {
    println!("wrote {}", out.join("scale.svg").display());
  }
  if let Err(e) = plot_throughput(&out, &report.variants, *SCALES.last().unwrap()) {
    eprintln!("plot throughput failed: {e}");
  } else {
    println!("wrote {}", out.join("throughput.svg").display());
  }
  if let Err(e) = plot_table(&out, &report) {
    eprintln!("plot table failed: {e}");
  } else {
    println!("wrote {}", out.join("table.svg").display());
  }

  print_summary(&report);
}
