# MDX Compiler — Build Progress

Atomic tasks, ordered by priority. Tick `[x]` when done, `[⚠]` when blocked. Reasons in `.session/log.md`.

## Phase 1 — Lexer gaps

- [x] L1: add `Token::HardBreak` + `Token::SoftBreak` variants and emit them in `lex_newline` (two newlines = HardBreak, single = SoftBreak)
- [x] L2: handle escape sequences `\*`, `\_`, `\` `\``, `\<`, `\{` in `lex_text` — emit raw char as Text, do not break out
- [ ] L3: parse link target `](href)` after `]` in `lex_link` — emit Text(href) inside ParenOpen/ParenClose
- [ ] L4: parse image `![alt](src)` fully — alt text + src
- [ ] L5: detect top-level `import` keyword at column 0, emit `Token::Import` with full statement up to newline (handle multi-line via balanced brackets)
- [ ] L6: detect top-level `export` keyword at column 0, emit `Token::Export` with full statement
- [ ] L7: JSX boundary heuristic — only enter `lex_jsx_tag` when `<` followed by `[A-Za-z/]`. Otherwise emit as Text.
- [ ] L8: JSX attribute expression value `prop={expr}` — current code only handles string. Add `{...}` branch consuming balanced braces.
- [ ] L9: JSX attribute boolean `<Foo disabled />` — attr with no `=` value
- [ ] L10: markdown comment `{/* ... */}` — emit `MarkdownCommentStart` + content + `MarkdownCommentEnd`
- [ ] L11: blockquote multi-line — `>` at line start, consume until blank line; nest with `>>`
- [ ] L12: thematic break detection — `---`, `***`, `___` on own line (current frontmatter path partially covers)
- [ ] L13: dispatch into `lex_expression` from main `lex_tokens` — currently `{` falls through to `lex_text`
- [ ] L14: JSX fragment `<>...</>` — empty tag name
- [ ] L15: lexer test suite — add `tests/lexer_tests.rs` with cases per token kind, use `pretty_assertions`

## Phase 2 — AST

- [ ] A1: create `duck-md-parser` crate with `src/ast.rs` defining `Node`, `Document`, `Frontmatter`, `Heading`, `Paragraph`, `Text`, `CodeBlock`, `Link`, `Image`, `List`, `ListItem`, `JsxElement`, `JsxSelfClosing`, `Import`, `Export` per plan.md
- [ ] A2: `JsxAttr` + `JsxAttrValue` in ast.rs
- [ ] A3: position/span on every node (reuse `duck_diagnostic::Span`)
- [ ] A4: `Display` impl for `Node` (debug-friendly tree print)

## Phase 3 — Parser

- [ ] P1: parser skeleton — `Parser` struct, `peek`/`advance`/`expect`, `parse()` entry returning `Document`
- [ ] P2: parse frontmatter block
- [ ] P3: parse top-level imports
- [ ] P4: parse top-level exports
- [ ] P5: parse heading (level + inline children)
- [ ] P6: parse paragraph + inline accumulation
- [ ] P7: parse fenced code block (lang + content + meta)
- [ ] P8: parse inline code
- [ ] P9: parse bold + italic (handle delimiter run pairing)
- [ ] P10: parse link (text + href + optional title)
- [ ] P11: parse image (alt + src)
- [ ] P12: parse unordered list + nesting by indent
- [ ] P13: parse ordered list + start number
- [ ] P14: parse blockquote + nested children
- [ ] P15: parse thematic break
- [ ] P16: parse soft/hard break
- [ ] P17: parse JSX self-closing element
- [ ] P18: parse JSX element with children (re-entrant block parse)
- [ ] P19: parse JSX expression `{expr}`
- [ ] P20: parse JSX fragment
- [ ] P21: parser test suite — `duck-md-parser/tests/parser_tests.rs`

## Phase 4 — Public API

- [ ] X1: `duck_md::parse(source: &str) -> Result<Document, DiagnosticEngine>` in `duck-md-core/src/lib.rs` (convert binary into lib + bin)
- [ ] X2: `duck_md::compile(source: &str) -> Result<String, ...>` glue parse + codegen
- [ ] X3: integration test fixtures dir `tests/fixtures/*.mdx` + golden output

## Phase 5 — Codegen (HTML)

- [ ] C1: `duck-md-codegen` crate with `Html` emitter
- [ ] C2: emit Document, Heading (with slug id), Paragraph, Text (escaped), Bold, Italic, InlineCode, CodeBlock
- [ ] C3: emit Link, Image, List, ListItem, Blockquote, ThematicBreak, HardBreak, SoftBreak
- [ ] C4: emit JSX self-closing, JSX element, JSX expression as passthrough JSX strings
- [ ] C5: emit Frontmatter as JSON sidecar, not in HTML body
- [ ] C6: codegen test suite + golden fixtures

## Phase 6 — Polish

- [ ] R1: `miette` rich diagnostics — wire `Code` enum through, label spans
- [ ] R2: error recovery — parser continues after recoverable errors
- [ ] R3: `criterion` bench `benches/parse.rs` on a 200-line MDX
- [ ] R4: README.md (only if explicitly requested) — skip for now
