# API cheatsheet

Most-used calls. Glance reference; click through to the per-crate
docs for details.

## TypeScript / npm

```ts
import { defineConfig, defineCollection, defineLoader, s, build, compile, latexToHtml } from "@gentleduck/md";

defineConfig({
  root: "content",
  output: { data: ".gentleduck", html: true },
  cacheEnabled: true,
  collections: {
    docs: {
      name: "doc",
      pattern: "docs/**/*.mdx",
      schema: s.object({ title: s.string(), tags: s.array(s.string()).optional() }),
    },
  },
});

const report = await build(config);
const out = compile("# hi");
const html = latexToHtml("E = mc^2", false);
```

## Schema builder

```ts
s.string()  s.number()  s.boolean()  s.date()  s.path()
s.string().min(1).max(99).regex(/^[a-z]+$/)
s.array(item)  s.object(fields)  s.record(key, value)
s.union([a, b])  s.literal(v)  s.enum(["a", "b"])
.optional()  .default(v)  .refine(pred, msg)  .transform(fn)
s.markdown()  s.mdx()
type T = s.infer<typeof schema>
```

## Hooks + loaders

```ts
defineConfig({
  prepare(data) { /* mutate or filter */ },
  complete(data) { /* final-stage hook */ },
  loaders: [defineLoader({ test: /\.yaml$/, load: ({ value }) => ({ data: parse(value) }) })],
});

defineCollection({
  name, pattern, schema,
  onRecord(rec) { return { ...rec, slug: makeSlug(rec.title) }; },
});
```

## CLI

```bash
dmc build [--config p] [--clean] [--strict]
dmc dev   [--config p]
dmc init
```

## Rust

```rust
use dmc::Engine;
use dmc::engine::compile::{Compiler, CompileConfig};
use dmc::engine::config::EngineConfig;
use dmc::engine::collection::Collection;
use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;

let mut diag = DiagnosticEngine::<Code>::new();

// One-shot
let out = Compiler::compile("# hi", &mut diag);

// Full engine
Engine::run(&cfg, None, &mut diag)?;
```

## Pipeline (transformer authors)

```rust
use dmc_transform::{Pipeline, PipelineConfig, Transformer, Visitor, NodeAction, walk_root};
use dmc_parser::ast::*;

struct MyPass;
impl Transformer for MyPass {
    fn name(&self) -> &str { "my-pass" }
    fn transform(&self, doc, _meta, _engine) {
        let mut v = Apply;
        walk_root(&mut doc.children, &mut v);
    }
}

struct Apply;
impl Visitor for Apply {
    fn visit_node(&mut self, node: &mut Node) -> NodeAction {
        // mutate
        NodeAction::Keep
    }
}

let p = Pipeline::with_defaults_for(&cfg).add(MyPass);
p.run(&mut doc, &meta, &mut diag);
```

## Compile config

```rust
CompileConfig {
    markdown_gfm: true,
    emit_html: true,
    emit_body: true,
    mdx_minify: false,
    mdx_output_format: None,
    markdown_remark_plugins: vec![],
    markdown_rehype_plugins: vec![],
    pretty_code: None,
    math_engine: Some(MathEngine::Mathml),
    copy_linked_files: false,
    output_assets: None,
    output_base: None,
    ..CompileConfig::default()
}
```

## Engine config

```rust
EngineConfig {
    root: PathBuf::from("content"),
    output_dir: PathBuf::from(".gentleduck"),
    output_format: Some("esm".into()),
    clean: false,
    cache_enabled: true,
    include_html: true,
    collections: vec![/* ... */],
    compile: CompileConfig::default(),
    ..EngineConfig::default()
}
```

## Highlight

```rust
use dmc_highlight::{highlight_code, highlight_code_multi, MultiToken, Theme, Grammar, THEMES, GRAMMARS};

let lines = highlight_code("fn x(){}", Some("rust"), "Catppuccin Mocha");
let multi = highlight_code_multi("fn x(){}", Some("rust"), &["Catppuccin Latte", "Catppuccin Mocha"]);
```

## Math

```rust
use dmc_transform::Math;

let html = Math::render("E = mc^2", false);
let pre = Math::preprocess_source("inline $x^2$ here");
Math::set_engine(dmc_transform::MathEngine::Mathml);
```

## Cache

```rust
use dmc::engine::cache::{FileCache, fingerprint};

let cache = FileCache::open(".gentleduck/.cache/dmc".into())?;
let key = FileCache::key(source.as_bytes(), path, &fingerprint(&cfg));
if let Some(hit) = cache.get(&key) { /* use */ }
cache.put(&key, &record);
```

## Diagnostic

```rust
use dmc_diagnostic::Code;
use duck_diagnostic::{DiagnosticEngine, Diagnostic, Label};

let mut e = DiagnosticEngine::<Code>::new();
e.emit(Diagnostic::new(Code::InvalidCharacter, "bad byte"));
for d in e.iter() { println!("[{}] {}", d.code.code(), d.message); }
```

## Sidecar (NDJSON)

```bash
node dmc-sidecar/index.mjs
{"id":1,"markdown":"# hi","remarkPlugins":[],"rehypePlugins":[]}
{"id":1,"html":"<h1>hi</h1>"}
```
