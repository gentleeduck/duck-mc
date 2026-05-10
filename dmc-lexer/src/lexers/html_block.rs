//! CommonMark 4.6 raw HTML blocks. Types 1, 6, 7 are handled by JSX in
//! MDX. This module covers the JSX-disjoint forms: types 3 (`<?...?>`),
//! 4 (`<!ABC ...>`), and 5 (`<![CDATA[...]]>`). Type 2 (`<!-- -->`) is
//! in `links.rs`.

use crate::{
  Lexer,
  token::{HtmlBlockKind, TokenKind},
};

impl<'eng, 'src: 'eng> Lexer<'eng, 'src> {
  /// CM 4.6 type 3: `<? ... ?>`. Caller is past `<`; expects `?` next.
  pub(crate) fn try_lex_processing_instruction(&mut self) -> bool {
    if self.peek() != Some('?') {
      return false;
    }
    self.advance();
    self.emit(TokenKind::HtmlBlockOpen(HtmlBlockKind::Type3));

    loop {
      match self.peek() {
        None => {
          if self.current > self.start {
            self.emit(TokenKind::Text);
          }
          return true;
        },
        Some('?') if self.peek_next() == Some('>') => {
          if self.current > self.start {
            self.emit(TokenKind::Text);
          }
          self.advance();
          self.advance();
          self.emit(TokenKind::HtmlBlockClose);
          return true;
        },
        _ => {
          self.advance();
        },
      }
    }
  }

  /// CM 4.6 type 4: `<!NAME ...>` (DOCTYPE etc.). Caller is past `<`;
  /// expects `!` then uppercase letter.
  pub(crate) fn try_lex_declaration(&mut self) -> bool {
    let bytes = self.source.as_bytes();
    if bytes.get(self.current) != Some(&b'!') {
      return false;
    }
    if !matches!(bytes.get(self.current + 1), Some(c) if c.is_ascii_uppercase()) {
      return false;
    }
    self.advance(); // !
    self.emit(TokenKind::HtmlBlockOpen(HtmlBlockKind::Type4));

    self.skip_until_byte(b'>');
    if self.current > self.start {
      self.emit(TokenKind::Text);
    }
    if self.peek() == Some('>') {
      self.advance();
      self.emit(TokenKind::HtmlBlockClose);
    }
    true
  }

  /// CM 4.6 type 5: `<![CDATA[ ... ]]>`. Caller is past `<`; expects
  /// `![CDATA[`.
  pub(crate) fn try_lex_cdata(&mut self) -> bool {
    let bytes = self.source.as_bytes();
    let need = b"![CDATA[";
    if bytes.len() < self.current + need.len() || &bytes[self.current..self.current + need.len()] != need {
      return false;
    }
    self.advance_bytes(need.len());
    self.emit(TokenKind::HtmlBlockOpen(HtmlBlockKind::Type5));

    loop {
      match self.peek() {
        None => {
          if self.current > self.start {
            self.emit(TokenKind::Text);
          }
          return true;
        },
        Some(']') => {
          let b = self.source.as_bytes();
          if b.get(self.current + 1) == Some(&b']') && b.get(self.current + 2) == Some(&b'>') {
            if self.current > self.start {
              self.emit(TokenKind::Text);
            }
            self.advance();
            self.advance();
            self.advance();
            self.emit(TokenKind::HtmlBlockClose);
            return true;
          }
          self.advance();
        },
        _ => {
          self.advance();
        },
      }
    }
  }
}
