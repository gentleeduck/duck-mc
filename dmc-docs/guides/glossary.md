# Glossary

Terms specific to dmc. Order: alphabetical.

## Accumulator

Third sink in the codegen walker. Pulls frontmatter, imports, exports,
plain text, TOC tuples off the AST. Lives in
`dmc-core/src/engine/accumlator.rs`.

## AST

Abstract Syntax Tree. Output of the parser; tree of `Node` variants
rooted at `Document`.

## Cache

Two persistent stores: per-file compile output
(`<output>/.cache/dmc/`) and math render cache
(`<output>/.cache/math.json`). Plus several in-memory caches
(SyntaxBundle, Mermaid SVG, KaTeX `Opts`).

## CompileOutput

Final per-file record after compile. Fields: frontmatter, content,
html, body, excerpt, metadata, toc, imports, exports.

## Collection

One `<name>.json` output. Defined by glob pattern + schema. Engine
runs each collection's process pass in sequence, files inside in
parallel.

## Compile pipeline

Full per-file flow: math preprocess -> lex -> parse -> transform
chain -> walker over sinks. Lives in
`dmc-core/src/engine/compile.rs::Compiler::compile_with_pipeline`.

## Diagnostic

Error or warning emitted through `DiagnosticEngine<Code>`. Every
layer (lexer / parser / transform / codegen) shares the same `Code`
enum gated by Cargo features.

## Engine

Top-level orchestrator. `dmc::Engine::run` runs one full build:
clean, warm caches, process collections, save caches, emit index.

## Feature flag

Cargo feature controlling which transformer / dependency compiles in.
Defaults: `mermaid`, `assets`, `npm-command`, `math`, `emoji`,
`pretty-code`. See [`../dmc-transform/feature-flags.md`](../dmc-transform/feature-flags.md).

## Fingerprint

blake3 hash of a config tuple. Goes into the cache key. Any field
that affects rendered output must be in the tuple, else stale-cache
bugs.

## Frontmatter

YAML block at the top of an MDX file (`---`...`---`). Captured by the
parser, validated by the schema, attached to the record.

## JSX

The MDX subset of React-like syntax. Parsed into `JsxElement`,
`JsxSelfClosing`, `JsxFragment`, `JsxExpression` nodes.

## Lexer

Token producer. Reads bytes, emits `Vec<Token>`. Whitespace tokens
preserved (needed for inline spacing around links).

## MathEngine

Enum: `Katex` (default, KaTeX HTML via quick-js, slow but rehype-katex
parity) or `Mathml` (pulldown-latex MathML, fast, plainer visual).

## Multi-theme

Pretty-code with multiple themes (e.g. light + dark). One parse,
multiple color resolutions over the same op stream. Emits
`--dmc-{mode}` CSS variables.

## Native transformer

Rust-side AST pass that absorbs work the JS sidecar would otherwise
do. Examples: `Math` (replaces remark-math + rehype-katex),
`PrettyCode` (replaces rehype-pretty-code + shiki).

## NodeAction

Visitor return value: `Keep`, `KeepSkipChildren`, `Replace(Vec<Node>)`,
`Remove`. Drives `walk_root`.

## NodeSink

Trait for codegen consumers (`HtmlEmitter`, `MdxBodyEmitter`,
`Accumulator`). Methods: `enter`, `leave`. Driven by `Walker`.

## Origin

Where source came from: `File(path)`, `Stdin`, `Inline(static)`,
`Memory`. Used by transformers (e.g. `code-import`) to resolve
relative paths.

## Pipeline

Ordered list of `Transformer` instances. Built by
`Pipeline::with_defaults_for(cfg)`. Single uniform place for every
feature gate.

## Plugin gate

`CompileConfig::has_js_plugins` strips JS plugin names whose work
native transformers handle. Stripped names: `remark-gfm`,
`remark-math`, `remark-emoji`, `rehype-pretty-code`, `shiki`,
`rehype-katex`, `rehype-mathjax`, `rehype-slug`,
`rehype-autolink-headings`. After stripping, if both lists are
empty, sidecar is never spawned.

## Schema descriptor

JSON tree emitted by JS-side `s.object(...)`. Compiled to a runtime
validator by `dmc_schema::compile_descriptor`.

## Sidecar

Long-lived Node child running unified-style remark + rehype plugins
the dmc engine cannot run natively. Pooled by `dmc-core::sidecar`.
NDJSON over stdio.

## Single-tokenize multi-color

`highlight_code_multi` algorithm: parse + scope walk runs once;
each theme contributes only color resolution over the shared op
list. Cuts per-file syntect cost ~25% vs N independent calls.

## Slug

URL-safe heading anchor. `Heading::slug()` returns the slugified
plain text via `slug::slugify`. Used by `AutolinkHeadings`.

## SourceMeta

`{ path, version, origin }`. Carries source location across layers.
Every diagnostic span ties back to one.

## SyntaxBundle

Process-global syntect bundle (themes + grammars). Lazy-loaded once
via `OnceLock`.

## Transformer

Implementor of `Transformer` trait. Mutates a `Document` in place.
Built-ins live in `dmc-transform/src/builtin/`.

## Walker

Single pre-order DFS driver over a `Document`. Fires `enter` then
`leave` on every sink. Same walk feeds Accumulator + HtmlEmitter +
MdxBodyEmitter.
