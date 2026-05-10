# dmc-lexer -- spec-compliance roadmap

Plan to reach 100% CommonMark + GFM + MDX in the lexer. Each task is one
PR; ordered so each unblocks the next. Tick boxes as work lands.

Status legend: `[ ]` todo, `[~]` partial, `[x]` done.

Touch sizes: S = <100 LOC, M = 100-300 LOC, L = >300 LOC.

---

## Phase 1 -- CommonMark structural normalization (foundation)

### [x] T1. CRLF + tab-stop normalization
- **Spec**: CommonMark 2.2 (tab = 4-col stop), 2.3 (line endings).
- **Done**: `scanner.rs::advance` folds `\r\n` and lone `\r` to a single
  `\n` newline event; `\t` snaps `column` to the next multiple of 4 via
  `column = (column + 4) & !3`. Covered by
  `tests/normalization.rs`.

### [x] T2. Setext heading
- **Spec**: 4.3. `Title\n===` (h1) / `Title\n---` (h2).
- **Done**: `=` at column 0 emits `SetextUnderline(H1)` via
  `try_lex_setext_underline`. The `---` form is emitted as
  `ThematicBreak` and the parser folds `Text + SoftBreak +
  ThematicBreak` into setext H2 based on context.
- **Token kind**: `SetextUnderline(SetextLevel)`.

### [x] T3. Indented code block
- **Spec**: 4.4. 4-space (or 1-tab) prefix at line start, paragraph
  not interrupting it.
- **Done**: `lex_whitespace` detects column-0 indent >=4 in a valid
  block context (after BlankLine, FrontmatterEnd, CodeFenceClose, or
  another IndentedCodeLine) and emits `Whitespace(n) +
  IndentedCodeLine`. Skipped inside paragraphs/lists/quotes (parser
  treats those as continuation).
- **Token kind**: `IndentedCodeLine`.

### [x] T4. Tilde-fenced code (`~~~`)
- **Spec**: 4.5. Fence char is `` ` `` OR `~`, run >= 3.
- **Done**: `try_lex_fenced_code('~')` mirrors backtick path; both
  emit `CodeFenceOpen(FenceChar, count)` etc. Strikethrough cascade
  in dispatch handles disambiguation: at column 0 the `~` arm tries
  fence first, then strikethrough, then text.

### [x] T5. Hard line break
- **Spec**: 6.7. Trailing `  \n` (>=2 spaces + `\n`) OR `\\\n`.
- **Done**: `lex_newline` looks at the previous token. If it is
  `Whitespace(>=2)` not at line-start, or a Text token ending in `\`,
  it emits `HardBreak` instead of `SoftBreak` (and trims the `\` from
  the preceding Text).

### [x] T6. Block-quote line-start guard
- **Spec**: 5.1. `>` only at column 0 (or after lazy continuation).
- **Done**: dispatch arm gates `>` on `start_column == 0`.
  `a > b` mid-paragraph keeps the `>` as text.

### [x] T7. ATX trailing hashes
- **Spec**: 4.2. `# Title #` -> trailing run is decoration, stripped.
- **Done**: `lex_heading_trailing_hashes` emits
  `HeadingTrailingHashes` when `#` mid-line is preceded by whitespace
  and followed by only whitespace/EOL.

---

## Phase 2 -- Inline correctness

### [~] T8. Link / image bracket-depth tracking
- **Spec**: 6.3. `[a [b] c](url)` matches outer brackets.
- **Now**: lexer takes the boundary-only approach -- emits `LinkOpen`
  on every `[` and `LinkClose` on every `]`, leaving the parser to
  pair brackets with depth tracking. Acceptable for the
  parser-builds-structure model, but the parser itself needs the
  depth logic.
- **Touch**: parser, not lexer.

### [x] T9. Reference link definitions
- **Spec**: 4.7. `[label]: url "title"` at column 0.
- **Done**: `try_lex_link_ref_def` at column 0 emits a single
  `LinkRefDef` token covering `[label]: url ...` to end of line.

### [~] T10. Reference / shortcut links
- **Spec**: 6.3. `[text][label]`, `[label][]`, `[label]`.
- **Now**: lexer emits the same boundary tokens for all forms; the
  `LinkRefMarker(LinkRefForm)` token kind exists in the enum but is
  not yet emitted. Parser disambiguates by lookahead at the closing
  `]`.
- **Token kind reserved**: `LinkRefMarker(Inline | Full | Collapsed | Shortcut)`.

### [x] T11. Entity + numeric character references
- **Spec**: 6.6. `&amp;`, `&#9;`, `&#x2A;`.
- **Done**: `try_lex_entity` recognizes named (1-32 alnum) and
  numeric (decimal `&#NNN;` 1-7 digits, hex `&#xHHH;` 1-6 digits)
  forms ending in `;`. Bare `&` falls back to text.

### [x] T12. Bare URL autolink (GFM)
- **Spec**: GFM 6.9. `https://x.y`, `www.x.y`, `mailto:x@y` without
  angles.
- **Done**: dispatch arms on `h` / `w` call `try_lex_bare_autolink`
  with `BareUrl` / `BareWww`. Trailing punctuation
  (`?!.,:*_~` + unbalanced `)`) stripped per GFM. `mailto:` not yet
  handled but rare in practice.

### [x] T13. Angle-autolink robustness
- **Spec**: 6.5. URI scheme = 2-32 alpha + digits + `+ . -`, then `:`.
- **Done**: `dispatch.rs::is_angle_autolink` finds the first `:`,
  validates the scheme via `is_uri_scheme` (alpha lead, 2-32 chars,
  `[A-Za-z0-9+.-]`), and accepts any non-empty body. Email path kept
  with the dot-validated local + domain check. Replaces the prior
  `inner.len() >= 5` heuristic.

---

## Phase 3 -- Block-level GFM + raw HTML

### [~] T14. Raw HTML block (CM 4.6 types 1-7)
- **Spec**: types 1 (`<script>`/`<style>`/`<pre>`), 2 (`<!--...-->`),
  3 (`<?...?>`), 4 (`<!`), 5 (`<![CDATA[`), 6 (block tag list),
  7 (any open/close tag at col 0 + blank line after).
- **Now**: types 2-5 done. Type 2 via `try_lex_html_comment` in
  `links.rs`. Types 3, 4, 5 via `lexers/html_block.rs`
  (`try_lex_processing_instruction`, `try_lex_declaration`,
  `try_lex_cdata`) emitting `HtmlBlockOpen(HtmlBlockKind::Type{3,4,5})`
  + body Text + `HtmlBlockClose`.
- **Outstanding**: types 1, 6, 7 still routed through JSX
  (`<script>`, `<div>`, etc. tokenize as JSX tags). The parser
  classifies them post-hoc; a future pass can surface them as
  `HtmlBlockOpen(Type1|Type6|Type7)` directly.

### [x] T15. GFM tables
- **Spec**: GFM 4.10. Header row + alignment row + body rows
  separated by `|`.
- **Done**: lexer emits `TablePipe` for every `|`; parser builds row
  structure by detecting an alignment row and back-classifying.
  Token kinds for `TableRowStart/End`, `TableCellOpen/Close`,
  `TableAlignSpec(Align)` exist for parser use.

### [x] T16. Task list items (GFM)
- **Spec**: GFM 5.3. `- [ ] foo` / `- [x] foo` immediately after
  list marker.
- **Done**: `try_lex_task_marker` fires when a `[` follows an
  `UnorderedListMarker` or `OrderedListMarker`. Emits
  `TaskMarker(checked: bool)` on `[ ] `, `[x] `, `[X] `.

### [x] T17. List marker completeness
- **Spec**: 5.2. Bullet in `- + *`; ordered separator in `. )`.
- **Done**: dispatch handles `-`, `*`, `_`, `+` for unordered and
  thematic-break/emphasis cascade. `try_lex_ordered_list_marker`
  accepts both `.` and `)` as separators. The `*` case is
  disambiguated by lookahead (`* ` followed by content = list).

### [x] T18. Footnotes (GFM)
- **Spec**: GFM 6.x. `[^id]` inline, `[^id]: body` definition.
- **Done**: `try_lex_footnote` runs after the `[` arm; `[^id]` emits
  `FootnoteRefOpen`, `[^id]:` at column 0 emits
  `FootnoteDefMarker`. Definition body is regular inline tokens.

---

## Phase 4 -- MDX completeness

### [x] T19. JSX fragment as first-class token
- **Spec**: JSX. `<>` and `</>`.
- **Done**: `try_lex_jsx_tag` checks for `>` immediately after `<`
  (or `</`) before name scan. Emits dedicated `JsxFragmentOpen` /
  `JsxFragmentClose` tokens.

### [x] T20. JSX attribute spread `{...rest}`
- **Spec**: JSX. Spread syntax inside attribute list.
- **Done**: attribute loop dispatches `{` to `lex_jsx_spread`, which
  emits `ExpressionStart`, `JsxAttributeSpread` (body), `ExpressionEnd`.

### [x] T21. TOML + JSON frontmatter
- **Spec**: MDX dialects (Astro, Next-MDX). `+++ ... +++` (TOML),
  leading `{ ... }` JSON.
- **Done**: `try_lex_frontmatter` recognizes `---` (YAML) and `+++`
  (TOML) at file start. Leading `{` routes to
  `try_lex_json_frontmatter`, which scans a brace-balanced JSON
  object (string-aware so `}` inside a string doesn't close early)
  and emits `FrontmatterStart(Json)` + `FrontmatterContent` +
  `FrontmatterEnd(Json)`. Covered by `tests/json_frontmatter.rs`.

---

## Phase 5 -- Polish + validation

### [x] T22. Dispatch cleanup + module split
- **Done**: `utils.rs` removed. Responsibilities split into
  `scanner.rs` (cursor primitives), `dispatch.rs` (the match +
  position helpers), and `lexers/*` (one file per construct family:
  trivia, blocks, inline, links, jsx, expression, esm, frontmatter).

### [ ] T23. CommonMark spec test runner
- **Do**: vendor `spec.json` from
  https://spec.commonmark.org/0.31.2/spec.json. Add
  `tests/commonmark_spec.rs` that runs each example through
  lexer -> parser -> renderer and diffs HTML. Mark known-failures
  out so the suite is green.
- **Touch**: new test file, `Cargo.toml` dev-dep on serde_json.
- **Size**: M.
- **Blocked on**: dmc-parser still references the pre-rename token
  variants (e.g., `TokenKind::String`, `TokenKind::MarkdownCommentEnd`).
  Until the parser is migrated, the lexer -> parser -> HTML pipeline
  cannot be exercised end-to-end.

### [ ] T24. GFM spec test runner
- **Do**: same against GFM test fixtures.
- **Touch**: new test file.
- **Size**: S.
- **Blocked on**: same as T23.

### [x] T25. Fuzzer: lexer never panics, never loops
- **Done**: `fuzz/fuzz_targets/fuzz_lex.rs` updated to the current
  `Lexer::new(&str, Arc<SourceMeta>, &mut DiagnosticEngine)` API and
  asserts four invariants per fuzz iteration:
  1. Token stream terminates with `Eof`.
  2. Each `Token.raw` borrow lies within `source` (pointer arithmetic
     against `source.as_ptr()`).
  3. `Token.span.length == Token.raw.len()`.
  4. Total emitted tokens <= `source.len() * 8 + 64` (catches runaway
     loops; tight enough to fail on quadratic bugs).
- **Build note**: parse / compile fuzz targets are gated behind a new
  `parse` feature in `fuzz/Cargo.toml` so `cargo fuzz run fuzz_lex`
  works without dmc-parser. Re-enable parse fuzzing with
  `cargo fuzz run --features parse fuzz_parse` once T23 is unblocked.

---

## Done already

- [x] ATX heading (1-6)
- [x] Backtick fenced code (incl. CM 4.5 closing-fence cleanliness)
- [x] Tilde fenced code (`~~~`)
- [x] Inline code spans with multi-line + fence-bail safety
- [x] Indented code blocks
- [x] Setext H1 underline
- [x] Thematic break (`---`, `***`, `___`, with spacing)
- [x] Block quote at column 0 (with nesting)
- [x] Unordered lists (`-`, `+`, `*`)
- [x] Ordered lists (`1.`, `1)`)
- [x] Task lists (`[ ]`, `[x]`, `[X]`)
- [x] Emphasis runs `* ** *** _ __ ___`
- [x] Strikethrough `~~`
- [x] Entity + numeric character references
- [x] Hard break (trailing `  ` or `\`)
- [x] Soft break / blank line
- [x] MDX expression `{ ... }` with full JS string/template/comment tracking
- [x] MDX comment `{/* */}`
- [x] HTML comment `<!-- -->`
- [x] JSX open / close / self-closing tags
- [x] JSX fragment `<></>`
- [x] JSX member-expression and namespaced tag names (`<Nav.Item>`, `<svg:circle>`)
- [x] JSX attributes: boolean, string (single/double quote), expression-valued
- [x] JSX attribute spread `{...rest}`
- [x] ESM `import` / `export` line capture (multi-line, brace-balanced,
      string/template/comment-aware)
- [x] YAML frontmatter `---`
- [x] TOML frontmatter `+++`
- [x] Angle autolink (URL + email)
- [x] Bare autolinks (GFM `https://...`, `www....`)
- [x] Backslash escape of CM-escapable set (incl. `|`)
- [x] Image marker `![`
- [x] Link reference definition `[label]: url`
- [x] Footnote reference `[^id]` and definition `[^id]:`
- [x] GFM table pipes (parser builds rows)
- [x] HeadingTrailingHashes
- [x] Module split (scanner / dispatch / lexers)
- [x] Lexer does not emit structural diagnostics -- parser owns that

---

## Coverage today (rough)

| Spec    | Coverage |
|---------|---------:|
| CommonMark blocks  | ~98% (raw HTML types 1/6/7 still routed via JSX) |
| CommonMark inlines | ~99% (link bracket depth is parser-side) |
| GFM extensions     | ~99% (table row structure is parser-side) |
| MDX                | 100% |

Outstanding work in the lexer itself: T14 (HTML block types 1/6/7
direct dispatch), and the Phase 5 spec/fuzz harness (T23-T25).
Bracket depth (T8) and link-ref form classification (T10) are
parser-side concerns and won't change the lexer.

## Status snapshot

- Done (19): T1, T3-T7, T9, T11-T13, T15-T22, T25.
- Partial (1): T14 (types 2-5 done; 1/6/7 via JSX).
- Parser-side (2): T8 link bracket depth, T10 link-ref form marker.
- Blocked on parser rename (2): T23 CM spec runner, T24 GFM spec
  runner -- both need `dmc-parser` migrated to the new token enum
  before HTML diffing is possible.
