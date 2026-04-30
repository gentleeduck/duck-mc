use crate::ast::*;
use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::{Origin, SourceMeta};
use dmc_lexer::Lexer;
use dmc_lexer::token::{Token, TokenKind};
use duck_diagnostic::{Diagnostic, DiagnosticEngine, Span};
use std::cell::{RefCell, RefMut};
use std::sync::Arc;

/// Token-stream cursor + diagnostic engine. Lifetime `'tokens` ties borrowed
/// lexemes back to the original source; `'eng` ties the diagnostic engine
/// borrow to the caller's `RefCell`.
pub struct Parser<'eng, 'tokens> {
  pub tokens: Vec<Token<'tokens>>,
  pub meta: Arc<SourceMeta>,
  pub pos: usize,
  pub engine: RefMut<'eng, DiagnosticEngine<Code>>,
}

impl<'eng, 'tokens> Parser<'eng, 'tokens> {
  /// Build a parser positioned at the first token. Diagnostics are emitted
  /// into `engine`, mirroring the lexer's pattern.
  pub fn new(
    tokens: Vec<Token<'tokens>>,
    meta: Arc<SourceMeta>,
    engine: RefMut<'eng, DiagnosticEngine<Code>>,
  ) -> Self {
    Self { tokens, meta, pos: 0, engine }
  }

  /// Drive the top-level loop until EOF, producing a `Document`. Force-advances
  /// on no-progress so a malformed token can never wedge the parser.
  pub fn parse(&mut self) -> Document {
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

  /// Forward a fully-built diagnostic to the engine. Mirrors `Lexer::emit_diagnostic`.
  pub(crate) fn emit_diagnostic(&mut self, diagnostic: Diagnostic<Code>) {
    self.engine.emit(diagnostic);
  }

  /// Build a primary-labelled diagnostic at the current cursor and emit it.
  /// Severity is carried by the `Code` and read by consumers via
  /// `DiagnosticCode::severity()`.
  pub(crate) fn diag(&mut self, code: Code, message: impl Into<String>) {
    let (line, column) =
      self.tokens.get(self.pos).map(|t| (t.span.line, t.span.column)).unwrap_or((0, 0));
    let span = Span::from_zero_based(self.meta.path.clone(), line, column, 1);
    self.emit_diagnostic(duck_diagnostic::diag!(code, span, message.into()));
  }

  /// Sugar for emitting a warning-severity diagnostic.
  pub(crate) fn warn(&mut self, code: Code, message: impl Into<String>) {
    self.diag(code, message);
  }

  /// Span of the token at the cursor. Used when constructing AST nodes so
  /// they carry real source coordinates instead of a placeholder.
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

  /// Raw lexeme of the upcoming token with its source-tied `'tokens`
  /// lifetime — decouples from the `&self` borrow when callers need to keep
  /// the slice across mutating calls.
  pub(crate) fn peek_raw_src(&self) -> Option<&'tokens str> {
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

/// Lex + parse `source` in one shot, dropping all diagnostics. Convenience
/// wrapper for tests + the `parse` bin; production callers should construct
/// their own `DiagnosticEngine` and inspect it after parsing.
pub fn parse(source: &str) -> Document {
  let meta = Arc::from(SourceMeta {
    path: Arc::from("<inline>"),
    version: 0,
    origin: Origin::Inline("<inline>"),
  });
  let lex_engine = RefCell::new(DiagnosticEngine::new());
  let mut lexer = Lexer::new(source, meta.clone(), lex_engine.borrow_mut());
  let _ = lexer.scan_tokens();
  let tokens = std::mem::take(&mut lexer.tokens);
  drop(lexer);

  let parse_engine = RefCell::new(DiagnosticEngine::new());
  let mut p = Parser::new(tokens, meta, parse_engine.borrow_mut());
  p.parse()
}
