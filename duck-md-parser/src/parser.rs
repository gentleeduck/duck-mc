use crate::ast::*;
use duck_diagnostic::DiagnosticEngine;
use duck_md_lexer::Lexer;
use duck_md_lexer::token::{Token, TokenKind};
use std::cell::RefCell;

pub struct Parser {
  pub tokens: Vec<Token>,
  pub pos: usize,
  pub diagnostics: Vec<ParseDiagnostic>,
}

impl Parser {
  pub fn new(tokens: Vec<Token>) -> Self {
    Self { tokens, pos: 0, diagnostics: Vec::new() }
  }

  pub(crate) fn warn(&mut self, message: impl Into<String>) {
    let (line, column) = self
      .tokens
      .get(self.pos)
      .map(|t| (t.span.line as u32, t.span.column as u32))
      .unwrap_or((0, 0));
    self.diagnostics.push(ParseDiagnostic {
      message: message.into(),
      line,
      column,
      severity: Severity::Warning,
    });
  }

  pub(crate) fn peek(&self) -> Option<&Token> {
    self.tokens.get(self.pos)
  }
  pub(crate) fn peek_kind(&self) -> Option<&TokenKind> {
    self.tokens.get(self.pos).map(|t| &t.kind)
  }
  pub(crate) fn advance(&mut self) -> Option<&Token> {
    let t = self.tokens.get(self.pos);
    if t.is_some() {
      self.pos += 1;
    }
    t
  }
  pub(crate) fn is_eof(&self) -> bool {
    matches!(self.peek_kind(), Some(TokenKind::Eof) | None)
  }

  pub fn parse(&mut self) -> Document {
    let mut children = Vec::new();
    while !self.is_eof() {
      let before = self.pos;
      if let Some(node) = crate::block::parse_block(self) {
        children.push(node);
      }
      // Avoid infinite loop if no progress was made.
      if self.pos == before {
        self.advance();
      }
    }
    let diagnostics = std::mem::take(&mut self.diagnostics);
    Document { children, diagnostics, span: default_span() }
  }
}

/// One-shot helper.
pub fn parse(source: &str) -> Document {
  let engine = RefCell::new(DiagnosticEngine::new());
  let mut lexer = Lexer::new(source.to_string(), engine.borrow_mut());
  let _ = lexer.scan_tokens();
  let tokens = std::mem::take(&mut lexer.tokens);
  drop(lexer);
  let mut p = Parser::new(tokens);
  p.parse()
}
