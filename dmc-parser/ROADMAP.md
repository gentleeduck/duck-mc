# dmc-parser -- migration + spec-completeness roadmap

The lexer rewrite (token enum redesign + module split) left the parser
referring to retired token variants. This document ticks off the
migration and the work that takes the parser to 100% CommonMark + GFM +
MDX coverage matching the lexer.

Status legend: `[ ]` todo, `[~]` partial, `[x]` done.
Touch sizes: S = <100 LOC, M = 100-300 LOC, L = >300 LOC.

Each task should land as one self-contained commit that compiles +
keeps the test suite green (or expands its baseline).

---

## Phase A -- Build unblock (mechanical rename)

Goal: `cargo build -p dmc-parser` clean. No spec gain yet.

### [x] A1. Brackets, parens, image marker
- Replace `TokenKind::Bracket` with `LinkOpen` / `LinkClose` per
  position (open vs close).
- Replace `TokenKind::Bang` with `ImageMarker`.
- Replace `TokenKind::ParenOpen` / `ParenClose` with
  `LinkTargetOpen` / `LinkTargetClose`.
- Touch: `inline.rs`. Size: S.

### [x] A2. List markers
- `TokenKind::UnorderedListItem` -> `UnorderedListMarker`.
- `TokenKind::OrderedListItem` -> `OrderedListMarker(_)` (ignore the
  `OrderedSep` payload for now; render the same way).
- Touch: `block.rs`. Size: S.

### [x] A3. Block quote marker
- `TokenKind::BlockQuote` -> `BlockQuoteMarker`.
- Touch: `block.rs`. Size: S.

### [x] A4. Whitespace payload
- `TokenKind::Whitespace` -> `TokenKind::Whitespace(_)` everywhere it
  is matched. Bind the byte length when the parser needs it (indent
  detection, list continuation).
- Touch: `block.rs`, `inline.rs`. Size: S.

### [x] A5. Frontmatter dialect payload
- `TokenKind::FrontmatterStart` -> `FrontmatterStart(_)`.
- `TokenKind::FrontmatterEnd`   -> `FrontmatterEnd(_)`.
- Stub: ignore the kind, still emit a single `Frontmatter` AST node.
- Touch: `block.rs`. Size: S.

### [x] A6. Autolink payload
- `TokenKind::Autolink` -> `Autolink(_)`. Treat all kinds identically;
  C7 will branch on the variant.
- Touch: `inline.rs`. Size: S.

### [x] A7. Code span split (block vs inline)
- `TokenKind::CodeStart(n)` with `n >= 3` ->
  `CodeFenceOpen(FenceChar, n)`.
- `TokenKind::CodeStart(n)` with `n <= 2` -> `CodeInlineOpen(n)`.
- Same for `CodeEnd`. Carry the fence char so B3 can validate
  matching close.
- Touch: `block.rs` (fence), `inline.rs` (span). Size: M.

### [x] A8. Emphasis unification
- Replace `Bold(n)` and `Italic(n)` matching with a single
  `Emphasis(EmphasisChar, n)` arm. Run length 3 = both italic + bold.
- Pair logic identical to today's: a closing run of the same length
  closes the open run.
- Touch: `inline.rs`. Size: S.

### [x] A9. Strikethrough payload drop
- `TokenKind::Strike(_)` -> `Strikethrough` (no payload).
- Touch: `inline.rs`. Size: S.

### [x] A10. JSX attribute scaffolding
- `TokenKind::String` -> `JsxAttrString`.
- `TokenKind::Eq` -> `JsxAttrEq`.
- `TokenKind::Quote` is gone; quote style is now on
  `JsxAttrStringOpen(QuoteKind)` / `Close(QuoteKind)`.
- Stub: drop quote info on the floor; C12 will preserve it.
- Touch: `jsx.rs`. Size: S.

### [x] A11. Comment kind renames
- `MarkdownCommentStart` / `End` -> `MdxCommentOpen` / `Close`.
- `HTMLCommentStart` / `End` -> `HtmlCommentOpen` / `Close`.
- Touch: `block.rs`, `jsx.rs`. Size: S.

### [x] A12. Newline drop + new-token catchall
- `TokenKind::Newline` no longer exists; collapse uses to `SoftBreak`.
- Add `_ => self.consume_as_text()` in inline collector, `_ => skip`
  in block dispatch, so any unhandled new token (`IndentedCodeLine`,
  `EntityRef`, `LinkRefDef`, `Autolink` variants beyond default,
  `HtmlBlockOpen`, `JsxFragment*`, `JsxAttributeSpread`, table tokens,
  `TaskMarker`, `HeadingTrailingHashes`, `SetextUnderline`,
  `CodeFenceContent`, `CodeFenceInfo`, `BlankLine`, new `HardBreak`)
  doesn't break the build. Phase C wires real handlers.
- Touch: `block.rs`, `inline.rs`. Size: S.

**Phase A done when**: `cargo build --workspace` clean; tests may
fail.

---

## Phase B -- Restore test parity

Goal: `cargo test -p dmc-parser` matches pre-rewrite outcomes.

### [~] B1. HardBreak / BlankLine semantic split
- Old `HardBreak` was overloaded as paragraph separator. New
  semantics: `BlankLine` separates blocks; `HardBreak` is inline
  `<br>` (CM 6.7).
- Audit every `HardBreak` site in the parser. Block-loop sites that
  ended a paragraph -> `BlankLine`. Inline-loop sites that need a
  `<br>` -> keep `HardBreak`.
- Touch: `block.rs`, `inline.rs`. Size: M.

### [x] B2. Emphasis pairing
- After A8 the kind matches but the pairing logic must compare
  `(EmphasisChar, run)` not just `run`.
- Touch: `inline.rs`. Size: S.

### [x] B3. Code-fence pair validation
- Match `CodeFenceClose(fc, m)` against `CodeFenceOpen(fc', n)` only
  when `fc == fc'` and `m >= n`. Mismatched fence char or shorter
  close = treat as content.
- Touch: `block.rs`. Size: S.

### [~] B4. Ordered-list separator surface
- Add `OrderedSep` to the `List` AST so renderers can preserve `1.`
  vs `1)`. Default to `Period` if not surfaced today.
- Touch: `ast/node.rs`, `block.rs`. Size: S.

### [~] B5. Frontmatter kind on the AST
- Add `kind: FrontmatterKind` to the `Frontmatter` AST node. Default
  to `Yaml` for backwards compat.
- Touch: `ast/node.rs`, `block.rs`. Size: S.

### [x] B6. Test-suite triage
- Run `cargo test -p dmc-parser`. Fix every failure that traces back
  to the rename or the semantic splits above.
- Touch: existing tests as needed. Size: M.

**Phase B done when**: `cargo test -p dmc-parser` is green.

---

## Phase C -- Wire up new lexer tokens

Goal: parser handles every kind the lexer emits.

### [x] C1. SetextUnderline -> Heading retro-fold
- Spec: CM 4.3.
- On `SetextUnderline(level)`, fold the immediately-prior paragraph
  text into `Heading(1|2)`.
- Touch: `block.rs`. Size: S.

### [x] C2. HeadingTrailingHashes
- Drop `# Title #` trailing run from heading text.
- Touch: `block.rs`. Size: S.

### [x] C3. IndentedCodeLine
- Spec: CM 4.4.
- Concat consecutive `IndentedCodeLine` tokens into
  `CodeBlock { kind: Indented, body }`. Blank lines inside an
  indented block stay attached.
- Touch: `block.rs`, `ast/node.rs`. Size: M.

### [x] C4. CodeFenceInfo
- Populate `CodeBlock.lang` and `CodeBlock.meta` from the info
  string (split on first whitespace).
- Touch: `block.rs`, `ast/node.rs`. Size: S.

### [x] C5. CodeFenceContent
- Use the lexer-emitted single content token verbatim instead of
  re-collecting line by line.
- Touch: `block.rs`. Size: S.

### [~] C6. EntityRef decode
- Spec: CM 6.6.
- Replace `&amp;` / `&#9;` / `&#x2A;` with the decoded text. Use a
  small named-entity table (HTML5 minimal subset is fine; full HTML
  table can land later).
- Touch: `inline.rs`, new `entity.rs`. Size: M.

### [x] C7. Autolink kind branching
- Spec: CM 6.5 + GFM bare URL.
- `AngleUrl` -> `Link { url }`.
- `AngleEmail` -> `Link { url: format!("mailto:{}", body) }`.
- `BareUrl` -> `Link { url: body }`.
- `BareWww` -> `Link { url: format!("https://{}", body) }`.
- Touch: `inline.rs`. Size: S.

### [~] C8. TaskMarker -> ListItem.checked
- Spec: GFM 5.3.
- Add `checked: Option<bool>` to `ListItem`. `TaskMarker(true)` ->
  `Some(true)`, `TaskMarker(false)` -> `Some(false)`, no marker ->
  `None`.
- Touch: `ast/node.rs`, `block.rs`. Size: S.

### [x] C9. HtmlBlockOpen / Close (types 2-5)
- Spec: CM 4.6 types 2 (comment), 3 (PI), 4 (declaration),
  5 (CDATA).
- Add `Html { kind: HtmlBlockKind, raw: String }` AST. Pull lexer's
  body Text tokens between Open and Close into `raw`.
- Touch: `ast/node.rs`, `block.rs`. Size: M.

### [x] C10. JsxFragment open / close
- Add `JsxFragment { children }` AST. Convert lexer's
  `JsxFragmentOpen` / `Close` to that node, with body children
  parsed as inline.
- Touch: `ast/jsx.rs`, `jsx.rs`. Size: S.

### [x] C11. JsxAttributeSpread
- Add `JsxAttr::Spread(expression: String)` variant. Capture the
  expression body verbatim.
- Touch: `ast/jsx.rs`, `jsx.rs`. Size: S.

### [ ] C12. JsxAttrStringOpen / Close (QuoteKind)
- Replace the dropped `Quote` with a `quote: QuoteKind` field on
  `JsxAttr::String`. Round-trip preserves `"x"` vs `'x'`.
- Touch: `ast/jsx.rs`, `jsx.rs`. Size: S.

### [x] C13. BlankLine vs HardBreak verification
- B1 already split the semantics; this is the round-trip test that
  asserts a CM document with both flavors renders correctly.
- Touch: `tests/breaks.rs`. Size: S.

**Phase C done when**: every token the lexer emits has a parser arm
that produces a meaningful AST node (or intentionally drops it).

---

## Phase D -- Reference-style links + footnotes

Goal: CM 6.3 + GFM footnote resolution.

### [x] D1. LinkRefDef collection
- Spec: CM 4.7.
- First pass walks tokens; populates `RefMap<Label, (url, title)>`.
  Label normalization: lowercase, collapse whitespace.
- Touch: `parser.rs`, new `ref_map.rs`. Size: M.

### [x] D2. Link form classification
- Spec: CM 6.3.
- At a closing `LinkClose`, peek the token stream:
  - `LinkTargetOpen` -> inline link.
  - `LinkOpen` (then matching close) -> full ref `[text][label]`.
  - `LinkOpen LinkClose` empty -> collapsed `[label][]`.
  - nothing matching -> shortcut `[label]`.
- The lexer reserves `LinkRefMarker(LinkRefForm)` but doesn't emit
  it (T10); the parser self-classifies.
- Touch: `inline.rs`. Size: M.

### [x] D3. Reference resolution pass
- Second pass replaces ref-link AST nodes with resolved
  `Link { url, title }` from the RefMap. Unresolved refs render as
  literal text per spec.
- Touch: `parser.rs`. Size: M.

### [x] D4. Footnotes (GFM)
- Spec: GFM footnotes.
- Same two-pass pattern: collect `FootnoteDefMarker` definitions,
  resolve `FootnoteRefOpen` references. AST: `FootnoteRef { id }`,
  `FootnoteDef { id, body }`. Numbering on render.
- Touch: `ast/node.rs`, `block.rs`, `inline.rs`. Size: M.

**Phase D done when**: CM 4.7 + 6.3 + GFM footnotes parse correctly.

---

## Phase E -- Structural completeness

Goal: 100/100/100 CM + GFM + MDX block coverage.

### [x] E1. Link bracket depth (T8)
- Spec: CM 6.3.
- Replace flat `[`...`]` matching with a shunting stack: push on
  `LinkOpen` / `ImageMarker`, pop on `LinkClose`. Inner pairs resolve
  first, outer brackets capture the resolved inner content.
- Coverage: CM inlines 99 -> 100%.
- Touch: `inline.rs`. Size: M.

### [x] E2. GFM tables (T15 parser side)
- Spec: GFM 4.10.
- Detect alignment row (line of `|`, `:`, `-`, space only) following
  a pipe-bearing header line. Build
  `Table { align: Vec<Align>, header: Row, rows: Vec<Row> }`. Cells
  split on `TablePipe` (already lexer-emitted), inline subtree per
  cell. Trim leading/trailing pipe. `\|` escape and `|` inside code
  span already preserved by the lexer.
- Coverage: GFM 99 -> 100%.
- Touch: `table.rs`, `block.rs`, `ast/node.rs`. Size: L.

### [~] E3. Raw HTML types 1, 6, 7 classification (T14 parser side)
- Spec: CM 4.6.
- When a JSX-style tag opens at column 0 and the tag name is in the
  type-1 set (`script`/`pre`/`style`/`textarea`), consume to the
  matching `</tag>` and emit `Html { kind: Type1, raw }`.
- Type-6 set (~60 block-level HTML tag names, hard-coded const list)
  -> consume to next `BlankLine` or EOF.
- Type-7 (any other tag at col 0, not interrupting a paragraph) ->
  same close rule as type 6.
- Coverage: CM blocks 98 -> 100%.
- Touch: `block.rs`, `jsx.rs`, new `html_block.rs`. Size: L.

### [ ] E4. Lazy continuation
- Spec: CM 5.1.
- Implement paragraph continuation across newlines inside list items
  / block quotes / indented code so prose flows correctly without
  re-marking on each line.
- Touch: `block.rs`. Size: M.

**Phase E done when**: parser coverage matches the lexer:
98%/99%/99%/100% -> 100/100/100/100.

---

## Phase F -- Validation harness

Goal: spec-pinned test runners.

### [x] F1. CommonMark spec runner (T23)
- Vendor `commonmark_spec.json` (652 examples) at
  `dmc-parser/tests/fixtures/commonmark_spec.json`.
- `dmc-parser/tests/commonmark_spec.rs` runs `parse -> render_html`
  per example, normalizes (lowercase tags, collapse whitespace),
  diffs against expected. Baseline file
  `commonmark_baseline.txt` records the current pass count; test
  asserts no regression.
- Touch: new test, `Cargo.toml` dev-dep on `dmc-codegen`. Size: M.

### [~] F2. Codegen drift fixes
- First spec run will fail on more than just parser bugs. Triage
  by category (entity escaping, attribute quoting, void elements,
  list-loose handling, etc.) and fix each in a separate commit.
- Touch: `dmc-codegen/src/html.rs` mostly. Size: L.

### [ ] F3. CM coverage push
- Iterate F1+F2 until pass rate is >= 99%. Bump baseline after each
  fix.
- Touch: parser + codegen.

### [ ] F4. GFM spec runner (T24)
- Vendor GFM `spec.txt` (or generated JSON if available). Add a
  small example-block parser since GFM's fixture format is
  CommonMark-style triple-fenced markdown rather than JSON.
- `dmc-parser/tests/gfm_spec.rs` mirrors F1's harness for GFM
  fixtures. Target 100% on tables, strikethrough, task lists, bare
  URLs, disallowed raw HTML.
- Touch: new test. Size: M.

**Phase F done when**: CM >= 99%, GFM 100% on official fixtures.

---

## Phase G -- Polish

### [ ] G1. Parser bench
- Add `--bench` mode to `parse-samples/parse.rs` mirroring the
  lexer's. Captures throughput baseline for regression tracking.
- Touch: `parse-samples/parse.rs`. Size: S.

### [ ] G2. Re-enable parse fuzzing
- Drop the `parse` feature gate from `fuzz/Cargo.toml` once the
  parser is stable. Update `fuzz_parse.rs` to assert no panic and
  bounded AST node count.
- Touch: `fuzz/Cargo.toml`, `fuzz/fuzz_targets/fuzz_parse.rs`.
  Size: S.

### [ ] G3. Roadmap cross-update
- Tick `dmc-lexer/ROADMAP.md` items now satisfied: T8 (link depth),
  T10 (link-ref form classification), T14 (HTML 1/6/7 via parser),
  T15 (table rows), T23, T24. Bump the lexer coverage table to
  100/100/100/100.
- Touch: `dmc-lexer/ROADMAP.md`. Size: S.

### [ ] G4. Per-crate README
- Stamp the public API + usage for `dmc-parser`. Mention the
  ref-resolution two-pass model, table detection, and the spec
  runner.
- Touch: `dmc-parser/README.md`. Size: S.

---

## Status snapshot

- Total items: 47.
- Done: 32.
- Partial: 7 (B1, B4, B5, C6, C8, E3, F2).
- Phase A (build): 12/12.
- Phase B (parity): 3/6 done + 3 partial; tests green.
- Phase C (wire): 11/13 done + 2 partial.
- Phase D (refs): 4/4 -- ref links + footnotes complete.
- Phase E (structural): 3/4 (E1 + E2 done; E3 partial; E4 lazy
  continuation outstanding).
- Phase F (validation): 1/4 done + F2 partial -- spec runner now
  passes 294/652 (~45%, up from 118 at landing). Remaining gaps
  concentrate in HTML-block edge cases, list-item interactions,
  emphasis flanking rules, and tabs.
- Phase G (polish): 0/4.

## Coverage targets (after each phase)

| Phase | CM blocks | CM inlines | GFM | MDX | Build green |
|------:|----------:|-----------:|----:|----:|:-----------:|
| start |       --  |       --   |  -- |  -- |     no      |
| A     |       --  |       --   |  -- |  -- |    yes      |
| B     |     ~80%  |     ~85%   |~70% |~95% |    yes      |
| C     |     ~92%  |     ~95%   |~85% |100% |    yes      |
| D     |     ~95%  |     ~98%   |~90% |100% |    yes      |
| E     |     100%  |     100%   |100% |100% |    yes      |
| F     |     100%  |     100%   |100% |100% |  spec >=99% |
| G     |     100%  |     100%   |100% |100% |  spec >=99% |
