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

    if self.start_column == 0
      && len >= 4
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
    self.skip_while_byte(b'\n');
    let count = self.current - self.start;
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
      // Trailing backslash on a Text token -> hard break, trim the `\`
      // off both the span and the borrowed lexeme so downstream emit
      // doesn't render the literal `\`.
      Some(Token { kind: TokenKind::Text, .. }) if prev_byte == Some(b'\\') => {
        let prev = self.tokens.last_mut().unwrap();
        prev.span.length -= 1;
        prev.raw = &prev.raw[..prev.raw.len() - 1];
        TokenKind::HardBreak
      },
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
    if self.start_column != 0 {
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
    while count < 3 && bytes.get(self.current) == Some(&mark) {
      self.advance();
      count += 1;
    }
    self.emit(TokenKind::Emphasis(kind, count as u8));
  }
}
