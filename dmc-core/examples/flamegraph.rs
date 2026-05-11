//! In-process flamegraph for the native compile path. Uses `pprof` (a
//! signal-driven sampler that doesn't need `perf_event_open`), so no
//! sudo / `perf_event_paranoid` toggle is required.
//!
//! Run:
//!   cargo run --release --example flamegraph --features pretty-code
//!
//! Output:
//!   duck-benchmarks/phase-7-g-hardening/flamegraph/flame.svg
//!
//! Drag the SVG into any browser. Zoom by clicking a frame; reset by
//! clicking the title bar.

use dmc_codegen::{HtmlEmitter, MdxBodyEmitter, NodeSink, Walker};
use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::{Origin, SourceMeta};
use dmc_lexer::Lexer;
use dmc_parser::Parser;
use dmc_transform::{Pipeline, PipelineConfig};
use duck_diagnostic::DiagnosticEngine;
use pprof::ProfilerGuardBuilder;
use std::sync::Arc;
use std::time::Instant;

const FIXTURE: &str = r#"---
title: "Sample doc"
description: "A reasonably realistic mdx file used as the flamegraph fixture."
---

# Heading

Paragraph with **bold**, _italic_, [link](https://example.com), and `inline code`.

## Sub-heading

- one
- two
- three

```rust
fn main() {
    let v: Vec<i32> = (0..100).collect();
    println!("sum = {}", v.iter().sum::<i32>());
}
```

```ts
interface Config<T> {
  fallback: T
  resolve: (input: string) => Promise<T | null>
}

const cfg: Config<number> = {
  fallback: 0,
  async resolve(s) { return Number.parseInt(s, 10) || null },
}
```

> A blockquote with `code` inside.

| col | data |
| --- | ---- |
| a   | 1    |
| b   | 2    |

<Callout type="warning">

  This is a callout block with **bold** content and an inline `<svg>` tag.

</Callout>
"#;

fn one_iter(pipeline: &Pipeline) {
  let meta = Arc::new(SourceMeta { path: Arc::from("flame.mdx"), origin: Origin::Inline("flame.mdx") });
  let mut diag = DiagnosticEngine::<Code>::new();
  let mut lex = Lexer::new(FIXTURE, meta.clone(), &mut diag);
  let _ = lex.scan_tokens();
  let mut doc = {
    let mut parser = Parser::new(lex.tokens, meta.clone(), &mut diag);
    parser.parse()
  };
  pipeline.run(&mut doc, &meta, &mut diag);
  let mut html = HtmlEmitter::new();
  let mut body = MdxBodyEmitter::new();
  let mut sinks: Vec<&mut dyn NodeSink> = vec![&mut html, &mut body];
  Walker::new(&doc).walk(sinks.as_mut_slice());
  let _ = html.into_parts();
  let _ = body.into_parts();
}

fn main() {
  let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .expect("workspace root")
    .join("duck-benchmarks/phase-7-g-hardening/flamegraph/flame.svg");
  std::fs::create_dir_all(out.parent().unwrap()).unwrap();

  let guard = ProfilerGuardBuilder::default()
    .frequency(997)
    .blocklist(&["libc", "libgcc", "pthread", "vdso"])
    .build()
    .expect("pprof guard");

  let pipeline_cfg = PipelineConfig::default();
  let pipeline = Pipeline::with_defaults_for(&pipeline_cfg);

  let started = Instant::now();
  let mut iters = 0u64;
  // Run for ~5s of CPU time so the sampler collects a healthy stack
  // population (at 997 Hz that's ~5000 stacks).
  while started.elapsed().as_secs_f32() < 5.0 {
    one_iter(&pipeline);
    iters += 1;
  }
  let elapsed = started.elapsed();

  let report = guard.report().build().expect("build report");
  let f = std::fs::File::create(&out).expect("open svg");
  report.flamegraph(f).expect("write svg");

  eprintln!("ran {iters} iterations in {:.2}s", elapsed.as_secs_f32());
  eprintln!("wrote {}", out.display());
}
