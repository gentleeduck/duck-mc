//! MDX/MD lexer. Produces a flat token stream; the parser handles structure.
//!
//! Run `cargo doc --open -p dmc-lexer` for inline rustdoc.

use std::sync::Arc;

use dmc_diagnostic::{Code, metadata::SourceMeta};
use duck_diagnostic::{DiagnosticEngine, Span};

use crate::token::{Token, TokenKind};

mod dispatch;
mod lexers;
mod scanner;
pub mod token;

/// Streaming lexer for MDX.
///
/// `start`/`current` are byte offsets; `start_line`/`start_column` snapshot
/// the position when the in-progress token began so `emit` can record an
/// accurate span without recomputing from scratch.
pub struct Lexer<'eng, 'src> {
  pub source: &'src str,
  pub meta: Arc<SourceMeta>,
  pub tokens: Vec<Token<'src>>,

  pub start: usize,
  pub current: usize,
  pub line: usize,
  pub column: usize,
  pub start_line: usize,
  pub start_column: usize,

  pub diag_engine: &'eng mut DiagnosticEngine<Code>,
}

impl<'eng, 'src: 'eng> Lexer<'eng, 'src> {
  /// Build a fresh lexer over `source`. Diagnostics flow into `diag_engine`.
  pub fn new(source: &'src str, meta: Arc<SourceMeta>, diag_engine: &'eng mut DiagnosticEngine<Code>) -> Self {
    Self {
      source,
      meta,
      tokens: Vec::with_capacity(source.len() / 8),
      start: 0,
      current: 0,
      line: 0,
      column: 0,
      start_line: 0,
      start_column: 0,
      diag_engine,
    }
  }

  /// Scan the entire source into `self.tokens`. Always returns `Ok`; lexing
  /// errors are reported through the diagnostic engine.
  pub fn scan_tokens(&mut self) -> Result<(), std::io::Error> {
    self.try_lex_frontmatter();

    while !self.is_eof() {
      self.start = self.current;
      self.start_line = self.line;
      self.start_column = self.column;
      let c = self.advance();
      self.lex_tokens(c);
    }

    self.emit(TokenKind::Eof);
    Ok(())
  }

  /// Emit a token spanning `[self.start, self.current)` and reset the
  /// in-progress token bookkeeping.
  pub(crate) fn emit(&mut self, kind: TokenKind) {
    let length = self.current - self.start;
    let span = Span::from_zero_based(self.meta.path.clone(), self.start_line, self.start_column, length);
    self.tokens.push(Token::new(kind, span, self.current_lexeme()));
    self.start = self.current;
    self.start_line = self.line;
    self.start_column = self.column;
  }
}
