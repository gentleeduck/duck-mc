//! Block-level openers: ATX/setext headings, block quotes, list markers,
//! and fenced code blocks.

use crate::{
  Lexer,
  token::{FenceChar, OrderedSep, SetextLevel, Token, TokenKind},
};

impl<'eng, 'src: 'eng> Lexer<'eng, 'src> {
  /// CM 4.2 ATX heading. The first `#` is already consumed. Emits
  /// `Heading(level)` if the run is 1-6 hashes followed by space/tab/EOL,
  /// otherwise falls back to `lex_text`.
  pub(crate) fn lex_heading(&mut self) {
    if self.start_column != 0 {
      return self.lex_text();
    }
    let mut level: u8 = 1;
    while self.peek() == Some('#') && level < 7 {
      self.advance();
      level += 1;
    }
    if level > 6 {
      return self.lex_text();
    }
    match self.peek() {
      None | Some('\n') | Some(' ') | Some('\t') => {},
      _ => return self.lex_text(),
    }
    self.emit(TokenKind::Heading(level));
  }

  /// CM 4.2 trailing `#` run on a heading line. The first `#` was already
  /// consumed. Returns `true` if a `HeadingTrailingHashes` was emitted.
  pub(crate) fn lex_heading_trailing_hashes(&mut self) -> bool {
    // Must be preceded by whitespace; otherwise it's part of a word.
    if !matches!(self.tokens.last(), Some(Token { kind: TokenKind::Whitespace(_), .. })) {
      return false;
    }
    self.skip_while_byte(b'#');

    // Whatever follows must be only spaces/tabs then \n or EOF.
    let bytes = self.source.as_bytes();
    let mut i = self.current;
    while i < bytes.len() {
      match bytes[i] {
        b' ' | b'\t' => i += 1,
        b'\n' => break,
        _ => return false,
      }
    }
    self.emit(TokenKind::HeadingTrailingHashes);
    true
  }

  /// CM 5.1 block quote marker. The `>` is already consumed. The optional
  /// single space/tab after `>` is folded into the marker. Successive
  /// `>` chars on the same line emit additional markers for nesting.
  pub(crate) fn lex_block_quote(&mut self) {
    if matches!(self.peek(), Some(' ' | '\t')) {
      self.advance();
    }
    self.emit(TokenKind::BlockQuoteMarker);

    while self.peek() == Some('>') {
      self.advance();
      if matches!(self.peek(), Some(' ' | '\t')) {
        self.advance();
      }
      self.emit(TokenKind::BlockQuoteMarker);
    }
  }

  /// CM 5.2 unordered list marker. The `-`, `+`, or `*` is already
  /// consumed. Must be followed by space/tab or EOL.
  pub(crate) fn lex_unordered_list_marker(&mut self) -> bool {
    if self.start_column != 0 {
      return false;
    }
    match self.peek() {
      Some(' ' | '\t') => {
        self.advance();
      },
      None | Some('\n') => {},
      _ => return false,
    }
    self.emit(TokenKind::UnorderedListMarker);
    true
  }

  /// CM 5.2 ordered list marker: `<digits>.` or `<digits>)` followed by
  /// space/tab/EOL. The first digit is already consumed; max 9 digits.
  pub(crate) fn try_lex_ordered_list_marker(&mut self) -> bool {
    if self.start_column != 0 {
      return false;
    }

    let bytes = self.source.as_bytes();
    let mut i = self.current;
    let mut digits: usize = 1;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
      digits += 1;
      i += 1;
      if digits > 9 {
        return false;
      }
    }

    let sep = match bytes.get(i) {
      Some(&b'.') => OrderedSep::Period,
      Some(&b')') => OrderedSep::Paren,
      _ => return false,
    };
    i += 1;

    match bytes.get(i) {
      Some(&b' ' | &b'\t') => i += 1,
      Some(&b'\n') | None => {},
      _ => return false,
    }

    self.advance_bytes(i - self.current);
    self.emit(TokenKind::OrderedListMarker(sep));
    true
  }

  /// CM 4.3 setext H1 underline (line of `=`s, optional trailing
  /// whitespace). The first `=` is already consumed. The parser folds
  /// `Text + SoftBreak + SetextUnderline` into a heading.
  pub(crate) fn try_lex_setext_underline(&mut self) -> bool {
    if self.start_column != 0 {
      return false;
    }
    let bytes = self.source.as_bytes();
    let mut i = self.current;
    while i < bytes.len() && bytes[i] == b'=' {
      i += 1;
    }
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
      i += 1;
    }
    match bytes.get(i) {
      Some(&b'\n') | None => {},
      _ => return false,
    }

    self.advance_bytes(i - self.current);
    self.emit(TokenKind::SetextUnderline(SetextLevel::H1));
    true
  }

  /// CM 4.5 fenced code block. The first fence char is already consumed.
  /// Emits opener, optional info string, content (verbatim), and closer
  /// (or none, if EOF reached before a matching close).
  pub(crate) fn try_lex_fenced_code(&mut self, fence_char: char) -> bool {
    if self.start_column != 0 {
      return false;
    }
    let fb = fence_char as u8;
    let kind = if fence_char == '`' { FenceChar::Backtick } else { FenceChar::Tilde };

    // Count opening fence run (1 already consumed).
    let bytes = self.source.as_bytes();
    let mut i = self.current;
    while i < bytes.len() && bytes[i] == fb {
      i += 1;
    }
    let open_count = i - self.start;
    if open_count < 3 {
      return false;
    }

    // Backtick fences forbid backticks in info string.
    if fence_char == '`' {
      let mut j = i;
      while j < bytes.len() && bytes[j] != b'\n' {
        if bytes[j] == b'`' {
          return false;
        }
        j += 1;
      }
    }

    // Emit opener.
    self.advance_bytes(i - self.current);
    self.emit(TokenKind::CodeFenceOpen(kind, open_count.min(255) as u8));

    // Info string (trim leading and trailing whitespace).
    let bytes = self.source.as_bytes();
    let info_start = self.current;
    let mut j = info_start;
    while j < bytes.len() && bytes[j] != b'\n' {
      j += 1;
    }
    let line_end = j;

    let mut s = info_start;
    while s < line_end && matches!(bytes[s], b' ' | b'\t') {
      s += 1;
    }
    let mut e = line_end;
    while e > s && matches!(bytes[e - 1], b' ' | b'\t') {
      e -= 1;
    }

    if s > info_start {
      self.skip_bytes(s - info_start);
    }
    if e > s {
      self.advance_bytes(e - s);
      self.emit(TokenKind::CodeFenceInfo);
    }
    if line_end > e {
      self.skip_bytes(line_end - e);
    }
    if self.peek() == Some('\n') {
      self.skip_bytes(1);
    }

    let close_count = self.consume_fenced_content(fb, open_count);
    if close_count > 0 {
      self.emit(TokenKind::CodeFenceClose(kind, close_count));
    }
    true
  }

  /// Scan content lines until a closing fence (>= `min_count` of `fb`,
  /// surrounded only by optional whitespace) or EOF. Emits one
  /// `CodeFenceContent` token; returns the closing fence's char count
  /// (0 if EOF reached without a closer).
  fn consume_fenced_content(&mut self, fb: u8, min_count: usize) -> u8 {
    let bytes = self.source.as_bytes();
    let mut i = self.current;

    loop {
      let line_start = i;
      while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
      }
      let line_end = i;

      // Up to 3 leading spaces allowed on close fence.
      let mut p = line_start;
      let mut leading = 0;
      while p < line_end && bytes[p] == b' ' && leading < 3 {
        p += 1;
        leading += 1;
      }
      let fence_run_start = p;
      while p < line_end && bytes[p] == fb {
        p += 1;
      }
      let fence_count = p - fence_run_start;

      if fence_count >= min_count {
        // Trailing must be only whitespace.
        let mut q = p;
        while q < line_end && matches!(bytes[q], b' ' | b'\t') {
          q += 1;
        }
        if q == line_end {
          // Emit content up to just before the close-line's leading newline.
          let content_end =
            if line_start > self.current && bytes[line_start - 1] == b'\n' { line_start - 1 } else { line_start };
          if content_end > self.current {
            self.advance_bytes(content_end - self.current);
            self.emit(TokenKind::CodeFenceContent);
          }
          if self.peek() == Some('\n') {
            self.skip_bytes(1);
          }
          if leading > 0 {
            self.skip_bytes(leading);
          }
          self.advance_bytes(fence_count);
          return fence_count.min(255) as u8;
        }
      }

      if i >= bytes.len() {
        // EOF, no closer.
        if i > self.current {
          self.advance_bytes(i - self.current);
          self.emit(TokenKind::CodeFenceContent);
        }
        return 0;
      }
      i += 1;
    }
  }
}
