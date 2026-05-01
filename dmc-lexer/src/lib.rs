use std::sync::Arc;

use dmc_diagnostic::{Code, metadata::SourceMeta};
use duck_diagnostic::{Diagnostic, DiagnosticEngine, Span};

use crate::token::{Token, TokenKind};

mod lexers;
pub mod token;
mod utils;

/// Streaming lexer for MDX. Holds a borrowed view of the source plus cursor
/// state (`start` = current token's begin, `current` = scanner head, `line` /
/// `column` = position bookkeeping for diagnostics). `frontmatter_reserved`
/// flips to `true` once a YAML frontmatter block has been emitted so a later
/// `---` line is unambiguously a thematic break.
pub struct Lexer<'eng, 'src> {
  pub source: &'src str,
  pub meta: Arc<SourceMeta>,
  pub tokens: Vec<Token<'src>>,
  pub start: usize,
  pub current: usize,
  pub line: usize,
  pub column: usize,
  pub engine: &'eng mut DiagnosticEngine<Code>,
  pub frontmatter_reserved: bool,
}

impl<'eng, 'src: 'eng> Lexer<'eng, 'src> {
  /// Build a fresh lexer over `source`. Diagnostics are emitted into `engine`.
  pub fn new(
    source: &'src str,
    meta: Arc<SourceMeta>,
    engine: &'eng mut DiagnosticEngine<Code>,
  ) -> Self {
    Self {
      source,
      meta,
      tokens: Vec::with_capacity(source.len() / 8),
      start: 0,
      current: 0,
      line: 0,
      column: 0,
      engine,
      frontmatter_reserved: false,
    }
  }

  /// Scan the entire source into `self.tokens`. Always succeeds; errors flow
  /// through the diagnostic engine, not the `Result`.
  pub fn scan_tokens(&mut self) -> Result<(), std::io::Error> {
    while !self.is_eof() {
      self.start = self.current;
      let c = self.advance();

      self.lex_tokens(c);
    }

    self.emit(TokenKind::Eof);
    Ok(())
  }

  /// Forward a diagnostic to the engine.
  pub(crate) fn emit_diagnostic(&mut self, diagnostic: Diagnostic<Code>) {
    self.engine.emit(diagnostic);
  }

  /// Emit a token spanning `[self.start, self.current)`. Skips trivia unless
  /// it's a line-leading run of >=4 spaces (indented code-block marker).
  fn emit(&mut self, kind: TokenKind) {
    let length = self.current - self.start;
    if kind.is_trivia() {
      let line_leading = matches!(kind, TokenKind::Whitespace)
        && self.column == length
        && length >= 4
        && self.get_current_lexeme().chars().all(|c| c == ' ');
      if !line_leading {
        self.start = self.current;
        return;
      }
    }

    let span = Span::from_zero_based(self.meta.path.clone(), self.line, self.column, length);
    self.tokens.push(Token::new(kind, span, self.get_current_lexeme()));
    self.start = self.current;
  }
}
