use crate::{Lexer, token::TokenKind};

impl<'engine> Lexer<'engine> {
  pub(crate) fn lex_tokens(&mut self, c: char) {
    match c {
      '\n' => self.lex_newline(),
      '\r' | '\t' | ' ' => self.lex_whitespace(),

      '(' => self.emit(TokenKind::ParenOpen),
      ')' => self.emit(TokenKind::ParenClose),
      '[' => self.lex_link(),
      '!' if self.peek() == Some('[') => self.lex_image(),
      '`' => self.lex_code(),
      '-' if self.peek() == Some(' ') => self.lex_unordered_list_item(),
      '0'..='9' if self.peek() == Some('.') => self.lex_ordered_list_item(),
      '-' if self.peek() == Some('-') && self.peek_next() == Some('-') => self.lex_frontmatter(),
      '#' => self.lex_heading(),
      '*' => self.lex_bold(),
      '_' => self.lex_italic(),
      '~' if self.peek() == Some('~') => self.lex_strike(),
      '<' if self.peek() == Some('!') => self.lex_comment(),
      '<' if self.is_angle_autolink() => self.lex_angle_autolink(),
      '<' if matches!(self.peek(), Some(c) if c.is_ascii_alphabetic() || c == '/' || c == '>') => self.lex_jsx_tag(),
      '<' => self.lex_text(),
      '>' => self.emit(TokenKind::BlockQuote),
      '=' => self.emit(TokenKind::Eq),
      'i' if self.column == 1 && self.starts_with_at_start("import") => self.lex_import(),
      'e' if self.column == 1 && self.starts_with_at_start("export") => self.lex_export(),
      '{' if self.peek() == Some('/') && self.peek_next() == Some('*') => self.lex_md_comment(),
      '{' => self.lex_expression(),
      _ => self.lex_text(),
    };
  }

  pub(crate) fn starts_with_at_start(&self, prefix: &str) -> bool {
    self.source.get(self.start..).is_some_and(|s| s.starts_with(prefix))
  }

  pub(crate) fn is_angle_autolink(&self) -> bool {
    let rest = match self.source.get(self.current..) {
      Some(s) => s,
      None => return false,
    };
    let bytes = rest.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
      if b == b'>' {
        let inner = &rest[..i];
        if inner.contains("://") && inner.len() >= 5 { return true; }
        if let Some(at) = inner.find('@') {
          let (local, domain) = inner.split_at(at);
          let domain = &domain[1..];
          if !local.is_empty() && domain.contains('.') && !domain.starts_with('.')
            && !domain.ends_with('.')
            && local.chars().all(|c| c.is_ascii_alphanumeric() || ".+-_".contains(c))
            && domain.chars().all(|c| c.is_ascii_alphanumeric() || ".-".contains(c))
          {
            return true;
          }
        }
        return false;
      }
      if matches!(b, b' ' | b'\n' | b'\t' | b'<') {
        return false;
      }
    }
    false
  }

  pub(crate) fn lex_angle_autolink(&mut self) {
    while let Some(c) = self.get_current_char() {
      if c == '>' { break; }
      self.advance();
    }
    if self.get_current_char() == Some('>') {
      self.advance();
    }
    self.emit(TokenKind::Autolink);
  }

  pub(crate) fn is_eof(&self) -> bool {
    self.current >= self.source.len()
  }

  pub(crate) fn advance(&mut self) -> char {
    if self.is_eof() {
      return '\0';
    }

    let remaining = &self.source[self.current..];
    let mut iter = remaining.char_indices();
    let (_, ch) = iter.next().unwrap();

    if let Some((next_byte_idx, _)) = iter.next() {
      self.current += next_byte_idx;
    } else {
      self.current = self.source.len();
    }

    use unicode_segmentation::UnicodeSegmentation;
    self.column += if ch.is_ascii() {
      1
    } else {
      let mut buf = [0u8; 4];
      let encoded = ch.encode_utf8(&mut buf);
      encoded.graphemes(true).count().max(1)
    };
    ch
  }

  pub(crate) fn peek(&self) -> Option<char> {
    if self.is_eof() {
      return None;
    }
    self.source[self.current..].chars().next()
  }

  pub(crate) fn peek_next(&self) -> Option<char> {
    if self.is_eof() {
      return None;
    }
    let mut iter = self.source[self.current..].chars();
    iter.next();
    iter.next()
  }

  pub(crate) fn get_current_lexeme(&self) -> &str {
    self.source.get(self.start..self.current).unwrap_or("")
  }

  pub(crate) fn match_current_char(&mut self, expected: char) -> bool {
    if let Some(c) = self.source[self.current..].chars().next()
      && c == expected
    {
      return true;
    }
    false
  }

  pub(crate) fn consume_while(&mut self, mut predicate: impl FnMut(char, Option<char>) -> bool) {
    while let Some(c) = self.peek() {
      let next = self.peek_next();
      if predicate(c, next) {
        self.advance();
      } else {
        break;
      }
    }
  }

  pub(crate) fn consume_till(&mut self, c: char) {
    while let Some(cc) = self.peek() {
      if c != cc {
        self.advance();
      } else {
        break;
      }
    }
  }

  pub(crate) fn get_current_char(&mut self) -> Option<char> {
    self.source[self.current..].chars().next()
  }

  pub(crate) fn consume_whitespaces(&mut self) {
    let mut n = 0;
    while let Some(c) = self.get_current_char() {
      if c == ' ' || c == '\t' {
        self.advance();
        n += 1;
      } else {
        break;
      }
    }
    if n > 0 {
      self.emit(TokenKind::Whitespace);
    }
  }
}
