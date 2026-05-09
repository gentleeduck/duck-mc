# How dmc works - full architecture

dmc is a Rust MDX compiler that drops in for velite. Same TS API surface (`defineConfig`, `s.*`), same JSON output shape, native pipeline by default with an opt-in Node sidecar for community remark/rehype plugins.

## Pipeline at a glance

```
.mdx source
  |
  v
+------------------+
| dmc-lexer    |  string -> Vec<Token> (frontmatter, GFM, JSX boundary)
`------------------+
  |
  v
+------------------+
| dmc-parser   |  tokens -> typed Document AST (block + inline + jsx + table)
`------------------+
  |
  v
+------------------+
| dmc-transform|  Pipeline of in-place AST mutations (slug, autolink, code-import, ...)
`------------------+
  |
  v
+------------------+
| dmc-codegen  |  AST -> html string + AST -> MDX function-body string
`------------------+
  |
  |---------->  s.markdown / s.mdx / s.toc / s.metadata fields filled
  |
  v
+------------------+
| dmc-schema   |  validates frontmatter; runs s.* primitives + dmc extras
`------------------+
  |
  v
+------------------+
| dmc-core     |  rayon par_iter per collection -> JSON + index.{js,d.ts}
|   (engine)       |
`------------------+
  |
  |---------->  optional: dmc-sidecar (Node child for user JS plugins)
  |
  v
+------------------+
| dmc-napi     |  TypeScript public surface (compile, build, definePlugin)
`------------------+
```

8 crates, sequential layers. Workspace declared in `Cargo.toml` at the repo root.

---

## Crate-by-crate

### 1. `dmc-lexer` - tokens

`Lexer { source, tokens, start, current, line, column }` in `dmc-lexer/src/lib.rs:13-22`.

Entry: `scan_tokens()` (`lib.rs:38`). Hot loop:

```rust
while !self.is_eof() {
  self.start = self.current;
  let c = self.advance();
  self.lex_tokens(c);
}
self.emit(TokenKind::Eof);
```

Per-character dispatch in `lexers/*.rs` produces:

- frontmatter delimiter (`---`)
- ATX heading (`#..######`), setext underlines (`==`, `--`)
- code fence (` ``` ` / `~~~`), inline code (`` ` ``)
- blockquote `>`, hr (`***`/`---`/`___`)
- list bullets / ordered numerals / task `[ ]`
- emphasis `*`, `**`, `_`, `__`, strikethrough `~~`
- link `[`, image `![`, autolink `<url>`
- JSX boundary `<` (capitalized -> JSX, lowercase -> HTML inline)
- escape `\x`, hard break `  \n` / `\\\n`, soft break

Trivia (whitespace, newline) discarded except **line-leading 4+ spaces** (`lib.rs:58-65`) - that one is preserved as the indented code-block marker.

Tokens carry `Span { file, line, column, length }` plus the lexeme as a `String`. Diagnostics flow through `duck-diagnostic::DiagnosticEngine<Code>` - lexer never panics, only emits + recovers.

### 2. `dmc-parser` - typed AST

Entry: `pub fn parse(source: &str) -> Document` (`dmc-parser/src/lib.rs`).

Files:

- `block.rs` (396 LOC) - paragraph, heading, list, blockquote, codeblock, hr, setext, frontmatter dispatch.
- `inline.rs` (257 LOC) - bold/italic/strike/code/link/image/escape/break.
- `jsx.rs` (148 LOC) - `<Component prop={...}>...</Component>`, fragments `<>...</>`, JSX expressions `{...}`.
- `table.rs` (130 LOC) - GFM pipe tables with alignment row.
- `parser.rs` - orchestration; `Document { children: Vec<Node>, diagnostics: Vec<Diagnostic> }`.

AST under `dmc-parser/src/ast/{node.rs,jsx.rs}`. Big `Node` enum: `Text`, `Heading`, `Paragraph`, `Bold`, `Italic`, `Strikethrough`, `InlineCode`, `CodeBlock`, `HorizontalRule`, `Link`, `Image`, `List`, `ListItem`, `TaskListItem`, `Blockquote`, `Table`, `JsxElement`, `JsxFragment`, `JsxExpression`, `Frontmatter`, `Import`, `Export`, `HardBreak`.

**No `serde_json::Value` on the hot path** - the AST is fully typed Rust enums. `Value` only appears for `Frontmatter.data` (already-parsed YAML/TOML/JSON) and at the final output boundary.

Heading `id` is filled in this layer (slug crate) so it's available to TOC builder + autolink transformer downstream.

### 3. `dmc-transform` - AST mutations

`Pipeline` of `Box<dyn Transformer>` in `dmc-transform/src/pipeline.rs:11-45`. Default set (`Pipeline::with_defaults()`, line 26):

| Transformer        | File                           | What it does |
| ------------------ | ------------------------------ | ------------ |
| `CodeImport`       | `builtin/code_import.rs`       | Replaces `file=foo.rs` fence meta with file contents (range `{1,3-5}` slicing) |
| `NpmCommand`       | `builtin/npm_command.rs`       | Rewrites `npm`-typed code blocks for npm/pnpm/yarn/bun tabs |
| `BareUrlAutolink`  | `builtin/bare_url.rs`          | Wraps bare `https://...` text nodes in `<a href>` |
| `AutolinkHeadings` | `builtin/autolink_headings.rs` | Adds `<a class="subheading-anchor">` to every heading |

Optional / config-driven (added by engine, not in defaults):

- `DisableGfm` (`builtin/disable_gfm.rs`) - strips GFM-specific nodes back to plain markdown when `markdown.gfm = false`.
- `CopyLinkedFiles` (`builtin/copy_linked_files.rs`) - hashes + copies `Image.url` / `Link.url` into `output.assets`, rewrites URLs to `output.base`.
- `Mermaid` (`builtin/mermaid.rs`) - feature-flagged; renders mermaid fences.
- `ComponentPreview`, `ComponentSource` (`builtin/component_*.rs`) - dmc extras for shadcn-style component docs.

Transformers see `&mut Document`, mutate in place, no allocations beyond the new node payloads. Order matters: `CodeImport` runs first so subsequent transformers see real code; `BareUrlAutolink` runs before HTML emission.

### 4. `dmc-codegen` - emitters

`dmc-codegen/src/lib.rs` re-exports two emitters:

- `render_html(&Document) -> String` - produces sanitized HTML (used by velite parity, by `s.markdown()`, and as input to the optional sidecar).
- `render_mdx_body(&Document) -> String` - produces a JS function body (the velite-compatible `_createMdxContent` factory). Output is consumable by `new Function(body)(jsxRuntime, components)` at runtime.

`escape.rs` - JS-safe and HTML-safe escapers, called per text/inline node.

`html.rs` (261 LOC) walks the AST and emits HTML; `mdx.rs` (192 LOC) emits `jsx(...)` / `jsxs(...)` calls. Both are heavy on `format!` returning fresh `String`s - see `docs/perf-plan.md` U4/U5 for the planned writer refactor.

MDX module wrapping (`mdx_output_format = "module"`) and minify (`mdx_minify = true`) happen post-emit in `engine.rs`.

### 5. `dmc-schema` - frontmatter validation + dmc extras

`Schema` trait (`dmc-schema/src/lib.rs:35-37`):

```rust
pub trait Schema: Send + Sync {
  fn parse(&self, value: &Value, ctx: &Ctx) -> Result<Value, ValidationError>;
}
```

`pub mod s` (line 39) is the velite-parity builder namespace. Primitives in `primitives.rs`:

- `string()`, `number()`, `boolean()`, `array(item)`, `object(fields)`, `record(value)`, `tuple(items)`
- `enum_(variants)`, `literal(expected)`, `union(variants)`, `intersection(l, r)`, `discriminated_union(disc, variants)`
- `optional(inner)`, `nullable(inner)`, `default_(inner, fallback)`
- `transform(inner, fn)`, `refine(inner, pred)`, `super_refine(inner, pred)`
- `coerce_string()`, `coerce_number()`, `coerce_boolean()`, `coerce_date()`

Velite-custom in `markdown.rs` + `asset.rs`:

- `raw()` - body markdown as-is.
- `markdown()` - body rendered to HTML.
- `mdx()` - body compiled to MDX function body.
- `toc()` - heading tree.
- `metadata()` - `{readingTime, wordCount}`.
- `excerpt()` - first N chars / first paragraph.
- `path()` - file path / slug from path.
- `slug()` - kebab-case from title or filename.
- `unique()` - deduplication marker (collection-level constraint).
- `isodate()` - ISO-8601 normalization.
- `file()` - hashed asset URL (uses `AssetPipeline`).
- `image()` - hashed asset URL + `{src, width, height, blurDataURL?, blurWidth?, blurHeight?}`.

`ctx.rs` carries `Ctx { meta: Value, assets: Option<&mut AssetPipeline> }`. `compile.rs` (193 LOC) takes a JSON descriptor (the napi side ships it from TS) and reconstructs a `Box<dyn Schema>` tree - this is how the TS `s.string().max(99)` chain reaches the Rust validator without re-implementing zod.

### 6. `dmc-core::loaders` - frontmatter format dispatch

`loaders/{matter,yaml,json}.rs` - selects parser by frontmatter delimiter or file extension. `matter` is the default (`---` YAML or TOML wrapper). All return `(data: Value, content: &str)` and feed `Node::Frontmatter`.

`LoaderRegistry` accepts user-registered loaders via `loaders[]` in the config (passed through napi).

### 7. `dmc-core::engine` - collections + rayon + output

`EngineConfig` (`engine.rs:18-83`) is the kitchen-sink struct: collections, output dirs, plugin lists, format flags, gfm toggle, include_html, mdx_minify, etc. Default values at line 60 mirror velite.

`run(cfg) -> EngineReport` orchestrates per-collection:

```rust
let outcomes: Vec<(Value, Option<EngineError>)> = paths
  .par_iter()                                // rayon parallel
  .map(|path| {
    let source = std::fs::read_to_string(path)?;
    let mut compiled = {
      let mut pipeline = dmc_transform::Pipeline::with_defaults();
      // optional DisableGfm + CopyLinkedFiles wired in here
      crate::compile_with_pipeline(&source, &pipeline)
    };
    if has_js_plugins(cfg) {
      compiled.html = run_sidecar(&compiled.content, cfg)?;   // WARN per-file Node spawn
    }
    if mdx_output_format == "module" { compiled.body = wrap_mdx_module(...); }
    if mdx_minify { compiled.body = minify_js(...); }
    let validated = schema.parse(&compiled.frontmatter, &ctx)?;
    build_velite_record(compiled, validated, path, base_dir, name, include_html)
  })
  .collect();
```

(`engine.rs:149-207`, abbreviated.) Then writes `<output_dir>/<collection>.json`, `index.js` (re-exports per collection), and `index.d.ts` (typed `Collections['<name>']['schema']['_output']`).

`single: true` collections write a single object instead of an array. `clean: true` removes the output dir before writing.

`build_velite_record` (referenced from `engine.rs:197`) is what shapes each entry into velite's exact JSON: frontmatter hoisted to top level, plus computed `body`, `code` (legacy alias), `excerpt`, `metadata`, `toc`, `slug`, `path`, `permalink`.

Watch mode (`Cmd::Dev`): `notify` crate watches each collection's `base_dir`. On change, rebuild only that collection (per-collection incremental, not full).

### 8. `dmc-sidecar` - opt-in JS plugin runner

`dmc-sidecar/index.mjs` is a tiny Node script. Reads one JSON request from stdin, returns one JSON response on stdout:

```
{ markdown, remarkPlugins, rehypePlugins }   ->   { html, messages }
```

Pipeline:

```js
unified()
  .use(remarkParse)
  .use(...userRemarkPlugins)
  .use(remarkRehype, { allowDangerousHtml: true })
  .use(rehypeRaw)
  .use(...userRehypePlugins)
  .use(rehypeStringify, { allowDangerousHtml: true })
```

Plugin resolution uses `createRequire(cwd/package.json)` so user-installed packages resolve correctly. Strings -> `import(spec)`, tuples `[name, opts]` -> `import(name)` then `proc.use(plugin, opts)`.

The engine spawns this via `Command::new("node")` once per file when `has_js_plugins(cfg)` returns true. **This is the single biggest perf bottleneck** - see `docs/perf-plan.md` U1.

### 9. `dmc-napi` - TypeScript public surface

`dmc-napi/mod.ts` (793 LOC) is a single TS-only entry. No compiled `mod.js`; consumers use `node` >=20 with `--experimental-strip-types` or bun/tsx, and the napi `index.js` (the Rust binding) is loaded via `createRequire`.

Exports:

- `compile(source: string): CompileOutput` - synchronous in-process compile.
- `compileMany(sources: string[]): CompileOutput[]` - batched.
- `build(config: UserConfig): Promise<EngineReport>` - runs the engine.
- `defineConfig(cfg)`, `defineCollection(c)`, `defineLoader(l)`, `defineSchema(s)` - identity-typed helpers (velite-shape).
- `s` - the `s.*` builder bridge: each call produces a JSON descriptor fed to the Rust `compile_descriptor` (see section 5).
- `definePlugin(plugin, options)` - type-safe `[plugin, options]` tuple. Generic infers `Params` from the plugin's first parameter type, so `definePlugin(rehypePrettyCode, { theme: 'invalid' })` is a TS error at config time.
- `Plugin`, `Pluggable` - re-exported from `unified`.

Sidecar path resolution (`mod.ts:47-56`): tries package-relative paths in order, sets `process.env.dmc_SIDECAR` so the Rust engine finds the entry without hard-coding.

User config is allowed to use plugin **function references** directly (not strings), because the JS-side `processWithUnified` runs unified in-process for the simple case. Only when the engine drops to the Rust `run()` path do plugin names get serialized to strings for the sidecar.

---

## End-to-end flow for a single file

1. `dmc build --config dmc.toml` -> `Cmd::Build` -> `cmd_build` loads TOML or evaluates TS config (via bun / `node --import tsx`).
2. `EngineConfig` constructed; `run(cfg)` enters per-collection loop.
3. `globwalk` resolves the collection pattern -> `Vec<PathBuf>`.
4. `paths.par_iter().map(...)` (rayon) - each file:
   1. `fs::read_to_string` -> `String`.
   2. `dmc_parser::parse(&source)` -> `Document` (lexer + parser).
   3. `Pipeline::with_defaults().run(&mut doc)` (transformers).
   4. `finalize(source, doc)` (compile.rs:46) computes `html`, `body`, `excerpt`, `metadata`, `toc`, plus `imports` / `exports` from the AST.
   5. If JS plugins configured: `run_sidecar` (Node child) overrides `html`.
   6. Optional `wrap_mdx_module` + `minify_js`.
   7. `schema.parse(&frontmatter, &ctx)` validates and runs `s.markdown` / `s.mdx` / `s.toc` / `s.image` / `s.file` etc, mutating `Ctx.assets` for hashed copies.
   8. `build_velite_record` shapes the record (camelCase, frontmatter hoisted).
5. `serde_json::to_string_pretty` -> `fs::write(output_dir/<collection>.json)`.
6. `write_index_js` + `write_index_dts` -> typed re-exports.
7. `EngineReport { collections, errors }` returned to napi caller (or printed by CLI).

## Public APIs

**CLI:**

```sh
dmc build --config dmc.config.ts
dmc dev   --config dmc.config.ts
dmc compile path/to/file.mdx
dmc init
```

**Rust crate:**

```rust
use dmc::{compile, run, EngineConfig};

let out = compile(source);     // CompileOutput { body, html, frontmatter, toc, ... }
let report = run(&cfg)?;       // EngineReport { collections: [...], errors: [...] }
```

**TypeScript (napi):**

```ts
import { defineConfig, s, definePlugin, compile } from "@duck/md";
import remarkGfm from "remark-gfm";
import rehypePrettyCode from "rehype-pretty-code";

export default defineConfig({
  collections: {
    docs: {
      name: "Doc",
      pattern: "content/docs/**/*.mdx",
      schema: s.object({
        title: s.string().max(99),
        body: s.mdx(),
      }),
    },
  },
  mdx: {
    remarkPlugins: [remarkGfm],
    rehypePlugins: [
      definePlugin(rehypePrettyCode, { theme: "github-dark" }),
    ],
  },
});
```

## Non-obvious design choices

- **Typed Rust AST end-to-end.** No `serde_json::Value` walking. Codegen emits directly from enum variants.
- **Schema is a JSON descriptor, not an FFI struct.** TS builds a small JSON tree with `s.*`; Rust reconstructs the trait objects from the JSON in one pass at load time. Keeps the FFI wire dumb.
- **Frontmatter formats are pluggable** at the loader layer, not the schema layer.
- **GFM is in the parser, not a transformer.** Tables / strikethrough / task lists are first-class AST nodes; `DisableGfm` is the opt-out, not the opt-in.
- **HTML and MDX-body emit from the same AST.** No second parse, no second walk. `s.markdown` and `s.mdx` are cheap because they read pre-computed fields.
- **rayon parallelism is per-file, not per-collection.** Collections run sequentially; files within a collection run on all cores. This is what gets the 159x scale lead at 999 files.
- **Sidecar is opt-in.** No JS in the build path unless the user configures a plugin list. Native pipeline ships the velite default plugin equivalents in Rust (slug, autolink-headings, gfm).

## Where to read next

- Parsing internals -> `dmc-parser/src/parser.rs` then `block.rs`, `inline.rs`.
- Transformer authoring -> `dmc-transform/src/pipeline.rs` + any `builtin/<name>.rs` as a template.
- Schema authoring -> `dmc-schema/src/primitives.rs` for the trait pattern; `markdown.rs` for AST-aware schemas.
- Engine wiring -> `dmc-core/src/engine.rs` from `pub fn run` downward.
- TS surface -> `dmc-napi/mod.ts` (single file).
- Performance -> `docs/perf-plan.md` and `docs/benchmarks.md`.
