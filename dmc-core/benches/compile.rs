use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use dmc::engine::compile::Compiler;
use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;

const FIXTURE: &str = r#"---
title: Sample
---

# Heading

Paragraph with **bold**, *italic*, [link](https://example.com), and `code`.

## Sub

- one
- two
- three

```rust
fn main() { println!("hi"); }
```
"#;

fn bench_compile_fixture(c: &mut Criterion) {
  c.bench_function("compile fixture", |b| {
    b.iter(|| {
      let mut diag = DiagnosticEngine::<Code>::new();
      let _ = Compiler::compile(black_box(FIXTURE), &mut diag);
    });
  });
}

fn bench_compile_simple(c: &mut Criterion) {
  let src = "# Hello\n\nworld\n";
  c.bench_function("compile simple", |b| {
    b.iter(|| {
      let mut diag = DiagnosticEngine::<Code>::new();
      let _ = Compiler::compile(black_box(src), &mut diag);
    });
  });
}

fn bench_parse_only(c: &mut Criterion) {
  c.bench_function("parse fixture", |b| {
    b.iter(|| {
      let _ = dmc::parse(black_box(FIXTURE));
    });
  });
}

criterion_group!(benches, bench_compile_fixture, bench_compile_simple, bench_parse_only);
criterion_main!(benches);
