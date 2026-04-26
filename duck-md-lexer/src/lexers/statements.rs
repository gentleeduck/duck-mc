use crate::{Lexer, token::TokenKind};

impl<'engine> Lexer<'engine> {
  pub(crate) fn lex_import(&mut self) {
    self.lex_statement("import", TokenKind::Import);
  }

  pub(crate) fn lex_export(&mut self) {
    self.lex_statement("export", TokenKind::Export);
  }

  fn lex_statement(&mut self, keyword: &str, kind: TokenKind) {
    // start points at the first char of the keyword. The first char has been
    // consumed by the caller (advance()), so `current` points at the second char.
    // Continue consuming the rest of the keyword.
    let already_consumed = self.current - self.start;
    let remaining = &keyword[already_consumed..];
    for expected in remaining.chars() {
      if self.peek() != Some(expected) {
        // Not actually our keyword (e.g. "important"), bail to text.
        return self.lex_text();
      }
      self.advance();
    }

    // After the keyword, the next char must be a space, tab, or `{`.
    match self.peek() {
      Some(' ') | Some('\t') | Some('{') => {},
      _ => return self.lex_text(),
    }

    // Now consume up to the terminating newline, tracking `{` `}` depth.
    // Stop when depth == 0 and we have just consumed a `\n`.
    let mut depth: i32 = 0;
    loop {
      if self.is_eof() {
        break;
      }
      let c = match self.peek() {
        Some(c) => c,
        None => break,
      };

      if c == '\n' {
        // Consume the newline.
        self.advance();
        self.line += 1;
        self.column = 0;
        if depth == 0 {
          break;
        }
        continue;
      }

      if c == '{' {
        depth += 1;
        self.advance();
        continue;
      }

      if c == '}' {
        depth -= 1;
        if depth < 0 {
          depth = 0;
        }
        self.advance();
        continue;
      }

      self.advance();
    }

    self.emit(kind);
  }
}
