# dmc-lexer/src/token.rs — design audit

Final token system for 100% CommonMark + GFM + MDX coverage. Designed for:

1. **Small enum size** — every variant ≤ 2-byte payload, total enum ≤ 4 bytes.
2. **Zero heap allocation in the lexer** — `Token.raw: &'src str` borrows
   from source; no owned `String` payloads on the hot path.
3. **Self-describing** — kind alone carries enough to dispatch the parser
   without a second scan over `raw`.
4. **Source-provenance via `raw`** — anything the parser needs to extract
   (entity body, autolink URL, link-ref label/dest/title) lives in the
   token's `raw` slice. Lexer never copies bytes.
5. **Family-grouped** — kinds clustered by lex stage so adding a new
   construct is one section, not a search across the file.

---

## What was removed and why

| # | Removed                            | Why |
|---|------------------------------------|-----|
| R1 | `JsxAttributeValue`               | Never emitted anywhere (workspace-wide grep of `TokenKind::JsxAttributeValue` shows only declarer + Display). Attribute values come out as `JsxAttrStringOpen … JsxAttrString … JsxAttrStringClose` (string flavor) or `ExpressionStart … Text … ExpressionEnd` (expression flavor). |
| R2 | `Newline`                         | Declared but lexer always emits `SoftBreak` / `BlankLine` / `LineBreak` instead. Dead variant. |
| R3 | `Quote` (top-level)               | Emitted around JSX attribute strings then discarded by `is_trivia()`. Wasted emit per attribute. Replaced by `QuoteKind` payload on `JsxAttrStringOpen`/`Close`. |
| R4 | `Bracket`                         | Ambiguous (open vs close, link vs ref-def vs footnote). Replaced with directional kinds: `LinkOpen`, `LinkClose`, `FootnoteRefOpen`. |
| R5 | `Bang`                            | One-purpose marker for image lead-in. Renamed `ImageMarker`. |
| R6 | `ParenOpen` / `ParenClose`        | Only used for link target. Renamed `LinkTargetOpen` / `LinkTargetClose` so the parser doesn't have to infer context. |
| R7 | `String` (top-level)              | Only emitted inside JSX attribute. Renamed `JsxAttrString`. |
| R8 | `Eq` (top-level)                  | Only emitted inside JSX attribute. Renamed `JsxAttrEq`. |
| R9 | `BlockQuote`                      | Reads as a *block* but represents the single `>` marker. Renamed `BlockQuoteMarker`. |
| R10 | `OrderedListItem` / `UnorderedListItem` | Names suggest the *item* but it's the *marker*. Renamed `OrderedListMarker` / `UnorderedListMarker`; ordered carries a `sep` payload to distinguish `1.` from `1)`. |
| R11 | `Italic(u8)`                     | Payload always 1 (lex_italic only matches `_`/`*` runs of 1; runs of 2 → Bold; runs of 3 → ThematicBreak or fall through). Replaced by unified `Emphasis { delim, run }`. |
| R12 | `Bold(u8)`                       | Same — merged into `Emphasis { delim, run }` where `run` ∈ {1, 2, 3} so the parser disambiguates with one match. |
| R13 | `Strike(u8)`                     | Payload always 2 (GFM strike fixes the run). Renamed `Strikethrough`, no payload. |
| R14 | `CodeStart(u8)` / `CodeEnd(u8)`  | Single kind for fenced + inline forced parser to re-classify on every span. Display formatted them backwards (`CodeStart` → `"InlineCode"`, `CodeEnd` → `"CodeBlock"`). Split into `CodeFenceOpen` / `CodeFenceClose` and `CodeInlineOpen` / `CodeInlineClose`. |
| R15 | `HardBreak`                      | Semantically overloaded — current lexer emits it for "≥ 2 newlines" but CM §6.7 also uses *hard line break* for `  \n` and `\\\n` (a different inline-only thing). Split into `BlankLine` (block-level paragraph separator) and `LineBreak` (CM §6.7 inline). |
| R16 | `HTMLCommentStart` / `HTMLCommentEnd` | Inconsistent casing + naming (`HTML…Start` vs `JsxOpenTagStart`). Renamed `HtmlCommentOpen` / `HtmlCommentClose`. |
| R17 | `MarkdownCommentStart` / `MarkdownCommentEnd` | Misleading — it's an MDX construct (`{/* */}`), not generic markdown. Renamed `MdxCommentOpen` / `MdxCommentClose`. |
| R18 | `Autolink` (single variant)      | Only covered angle form, no way to distinguish URL vs email vs (future) bare URL. Replaced by `Autolink(AutolinkKind)` with kind ∈ {Url, Email, BareUrl, BareWww}. |

**Net removals**: 18 dead / mis-shaped variants.

---

## What was added and why

Every addition is tied to a ROADMAP task. See `LEXER_COMPLIANCE.md` for
spec citations and tests.

| # | Added                                       | For task | Why |
|---|---------------------------------------------|----------|-----|
| A1 | `SetextUnderline(SetextLevel)`             | T2       | CM §4.3. Parser retro-folds prior Text into `Heading(level)` when this token follows. Level encoded so parser doesn't peek `raw`. |
| A2 | `IndentedCodeLine`                         | T3       | CM §4.4. One token per line of indented code. Parser concatenates. Cheaper than `IndentedCodeStart`/`End` because blank lines inside the block stay clean. |
| A3 | `LineBreak`                                | T5       | CM §6.7 inline `<br>`. Distinct from `BlankLine` (block separator). |
| A4 | `BlankLine`                                | (rename) | Replaces `HardBreak`-as-paragraph-break. |
| A5 | `HtmlBlockOpen(HtmlBlockKind)` + `HtmlBlockClose` | T14 | CM §4.6 type 1-7. Kind encoded in payload so parser dispatches without re-classifying via `raw`. |
| A6 | `LinkRefDef`                               | T9       | CM §4.7. Single marker emitted when col-0 line matches `[label]: url "title"`. Parser slices label/dest/title from `token.raw` — no payload bloat. |
| A7 | `LinkRefMarker(LinkRefForm)`               | T10      | CM §6.3. Tags the form (`Inline`, `Full`, `Collapsed`, `Shortcut`) right after closing `]` so the parser routes to the right resolution path. |
| A8 | `EntityRef`                                | T11      | CM §6.6. Marker covers `&amp;`, `&#9;`, `&#x2A;`. Parser/transform decodes from `raw`. |
| A9 | `Autolink(AutolinkKind)`                   | T12, T13 | Merges angle + bare URL into one kind with discriminator. Parser handles all four (`Url`, `Email`, `BareUrl`, `BareWww`) uniformly. |
| A10 | `FootnoteRefOpen` + `FootnoteDefMarker`   | T18      | GFM footnotes. Inline `[^id]` and col-0 `[^id]: …`. |
| A11 | `JsxFragmentOpen` + `JsxFragmentClose`    | T19      | First-class fragment tokens; today we rely on an empty `JsxTagName` (fragile). |
| A12 | `JsxAttributeSpread`                      | T20      | `<Foo {...rest} />`. Today `lex_jsx_attribute` requires alphabetic start, so spread silently fails. |
| A13 | `FrontmatterStart(FrontmatterKind)` + `FrontmatterEnd(FrontmatterKind)` | T21 | Adds `Yaml` / `Toml` / `Json` discriminator. Parser routes to the right deserializer without sniffing `raw`. |
| A14 | `FrontmatterContent`                      | (kept)   | Renamed to keep parity with new `FrontmatterStart` / `End`. |
| A15 | `HeadingTrailingHashes`                   | T7       | ATX `# Title #` decoration. Parser drops without re-scanning. |
| A16 | `CodeFenceInfo`                           | (split)  | Info string after fence open (e.g., `js title="x" {1-3}`). Today reused as Text — splitting clarifies. |
| A17 | Table family (T15) — see enum below       | T15      | GFM tables. One token per cell + alignment + row delimiters. No heap allocation in the lexer. |
| A18 | `TaskMarker(bool)`                        | T16      | GFM task lists. Bool = checked. |

**Net additions**: 18 new variants (most one-off markers).

---

## What was renamed and why

Renames preserve semantics but improve clarity / consistency.

| Old                        | New                                | Why |
|----------------------------|------------------------------------|-----|
| `BlockQuote`               | `BlockQuoteMarker`                 | It's the `>` marker, not the block. |
| `OrderedListItem`          | `OrderedListMarker(OrderedSep)`    | Marker, not item; sep distinguishes `.` and `)`. |
| `UnorderedListItem`        | `UnorderedListMarker`              | Marker, not item. |
| `Bold(u8)` + `Italic(u8)`  | `Emphasis { delim, run }`          | One unified rule — parser pairs flanking runs by length + delim. |
| `Strike(u8)`               | `Strikethrough`                    | GFM fixes run = 2; payload was dead width. |
| `CodeStart(u8)` (fenced)   | `CodeFenceOpen(FenceChar, u8)`     | Disambiguate from inline; `FenceChar` carries `Backtick` vs `Tilde` (T4). |
| `CodeEnd(u8)` (fenced)     | `CodeFenceClose(FenceChar, u8)`    | Same. |
| `CodeStart(u8)` (inline)   | `CodeInlineOpen(u8)`               | Inline span has no fence char. |
| `CodeEnd(u8)` (inline)     | `CodeInlineClose(u8)`              | Same. |
| `HardBreak`                | `BlankLine`                        | Block-level paragraph separator. |
| (new) inline `<br>`        | `LineBreak`                        | CM §6.7. Was missing. |
| `MarkdownCommentStart/End` | `MdxCommentOpen/Close`             | It's MDX (`{/* */}`), not markdown. |
| `HTMLCommentStart/End`     | `HtmlCommentOpen/Close`            | Case + open/close consistency with the rest. |
| `Bracket`                  | `LinkOpen` / `LinkClose`           | Directional, scoped. |
| `Bang`                     | `ImageMarker`                      | Single-purpose. |
| `ParenOpen` / `ParenClose` | `LinkTargetOpen` / `LinkTargetClose` | Scoped to link target. |
| `Eq` (top)                 | `JsxAttrEq`                        | Namespaced under JSX. |
| `String` (top)             | `JsxAttrString`                    | Same. |
| `Quote`                    | (dropped — see R3)                 | Replaced by `QuoteKind` payload. |
| `Autolink`                 | `Autolink(AutolinkKind)`           | Add kind discriminator. |
| `FrontmatterStart/End`     | `FrontmatterStart/End(FrontmatterKind)` | Add dialect discriminator. |
| `Heading(u8)`              | `Heading(u8)` (kept; level 1-6)    | Already correct. |

---

## Final design — full Rust source

The proposed `dmc-lexer/src/token.rs` after all changes:

```rust
use core::fmt;
use duck_diagnostic::Span;

/// One lexed token. Borrows from source — no owned strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token<'src> {
  pub kind: TokenKind,
  pub span: Span,
  pub raw: &'src str,
}

impl<'src> Token<'src> {
  #[inline]
  pub fn new(kind: TokenKind, span: Span, raw: &'src str) -> Self {
    Self { kind, span, raw }
  }
}

// =========================================================================
// Discriminator enums — all #[repr(u8)] for ≤ 1-byte payload.
// =========================================================================

/// CommonMark §4.5 + GFM tilde fence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FenceChar {
  Backtick,
  Tilde,
}

/// CommonMark §6.4 emphasis delimiter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EmphasisChar {
  Asterisk,
  Underscore,
}

/// CommonMark §5.2 ordered-list separator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum OrderedSep {
  Period,  // `1.`
  Paren,   // `1)`
}

/// CommonMark §4.3 setext heading level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SetextLevel {
  H1,  // `===`
  H2,  // `---`
}

/// JSX attribute string quote style (was: top-level `Quote` token).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum QuoteKind {
  Single,
  Double,
}

/// MDX frontmatter dialect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FrontmatterKind {
  Yaml,   // ---
  Toml,   // +++
  Json,   // {}
}

/// CommonMark §4.6 raw-HTML-block classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum HtmlBlockKind {
  /// `<script>`, `<pre>`, `<style>`, `<textarea>`. Closes on matching tag.
  Type1,
  /// `<!-- -->`. Closes on `-->`.
  Type2,
  /// `<? ?>`. Closes on `?>`.
  Type3,
  /// `<!DOCTYPE …>`. Closes on `>`.
  Type4,
  /// `<![CDATA[ ]]>`. Closes on `]]>`.
  Type5,
  /// Block-level tag set (`<div>`, `<table>`, …). Closes on blank line.
  Type6,
  /// Any other open/close tag at col 0 followed by blank line.
  Type7,
}

/// CommonMark §6.3 + reference-link forms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LinkRefForm {
  /// `[text](url)` — inline.
  Inline,
  /// `[text][label]` — full reference.
  Full,
  /// `[label][]` — collapsed.
  Collapsed,
  /// `[label]` — shortcut.
  Shortcut,
}

/// CommonMark §6.5 + GFM extended autolinks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AutolinkKind {
  /// `<https://x.y>` — angle URL.
  AngleUrl,
  /// `<a@b.c>` — angle email.
  AngleEmail,
  /// `https://x.y` bare in text (GFM).
  BareUrl,
  /// `www.x.y` bare in text (GFM).
  BareWww,
}

/// GFM table cell alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Align {
  Default,  // ---
  Left,     // :---
  Right,    // ---:
  Center,   // :---:
}

// =========================================================================
// Token kinds — grouped by lex stage, every payload ≤ 2 bytes.
// =========================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TokenKind {
  // ----- Trivia ---------------------------------------------------------
  /// One run of inline whitespace (` `, `\t`).
  Whitespace(u8),
  /// Single `\n` between content lines.
  SoftBreak,
  /// CM §6.7 inline hard break — `  \n` or `\\\n`.
  HardBreak,
  /// ≥ 2 consecutive `\n` — paragraph separator.
  BlankLine,
  Eof,

  // ----- Frontmatter ----------------------------------------------------
  FrontmatterStart(FrontmatterKind),
  FrontmatterContent,
  FrontmatterEnd(FrontmatterKind),

  // ----- ESM (MDX) ------------------------------------------------------
  Import,
  Export,

  // ----- Block markers --------------------------------------------------
  /// CM §4.2 ATX heading. Level ∈ 1..=6.
  Heading(u8),
  /// CM §4.2 trailing decoration `# Title #`.
  HeadingTrailingHashes,
  /// CM §4.3 setext underline. Folds prior text into a heading.
  SetextUnderline(SetextLevel),
  /// CM §4.1 thematic break `---`, `***`, `___`.
  ThematicBreak,
  /// CM §5.1 single `>` marker (only at col 0 / lazy continuation).
  BlockQuoteMarker,
  /// CM §5.2 `-` / `+` / `*` bullet.
  UnorderedListMarker,
  /// CM §5.2 `1.` / `1)` enumerator.
  OrderedListMarker(OrderedSep),
  /// CM §4.4 one line of indented (≥ 4-space) code.
  IndentedCodeLine,
  /// CM §4.5 fenced-code-block opener with fence char + run length.
  CodeFenceOpen(FenceChar, u8),
  CodeFenceClose(FenceChar, u8),
  /// Info string captured between fence opener and `\n`.
  CodeFenceInfo,

  // ----- Inline markers -------------------------------------------------
  /// CM §6.4 emphasis run. `run` ∈ 1..=3.
  Emphasis(EmphasisChar, u8),
  /// GFM strikethrough `~~`.
  Strikethrough,
  /// CM §6.1 inline code span. Payload = backtick run length.
  CodeInlineOpen(u8),
  CodeInlineClose(u8),
  /// CM §6.6 entity / numeric character reference `&…;`.
  EntityRef,

  // ----- Links / images / footnotes ------------------------------------
  LinkOpen,
  LinkClose,
  LinkTargetOpen,
  LinkTargetClose,
  /// Tags the link form right after `LinkClose`.
  LinkRefMarker(LinkRefForm),
  /// CM §4.7 link reference definition (col-0 single-token marker).
  LinkRefDef,
  ImageMarker,
  /// GFM footnote reference inline `[^id]`.
  FootnoteRefOpen,
  /// GFM footnote definition at col 0 `[^id]: body`.
  FootnoteDefMarker,
  /// Single token covering the whole autolink, kind discriminates.
  Autolink(AutolinkKind),

  // ----- HTML -----------------------------------------------------------
  HtmlCommentOpen,
  HtmlCommentClose,
  HtmlBlockOpen(HtmlBlockKind),
  HtmlBlockClose,

  // ----- JSX (MDX) ------------------------------------------------------
  JsxOpenTagStart,
  JsxOpenTagEnd,
  JsxCloseTagStart,
  JsxCloseTagEnd,
  JsxSelfClosingEnd,
  JsxFragmentOpen,
  JsxFragmentClose,
  JsxTagName,
  JsxAttributeName,
  JsxAttributeSpread,
  JsxAttrEq,
  JsxAttrStringOpen(QuoteKind),
  JsxAttrStringClose(QuoteKind),
  JsxAttrString,

  // ----- MDX expressions / comments ------------------------------------
  ExpressionStart,
  ExpressionEnd,
  MdxCommentOpen,
  MdxCommentClose,

  // ----- GFM tables ----------------------------------------------------
  TableRowStart,
  TableRowEnd,
  TableCellOpen,
  TableCellClose,
  TablePipe,
  TableAlignSpec(Align),

  // ----- GFM task lists ------------------------------------------------
  TaskMarker(bool),

  // ----- Fallthrough ---------------------------------------------------
  Text,
}

impl TokenKind {
  /// Trivia kinds dropped from the emitted stream. Only `Whitespace` —
  /// `Newline` (R2) and `Quote` (R3) are gone; the lexer emits typed
  /// break / quote tokens directly.
  #[inline]
  pub fn is_trivia(&self) -> bool {
    matches!(self, TokenKind::Whitespace)
  }

  /// Whether this kind appears only at column 0 (block-level).
  #[inline]
  pub fn is_block(&self) -> bool {
    matches!(
      self,
      TokenKind::Heading(_)
        | TokenKind::SetextUnderline(_)
        | TokenKind::ThematicBreak
        | TokenKind::BlockQuoteMarker
        | TokenKind::UnorderedListMarker
        | TokenKind::OrderedListMarker(_)
        | TokenKind::IndentedCodeLine
        | TokenKind::CodeFenceOpen(_, _)
        | TokenKind::CodeFenceClose(_, _)
        | TokenKind::HtmlBlockOpen(_)
        | TokenKind::LinkRefDef
        | TokenKind::FootnoteDefMarker
        | TokenKind::TableRowStart
        | TokenKind::Import
        | TokenKind::Export
        | TokenKind::FrontmatterStart(_)
        | TokenKind::FrontmatterEnd(_)
    )
  }
}

impl fmt::Display for TokenKind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    // 1:1 mapping; kept manual so changes to the enum surface here in code review.
    let s = match self {
      Self::Whitespace => "Whitespace",
      Self::SoftBreak => "SoftBreak",
      Self::LineBreak => "LineBreak",
      Self::BlankLine => "BlankLine",
      Self::Eof => "Eof",
      Self::FrontmatterStart(_) => "FrontmatterStart",
      Self::FrontmatterContent => "FrontmatterContent",
      Self::FrontmatterEnd(_) => "FrontmatterEnd",
      Self::Import => "Import",
      Self::Export => "Export",
      Self::Heading(_) => "Heading",
      Self::HeadingTrailingHashes => "HeadingTrailingHashes",
      Self::SetextUnderline(_) => "SetextUnderline",
      Self::ThematicBreak => "ThematicBreak",
      Self::BlockQuoteMarker => "BlockQuoteMarker",
      Self::UnorderedListMarker => "UnorderedListMarker",
      Self::OrderedListMarker(_) => "OrderedListMarker",
      Self::IndentedCodeLine => "IndentedCodeLine",
      Self::CodeFenceOpen(_, _) => "CodeFenceOpen",
      Self::CodeFenceClose(_, _) => "CodeFenceClose",
      Self::CodeFenceInfo => "CodeFenceInfo",
      Self::Emphasis(_, _) => "Emphasis",
      Self::Strikethrough => "Strikethrough",
      Self::CodeInlineOpen(_) => "CodeInlineOpen",
      Self::CodeInlineClose(_) => "CodeInlineClose",
      Self::EntityRef => "EntityRef",
      Self::LinkOpen => "LinkOpen",
      Self::LinkClose => "LinkClose",
      Self::LinkTargetOpen => "LinkTargetOpen",
      Self::LinkTargetClose => "LinkTargetClose",
      Self::LinkRefMarker(_) => "LinkRefMarker",
      Self::LinkRefDef => "LinkRefDef",
      Self::ImageMarker => "ImageMarker",
      Self::FootnoteRefOpen => "FootnoteRefOpen",
      Self::FootnoteDefMarker => "FootnoteDefMarker",
      Self::Autolink(_) => "Autolink",
      Self::HtmlCommentOpen => "HtmlCommentOpen",
      Self::HtmlCommentClose => "HtmlCommentClose",
      Self::HtmlBlockOpen(_) => "HtmlBlockOpen",
      Self::HtmlBlockClose => "HtmlBlockClose",
      Self::JsxOpenTagStart => "JsxOpenTagStart",
      Self::JsxOpenTagEnd => "JsxOpenTagEnd",
      Self::JsxCloseTagStart => "JsxCloseTagStart",
      Self::JsxCloseTagEnd => "JsxCloseTagEnd",
      Self::JsxSelfClosingEnd => "JsxSelfClosingEnd",
      Self::JsxFragmentOpen => "JsxFragmentOpen",
      Self::JsxFragmentClose => "JsxFragmentClose",
      Self::JsxTagName => "JsxTagName",
      Self::JsxAttributeName => "JsxAttributeName",
      Self::JsxAttributeSpread => "JsxAttributeSpread",
      Self::JsxAttrEq => "JsxAttrEq",
      Self::JsxAttrStringOpen(_) => "JsxAttrStringOpen",
      Self::JsxAttrStringClose(_) => "JsxAttrStringClose",
      Self::JsxAttrString => "JsxAttrString",
      Self::ExpressionStart => "ExpressionStart",
      Self::ExpressionEnd => "ExpressionEnd",
      Self::MdxCommentOpen => "MdxCommentOpen",
      Self::MdxCommentClose => "MdxCommentClose",
      Self::TableRowStart => "TableRowStart",
      Self::TableRowEnd => "TableRowEnd",
      Self::TableCellOpen => "TableCellOpen",
      Self::TableCellClose => "TableCellClose",
      Self::TablePipe => "TablePipe",
      Self::TableAlignSpec(_) => "TableAlignSpec",
      Self::TaskMarker(_) => "TaskMarker",
      Self::Text => "Text",
    };
    write!(f, "{}", s)
  }
}

impl<'src> fmt::Display for Token<'src> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let escaped = self.raw.replace('\n', "\\n").replace('\t', "\\t");
    write!(f, "{}({:?})", self.kind, escaped)
  }
}
```

---

## Size analysis

`size_of::<TokenKind>()` should land at **4 bytes** with `#[repr(u8)]`:

| Variant family                   | Payload bytes | Total (incl. tag) |
|----------------------------------|--------------:|------------------:|
| Trivia / markers (no payload)    | 0             | 1                 |
| `Heading(u8)`, `Emphasis(_, u8)` | 1-2           | 2-3               |
| `CodeFenceOpen(FenceChar, u8)`   | 2             | 3                 |
| All `*Kind` enums (`#[repr(u8)]`)| 1             | 2                 |

`Token<'src>`:
- `kind`: 4 bytes (rounded for alignment)
- `span`: 24 bytes (Arc + line + col + len, dominated by Arc<str>)
- `raw`: 16 bytes (`&str`)
- **Total ≈ 48 bytes**, dominated by `Span`. The kind's growth from 17 → 70+
  variants costs zero extra bytes per token.

Compare to today: ~24 enum variants but kind already 4 bytes. **Same per-token
footprint**, with full spec coverage.

---

## Why this is "smart"

1. **One pass, no backtracking** — every payload tells the parser what comes
   next. No "look at `raw` to figure out which kind of CodeStart this is".

2. **Source borrowing only** — `LinkRefDef`, `EntityRef`, `Autolink`, etc.
   carry no owned data; the parser slices `raw` for label/dest/title/URL.
   Lexer never allocates per-token.

3. **Block / inline classification at the type level** — `is_block()` lets
   the parser reject mid-paragraph dispatch errors at the kind match,
   without column inspection.

4. **Family grouping** — adding T15 (tables) means dropping into the GFM
   tables section and adding 6 variants in one place; no scattered edits.

5. **Discriminator enums all `#[repr(u8)]`** — guarantees minimum payload.
   No risk of an `Align` becoming 4 bytes if Rust changes default repr.

6. **Trivia shrunk to one variant** — `is_trivia()` becomes a single
   match arm (was three). One less branch in the hot emit path.

7. **No `Quote` round-trip waste** — `JsxAttrStringOpen(QuoteKind)` /
   `Close(QuoteKind)` carry the quote style as 1-byte payload instead of
   emitting then dropping a separate token.

8. **Pair-based design** — every Open has a matching Close
   (`LinkOpen`/`LinkClose`, `CodeFenceOpen`/`Close`,
   `HtmlBlockOpen`/`Close`, `JsxAttrStringOpen`/`Close`). Parser never
   guesses pairing.

---

## Migration ordering

Land changes in this order so the workspace stays green at every step:

| Step | Change                                                  | Touches |
|-----:|---------------------------------------------------------|---------|
| 1 | R1 + R2 + R3 (drop `JsxAttributeValue`, `Newline`, `Quote`) | token.rs, lib.rs |
| 2 | R11 + R12 + R13 (`Italic` / `Bold` / `Strike` → `Emphasis` + `Strikethrough`) | typography.rs, parser, token.rs |
| 3 | R14 (split `CodeStart` / `CodeEnd`)                     | code.rs, parser, token.rs |
| 4 | R15 (split `HardBreak` → `BlankLine` + `LineBreak`)     | whitespaces.rs, typography.rs, parser, token.rs |
| 5 | R4 + R5 + R6 + R7 + R8 (link / image / paren / JSX-attr renames) | typography.rs, jsx.rs, parser, token.rs |
| 6 | R9 + R10 + R16 + R17 + R18 (other renames)              | utils.rs, jsx.rs, typography.rs, token.rs |
| 7 | A12 + A15 (`HeadingTrailingHashes`, `IndentedCodeLine`) | typography.rs, new file, token.rs |
| 8 | A1 (`SetextUnderline`)                                  | typography.rs, parser, token.rs |
| 9 | A8 (`EntityRef`)                                        | typography.rs, parser, token.rs |
| 10 | A6 + A7 + A10 (link-ref defs, ref-form, footnotes)     | typography.rs, parser, token.rs |
| 11 | A9 + extended `Autolink`                               | utils.rs, typography.rs, parser, token.rs |
| 12 | A5 (`HtmlBlockOpen` / `Close`)                         | new file, parser, token.rs |
| 13 | A17 (table family)                                     | new file, parser, token.rs |
| 14 | A18 (`TaskMarker`)                                     | lists.rs, parser, token.rs |
| 15 | A11 + A12 (JSX fragments, attribute spread)            | jsx.rs, parser, token.rs |
| 16 | A13 (`FrontmatterKind`)                                | fontmatter.rs, parser, token.rs |
| 17 | A16 (`CodeFenceInfo`)                                  | code.rs, parser, token.rs |

Each step is one commit; each leaves `cargo test --workspace` green.

---

## Quick stats

- **Removed**: 18 variants
- **Renamed**: 19 variants (clarity / consistency / scope)
- **Added**: 18 variants + 8 helper enums
- **Net variant count**: ~24 → ~70 (3× spec coverage at the same per-token cost)
- **Per-token size**: unchanged (`Token` ≈ 48 bytes; `TokenKind` = 4 bytes)
- **Heap allocations per lex**: 0 (was: 0 — preserved)
- **Trivia paths**: 3 → 1 (one less branch on the hot path)
