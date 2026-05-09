//! User-facing walkthrough: ../../dmc-docs/dmc-lexer/
//! Run `cargo doc --open -p dmc-lexer` for the inline rustdoc.

use std::sync::Arc;

use dmc_diagnostic::{Code, metadata::SourceMeta};
use duck_diagnostic::{DiagnosticEngine, Span};

use crate::token::{Token, TokenKind};

mod lexers;
pub mod token;
mod utils;

/// Streaming lexer for MDX. `start` = current token's begin, `current` =
/// scanner head, `line`/`column` track position for diagnostics.
/// `frontmatter_reserved` flips to `true` once a YAML frontmatter block has
/// been emitted so a later `---` line is unambiguously a thematic break.
pub struct Lexer<'eng, 'src> {
  pub source: &'src str,
  pub meta: Arc<SourceMeta>,
  pub tokens: Vec<Token<'src>>,
  pub start: usize,
  pub current: usize,
  pub line: usize,
  pub column: usize,
  pub diag_engine: &'eng mut DiagnosticEngine<Code>,
  pub frontmatter_reserved: bool,
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
      diag_engine,
      frontmatter_reserved: false,
    }
  }

  /// Scan the entire source into `self.tokens`. Always succeeds; errors are
  /// reported through the diagnostic engine, not the `Result`.
  pub fn scan_tokens(&mut self) -> Result<(), std::io::Error> {
    while !self.is_eof() {
      self.start = self.current;
      let c = self.advance();

      self.lex_tokens(c);
    }

    self.emit(TokenKind::Eof);
    Ok(())
  }

  /// Emit a token spanning `[self.start, self.current)`. Inline whitespace
  /// is kept as `Whitespace` tokens so the inline parser can preserve
  /// spacing around block tokens (e.g. between `]( )` and the next text).
  /// `Newline` and `Quote` trivia are still dropped.
  fn emit(&mut self, kind: TokenKind) {
    let length = self.current - self.start;
    if kind.is_trivia() && !matches!(kind, TokenKind::Whitespace) {
      self.start = self.current;
      return;
    }

    let span = Span::from_zero_based(self.meta.path.clone(), self.line, self.column, length);
    self.tokens.push(Token::new(kind, span, self.current_lexeme()));
    self.start = self.current;
  }
}
