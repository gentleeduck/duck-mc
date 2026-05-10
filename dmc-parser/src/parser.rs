use crate::ast::*;
use crate::refs::{RefMap, parse_link_ref_def};
use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::{Origin, SourceMeta};
use dmc_lexer::Lexer;
use dmc_lexer::token::{Token, TokenKind};
use duck_diagnostic::{Diagnostic, DiagnosticEngine, Span};
use std::sync::Arc;

/// Token-stream cursor + diagnostic engine. `'tokens` ties borrowed lexemes to
/// the source; `'eng` ties the engine borrow to the caller.
pub struct Parser<'eng, 'tokens> {
  pub tokens: Vec<Token<'tokens>>,
  pub meta: Arc<SourceMeta>,
  pub pos: usize,
  pub refs: RefMap,
  pub diag_engine: &'eng mut DiagnosticEngine<Code>,
}

impl<'eng, 'tokens> Parser<'eng, 'tokens> {
  /// Build a parser positioned at the first token.
  pub fn new(
    tokens: Vec<Token<'tokens>>,
    meta: Arc<SourceMeta>,
    diag_engine: &'eng mut DiagnosticEngine<Code>,
  ) -> Self {
    Self { tokens, meta, pos: 0, refs: RefMap::new(), diag_engine }
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
    for tok in &self.tokens {
      if matches!(tok.kind, TokenKind::LinkRefDef)
        && let Some((label, url, title)) = parse_link_ref_def(tok.raw)
      {
        // Unescape `\X` then decode `&...;` entity references in url +
        // title per CM 4.7 + 6.6 so the rendered link uses the
        // canonical destination text.
        let url = crate::inline::decode_entities_in(&unescape_link_part(&url));
        let title = title.map(|t| crate::inline::decode_entities_in(&unescape_link_part(&t)));
        self.refs.insert(&label, url, title);
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
  let meta = Arc::from(SourceMeta { path: Arc::from("<inline>"), origin: Origin::Inline("<inline>") });
  let mut lex_engine = DiagnosticEngine::new();
  let mut lexer = Lexer::new(source, meta.clone(), &mut lex_engine);
  let _ = lexer.scan_tokens();
  let tokens = std::mem::take(&mut lexer.tokens);
  drop(lexer);

  let mut parse_engine = DiagnosticEngine::new();
  let mut p = Parser::new(tokens, meta, &mut parse_engine);
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
  let mut p = Parser::new(tokens, meta, &mut parse_engine);
  p.collect_inline_until_break()
}
