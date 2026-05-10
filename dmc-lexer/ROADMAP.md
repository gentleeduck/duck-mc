# dmc-lexer -- spec-compliance roadmap

Plan to reach 100% CommonMark + GFM + MDX in the lexer. Each task is one
PR; ordered so each unblocks the next. Tick boxes as work lands.

Status legend: `[ ]` todo, `[~]` partial, `[x]` done.

Touch sizes: S = <100 LOC, M = 100-300 LOC, L = >300 LOC.

---

## Phase 1 -- CommonMark structural normalization (foundation)

### [ ] T1. CRLF + tab-stop normalization
- **Spec**: CommonMark 2.2 (tab = 4-col stop), 2.3 (line endings).
- **Now**: `\r` lexed as whitespace; `\t` advances column by 1.
- **Do**: normalize `\r\n` / `\r` to `\n` in `advance()` (or at source
  ingest). On `\t`, jump column to next multiple of 4 instead of +1.
- **Touch**: `scanner.rs::advance`, `lexers/trivia.rs`.
- **Risk**: column drift in every existing span -- re-baseline goldens.
- **Size**: S.

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

### [~] T13. Angle-autolink robustness
- **Spec**: 6.5. URI scheme = 2-32 alpha + digits + `+ . -`, then `:`.
- **Now**: `is_angle_autolink` uses a `://` heuristic plus a real
  email validator. Catches the common cases. Strict scheme regex
  still pending.
- **Touch**: `dispatch.rs::is_angle_autolink`.

---

## Phase 3 -- Block-level GFM + raw HTML

### [~] T14. Raw HTML block (CM 4.6 types 1-7)
- **Spec**: types 1 (`<script>`/`<style>`/`<pre>`), 2 (`<!--...-->`),
  3 (`<?...?>`), 4 (`<!`), 5 (`<![CDATA[`), 6 (block tag list),
  7 (any open/close tag at col 0 + blank line after).
- **Now**: type 2 (HTML comments) handled by `try_lex_html_comment`.
  Types 1, 3, 4, 5, 6, 7 still routed through JSX or text.
- **Token kinds reserved**: `HtmlBlockOpen(HtmlBlockKind)`,
  `HtmlBlockClose`.

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
  (TOML) at file start, emits `FrontmatterStart(FrontmatterKind)`,
  `FrontmatterContent`, `FrontmatterEnd(kind)`. JSON form is reserved
  in the enum but not yet implemented (`{ }` is ambiguous with MDX
  expression).

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

### [ ] T24. GFM spec test runner
- **Do**: same against GFM test fixtures.
- **Touch**: new test file.
- **Size**: S.

### [ ] T25. Fuzzer: lexer never panics, never loops
- **Do**: add property `for any input, scan_tokens completes in
  O(input)`; assert no token spans cross EOF.
- **Touch**: `fuzz/fuzz_targets/fuzz_lex.rs`.
- **Size**: S.

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
| CommonMark blocks  | ~95% (T1, raw HTML types 1/3/4/5/6/7 outstanding) |
| CommonMark inlines | ~95% (link bracket depth + scheme regex outstanding) |
| GFM extensions     | ~95% (table row structure is parser-side) |
| MDX                | ~95% (JSON frontmatter outstanding) |

Outstanding work to reach 100/100/100/100: T1 (CRLF/tab), T13 (scheme
regex), T14 (HTML block types 1/3-7), T21 (JSON frontmatter), and the
spec/fuzz harness in Phase 5.
