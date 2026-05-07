# Compiler

`Compiler::compile_with_pipeline` is the per-file entry. One source
string in, one `CompileOutput` out.

## Stages

```mermaid
flowchart LR
    S[source: &str] --> M[Math::preprocess_source<br/>$...$ -> MathMl JSX]
    M --> L[Lexer::scan_tokens]
    L --> P[Parser::parse]
    P --> Pipe[Pipeline::with_defaults_for cfg]
    Pipe --> Doc[Document mut]
    Doc --> W[Walker over sinks]
    W --> Acc[Accumulator]
    W --> Html[HtmlEmitter]
    W --> Body[MdxBodyEmitter]
    Acc & Html & Body --> Out[CompileOutput]
```

## Source preprocess

```rust
#[cfg(feature = "math")]
let preprocessed = dmc_transform::Math::preprocess_source(source);
#[cfg(feature = "math")]
let source: &str = &preprocessed;
```

Rewrites `$...$` and `$$...$$` to `<MathMl mathml="..."/>` JSX before
the lexer runs. Avoids the parser interpreting `_`/`^` inside math as
emphasis markers.

## Lex + parse

```rust
let mut lexer = Lexer::new(source, meta.clone(), diag_engine);
let _ = lexer.scan_tokens();

let mut doc = {
    let mut parser = Parser::new(lexer.tokens, meta.clone(), diag_engine);
    parser.parse()
};
```

One `DiagnosticEngine` shared across both layers; codes are namespaced
by prefix (`E*` lexer, `P*` parser).

## Pipeline

```rust
let pipeline_cfg = compile_cfg.pipeline_config(path);
let pipeline = dmc_transform::Pipeline::with_defaults_for(&pipeline_cfg);
pipeline.run(&mut doc, &meta, diag_engine);
```

`with_defaults_for(cfg)` is the single uniform place where every
feature-gated transformer registers (DisableGfm, NpmCommand, Mermaid,
Emoji, Math, PrettyCode, CopyLinkedFiles). See
`dmc-docs/dmc-transform/pipeline.md`.

## Walker + sinks

`Walker::new(&doc).walk(sinks)` does one pre-order DFS over
`doc.children`. Each sink (Accumulator, HtmlEmitter, MdxBodyEmitter)
sees every node. See `dmc-docs/dmc-codegen/walker.md`.

```rust
let mut acc = Accumulator::new();
let mut html_sink = if cfg.emit_html { Some(HtmlEmitter::new()) } else { None };
let mut body_sink = if cfg.emit_body { Some(MdxBodyEmitter::new()) } else { None };

let mut sinks: Vec<&mut dyn dmc_codegen::NodeSink> = Vec::with_capacity(3);
sinks.push(&mut acc);
if let Some(ref mut h) = html_sink { sinks.push(h); }
if let Some(ref mut b) = body_sink { sinks.push(b); }

Walker::new(&doc).walk(sinks.as_mut_slice());
```

`emit_html` / `emit_body` toggle whether the sink runs. When the JS
sidecar will produce HTML downstream, `for_render` flips `emit_html`
off so we do not double-render.

## `CompileOutput`

| field | source |
|-------|--------|
| `frontmatter` | parsed YAML as `Value` (via `Accumulator`) |
| `frontmatter_raw` | original YAML string |
| `content` | normalised markdown (post-preprocess) |
| `html` | `HtmlEmitter` output |
| `body` | `MdxBodyEmitter` output (JS function body) |
| `excerpt` | first paragraph plain text |
| `metadata` | reading time + word count from plain text |
| `toc` | nested heading list from `Accumulator` |
| `imports` / `exports` | top-level `import`/`export` statements |

## `for_render`

```rust
pub fn for_render(&self) -> Self {
    let mut c = self.clone();
    c.emit_html = !self.has_js_plugins();
    c
}
```

Per-file config used by `Collection::process`. Skips native HTML when
the sidecar will render it, avoiding double work.

## Plugin gate

`is_native_owned_remark` and `is_native_owned_rehype` filter the
user's plugin list before the sidecar runs. When a plugin name is
"native-owned" (a Rust transformer already does the work) AND the
matching Cargo feature is on, the name is stripped from the sidecar
payload.

```rust
fn is_native_owned_rehype(plugin: &Value) -> bool {
    let Some(name) = plugin_name(plugin) else { return false };
    match name {
        "rehype-pretty-code" | "shiki" => cfg!(feature = "pretty-code"),
        "rehype-katex" | "rehype-mathjax" => cfg!(feature = "math"),
        "rehype-slug" | "rehype-autolink-headings" => true,
        _ => false,
    }
}
```

`CompileConfig::effective_*_plugins()` returns the user's list with
native-owned names removed. `has_js_plugins()` returns `true` only
when something foreign remains; if everything is native-owned, the
sidecar is skipped entirely.

### Override the gate

Two knobs on `CompileConfig`:

```rust
pub force_sidecar: bool,            // global: bypass gate for every name
pub prefer_sidecar: Vec<String>,    // per-name: bypass gate for these
```

When `prefer_sidecar` lists a name, two things happen:

1. `is_native_owned_*` returns `false` for that name (gate does not
   strip), so the sidecar payload keeps the plugin entry and the
   sidecar runs it.
2. `pipeline_config()` sets the matching `Option<...>` to `None`, so
   `Pipeline::with_defaults_for(cfg)` does not push the native
   transformer into the chain.

Net effect: the JS plugin runs in the sidecar, the native equivalent
does not run, no double work.

When `force_sidecar = true`, every recognised name is treated as
preferred (every plugin runs in sidecar, every native transformer is
dropped). Useful when you want the velite-style JS-only behaviour
without rebuilding the binary with `--no-default-features`.
