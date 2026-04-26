use std::cell::RefMut;

use duck_diagnostic::{Diagnostic, DiagnosticEngine, Span};

use crate::diagnostic::Code;
use crate::token::{Token, TokenKind};

pub mod diagnostic;
mod lexers;
pub mod token;
mod utils;

pub struct Lexer<'engine> {
  pub source: String,
  pub tokens: Vec<Token>,
  pub start: usize,
  pub current: usize,
  pub line: usize,
  pub column: usize,
  pub engine: RefMut<'engine, DiagnosticEngine<Code>>,
  pub frontmatter_reserved: bool,
}

impl<'engine> Lexer<'engine> {
  pub fn new(source: String, engine: RefMut<'engine, DiagnosticEngine<Code>>) -> Self {
    Self {
      source,
      tokens: Vec::new(),
      start: 0,
      current: 0,
      line: 0,
      column: 0,
      engine,
      frontmatter_reserved: false,
    }
  }

  pub fn scan_tokens(&mut self) -> Result<(), std::io::Error> {
    while !self.is_eof() {
      self.start = self.current;
      let c = self.advance();

      self.lex_tokens(c);
    }

    self.emit(TokenKind::Eof);
    Ok(())
  }

  pub(crate) fn emit_diagnostic(&mut self, diagnostic: Diagnostic<Code>) {
    self.engine.emit(diagnostic);
  }

  fn emit(&mut self, kind: TokenKind) {
    let length = self.current - self.start;
    if kind.is_trivia() {
      // Preserve line-leading runs of 4+ spaces — indented code block marker.
      let line_leading = matches!(kind, TokenKind::Whitespace)
        && self.column == length
        && length >= 4
        && self.get_current_lexeme().chars().all(|c| c == ' ');
      if !line_leading {
        self.start = self.current;
        return;
      }
    }

    let span = Span::from_zero_based("index.mdx", self.line, self.column, length);
    self.tokens.push(Token::new(kind, span, self.get_current_lexeme().to_string()));
    self.start = self.current;
  }
}
