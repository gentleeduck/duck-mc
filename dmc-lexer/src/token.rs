use core::fmt;

use duck_diagnostic::Span;

/// One lexed token.
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

impl<'src> fmt::Display for Token<'src> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let escaped = self.raw.replace('\n', "\\n").replace('\t', "\\t");
    write!(f, "{}({:?})", self.kind, escaped)
  }
}

/// CommonMark 4.5 + GFM tilde fence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FenceChar {
  Backtick,
  Tilde,
}

/// CommonMark 6.4 emphasis delimiter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EmphasisChar {
  Asterisk,
  Underscore,
}

/// CommonMark 5.2 ordered-list separator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum OrderedSep {
  /// `1.`
  Period,
  /// `1)`
  Paren,
}

/// CommonMark 4.3 setext heading level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SetextLevel {
  /// `===`
  H1,
  /// `---`
  H2,
}

/// JSX attribute string quote style.
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
  /// `---`
  Yaml,
  /// `+++`
  Toml,
  /// `{}`
  Json,
}

/// CommonMark 4.6 raw-HTML-block classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum HtmlBlockKind {
  /// `<script>`, `<pre>`, `<style>`, `<textarea>`. Closes on matching tag.
  Type1,
  /// `<!-- -->`. Closes on `-->`.
  Type2,
  /// `<? ?>`. Closes on `?>`.
  Type3,
  /// `<!DOCTYPE ...>`. Closes on `>`.
  Type4,
  /// `<![CDATA[ ]]>`. Closes on `]]>`.
  Type5,
  /// Block-level tag set (`<div>`, `<table>`, ...). Closes on blank line.
  Type6,
  /// Any other open/close tag at col 0 followed by blank line.
  Type7,
}

/// CommonMark 6.3 + reference-link forms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LinkRefForm {
  /// `[text](url)` inline.
  Inline,
  /// `[text][label]` full reference.
  Full,
  /// `[label][]` collapsed.
  Collapsed,
  /// `[label]` shortcut.
  Shortcut,
}

/// CommonMark 6.5 + GFM extended autolinks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AutolinkKind {
  /// `<https://x.y>` angle URL.
  AngleUrl,
  /// `<a@b.c>` angle email.
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
  /// `---`
  Default,
  /// `:---`
  Left,
  /// `---:`
  Right,
  /// `:---:`
  Center,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TokenKind {
  // ----- Trivia ---------------------------------------------------------
  /// One run of inline whitespace (` `, `\t`).
  Whitespace(u8),
  /// Single `\n` between content lines.
  SoftBreak,
  /// CM inline hard break: `  \n` or `\\\n`.
  HardBreak,
  /// Two or more consecutive `\n` -- paragraph separator.
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
  /// CM 4.2 ATX heading. Level in 1..=6.
  Heading(u8),
  /// CM 4.2 trailing decoration `# Title #`.
  HeadingTrailingHashes,
  /// CM 4.3 setext underline. Folds prior text into a heading.
  SetextUnderline(SetextLevel),
  /// CM 4.1 thematic break `---`, `***`, `___`.
  ThematicBreak,
  /// CM 5.1 single `>` marker (only at col 0 / lazy continuation).
  BlockQuoteMarker,
  /// CM 5.2 `-` / `+` / `*` bullet.
  UnorderedListMarker,
  /// CM 5.2 `1.` / `1)` enumerator.
  OrderedListMarker(OrderedSep),
  /// CM 4.4 one line of indented (>= 4-space) code.
  IndentedCodeLine,
  /// CM 4.5 fenced-code-block opener with fence char + run length.
  CodeFenceOpen(FenceChar, u8),
  CodeFenceClose(FenceChar, u8),
  CodeFenceContent,
  /// Info string captured between fence opener and `\n`.
  CodeFenceInfo,

  // ----- Inline markers -------------------------------------------------
  /// CM 6.4 emphasis run. `run` in 1..=3.
  Emphasis(EmphasisChar, u8),
  /// GFM strikethrough `~~`.
  Strikethrough,
  /// CM 6.1 inline code span. Payload = backtick run length.
  CodeInlineOpen(u8),
  CodeInlineClose(u8),
  /// CM 6.6 entity / numeric character reference `&...;`.
  EntityRef,

  // ----- Links / images / footnotes ------------------------------------
  LinkOpen,
  LinkClose,
  LinkTargetOpen,
  LinkTargetClose,
  /// Tags the link form right after `LinkClose`.
  LinkRefMarker(LinkRefForm),
  /// CM 4.7 link reference definition (col-0 single-token marker).
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
  /// Whether this kind is whitespace-like trivia (the parser typically
  /// treats them as separators rather than content).
  pub fn is_trivia(&self) -> bool {
    matches!(self, TokenKind::Whitespace(_) | TokenKind::SoftBreak | TokenKind::HardBreak | TokenKind::BlankLine)
  }
}

impl fmt::Display for TokenKind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let s = match self {
      Self::Whitespace(_) => "Whitespace",
      Self::SoftBreak => "SoftBreak",
      Self::HardBreak => "HardBreak",
      Self::BlankLine => "BlankLine",
      Self::Text => "Text",
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
      Self::CodeFenceContent => "CodeFenceContent",
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
    };
    write!(f, "{}", s)
  }
}
