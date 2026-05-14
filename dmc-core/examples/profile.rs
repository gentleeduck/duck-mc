//! Phase-by-phase timing via `Instant::now()` accumulation.
//! Run: cargo run --release --example profile --features pretty-code

use dmc_codegen::{HtmlEmitter, MdxBodyEmitter, NodeSink, Walker};
use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::{Origin, SourceMeta};
use dmc_lexer::Lexer;
use dmc_parser::Parser;
use dmc_transform::{
  AssignHeadingIds, AutolinkHeadings, BareUrlAutolink, CodeImport, Pipeline, PipelineConfig, PrettyCode, Transformer,
};
use duck_diagnostic::DiagnosticEngine;
use std::sync::Arc;
use std::time::{Duration, Instant};

const FIXTURE: &str = r#"---
title: "Sample doc"
description: "A reasonably realistic mdx file used as the profile fixture."
---

# Heading

Some prose with **bold** and *italic* and `inline code` and a [link](https://example.com).

## Subheading

```rust
fn main() {
    let x = 42;
    println!("{}", x);
}
```

A list:

- item one
- item two with `code`
- item three

> A blockquote with **bold**.

| col1 | col2 | col3 |
|------|------|------|
| a    | b    | c    |
| d    | e    | f    |

```typescript
export interface Foo {
  bar: string;
  baz?: number;
}
```

End paragraph with another `code span` and trailing text.
"#;

fn main() {
  let iters: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(5_000);
  let pipeline_cfg = PipelineConfig::default();
  let pipeline = Pipeline::with_defaults_for(&pipeline_cfg);
  let mut t_lex = Duration::ZERO;
  let mut t_parse = Duration::ZERO;
  let mut t_transform = Duration::ZERO;
  let mut t_codegen = Duration::ZERO;
  let mut t_assign_heading_ids = Duration::ZERO;
  let mut t_code_import = Duration::ZERO;
  let mut t_bare_url = Duration::ZERO;
  let mut t_autolink_headings = Duration::ZERO;
  let mut t_pretty_code = Duration::ZERO;

  for _ in 0..iters {
    let mut diag = DiagnosticEngine::<Code>::new();
    let meta = Arc::new(SourceMeta { path: Arc::from("medium.mdx"), origin: Origin::Inline("medium.mdx") });

    let t0 = Instant::now();
    let mut lexer = Lexer::new(FIXTURE, meta.clone(), &mut diag);
    let _ = lexer.scan_tokens();
    t_lex += t0.elapsed();

    let t1 = Instant::now();
    let mut doc = {
      let mut parser = Parser::new(lexer.tokens, meta.clone(), &mut diag);
      parser.parse()
    };
    t_parse += t1.elapsed();

    let t2 = Instant::now();
    pipeline.run(&mut doc, &meta, &mut diag);
    t_transform += t2.elapsed();

    // Per-transformer timing: fresh AST so each sees identical input.
    let mut diag2 = DiagnosticEngine::<Code>::new();
    let mut lexer2 = Lexer::new(FIXTURE, meta.clone(), &mut diag2);
    let _ = lexer2.scan_tokens();
    let base_doc = {
      let mut p = Parser::new(lexer2.tokens, meta.clone(), &mut diag2);
      p.parse()
    };

    macro_rules! time_t {
      ($slot:ident, $t:expr) => {{
        let t = $t;
        let mut d = base_doc.clone();
        let s = Instant::now();
        t.transform(&mut d, &meta, &mut diag2);
        $slot += s.elapsed();
      }};
    }
    time_t!(t_assign_heading_ids, AssignHeadingIds);
    time_t!(t_code_import, CodeImport::new());
    time_t!(t_bare_url, BareUrlAutolink);
    time_t!(t_autolink_headings, AutolinkHeadings::new());
    time_t!(t_pretty_code, PrettyCode::default());

    let t3 = Instant::now();
    let mut html = HtmlEmitter::new();
    let mut body = MdxBodyEmitter::new();
    let mut sinks: Vec<&mut dyn NodeSink> = vec![&mut html, &mut body];
    Walker::new(&doc).walk(sinks.as_mut_slice());
    let (_h, _hd) = html.into_parts();
    let (_b, _bd) = body.into_parts();
    t_codegen += t3.elapsed();
  }

  let total = t_lex + t_parse + t_transform + t_codegen;
  let pct = |d: Duration| 100.0 * d.as_secs_f64() / total.as_secs_f64();
  let per_iter_us = |d: Duration| d.as_secs_f64() * 1_000_000.0 / iters as f64;
  println!("iterations: {iters}");
  println!("total:     {:>10.2} us/iter", total.as_secs_f64() * 1_000_000.0 / iters as f64);
  println!();
  println!("phase     | us/iter   | share");
  println!("----------|-----------|------");
  println!("lex       | {:>9.2} | {:>5.1}%", per_iter_us(t_lex), pct(t_lex));
  println!("parse     | {:>9.2} | {:>5.1}%", per_iter_us(t_parse), pct(t_parse));
  println!("transform | {:>9.2} | {:>5.1}%", per_iter_us(t_transform), pct(t_transform));
  println!("codegen   | {:>9.2} | {:>5.1}%", per_iter_us(t_codegen), pct(t_codegen));
  println!();
  println!("per-transformer (independent runs, share of named transformers):");
  let trans_total = t_assign_heading_ids + t_code_import + t_bare_url + t_autolink_headings + t_pretty_code;
  let pct_t = |d: Duration| 100.0 * d.as_secs_f64() / trans_total.as_secs_f64();
  println!("transformer         | us/iter   | share");
  println!("--------------------|-----------|------");
  println!("assign_heading_ids  | {:>9.2} | {:>5.1}%", per_iter_us(t_assign_heading_ids), pct_t(t_assign_heading_ids));
  println!("code_import         | {:>9.2} | {:>5.1}%", per_iter_us(t_code_import), pct_t(t_code_import));
  println!("bare_url            | {:>9.2} | {:>5.1}%", per_iter_us(t_bare_url), pct_t(t_bare_url));
  println!("autolink_headings   | {:>9.2} | {:>5.1}%", per_iter_us(t_autolink_headings), pct_t(t_autolink_headings));
  println!("pretty_code         | {:>9.2} | {:>5.1}%", per_iter_us(t_pretty_code), pct_t(t_pretty_code));
}
