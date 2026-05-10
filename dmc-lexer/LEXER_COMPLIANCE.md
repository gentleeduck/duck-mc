# dmc-lexer — full compliance checklist

Every construct the lexer must tokenize correctly. Tick when implementation
+ tests both land. Each item lists: **syntax**, **edge cases**, **expected
token shape**, **tests to write**.

Scope: CommonMark 0.31.2 + GFM + MDX. Numbering matches the spec where
possible.

Status legend: `[ ]` todo · `[~]` partial · `[x]` done.

---

## §2. Preliminaries

### [ ] §2.1 Character set
- Treat input as UTF-8. Reject NUL or replace with U+FFFD (CM §2.3).
- **Tests**:
  - input with embedded `\0` → replaced or rejected, no panic
  - input with surrogate pair → handled as text
  - 4-byte UTF-8 (emoji) survives round-trip in span / lexeme

### [ ] §2.2 Tabs (4-stop)
- `\t` advances column to next multiple of 4, NOT +1.
- **Edge**: tab inside heading text vs. tab as indent prefix differ
  visually but lex the same.
- **Tests**:
  - `\t#` → not a heading (4-space indent code), `# \t` → heading
  - column reported by token spans matches CM tab-stop rules

### [ ] §2.3 Insecure characters / line endings
- Normalize `\r\n` and lone `\r` → `\n`.
- **Tests**:
  - mixed CRLF + LF document → token line numbers correct, no off-by-one
  - file ending with `\r` (no LF) → terminates cleanly

---

## §3. Blocks and inlines (precedence)

Lexer's job is structural tokens; precedence is the parser's. Lexer must
still:

### [x] Lazy continuation
- Lexer emits raw lines; parser handles continuation. No lexer change
  needed.

---

## §4. Leaf blocks

### [x] §4.1 Thematic break
- `---`, `***`, `___` (3+ chars, optional internal spaces).
- **Edge cases**:
  - `* * *` (spaces between) — currently NOT handled; lex_bold sees
    `*` runs separately
  - `- - -` similarly not handled
  - `---` at file start vs mid-document (frontmatter vs thematic)
- **Tests**:
  - `---` at line 0 with no closing `---` later → ThematicBreak
  - `---` mid-document → ThematicBreak
  - `* * *` → ThematicBreak (currently fails)
  - `***` followed by text on same line → not thematic

### [x] §4.2 ATX heading
- `# … ######` (1-6 `#` + space + content).
- **Edge cases**:
  - `#` with no space → text, not heading
  - `####### ` (7 `#`) → not a heading, plain text
  - trailing `# ###` → decoration, parser strips
  - empty heading `# ` → valid h1, empty content
  - leading 1-3 spaces allowed
- **Tests**:
  - `# x` through `###### x` levels 1-6
  - `####### x` → not a heading
  - `#x` (no space) → text
  - `# x #` → trailing hashes (T7 needed)
  - `   ## x` (3-space indent) → valid h2

### [ ] §4.3 Setext heading (T2)
- `Title\n===` (h1) / `Title\n---` (h2). Underline is `=+` or `-+`.
- **Edge cases**:
  - `---` after blank line → thematic break, NOT setext
  - `---` after text line → setext h2
  - `===` after text → setext h1 (no thematic-break alternative)
  - underline can have trailing spaces, no other content
  - multi-line title above the underline is allowed (becomes single h)
- **Tests**:
  - `Title\n===` → Heading(1) + content
  - `Title\n---` → Heading(2) + content (NOT thematic!)
  - `\n---` (blank above) → ThematicBreak
  - `Title\n=== ` (trailing space) → still setext
  - `Title\n=foo` → not setext, falls through

### [ ] §4.4 Indented code block (T3)
- 4 spaces or 1 tab at line start, paragraph not interrupting.
- **Edge cases**:
  - 4 spaces inside an existing list item are list-item content, not
    code (parser disambiguates — lexer emits indent marker, parser
    decides)
  - blank lines inside indented code are kept (not break)
  - first non-indented non-blank line ends the block
- **Tests**:
  - `    code` at col 0 → IndentedCodeStart
  - `   code` (3 spaces) → not code
  - `\tcode` → indented code (tab = 4)
  - `    a\n\n    b` → single block, blank line preserved
  - `1. item\n        nested` → list-item content (parser)

### [~] §4.5 Fenced code block
- Backtick: `` ``` `` ✓ done. Tilde: `~~~` ✗ pending (T4).
- **Edge cases**:
  - info string `` ```js title="x.ts" {1-3} `` — captured as Text after
    fence open, parser parses
  - fence with internal `` ``` `` of fewer backticks — content
  - close-fence cleanliness (CM §4.5: only spaces after closing run) ✓
    handled (`close_is_clean` check in `lex_fenced_code`)
  - missing close fence → consume to EOF, emit truncated body
  - 4-fence open closes only on 4-fence close (not 3)
- **Tests**:
  - `` ```js\nfoo\n``` `` round-trips
  - `` ```js title=\"x\" {1-3}\nfoo\n``` `` info string preserved
  - `` ````js\nfoo\n```\nbar\n```` `` 4-fence closes only on 4-fence
  - `` ~~~js\nfoo\n~~~ `` (T4) tilde fence
  - `` ```\n``` foo `` close has trailing non-space → not a close
  - unterminated fence → consume to EOF

### [ ] §4.6 HTML block (T14)
- Seven types per CM §4.6. Currently routes to JSX → wrong.
- **Type 1**: `<script>` / `<pre>` / `<style>` / `<textarea>`,
  closes on matching `</…>`.
- **Type 2**: `<!--` … `-->` ✓ partial (existing HTML comment).
- **Type 3**: `<?` … `?>` (processing instruction).
- **Type 4**: `<!` declaration (`<!DOCTYPE html>`).
- **Type 5**: `<![CDATA[` … `]]>`.
- **Type 6**: known block-tag set (`<div>`, `<table>`, `<p>`, …).
  Closes on blank line.
- **Type 7**: any other tag at col 0; closes on blank line. Cannot
  interrupt a paragraph.
- **Edge cases**:
  - `<div>foo</div>` on one line → Type 6, closes on blank line
  - `<MyComponent>` at col 0 → MDX JSX, NOT type 7
  - `<div>` followed immediately by code fence → Type 6 ends?
- **Tests**: one fixture per type 1-7; one mixed JSX-vs-html-block
  disambiguation test.

### [ ] §4.7 Link reference definition (T9)
- `[label]: url` or `[label]: url "title"` or
  `[label]: url\n"title"` (title can be on next line).
- **Edge cases**:
  - label is case-insensitive, normalized whitespace
  - dest can be `<bracketed>` or bare
  - title can be `"…"`, `'…'`, or `(…)`
  - multiple defs with same label — first wins
- **Tests**:
  - `[foo]: /url` → emit LinkRefDef
  - `[foo]: /url "title"` → with title
  - `[foo]: <a b>` → bracketed dest with space
  - `[foo]: /url\n  "title"` → multi-line title
  - `[FOO]: /url` then `[foo]` ref → resolves (case-insensitive)

### [x] §4.8 Paragraph
- Implicit. Any run of non-blank lines that doesn't match another block.
- **Tests**: covered by integration tests.

### [x] §4.9 Blank lines
- ≥1 blank line separates blocks. Lexer emits `HardBreak` (rename to
  `BlankLine` per A3).

---

## §5. Container blocks

### [~] §5.1 Block quotes
- `> ` at column 0; content lazily continues.
- **Edge cases**:
  - `>` mid-line → text, NOT blockquote (T6 fix needed)
  - `>>>` nested quotes
  - `> ` followed by lazy continuation lines without `>`
  - `>` immediately followed by code fence
- **Tests**:
  - `> quote` → BlockQuoteMarker + Text
  - `a > b` → text only, no marker (T6)
  - `>> nested` → two markers
  - `> a\nb` → lazy continuation (parser handles)

### [~] §5.2 List items
- Bullets: `-` ✓, `+` ✗, `*` ✗ (taken by bold).
- Ordered: `1.` ✓, `1)` ✗ (T17).
- **Edge cases**:
  - `-foo` (no space after) → text, not list
  - `1.foo` (no space) → text
  - `999999999. foo` valid (CM allows up to 9 digits)
  - `10000000000. foo` invalid (10+ digits)
  - `1.   foo` (multi-space) → indent of first non-space sets content
    column
  - tight vs loose list (parser concern)
  - `- [ ] task` → task list (T16)
- **Tests**:
  - all three bullet flavors
  - `1.` and `1)` separators
  - `-foo` → text
  - `9999999999. x` → text (overflow)
  - 3-space indented list marker

---

## §6. Inlines

### [x] §6.1 Backslash escape
- `\\`, `\*`, `\_`, etc. (escapable set in CM appendix).
- **Edge cases**:
  - `\<` escapes — recognised in `lex_text`
  - `\a` (non-escapable) → kept literal `\` + `a`
  - `\\\*` → `\` then escaped `*`
- **Tests**:
  - every char in CM escapable set
  - `\a` non-escapable
  - `\` at EOL (T5: hard break vs trailing backslash)

### [ ] §6.2 Entity / numeric character refs (T11)
- `&amp;`, `&copy;`, `&#9;`, `&#x2A;`.
- **Edge cases**:
  - unknown name (`&fakename;`) → kept as text
  - missing semicolon → kept as text
  - numeric out of Unicode range → text
  - `&#0;` → U+FFFD per HTML spec
- **Tests**:
  - named, decimal, hex flavors
  - missing `;` — falls through
  - `&unknownname;` — text

### [x] §6.3 Code spans
- `` `code` ``, `` ``code with ` inside`` ``.
- **Edge cases**:
  - matching backtick run length (already handled)
  - leading/trailing single space stripped by parser, not lexer
  - line endings inside become spaces
  - `` `` ` `` `` valid (escape ` via doubling)
- **Tests**:
  - 1-, 2-, 3-tick spans
  - mismatched lengths → fall through to text
  - newline inside, blank-line bail
  - column-0 fence-open inside inline span (covered)

### [~] §6.4 Emphasis and strong emphasis
- `*` / `_` / `**` / `__` / `***` / `___`.
- **Edge cases (parser-side mostly)**:
  - left-flanking vs right-flanking (CM §6.4)
  - intra-word `_` no-emphasis rule (`foo_bar_baz` → no italics)
  - intra-word `*` IS allowed
- **Lexer side**:
  - emit run lengths so parser pairs them
  - `**` standalone vs `***` standalone
- **Tests**:
  - `*x*`, `**x**`, `***x***`
  - `*x*y*` → ambiguous, parser decides
  - `__x__y` → no emphasis
  - `*foo bar*` valid

### [x] §6.5 Links (auto)
- `<https://x.y>`, `<a@b.c>`. Done with limitations (T13).
- **Edge cases**:
  - `<scheme:path>` — short URI rejected today (`inner.len() >= 5`)
  - `<mailto:x@y>` — currently rejected by URI path
  - email with plus-sign `<a+b@c.d>` — allowed
- **Tests**:
  - all valid URI schemes including `mailto`, `irc`, `data`
  - email round-trip
  - invalid: `<not a url>` → text

### [~] §6.6 Inline links
- `[text](url)`, `[text](url "title")`.
- **Edge cases**:
  - nested brackets `[foo [bar] baz](u)` (T8 — depth)
  - escaped brackets `\[a\]` in text
  - empty text `[]( )` valid
  - title with single, double, or paren delimiters
  - dest `<bracketed url>` (allows spaces)
  - newline inside dest illegal — fall through
- **Tests**:
  - all delim flavors for title
  - bracketed dest with embedded space
  - depth-2 nested brackets
  - `[a\]b](u)` escape

### [~] §6.7 Images
- `![alt](src)` — parses via `lex_link` after `Bang`.
- **Edge cases**: same as links.
- **Tests**: per §6.6 plus `![alt][ref]` reference image (T10).

### [ ] §6.8 Hard line breaks (T5)
- `  \n` (2+ trailing spaces) or `\\\n` → `<br>`.
- **Edge cases**:
  - `\` at EOF (no newline) → text
  - 1 trailing space + `\n` → soft break
  - `   \n` (3 spaces) → still hard
- **Tests**:
  - `a  \nb` → HardBreak/LineBreak between
  - `a\\\nb` → LineBreak
  - `a \nb` → SoftBreak
  - `a\` at EOF → text

### [x] §6.9 Soft line breaks
- Single `\n` between content.
- **Tests**: `a\nb` → SoftBreak.

### [ ] §6.10 Textual content
- Everything else.
- **Tests**: covered by integration goldens.

---

## GFM extensions

### [x] GFM §6.5 Strikethrough
- `~~text~~`. Two tildes only (CM allows 1-2; GFM picks 2).
- **Edge cases**:
  - `~~~` → 3 tildes is fence (T4) not strike
  - `~text~` → not strike (single tilde rejected, falls to text)
- **Tests**:
  - `~~x~~`
  - `~x~` → text
  - `~~~js` → tilde fence open (T4 once added)

### [ ] GFM §4.10 Tables (T15)
- Header row + alignment row + body.
- **Edge cases**:
  - `:---:` center, `---:` right, `:---` left, `---` default
  - escaped pipe `\|` in cell
  - missing leading/trailing pipes valid
  - alignment row alone with no header → not a table
  - inline content per cell parses normally
- **Tests**:
  - all 4 alignments × 3 columns
  - escaped pipe in cell
  - header without leading `|`
  - alignment-only line → text
  - cell with code span containing `|` literal

### [ ] GFM §5.3 Task list items (T16)
- `- [ ] foo` / `- [x] foo` / `- [X] foo`.
- **Edge cases**:
  - must be first non-whitespace inside the list item
  - `[X]` and `[x]` both checked
  - `[]` (no inner space) → not a task
  - nested list with task on inner item
- **Tests**:
  - `- [ ] x` unchecked
  - `- [x] x` checked
  - `- [X] x` checked uppercase
  - `- [] x` → not a task (parses as link bracket)
  - `1. [ ] x` ordered task

### [ ] GFM bare URL autolink (T12)
- `https://x.y`, `www.x.y`, `mailto:` patterns inline.
- **Edge cases**:
  - trailing punctuation `https://x.y/foo.` — period excluded
  - parens `https://x.y/(a)` — balanced parens included
  - inside angle brackets falls to §6.5 path
  - inside code span — NOT autolinked (lexer state-aware?)
- **Tests**:
  - `https://example.com` mid-paragraph
  - `www.example.com` (auto-prefixed http)
  - trailing `.`, `,`, `:`, `;`, `!`, `?` stripped
  - `(see https://x.y)` — paren not consumed

### [ ] GFM Footnotes (T18)
- `[^id]` ref + `[^id]: body` def.
- **Edge cases**:
  - id is alphanumeric + `-`/`_`
  - body can span multiple indented lines
  - duplicate ids — first def wins
- **Tests**:
  - `[^1]` ref
  - `[^foo]: body` def
  - multi-line body
  - ref before def vs after def

### [x] GFM disallowed raw HTML
- Subset of tags (`<title>`, `<textarea>`, `<style>`, …) replaced with
  literal text. Transform-side, not lexer.

---

## MDX extensions

### [x] MDX ESM imports / exports
- `import …` / `export …` lines at column 0.
- **Edge cases**:
  - multi-line imports with `{ a, b, c }` on next line
  - `export default function …`
  - `export const x = { … }` with object literal
- **Tests**:
  - single-line import
  - multi-line destructured import
  - `export default async function`
  - `import` mid-paragraph → text (column guard)
  - identifier `important` → text (full-keyword match)

### [x] MDX JSX flow elements
- `<Component>…</Component>`, `<Component />`, lowercase `<div>`.
- **Edge cases**:
  - dotted name `<Foo.Bar />`
  - mixed case sub-namespacing
  - JSX inside MDX paragraph vs at column 0 (block vs inline)
  - whitespace between `<` and tag name
- **Tests**:
  - all open / close / self-closing forms
  - dotted names
  - whitespace-tolerant
  - JSX inside list item
  - JSX inline within paragraph

### [ ] MDX JSX fragments (T19)
- `<>…</>`.
- **Tests**:
  - empty `<></>`
  - `<>text</>`
  - nested fragments

### [x] MDX JSX attributes
- `name`, `name="v"`, `name='v'`, `name={expr}`.
- **Edge cases**:
  - `name` only → boolean true
  - expression value with object literal `{{ a: 1 }}`
  - expression spanning newlines
  - quoted value with embedded JSX `<` (literal in string, OK)
  - escaped quote `"a \"b\" c"`
- **Tests**:
  - all four forms
  - boolean attr
  - object-literal expression value
  - escaped quotes
  - multi-line expression

### [ ] MDX JSX attribute spread (T20)
- `<Foo {...props} />`.
- **Tests**:
  - `<Foo {...rest} />`
  - mixed `<Foo a={1} {...rest} b="x" />`

### [x] MDX comment `{/* … */}`
- **Edge cases**:
  - multi-line comment
  - comment with `*/` inside string-like content (falls back to first
    `*/` — fine, JS rules)
- **Tests**:
  - single-line
  - multi-line
  - empty `{/**/}`

### [x] MDX top-level expression `{ … }`
- **Edge cases**:
  - object literal `{{ a: 1 }}`
  - template literal `{`hello`}` with backticks
  - comment-only `{/* … */}` routes to MarkdownComment, not Expression

### [ ] MDX frontmatter dialects (T21)
- YAML `---…---` ✓; TOML `+++…+++` ✗; JSON `{…}` ✗.
- **Tests**:
  - YAML round-trip ✓
  - TOML round-trip
  - JSON-frontmatter at file start

---

## Cross-cutting / infrastructure

### [ ] Token spans never cross EOF
- Property test: every emitted token's span end ≤ source.len().
- **Test**: fuzz harness with `assert!(token.span.end <= source.len())`.

### [ ] Token spans contiguous + non-overlapping
- Token N's end == Token N+1's start (modulo trivia).
- **Test**: golden test verifying contiguity.

### [ ] Column tracking under tabs + multibyte
- Column matches CM tab-stop after T1.
- **Test**: source with `\t` + emoji + ASCII → spans hand-verified.

### [ ] Determinism
- Same input → same token stream every run.
- **Test**: run scan_tokens twice, assert byte-equal.

### [ ] Linear time
- O(n) on input length.
- **Test**: 1MB synthetic input completes < 100ms; doubling input
  doubles time (no quadratic blowup on pathological backticks).

### [ ] Lexer emits no diagnostics
- Per current design (post-lexer-strip).
- **Test**: lex any input, assert engine.error_count() == 0.

### [ ] Empty input
- `""` produces single `Eof` token.
- **Test**: lex empty string.

### [ ] Single-char input
- Each top-level dispatch char alone produces deterministic output.
- **Test**: parametrized over every dispatch char.

### [ ] Pathological inputs (fuzz target list)
- 10MB of `` ` `` — no quadratic
- 10MB of `<` — no quadratic
- Deeply nested `{{{{…}}}}` — depth-1024 OK
- Long line (1MB no `\n`) — no stack overflow
- Mixed CRLF / LF / lone CR
- BOM at file start
- Trailing partial UTF-8 sequence

---

## Spec test suite

### [ ] CommonMark official tests
- Vendor `https://spec.commonmark.org/0.31.2/spec.json` (652 examples).
- Run lexer → parser → renderer → diff HTML.
- **Pass criterion**: ≥ 99% pass rate. Document any known-failures with
  rationale.

### [ ] GFM official tests
- Vendor GFM test fixtures.
- **Pass criterion**: 100% pass on tables / strikethrough / task lists /
  bare URLs.

### [ ] MDX test corpus
- Fixtures in `dmc-lexer/tests/fixtures/mdx/`.
- Cover every documented feature.

### [ ] Golden round-trip
- `lex(source).render() == source` for whitespace-significant cases.

---

## Performance contract

### [ ] Throughput baseline
- Lex 1MB MDX in < 25ms on reference hardware.
- Capture in `duck-benchmarks/`.

### [ ] No allocations on the hot path
- Token `raw` borrows `&'src str`, no `String` clones.
- **Test**: heap-track via `dhat` on a 1MB lex run, assert allocations
  scale linearly with token count, not bytes.

### [ ] SIMD scanners exercised
- `memchr` paths in `skip_until_byte` etc. cover the bulk of source.
- **Test**: profile shows >50% time in memchr on prose-heavy input.

---

## Done-checklist (sign-off)

When every box above is ticked:

- [ ] CommonMark spec-suite ≥ 99% pass
- [ ] GFM spec-suite 100% pass
- [ ] MDX corpus 100% pass
- [ ] Fuzz target green for 24h CPU-hours
- [ ] No `unwrap`/`panic` on any input
- [ ] Throughput within 10% of baseline
- [ ] Cross-crate parser/transform/codegen tests still green
- [ ] All TokenKind variants have at least one test fixture

Lexer = 100% spec compliant.
