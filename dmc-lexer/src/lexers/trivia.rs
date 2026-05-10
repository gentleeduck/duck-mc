//! Whitespace, line breaks, paragraph separators, indented code blocks,
//! and emphasis-delimiter runs.

use crate::{
  Lexer,
  token::{EmphasisChar, Token, TokenKind},
};

impl<'eng, 'src: 'eng> Lexer<'eng, 'src> {
  /// Consume a run of inline whitespace and emit `Whitespace(len)`.
  ///
  /// Special case: at column 0 with >= 4 spaces, when the surrounding
  /// context allows it (after a blank line, frontmatter end, fenced code
  /// close, or another indented code line), the rest of the line is
  /// emitted as `IndentedCodeLine`.
  pub(crate) fn lex_whitespace(&mut self) {
    self.skip_while_ascii(|b| b == b' ' || b == b'\t');
    let len = (self.current - self.start).min(255) as u8;
    // CM 2.2: a tab snaps to the next 4-col stop, so 1-3 spaces + a
    // tab also reach column 4. Use the column delta rather than the
    // byte length so tab-only / mixed indents trigger indented code.
    let column_delta = self.column.saturating_sub(self.start_column);

    if self.start_column == 0
      && column_delta >= 4
      && self.is_indented_code_context()
      && !matches!(self.peek(), None | Some('\n'))
    {
      self.emit(TokenKind::Whitespace(len));
      self.skip_until_byte(b'\n');
      self.emit(TokenKind::IndentedCodeLine);
      return;
    }

    self.emit(TokenKind::Whitespace(len));
  }

  /// True if the previous emitted token allows an indented code block to
  /// start (or continue) here. Walks past whitespace and soft/hard breaks.
  fn is_indented_code_context(&self) -> bool {
    for tok in self.tokens.iter().rev() {
      match tok.kind {
        TokenKind::SoftBreak | TokenKind::HardBreak | TokenKind::Whitespace(_) => continue,
        TokenKind::BlankLine
        | TokenKind::FrontmatterEnd(_)
        | TokenKind::CodeFenceClose(_, _)
        | TokenKind::IndentedCodeLine => return true,
        _ => return false,
      }
    }
    true
  }

  /// Consume a run of `\n` and emit `BlankLine` (>=2), `HardBreak`
  /// (preceded by `  ` or trailing `\`), or `SoftBreak` (single `\n`).
  pub(crate) fn lex_newline(&mut self) {
    let nl_start = self.start;
    // Absorb additional CRLF / LF / CR runs so the count reflects
    // *line breaks*, not bytes (CRLF is one line break, not two).
    loop {
      let b = self.source.as_bytes().get(self.current).copied();
      match b {
        Some(b'\n') => {
          self.advance();
        },
        Some(b'\r') => {
          self.advance();
        },
        _ => break,
      }
    }
    let bytes = self.source.as_bytes();
    let mut count = 0usize;
    let mut i = nl_start;
    while i < self.current {
      match bytes[i] {
        b'\r' => {
          count += 1;
          if i + 1 < self.current && bytes[i + 1] == b'\n' {
            i += 2;
          } else {
            i += 1;
          }
        },
        b'\n' => {
          count += 1;
          i += 1;
        },
        _ => i += 1,
      }
    }
    if count >= 2 {
      self.emit(TokenKind::BlankLine);
      return;
    }

    let prev_byte = nl_start.checked_sub(1).and_then(|i| self.source.as_bytes().get(i)).copied();

    let kind = match self.tokens.last() {
      // Two-or-more trailing spaces before \n -> hard break. Skip if the
      // whitespace was at column 1, which is the start of an indented
      // code block / blank-ish line inside one.
      Some(Token { kind: TokenKind::Whitespace(n), span, .. }) if *n >= 2 && span.column != 1 => TokenKind::HardBreak,
      // Trailing backslash on a Text token -> hard break. The `\` stays
      // in the preceding Text token; the parser strips it when the
      // hard break has content after it, and keeps it literal when the
      // break is the last inline (CM 6.7).
      Some(Token { kind: TokenKind::Text, .. }) if prev_byte == Some(b'\\') => TokenKind::HardBreak,
      _ => TokenKind::SoftBreak,
    };
    self.emit(kind);
  }

  /// CM 4.1 thematic break: 3+ of `-`, `_`, or `*` (same char) on a line,
  /// optionally separated by spaces/tabs, with nothing else. Caller
  /// already consumed the first marker char.
  ///
  /// Returns `true` if the break was emitted, `false` otherwise.
  pub(crate) fn lex_thematic_break(&mut self, marker: char) -> bool {
    if !self.at_block_marker_position() {
      return false;
    }
    let mb = marker as u8;

    let bytes = self.source.as_bytes();
    let mut i = self.current;
    let mut count: usize = 1;
    while i < bytes.len() {
      match bytes[i] {
        b if b == mb => {
          count += 1;
          i += 1;
        },
        b' ' | b'\t' => i += 1,
        b'\n' => break,
        _ => return false,
      }
    }
    if count < 3 {
      return false;
    }

    self.advance_bytes(i - self.current);
    self.emit(TokenKind::ThematicBreak);
    true
  }

  /// Run of `*` or `_` (1-3 chars). The first char is already consumed.
  /// Always emits `Emphasis(kind, len)` capped at length 3.
  pub(crate) fn lex_emphasis(&mut self, c: char) {
    let kind = if c == '*' { EmphasisChar::Asterisk } else { EmphasisChar::Underscore };
    let mark = c as u8;
    let bytes = self.source.as_bytes();
    let mut count: usize = 1;
    while bytes.get(self.current) == Some(&mark) {
      self.advance();
      count += 1;
    }
    self.emit(TokenKind::Emphasis(kind, count.min(255) as u8));
  }
}
