# MDX Compiler — Build Progress

Tick `[x]` done, `[⚠]` blocked, `[~]` partial. Reasons in `.session/log.md`.

Goal: drop-in Rust replacement for Velite's role in `apps/duck` (see `SURVEY.md`). Tests required for every phase.

## Phase 1 — Lexer gaps

- [x] L1: `Token::HardBreak` + `Token::SoftBreak` from `lex_newline`
- [x] L2: backslash escapes in `lex_text`
- [x] L3: lex `](href)` after `]`
- [x] L4: image (covered via lex_link reuse)
- [x] L5: detect top-level `import` keyword at column 0 → `Token::Import` (single statement, balanced brackets for multi-line)
- [x] L6: detect top-level `export` keyword at column 0 → `Token::Export` (covered by L5 statements.rs)
- [x] L7: JSX boundary heuristic — `<` only enters JSX when next is `[A-Za-z/>]` (frag) else fall to text
- [x] L8: JSX attribute expression value `prop={expr}` — balanced-brace consumer
- [x] L9: JSX attribute boolean (no `=` value)
- [x] L10: markdown comment `{/* ... */}` — `MarkdownCommentStart` + `MarkdownCommentEnd`
- [ ] L11: blockquote multi-line; nest `>>`
- [ ] L12: thematic break `---|***|___` on own line (split from frontmatter path)
- [x] L13: dispatch `{` → `lex_expression` from main `lex_tokens`
- [ ] L14: JSX fragment `<>...</>`
- [ ] L15: lexer test crate `tests/lexer/*.rs` covering every TokenKind via `pretty_assertions` + table-driven tests
- [ ] L16: GFM table pipe `|` row detection (block-start)
- [ ] L17: GFM task list `- [ ] / - [x]`
- [ ] L18: GFM strikethrough `~~text~~`
- [ ] L19: GFM autolink `<https://...>` and bare URL detection
- [ ] L20: setext headings `===` / `---` underline
- [ ] L21: indented code block (4-space)
- [ ] L22: HTML inline tag passthrough (lowercase tags as raw HTML when not JSX)
- [ ] L23: Span column tracking — fix `column` to count graphemes, not bytes; track utf8 width
- [x] L24: bug fix — `lex_jsx_tag` after attrs now emits `JsxSelfClosingEnd` cleanly; parser workaround removed

## Phase 2 — AST

- [x] A1: new crate `duck-md-ast`. Workspace member. Define `Node`, `Document`, `Frontmatter`, `Heading`, `Paragraph`, `Text`, `CodeBlock`, `Link`, `Image`, `List`, `ListItem`, `Blockquote`, `ThematicBreak`, `HardBreak`, `SoftBreak`, `JsxElement`, `JsxSelfClosing`, `JsxExpression`, `JsxFragment`, `Import`, `Export`, `Bold`, `Italic`, `InlineCode`, `Strikethrough`, `Table`, `TableRow`, `TableCell`, `TaskListItem`
- [x] A2: `JsxAttr`, `JsxAttrValue::{String,Expression,Boolean}`
- [x] A3: `Span` on every node (reuse `duck_diagnostic::Span`); add `Position { line, column, offset }` (Span carries line/column/length already; offset deferred)
- [ ] A4: `Display` impl + tree-print debug helper
- [x] A5: `serde::Serialize` on every node so AST can be JSON-dumped (Span skipped — duck_diagnostic doesn't derive Serialize)
- [x] A6: `duck-md-ast/tests/ast_smoke.rs` — round-trip serialize/deserialize on a hand-built doc (named smoke.rs)

## Phase 3 — Parser

- [x] P1: new crate `duck-md-parser`. `Parser` struct, `peek/advance/expect`, `parse() -> Document` entry
- [x] P2: parse frontmatter
- [x] P3: parse top-level imports
- [x] P4: parse top-level exports
- [x] P5: parse heading + inline children
- [x] P6: parse paragraph + inline accumulator
- [x] P7: parse fenced code block (lang + meta)
- [x] P8: parse inline code
- [x] P9: parse bold + italic delimiter pairing
- [x] P10: parse link (text + href + optional title)
- [x] P11: parse image (alt + src + title)
- [ ] P12: parse unordered list + nested by indent
- [ ] P13: parse ordered list + start number
- [ ] P14: parse blockquote with nested children
- [ ] P15: parse thematic break
- [ ] P16: parse soft/hard break
- [x] P17: parse JSX self-closing
- [x] P18: parse JSX element (re-entrant block parse for children)
- [x] P19: parse JSX expression `{expr}`
- [x] P20: parse JSX fragment
- [ ] P21: parse GFM table
- [ ] P22: parse GFM task list item
- [ ] P23: parse GFM strikethrough
- [ ] P24: parser test suite — `duck-md-parser/tests/*.rs` per construct
- [ ] P25: error recovery — synthesize missing-close on unterminated JSX, continue; collect into `Diagnostic`s

## Phase 4 — Public API

- [x] X1: convert `duck-md-core` from binary to library + binary. `lib.rs` exposes `parse(source) -> Document`
- [x] X2: `compile(source) -> CompileOutput { frontmatter, frontmatter_raw, content, html, excerpt, metadata, toc, imports, exports }`
- [ ] X3: integration fixtures `duck-md-core/tests/fixtures/*.mdx` + golden JSON outputs
- [ ] X4: `duck-md-core/tests/integration.rs` — golden diff per fixture

## Phase 5 — Codegen (HTML)

- [x] C1: new crate `duck-md-codegen`. `HtmlEmitter` struct with output buffer + escape helpers
- [x] C2: emit Heading (with id slug), Paragraph, Text (escaped), Bold, Italic, InlineCode, Strikethrough, CodeBlock (no highlight)
- [x] C3: emit Link, Image, List, ListItem, TaskListItem, Blockquote, ThematicBreak, HardBreak (`<br/>`), SoftBreak (newline)
- [ ] C4: emit Table, TableRow, TableCell with align attrs
- [x] C5: emit JSX self-closing, JSX element, JSX expression as JSX-string passthrough into HTML
- [x] C6: tests — `duck-md-codegen/tests/html.rs` golden per construct

## Phase 6 — Codegen (MDX body — JS function source)

See SURVEY.md §I for required output shape.

- [x] M1: `MdxBodyEmitter` struct producing the `function _createMdxContent(props) { ... }` shell
- [x] M2: emit Heading/Paragraph/Text/Bold/etc as `jsx("h1", {id, children: [...]})`
- [x] M3: emit JSX elements as `jsx(ComponentName, { ...props, children: [...] })`
- [x] M4: emit JSX expressions as embedded JS expressions in children arrays (passed through verbatim)
- [x] M5: emit Imports/Exports at module scope of the body
- [x] M6: tests — `duck-md-codegen/tests/mdx_body.rs` golden vs hand-checked snippets

## Phase 7 — Schema (Velite primitive parity)

- [ ] S1: new crate `duck-md-schema`. Type-level builder: `string(), boolean(), number(), object(map), array(item), optional(), default(value), max(N), min(N), regex(pat), enum_(["a","b"])`
- [x] S2: `mdx()` — `CompileOutput.body` populated via `render_mdx_body`
- [x] S3: `markdown()` — provided as `CompileOutput.content` (raw md sans frontmatter)
- [x] S4: `excerpt(opts)` — provided as `CompileOutput.excerpt` (strip MD, truncate 260)
- [x] S5: `metadata()` — provided as `CompileOutput.metadata` (200 wpm)
- [x] S6: `toc()` — provided as `CompileOutput.toc` (nested via index-path stack)
- [ ] S7: schema parse error type with rich path (e.g. `frontmatter.title: too long`)
- [ ] S8: `transform(fn)` post-step
- [ ] S9: tests — `duck-md-schema/tests/*.rs` per primitive

## Phase 8 — Transform pipeline

- [ ] T1: new crate `duck-md-transform`. `MdastVisitor` trait + `walk_mdast` + mutate-in-place
- [ ] T2: `HastVisitor` trait + `walk_hast` + mutate-in-place (we'll synthesize hast from our AST first)
- [ ] T3: hast node types (Element, Text, Comment, Root) in `duck-md-ast` or here
- [ ] T4: ordering API matching velite's `before`/`after` hook split
- [ ] T5: pipeline runner: source → lex → parse → mdast transforms → hast → hast transforms → emit
- [ ] T6: tests — `duck-md-transform/tests/walk.rs` for visitor mutation correctness

## Phase 9 — Built-in transformers (mirror velite plugins)

- [ ] B1: GFM helper transforms (autolink bare URL → `<a>`, expand task list class)
- [ ] B2: `code_import` — read `file=...` meta, inline file content respecting `{1,2-3}` ranges
- [ ] B3: `slug` — `id` on every heading via `slug` crate
- [ ] B4: `pretty_code` — syntect highlight, dual themes (catppuccin-mocha + github-light), wrap `<div data-rehype-pretty-code-fragment>` with paired `<pre>`, line/word marks
- [ ] B5: `metadata_plugin` — `__rawString__`, `__title__`, `__marks__` on `<code>` from fence meta
- [ ] B6: `pretty_code_title` — rename `<div data-rehype-pretty-code-title>` → `<figcaption>`
- [ ] B7: `pre_block_source` — propagate `__rawString__` to `<pre>` children
- [ ] B8: `npm_command` — derive yarn/pnpm/bun from `npm install` / `npx`
- [ ] B9: `autolink_headings` — wrap with `<a class="subheading-anchor" aria-label="Link to section">`
- [ ] B10: `component_source` — read `path` attr, list-or-file → tsx code block
- [ ] B11: `component_preview` — read `name`, look up registry index, locate file, rewrite imports, emit tsx
- [ ] B12: `mermaid` — feature-gated; shell out to `mmdc` if present; otherwise pass-through
- [ ] B13: tests — every transformer has `tests/transformers/<name>.rs` golden test

## Phase 10 — Frontmatter

- [ ] F1: YAML parse via `serde_yaml`
- [ ] F2: validate against schema; emit rich error
- [ ] F3: tests — typed/untyped, missing-required, type-mismatch

## Phase 11 — Collections + globs

- [x] G1: `CollectionConfig { name, pattern, base_dir }` type in `duck-md-core::engine`
- [x] G2: glob walk via `globwalk`
- [x] G3: per-file pipeline: read → compile → record
- [ ] G4: parallelism via `rayon` (deferred)
- [x] G5: tests — `tests/engine.rs` 2/2

## Phase 12 — CLI

- [ ] U1: `duck-md` binary in `duck-md-core` (or new `duck-md-cli` crate). Commands: `build`, `dev`, `init`
- [ ] U2: `build` — read `duck-md.toml` config, run pipeline, write `.duck-md/`
- [ ] U3: `dev` — `notify` watcher, incremental rebuild
- [ ] U4: `init` — scaffold default config
- [ ] U5: tests — `assert_cmd` integration test on a fixture project

## Phase 13 — Output

- [x] O1: write `.duck-md/<collection>.json` (array of records)
- [x] O2: write `.duck-md/index.js` mirroring velite's `export { default as <name> } from './...json' with { type: 'json' }`
- [x] O3: write `.duck-md/index.d.ts` (minimal `any[]`; richer types deferred)
- [ ] O4: tests — diff `.duck-md/docs.json` against `apps/duck/.velite/docs.json` on shared fixtures (sample subset)

## Phase 14 — Velite parity verification

- [ ] V1: vendor 5 representative MDX files from `apps/duck/content/docs/` into `tests/fixtures/velite-parity/`
- [ ] V2: vendor matching expected records from `apps/duck/.velite/docs.json` (slim down to schema fields we currently support)
- [ ] V3: golden test crate `tests/parity/*.rs` — load fixture, run pipeline, compare output structure (allow body/JS-source-string diffs but compare AST shape)
- [ ] V4: doc README parity reporting CLI: `duck-md parity --against <velite_dir>`

## Phase 15 — Hardening

- [ ] H1: `cargo clippy --all-targets -- -D warnings` clean
- [ ] H2: `cargo fmt` enforced via `rustfmt.toml` (already exists)
- [ ] H3: `criterion` bench `benches/parse_200line.rs`
- [ ] H4: `criterion` bench `benches/full_pipeline.rs`
- [ ] H5: fuzz target via `cargo-fuzz` for lexer (`fuzz_targets/fuzz_lex.rs`)
- [ ] H6: fuzz target for parser
- [ ] H7: error recovery — never panic on malformed input
- [ ] H8: snapshot tests via `insta` for AST + HTML output
- [ ] H9: feature flags: `gfm`, `mermaid`, `pretty-code`, `cli`
- [ ] H10: minimum supported Rust version pinned

## Phase 16 — Continuous expansion (after the above)

If all above ticked AND time/tokens remain, append more atomic tasks here and continue. Areas:
- more transformers (footnotes, definition list, callout, math/katex, abbr, emoji shortcodes, container syntax `:::tip`)
- more codegen targets (server components, MDX 2 esm output)
- LSP server: hover, diagnostics, go-to-component-source
- VSCode extension hookup
- WASM bundle for browser previews
- TypeScript bindings via `napi-rs`
- larger parity test corpus
- end-to-end consume `body` in a tiny renderer harness to verify output is valid JS
