use std::hint::black_box;
use std::sync::LazyLock;
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

const TEMPLATE: &str = r#"# Benchmark Fixture

Paragraph with **bold**, *italic*, [link](https://example.com), `code`, and <span>inline HTML</span>.

> Quoted line one
> quoted line two

- alpha
- beta
- gamma

1. first
2. second

| name | value |
| --- | ---: |
| alpha | 1 |
| beta | 2 |

```rust
fn fixture(i: usize) -> usize {
  i * 2 + 1
}
```

Reference [docs][fixture].

[fixture]: https://example.com/docs "Fixture docs"
"#;

static SMALL: LazyLock<String> = LazyLock::new(|| build_corpus(1024));
static MEDIUM: LazyLock<String> = LazyLock::new(|| build_corpus(100 * 1024));
static LARGE: LazyLock<String> = LazyLock::new(|| build_corpus(5 * 1024 * 1024));

fn build_corpus(target_bytes: usize) -> String {
  let mut src = String::with_capacity(target_bytes + TEMPLATE.len());
  while src.len() < target_bytes {
    src.push_str(TEMPLATE);
    src.push('\n');
  }
  src
}

fn corpora() -> [(&'static str, &'static str); 3] {
  [("small", SMALL.as_str()), ("medium", MEDIUM.as_str()), ("large", LARGE.as_str())]
}

fn bench_parse(c: &mut Criterion) {
  let mut group = c.benchmark_group("parse");
  for (name, src) in corpora() {
    group.throughput(Throughput::Bytes(src.len() as u64));
    group.bench_with_input(BenchmarkId::new("dmc", name), &src, |b, src| {
      b.iter(|| {
        let doc = dmc_parser::parse(black_box(src));
        black_box(doc);
      });
    });
  }
  group.finish();
}

fn bench_parse_and_render_html(c: &mut Criterion) {
  let mut group = c.benchmark_group("parse_and_render_html");
  for (name, src) in corpora() {
    group.throughput(Throughput::Bytes(src.len() as u64));
    group.bench_with_input(BenchmarkId::new("dmc", name), &src, |b, src| {
      b.iter(|| {
        let doc = dmc_parser::parse(black_box(src));
        let html = dmc_codegen::render_html(&doc);
        black_box(html);
      });
    });
  }
  group.finish();
}

criterion_group! {
  name = benches;
  config = Criterion::default()
    .sample_size(10)
    .warm_up_time(Duration::from_secs(1))
    .measurement_time(Duration::from_secs(2));
  targets = bench_parse, bench_parse_and_render_html
}
criterion_main!(benches);
