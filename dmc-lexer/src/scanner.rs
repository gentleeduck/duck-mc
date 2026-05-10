//! Low-level scanning primitives.
//!
//! Methods here move the cursor and inspect bytes/chars but never emit
//! tokens. Higher-level lexers in `crate::lexers` build on top of these.

use crate::Lexer;

impl<'eng, 'src: 'eng> Lexer<'eng, 'src> {
  /// True once the cursor is at or past the end of the source.
  #[inline]
  pub(crate) fn is_eof(&self) -> bool {
    self.current >= self.source.len()
  }

  /// Consume one Unicode scalar at `current`. Updates `current`/`line`/
  /// `column`. Returns `'\0'` at EOF (idempotent so callers can poll).
  pub(crate) fn advance(&mut self) -> char {
    if self.is_eof() {
      return '\0';
    }
    let ch = self.source[self.current..].chars().next().unwrap();
    self.current += ch.len_utf8();
    match ch {
      '\n' => {
        self.line += 1;
        self.column = 0;
      },
      '\r' => {
        // CRLF: also consume trailing '\n'.
        if self.source.as_bytes().get(self.current) == Some(&b'\n') {
          self.current += 1;
        }
        self.line += 1;
        self.column = 0;
        return '\n';
      },
      '\t' => self.column = (self.column + 4) & !3,
      _ => self.column += 1,
    }
    ch
  }

  /// Char at the cursor without consuming it.
  #[inline]
  pub(crate) fn peek(&self) -> Option<char> {
    self.source[self.current..].chars().next()
  }

  /// Char one position ahead of the cursor without consuming.
  pub(crate) fn peek_next(&self) -> Option<char> {
    let mut iter = self.source[self.current..].chars();
    iter.next();
    iter.next()
  }

  /// Source slice for the in-progress token.
  #[inline]
  pub(crate) fn current_lexeme(&self) -> &'src str {
    self.source.get(self.start..self.current).unwrap_or("")
  }

  /// Whether the in-progress lexeme (slice from `self.start`) begins with
  /// `prefix`. Used by keyword arms (`import`, `export`, `www.`, `http`).
  #[inline]
  pub(crate) fn lexeme_starts_with(&self, prefix: &str) -> bool {
    self.source.get(self.start..).is_some_and(|s| s.starts_with(prefix))
  }

  // ---- byte-level fast scanners (memchr-backed where useful) -------------

  /// Bulk-skip `n` bytes from `current`. Updates line/column by counting
  /// newlines. The slice MUST be valid UTF-8 (true when `n` came from a
  /// memchr hit on an ASCII byte, since ASCII is a UTF-8 boundary).
  pub(crate) fn advance_bytes(&mut self, n: usize) {
    if n == 0 {
      return;
    }
    let bytes = self.source.as_bytes();
    let chunk = std::str::from_utf8(&bytes[self.current..self.current + n]).unwrap();

    let mut prev_cr = false;
    for ch in chunk.chars() {
      match ch {
        '\n' => {
          if !prev_cr {
            self.line += 1;
          }
          self.column = 0;
          prev_cr = false;
        },
        '\r' => {
          self.line += 1;
          self.column = 0;
          prev_cr = true;
        },
        '\t' => {
          self.column = (self.column + 4) & !3;
          prev_cr = false;
        },
        _ => {
          self.column += 1;
          prev_cr = false;
        },
      }
    }
    self.current += n;
  }

  /// Skip until the first `delim` byte (or EOF).
  pub(crate) fn skip_until_byte(&mut self, delim: u8) {
    let rest = &self.source.as_bytes()[self.current..];
    let n = memchr::memchr(delim, rest).unwrap_or(rest.len());
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

  /// Skip a run of ASCII bytes that satisfy `f`. Stops at any non-ASCII
  /// byte (high bit set) or first byte where `f` returns false.
  pub(crate) fn skip_while_ascii<F: Fn(u8) -> bool>(&mut self, f: F) {
    let bytes = self.source.as_bytes();
    let mut i = self.current;
    while i < bytes.len() && bytes[i] < 0x80 && f(bytes[i]) {
      i += 1;
    }
    self.advance_bytes(i - self.current);
  }

  /// Advance `n` bytes and discard them. `start` moves with `current` so
  /// the next emit doesn't include them. Used for stripping fence
  /// delimiters and trailing whitespace inside structured tokens.
  pub(crate) fn skip_bytes(&mut self, n: usize) {
    self.advance_bytes(n);
    self.start = self.current;
    self.start_line = self.line;
    self.start_column = self.column;
  }
}
