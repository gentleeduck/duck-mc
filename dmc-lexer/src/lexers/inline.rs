//! Inline tokens that don't depend on bracket/angle context: plain text,
//! inline code spans, strikethrough, entity references, and task markers.

use crate::{
  Lexer,
  token::{Token, TokenKind},
};

impl<'eng, 'src: 'eng> Lexer<'eng, 'src> {
  /// Consume a run of plain text up to the next interesting char (delimiter,
  /// fence, JSX boundary, escape). Honors backslash escapes for the standard
  /// markdown escapable set.
  pub(crate) fn lex_text(&mut self) {
    while let Some(c) = self.peek() {
      match c {
        '\\' => match self.peek_next() {
          Some(nx) if Self::is_escapable(nx) => {
            self.advance();
            self.advance();
          },
          _ => {
            self.advance();
          },
        },
        '\n' | '\r' | ' ' | '\t' | '`' | '{' | '[' | ']' | '(' | ')' | '*' | '_' | '&' | '|' | '!' => break,
        '<' => match self.peek_next() {
          Some(nx) if nx.is_ascii_alphabetic() || nx == '/' || nx == '>' => break,
          _ => {
            self.advance();
          },
        },
        '/' if self.peek_next() == Some('>') => break,
        '~' if self.peek_next() == Some('~') => break,
        _ => {
          self.advance();
        },
      }
    }
    self.emit(TokenKind::Text);
  }

  /// CM appendix escapable set. Same shape as the dispatch helper so
  /// `\X` resolves uniformly whether dispatch or lex_text observed it
  /// first.
  #[inline]
  fn is_escapable(c: char) -> bool {
    matches!(
      c,
      '!'
        | '"'
        | '#'
        | '$'
        | '%'
        | '&'
        | '\''
        | '('
        | ')'
        | '*'
        | '+'
        | ','
        | '-'
        | '.'
        | '/'
        | ':'
        | ';'
        | '<'
        | '='
        | '>'
        | '?'
        | '@'
        | '['
        | '\\'
        | ']'
        | '^'
        | '_'
        | '`'
        | '{'
        | '|'
        | '}'
        | '~'
    )
  }

  /// CM 6.1 inline code span. The first backtick is already consumed.
  /// Returns `true` if a closing run was found and the span emitted.
  pub(crate) fn try_lex_inline_code(&mut self) -> bool {
    let bytes = self.source.as_bytes();
    let mut i = self.current;
    while i < bytes.len() && bytes[i] == b'`' {
      i += 1;
    }
    let open_count = i - self.start;
    if open_count > 255 {
      return false;
    }

    // Look ahead for a matching close (a run of exactly `open_count`
    // backticks, no blank line between opener and candidate).
    let mut search = i;
    let close_start = loop {
      let Some(rel) = memchr::memchr(b'`', &bytes[search..]) else {
        return false;
      };
      let pos = search + rel;
      if bytes[search..pos].windows(2).any(|w| w == b"\n\n") {
        return false;
      }
      let mut j = pos;
      while j < bytes.len() && bytes[j] == b'`' {
        j += 1;
      }
      let run = j - pos;
      if run == open_count {
        break pos;
      }
      search = j;
    };

    self.advance_bytes(open_count - 1);
    self.emit(TokenKind::CodeInlineOpen(open_count.min(255) as u8));

    let body_len = close_start - self.current;
    if body_len > 0 {
      self.advance_bytes(body_len);
      self.emit(TokenKind::Text);
    }

    self.advance_bytes(open_count);
    self.emit(TokenKind::CodeInlineClose(open_count.min(255) as u8));
    true
  }

  /// GFM strikethrough delimiter `~~`. The first `~` is already consumed.
  /// Rejects 3+ tildes (those are tilde fence territory).
  pub(crate) fn try_lex_strikethrough(&mut self) -> bool {
    if self.peek() != Some('~') {
      return false;
    }
    self.advance();
    if self.peek() == Some('~') {
      return false;
    }
    self.emit(TokenKind::Strikethrough);
    true
  }

  /// CM 6.6 entity or numeric character reference. The `&` is already
  /// consumed. Returns `true` if a complete `&...;` was recognized.
  pub(crate) fn try_lex_entity(&mut self) -> bool {
    let bytes = self.source.as_bytes();
    let mut i = self.current;

    // Numeric form: `&#...;` or `&#x...;`
    if bytes.get(i) == Some(&b'#') {
      i += 1;
      let (digit_start, is_hex) = if matches!(bytes.get(i), Some(b'x' | b'X')) { (i + 1, true) } else { (i, false) };
      let mut j = digit_start;
      let max = if is_hex { 6 } else { 7 };
      while j < bytes.len() && j - digit_start < max {
        let ok = if is_hex { bytes[j].is_ascii_hexdigit() } else { bytes[j].is_ascii_digit() };
        if !ok {
          break;
        }
        j += 1;
      }
      if j == digit_start || bytes.get(j) != Some(&b';') {
        return false;
      }
      self.advance_bytes(j + 1 - self.current);
      self.emit(TokenKind::EntityRef);
      return true;
    }

    // Named form: `&name;` -- letters and digits, must start with letter.
    if !matches!(bytes.get(i), Some(c) if c.is_ascii_alphabetic()) {
      return false;
    }
    let name_start = i;
    while i < bytes.len() && bytes[i].is_ascii_alphanumeric() {
      i += 1;
    }
    if i - name_start > 32 || bytes.get(i) != Some(&b';') {
      return false;
    }

    self.advance_bytes(i + 1 - self.current);
    self.emit(TokenKind::EntityRef);
    true
  }

  /// GFM task list marker. The opening `[` is already consumed; must
  /// follow a list marker. Form: `[ ]` / `[x]` / `[X]` followed by
  /// space/tab.
  pub(crate) fn try_lex_task_marker(&mut self) -> bool {
    let after_list = matches!(
      self.tokens.last(),
      Some(Token { kind: TokenKind::UnorderedListMarker | TokenKind::OrderedListMarker(_), .. })
    );
    if !after_list {
      return false;
    }

    let bytes = self.source.as_bytes();
    let checked = match bytes.get(self.current) {
      Some(b' ') => false,
      Some(b'x' | b'X') => true,
      _ => return false,
    };
    if bytes.get(self.current + 1) != Some(&b']') {
      return false;
    }
    if !matches!(bytes.get(self.current + 2), Some(b' ' | b'\t')) {
      return false;
    }

    self.advance_bytes(3);
    self.emit(TokenKind::TaskMarker(checked));
    true
  }
}
