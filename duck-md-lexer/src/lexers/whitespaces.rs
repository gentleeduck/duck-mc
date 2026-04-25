use crate::{Lexer, token::TokenKind};

impl<'engine> Lexer<'engine> {
  pub(crate) fn lex_whitespace(&mut self) {
    while let Some(c) = self.peek() {
      if c == ' ' || c == '\t' || c == '\r' {
        self.advance();
      } else {
        break;
      }
    }
    self.emit(TokenKind::Whitespace)
  }

  pub(crate) fn lex_newline(&mut self) {
    // The original `\n` that triggered this call is already consumed by the caller.
    self.line += 1;
    self.column = 0;

    let mut additional: usize = 0;
    self.consume_while(|c, _| {
      if c == '\n' {
        additional += 1;
        true
      } else {
        false
      }
    });
    for _ in 0..additional {
      self.line += 1;
      self.column = 0;
    }

    let total = additional + 1;
    if total >= 2 {
      self.emit(TokenKind::HardBreak)
    } else {
      self.emit(TokenKind::SoftBreak)
    }
  }
}
