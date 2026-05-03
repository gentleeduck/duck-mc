use crate::{Lexer, token::TokenKind};

impl<'eng, 'src: 'eng> Lexer<'eng, 'src> {
  /// Consume a run of inline whitespace (space, tab, CR) and emit `Whitespace`.
  pub(crate) fn lex_whitespace(&mut self) {
    self.skip_while_ascii(|b| b == b' ' || b == b'\t' || b == b'\r');
    self.emit(TokenKind::Whitespace)
  }

  /// Account for the just-consumed `\n` plus any consecutive newlines. Emits
  /// `HardBreak` for blank-line separators (>=2 newlines), else `SoftBreak`.
  pub(crate) fn lex_newline(&mut self) {
    // The original `\n` that triggered this call is already consumed by the caller.
    self.line += 1;
    self.column = 0;

    // Count + skip additional consecutive `\n` bytes via byte loop.
    let bytes = self.source.as_bytes();
    let mut additional: usize = 0;
    let mut i = self.current;
    while i < bytes.len() && bytes[i] == b'\n' {
      additional += 1;
      i += 1;
    }
    if additional > 0 {
      self.current = i;
      self.line += additional;
      self.column = 0;
    }

    let total = additional + 1;
    if total >= 2 { self.emit(TokenKind::HardBreak) } else { self.emit(TokenKind::SoftBreak) }
  }
}
