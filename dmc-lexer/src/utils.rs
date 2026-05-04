use crate::{Lexer, token::TokenKind};

impl<'eng, 'src: 'eng> Lexer<'eng, 'src> {
  /// Dispatch the just-consumed `c` to the matching sub-lexer.
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
      'i' if self.column == 1 && self.lexeme_starts_with("import") => self.lex_import(),
      'e' if self.column == 1 && self.lexeme_starts_with("export") => self.lex_export(),
      '{' if self.peek() == Some('/') && self.peek_next() == Some('*') => self.lex_md_comment(),
      '{' => self.lex_expression(),
      _ => self.lex_text(),
    };
  }

  /// Whether the in-progress lexeme (slice from `self.start`) begins with `prefix`.
  pub(crate) fn lexeme_starts_with(&self, prefix: &str) -> bool {
    self.source.get(self.start..).is_some_and(|s| s.starts_with(prefix))
  }

  /// Lookahead test for `<...>` autolinks. True when the upcoming `>` closes
  /// either a URL (`<https://...>`) or an email (`<a@b.c>`).
  pub(crate) fn is_angle_autolink(&self) -> bool {
    let rest = match self.source.get(self.current..) {
      Some(s) => s,
      None => return false,
    };
    let bytes = rest.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
      if b == b'>' {
        let inner = &rest[..i];
        if inner.contains("://") && inner.len() >= 5 {
          return true;
        }
        if let Some(at) = inner.find('@') {
          let (local, domain) = inner.split_at(at);
          let domain = &domain[1..];
          if !local.is_empty()
            && domain.contains('.')
            && !domain.starts_with('.')
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

  /// Consume `<...>` and emit a single `Autolink`. Caller has already advanced
  /// past the opening `<`.
  pub(crate) fn lex_angle_autolink(&mut self) {
    while let Some(c) = self.current_char() {
      if c == '>' {
        break;
      }
      self.advance();
    }
    if self.current_char() == Some('>') {
      self.advance();
    }
    self.emit(TokenKind::Autolink);
  }

  /// True once the cursor is at or past the end of the source.
  pub(crate) fn is_eof(&self) -> bool {
    self.current >= self.source.len()
  }

  /// Consume one Unicode scalar at `current`, advance `current` + `column`,
  /// return the consumed `char`. Returns `'\0'` at EOF (idempotent).
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

  /// Look at the char under the cursor without consuming it.
  pub(crate) fn peek(&self) -> Option<char> {
    if self.is_eof() {
      return None;
    }
    self.source[self.current..].chars().next()
  }

  /// Look at the second char ahead of the cursor without consuming.
  pub(crate) fn peek_next(&self) -> Option<char> {
    if self.is_eof() {
      return None;
    }
    let mut iter = self.source[self.current..].chars();
    iter.next();
    iter.next()
  }

  /// Source slice between `start` and `current` - the lexeme of the
  /// in-progress token. Borrows from the original source.
  pub(crate) fn current_lexeme(&self) -> &'src str {
    self.source.get(self.start..self.current).unwrap_or("")
  }

  /// Whether the char under the cursor equals `expected`. Does not consume.
  pub(crate) fn peek_is(&mut self, expected: char) -> bool {
    if let Some(c) = self.source[self.current..].chars().next()
      && c == expected
    {
      return true;
    }
    false
  }

  /// Advance until the cursor sits on `c` (or EOF). ASCII delimiters use
  /// `memchr` for one SIMD scan; non-ASCII falls back to a char loop.
  pub(crate) fn consume_until(&mut self, c: char) {
    if c.is_ascii() {
      let bytes = self.source.as_bytes();
      let rest = &bytes[self.current..];
      let end = memchr::memchr(c as u8, rest).unwrap_or(rest.len());
      let chunk = std::str::from_utf8(&rest[..end]).unwrap();
      // bookkeeping
      for ch in chunk.chars() {
        if ch == '\n' {
          self.line += 1;
          self.column = 0;
        } else {
          self.column += 1;
        }
      }
      self.current += end;
    } else {
      // fallback to char loop for non-ASCII delimiter
      while let Some(cc) = self.peek() {
        if c == cc {
          break;
        }
        self.advance();
      }
    }
  }

  /// Same as `peek` but takes `&mut self` for ergonomic chaining.
  pub(crate) fn current_char(&mut self) -> Option<char> {
    self.source[self.current..].chars().next()
  }

  /// Consume a run of inline whitespace (spaces + tabs) and emit one
  /// `Whitespace` token if any was consumed.
  pub(crate) fn consume_whitespaces(&mut self) {
    let mut n = 0;
    while let Some(c) = self.current_char() {
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

  // ---------- byte-level fast scanners ----------

  /// Bulk-skip `n` bytes from `current`. Updates `line` + `column` by counting
  /// newlines + chars after the last newline. The `current..current+n` slice
  /// MUST be valid UTF-8 (true when `n` came from a `memchr` hit on an ASCII
  /// delimiter, since ASCII bytes are char boundaries).
  pub(crate) fn advance_bytes(&mut self, n: usize) {
    if n == 0 {
      return;
    }
    let bytes = self.source.as_bytes();
    let chunk = &bytes[self.current..self.current + n];

    // Count newlines using memchr (SIMD).
    let newlines = memchr::memchr_iter(b'\n', chunk).count();
    if newlines > 0 {
      let last_nl = memchr::memrchr(b'\n', chunk).unwrap();
      let after_nl = &chunk[last_nl + 1..];
      let tail = std::str::from_utf8(after_nl).unwrap();
      self.line += newlines;
      self.column = tail.chars().count();
    } else {
      let s = std::str::from_utf8(chunk).unwrap();
      self.column += s.chars().count();
    }
    self.current += n;
  }

  /// Skip until the first `delim` byte (or EOF). 5-10x faster than a char loop
  /// via `memchr`.
  pub(crate) fn skip_until_byte(&mut self, delim: u8) {
    let rest = &self.source.as_bytes()[self.current..];
    let n = memchr::memchr(delim, rest).unwrap_or(rest.len());
    self.advance_bytes(n);
  }

  /// Skip until first occurrence of either `a` or `b` (or EOF).
  pub(crate) fn skip_until_any2(&mut self, a: u8, b: u8) {
    let rest = &self.source.as_bytes()[self.current..];
    let n = memchr::memchr2(a, b, rest).unwrap_or(rest.len());
    self.advance_bytes(n);
  }

  /// Skip a run of identical ASCII bytes equal to `b`.
  pub(crate) fn skip_while_byte(&mut self, b: u8) {
    let bytes = self.source.as_bytes();
    let mut i = self.current;
    while i < bytes.len() && bytes[i] == b {
      i += 1;
    }
    self.advance_bytes(i - self.current);
  }

  /// Skip a run of ASCII bytes that satisfy `f`. Stops at the first non-ASCII
  /// byte (high bit set) or first byte where `f` returns false.
  pub(crate) fn skip_while_ascii<F: Fn(u8) -> bool>(&mut self, f: F) {
    let bytes = self.source.as_bytes();
    let mut i = self.current;
    while i < bytes.len() && bytes[i] < 0x80 && f(bytes[i]) {
      i += 1;
    }
    self.advance_bytes(i - self.current);
  }
}
