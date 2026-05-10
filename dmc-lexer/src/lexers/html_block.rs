//! CommonMark 4.6 raw HTML blocks. Types 1, 6, 7 are handled by JSX in
//! MDX. This module covers the JSX-disjoint forms: types 3 (`<?...?>`),
//! 4 (`<!ABC ...>`), and 5 (`<![CDATA[...]]>`). Type 2 (`<!-- -->`) is
//! in `links.rs`.

use crate::{
  Lexer,
  token::{HtmlBlockKind, TokenKind},
};

/// CM 4.6 type-6 block tag names (see spec).
const TYPE6_TAGS: &[&str] = &[
  "address",
  "article",
  "aside",
  "base",
  "basefont",
  "blockquote",
  "body",
  "caption",
  "center",
  "col",
  "colgroup",
  "dd",
  "details",
  "dialog",
  "dir",
  "div",
  "dl",
  "dt",
  "fieldset",
  "figcaption",
  "figure",
  "footer",
  "form",
  "frame",
  "frameset",
  "h1",
  "h2",
  "h3",
  "h4",
  "h5",
  "h6",
  "head",
  "header",
  "hr",
  "html",
  "iframe",
  "legend",
  "li",
  "link",
  "main",
  "menu",
  "menuitem",
  "nav",
  "noframes",
  "ol",
  "optgroup",
  "option",
  "p",
  "param",
  "search",
  "section",
  "summary",
  "table",
  "tbody",
  "td",
  "tfoot",
  "th",
  "thead",
  "title",
  "tr",
  "track",
  "ul",
];

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

  /// CM 4.6 type 6: an open tag for one of a fixed list of block-level
  /// HTML element names at line start. Caller is past `<` and at the
  /// tag name. The block runs until the next blank line.
  ///
  /// Unlike type 1/7 we do NOT require the tag to be properly closed
  /// on the same line -- spec example 156 (`<div id="foo"\n*hi*`) ends
  /// only on a blank line, with the unterminated open tag preserved
  /// verbatim.
  pub(crate) fn try_lex_html_block_type6(&mut self) -> bool {
    let bytes = self.source.as_bytes();
    let mut name_end = self.current;
    let starts_close = bytes.get(name_end) == Some(&b'/');
    if starts_close {
      name_end += 1;
    }
    let name_start = name_end;
    while name_end < bytes.len() && bytes[name_end].is_ascii_alphanumeric() {
      name_end += 1;
    }
    if name_end == name_start {
      return false;
    }
    let name = self.source[name_start..name_end].to_ascii_lowercase();
    if !TYPE6_TAGS.contains(&name.as_str()) {
      return false;
    }
    // After the tag name we accept whitespace, `>`, `/`, end-of-line,
    // or end-of-input. Any other char (e.g. `=` outside an attr name)
    // disqualifies the type-6 form.
    let next = bytes.get(name_end).copied();
    let ok_terminator = match next {
      None => true,
      Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'>') => true,
      Some(b'/') if bytes.get(name_end + 1) == Some(&b'>') => true,
      _ => false,
    };
    if !ok_terminator {
      return false;
    }

    self.emit(TokenKind::HtmlBlockOpen(HtmlBlockKind::Type6));

    // Consume until the end of a blank line. The blank-line terminator
    // is the line consisting of only whitespace (or end-of-input).
    loop {
      match self.peek() {
        None => {
          if self.current > self.start {
            self.emit(TokenKind::Text);
          }
          return true;
        },
        Some('\n') => {
          // Lookahead: next line all whitespace?
          let after_nl = self.current + 1;
          let mut k = after_nl;
          let b = self.source.as_bytes();
          while k < b.len() && matches!(b[k], b' ' | b'\t') {
            k += 1;
          }
          if k >= b.len() || b[k] == b'\n' {
            // Next line is blank -- close the block at the newline.
            if self.current > self.start {
              self.emit(TokenKind::Text);
            }
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
