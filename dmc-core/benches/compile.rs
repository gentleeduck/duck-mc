use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dmc::engine::compile::Compiler;
use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;

fn bench_compile_skills(c: &mut Criterion) {
  let src = include_str!("../../tests/fixtures/velite-parity/skills.mdx");
  c.bench_function("compile skills.mdx", |b| {
    b.iter(|| {
      let mut diag = DiagnosticEngine::<Code>::new();
      let _ = Compiler::compile(black_box(src), &mut diag);
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
  let src = include_str!("../../tests/fixtures/velite-parity/skills.mdx");
  c.bench_function("parse skills.mdx", |b| {
    b.iter(|| {
      let _ = dmc::parse(black_box(src));
    });
  });
}

criterion_group!(benches, bench_compile_skills, bench_compile_simple, bench_parse_only);
criterion_main!(benches);
