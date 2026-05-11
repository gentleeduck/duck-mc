use crate::ast::*;
use crate::refs::{RefMap, parse_link_ref_def};
use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::{Origin, SourceMeta};
use dmc_lexer::Lexer;
use dmc_lexer::token::{Token, TokenKind};
use duck_diagnostic::{Diagnostic, DiagnosticEngine, Span};
use std::sync::Arc;

/// Dialect knobs that change parse behavior between strict CommonMark and
/// MDX. Default is MDX-friendly so capital JSX components round-trip as
/// `JsxElement` nodes; spec runners can flip `cm_strict_html_blocks` to
/// treat capital lowercase tags as CM 4.6 type-7 raw HTML.
#[derive(Debug, Clone, Copy, Default)]
pub struct ParseOptions {
  /// CM 4.6 strict raw-HTML block detection. Treats uppercase JSX
  /// (`<Warning>`) as type-7 raw HTML instead of routing through the
  /// MDX `JsxElement` path. Spec runner only.
  pub cm_strict_html_blocks: bool,
  /// GFM autolink extension. Wraps `http(s)://` and `www....` runs in
  /// `Link` nodes during inline parsing. Default off so the
  /// `BareUrlAutolink` transformer owns this for MDX consumers.
  pub gfm_autolinks: bool,
  /// Legacy GFM 0.29 emphasis rendering. Flattens redundant nested
  /// `<strong>` / `<em>` structure so the GFM spec runner can keep the
  /// older delimiter behavior without regressing CommonMark 0.31.2.
  pub legacy_gfm_emphasis: bool,
}

/// Token-stream cursor + diagnostic engine. `'tokens` ties borrowed lexemes to
/// the source; `'eng` ties the engine borrow to the caller.
pub struct Parser<'eng, 'tokens> {
  pub tokens: Vec<Token<'tokens>>,
  pub meta: Arc<SourceMeta>,
  pub pos: usize,
  pub refs: RefMap,
  pub diag_engine: &'eng mut DiagnosticEngine<Code>,
  pub options: ParseOptions,
  /// Original source string, if the caller supplied it (via
  /// `with_source`). Enables a safe, provenance-correct byte-offset
  /// reslice in `raw_source_for_token_range` instead of pointer
  /// arithmetic across token slices.
  pub source: Option<&'tokens str>,
  /// Current `[...]` link-label nesting depth. Recursive label parsing
  /// (and the unresolved-shortcut replay) is super-linear in the number
  /// of nested brackets; once this exceeds [`MAX_LINK_LABEL_DEPTH`] a
  /// `[` is treated as literal text instead of opening yet another
  /// recursive parse. No real document nests link labels that deeply
  /// (CM forbids links inside link text), so this only bounds adversarial
  /// `[[[[[...` input.
  pub link_label_depth: u16,
}

/// Maximum `[...]` link-label nesting before `[` is treated as literal.
/// Kept small because an unresolved-shortcut fallback re-parses its
/// label into the outer delimiter stack, so total work is exponential
/// in this depth on adversarial `[[[[...]]]]` input. CommonMark never
/// nests link labels more than a couple deep (links cannot contain
/// links), so 12 is far more than any real document needs.
pub(crate) const MAX_LINK_LABEL_DEPTH: u16 = 12;

impl<'eng, 'tokens> Parser<'eng, 'tokens> {
  /// Build a parser positioned at the first token.
  pub fn new(
    tokens: Vec<Token<'tokens>>,
    meta: Arc<SourceMeta>,
    diag_engine: &'eng mut DiagnosticEngine<Code>,
  ) -> Self {
    Self { tokens, meta, pos: 0, refs: RefMap::new(), diag_engine, options: ParseOptions::default(), source: None, link_label_depth: 0 }
  }

  /// Build a parser with explicit `ParseOptions`.
  pub fn new_with_options(
    tokens: Vec<Token<'tokens>>,
    meta: Arc<SourceMeta>,
    diag_engine: &'eng mut DiagnosticEngine<Code>,
    options: ParseOptions,
  ) -> Self {
    Self { tokens, meta, pos: 0, refs: RefMap::new(), diag_engine, options, source: None, link_label_depth: 0 }
  }

  /// Attach the original source string so verbatim-slice reconstruction
  /// (raw HTML blocks, malformed-link bodies) can reslice it directly
  /// instead of reconstructing a pointer range across token lexemes.
  pub fn with_source(mut self, source: &'tokens str) -> Self {
    self.source = Some(source);
    self
  }

  /// Drive the top-level loop until EOF. Force-advances on no-progress so a
  /// malformed token cannot wedge the parser.
  pub fn parse(&mut self) -> Document {
    self.collect_refs();
    let span = self.tokens.first().map(|t| t.span.clone()).unwrap_or_else(default_span);
    let mut children = Vec::new();
    while !self.is_eof() {
      let before = self.pos;
      if let Some(node) = self.parse_block() {
        children.push(node);
      }
      if self.pos == before {
        self.advance();
      }
    }
    Document { children, span }
  }

  /// First pass: harvest every `LinkRefDef` token's `[label]: url "title"`
  /// payload into `self.refs`. Cursor is left untouched; the main parse
  /// loop then resolves shortcut / full / collapsed refs against the map.
  fn collect_refs(&mut self) {
    // CM 4.7: a link reference definition cannot interrupt a paragraph.
    // Track per-line whether the current line started with a paragraph-
    // worthy inline run; the line ends at SoftBreak/HardBreak. If a
    // LinkRefDef appears on a line whose predecessor line was paragraph
    // text (no intervening blank / heading / etc.), skip the def.
    let mut in_paragraph = false;
    let mut on_heading_line = false;
    for tok in &self.tokens {
      match &tok.kind {
        TokenKind::LinkRefDef => {
          if !in_paragraph && let Some((label, url, title)) = parse_link_ref_def(tok.raw) {
            let url = crate::inline::decode_entities_in(&unescape_link_part(&url));
            let title = title.map(|t| crate::inline::decode_entities_in(&unescape_link_part(&t)));
            self.refs.insert(&label, url, title);
          }
        },
        TokenKind::BlankLine
        | TokenKind::CodeFenceOpen(_, _)
        | TokenKind::CodeFenceClose(_, _)
        | TokenKind::ThematicBreak
        | TokenKind::FrontmatterEnd(_) => {
          in_paragraph = false;
          on_heading_line = false;
        },
        TokenKind::Heading(_) => {
          // ATX heading line: content on this line is heading content,
          // not a paragraph. After the line break, in_paragraph resets.
          in_paragraph = false;
          on_heading_line = true;
        },
        TokenKind::BlockQuoteMarker => {
          in_paragraph = false;
          on_heading_line = false;
        },
        TokenKind::SoftBreak | TokenKind::HardBreak => {
          if on_heading_line {
            in_paragraph = false;
          }
          on_heading_line = false;
        },
        TokenKind::Whitespace(_) | TokenKind::Eof => {},
        _ => {
          if !on_heading_line {
            in_paragraph = true;
          }
        },
      }
    }
  }

  /// Forward a fully-built diagnostic to the engine.
  pub(crate) fn emit_diagnostic(&mut self, diagnostic: Diagnostic<Code>) {
    self.diag_engine.emit(diagnostic);
  }

  /// Build a primary-labelled diagnostic at the cursor and emit it.
  pub(crate) fn diag(&mut self, code: Code, message: impl Into<String>) {
    let (line, column) = self.tokens.get(self.pos).map(|t| (t.span.line, t.span.column)).unwrap_or((0, 0));
    let span = Span::from_zero_based(self.meta.path.clone(), line, column, 1);
    self.emit_diagnostic(duck_diagnostic::diag!(code, span, message.into()));
  }

  /// Sugar for emitting a warning-severity diagnostic.
  pub(crate) fn warn(&mut self, code: Code, message: impl Into<String>) {
    self.diag(code, message);
  }

  /// Span of an arbitrary token position, or a default EOF-adjacent span.
  pub(crate) fn span_at(&self, pos: usize) -> Span {
    self.tokens.get(pos).map(|t| t.span.clone()).unwrap_or_else(default_span)
  }

  /// Rebuild the verbatim source slice covered by `tokens[start..end)`.
  /// Returns an empty string for empty / invalid ranges.
  ///
  /// When the caller attached the original source (`with_source`), the
  /// span is recovered as a safe byte-offset reslice of that `&str` —
  /// no `unsafe`, no provenance hazard. Without it (a few sample bins
  /// and the inline-string helper), we fall back to concatenating the
  /// covered tokens' lexemes; that loses any JSX-internal whitespace
  /// the lexer normalized away, but those callers don't reconstruct
  /// raw HTML blocks where that distinction matters.
  pub(crate) fn raw_source_for_token_range(&self, start: usize, end: usize) -> String {
    if start >= end {
      return String::new();
    }
    let Some(start_tok) = self.tokens.get(start) else {
      return String::new();
    };
    let Some(end_tok) = self.tokens.get(end - 1) else {
      return String::new();
    };

    if let Some(source) = self.source {
      let base = source.as_ptr() as usize;
      let src_lo = base;
      let src_hi = base + source.len();
      let lo = start_tok.raw.as_ptr() as usize;
      let hi = end_tok.raw.as_ptr() as usize + end_tok.raw.len();
      debug_assert!(lo <= hi, "token slice start pointer exceeded end pointer");
      debug_assert!(lo >= src_lo, "token slice start pointer fell before the source buffer");
      debug_assert!(hi <= src_hi, "token slice end pointer exceeded the source buffer");
      if lo < src_lo || hi > src_hi || lo > hi {
        return String::new();
      }
      let off_lo = lo - base;
      let off_hi = hi - base;
      // `&str` indexing handles the UTF-8 boundary check; these offsets
      // came from `Token.raw` slices of `source`, so they're aligned.
      return source.get(off_lo..off_hi).map(|s| s.to_string()).unwrap_or_default();
    }

    // Fallback: concatenate the covered tokens' raw lexemes.
    let mut out = String::new();
    for tok in &self.tokens[start..end] {
      out.push_str(tok.raw);
    }
    out
  }

  /// Span of the token at the cursor, or a default span at EOF.
  pub(crate) fn current_span(&self) -> Span {
    self.tokens.get(self.pos).map(|t| t.span.clone()).unwrap_or_else(default_span)
  }

  /// Token under the cursor (no consume).
  pub(crate) fn peek(&'_ self) -> Option<&'_ Token<'_>> {
    self.tokens.get(self.pos)
  }

  /// Kind of the token under the cursor (no consume).
  pub(crate) fn peek_kind(&self) -> Option<&TokenKind> {
    self.tokens.get(self.pos).map(|t| &t.kind)
  }

  /// Raw lexeme of the upcoming token with its source-tied `'tokens` lifetime,
  /// decoupled from the `&self` borrow so callers can hold it across mutations.
  pub(crate) fn peek_raw(&self) -> Option<&'tokens str> {
    self.tokens.get(self.pos).map(|t| t.raw)
  }

  /// Consume one token and return it. No-op at EOF.
  pub(crate) fn advance(&'_ mut self) -> Option<&'_ Token<'_>> {
    let t = self.tokens.get(self.pos);
    if t.is_some() {
      self.pos += 1;
    }
    t
  }

  /// True at the `Eof` token or past the end of the stream.
  pub(crate) fn is_eof(&self) -> bool {
    matches!(self.peek_kind(), Some(TokenKind::Eof) | None)
  }
}

/// CM-escape decoder for link destinations and titles harvested from
/// `LinkRefDef` tokens. Mirrors the inline path's `unescape_markdown`.
fn unescape_link_part(s: &str) -> String {
  if !s.contains('\\') {
    return s.to_string();
  }
  let mut out = String::with_capacity(s.len());
  let bytes = s.as_bytes();
  let mut i = 0;
  while i < bytes.len() {
    if bytes[i] == b'\\' && i + 1 < bytes.len() {
      let nx = bytes[i + 1];
      if matches!(
        nx,
        b'!'
          | b'"'
          | b'#'
          | b'$'
          | b'%'
          | b'&'
          | b'\''
          | b'('
          | b')'
          | b'*'
          | b'+'
          | b','
          | b'-'
          | b'.'
          | b'/'
          | b':'
          | b';'
          | b'<'
          | b'='
          | b'>'
          | b'?'
          | b'@'
          | b'['
          | b'\\'
          | b']'
          | b'^'
          | b'_'
          | b'`'
          | b'{'
          | b'|'
          | b'}'
          | b'~'
      ) {
        out.push(nx as char);
        i += 2;
        continue;
      }
    }
    out.push(bytes[i] as char);
    i += 1;
  }
  out
}

/// Lex + parse `source` in one shot, dropping all diagnostics. Convenience for
/// tests + the `parse` bin; production callers should construct their own
/// `DiagnosticEngine`.
pub fn parse(source: &str) -> Document {
  parse_with(source, ParseOptions::default())
}

/// `parse` variant with explicit `ParseOptions`. Used by the CM spec
/// runner to opt into CM-strict HTML block detection.
pub fn parse_with(source: &str, options: ParseOptions) -> Document {
  let meta = Arc::from(SourceMeta { path: Arc::from("<inline>"), origin: Origin::Inline("<inline>") });
  let mut lex_engine = DiagnosticEngine::new();
  let mut lexer = Lexer::new(source, meta.clone(), &mut lex_engine);
  let _ = lexer.scan_tokens();
  let tokens = std::mem::take(&mut lexer.tokens);
  drop(lexer);

  let mut parse_engine = DiagnosticEngine::new();
  let mut p = Parser::new_with_options(tokens, meta, &mut parse_engine, options).with_source(source);
  p.parse()
}

/// Lex `s` and run the inline parser on it. Returns the inline `Node`
/// list (Text, InlineCode, Bold, Italic, Strikethrough, Link, ...).
/// Used by table cells, which receive raw cell strings rather than
/// pre-tokenised inline content.
pub fn parse_inline_str(s: &str) -> Vec<crate::ast::Node> {
  let meta = Arc::from(SourceMeta { path: Arc::from("<inline>"), origin: Origin::Inline("<inline>") });
  let mut lex_engine = DiagnosticEngine::new();
  let mut lexer = Lexer::new(s, meta.clone(), &mut lex_engine);
  let _ = lexer.scan_tokens();
  let tokens = std::mem::take(&mut lexer.tokens);
  drop(lexer);
  let mut parse_engine = DiagnosticEngine::new();
  let mut p = Parser::new(tokens, meta, &mut parse_engine).with_source(s);
  p.collect_inline_until_break()
}
