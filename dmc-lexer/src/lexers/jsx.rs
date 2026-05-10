//! JSX tag lexing: open/close tags, fragments, attributes (boolean,
//! string-valued, expression-valued, spread).

use crate::{
  Lexer,
  token::{QuoteKind, TokenKind},
};

impl<'eng, 'src: 'eng> Lexer<'eng, 'src> {
  /// Try lexing a JSX tag. The opening `<` is already consumed by the
  /// dispatcher. Returns `true` on success.
  pub(crate) fn try_lex_jsx_tag(&mut self) -> bool {
    let saved_current = self.current;
    let saved_start = self.start;
    let saved_line = self.line;
    let saved_column = self.column;
    let saved_start_line = self.start_line;
    let saved_start_column = self.start_column;
    let saved_token_count = self.tokens.len();
    let result = self.try_lex_jsx_tag_inner();
    if !result {
      self.tokens.truncate(saved_token_count);
      self.current = saved_current;
      self.start = saved_start;
      self.line = saved_line;
      self.column = saved_column;
      self.start_line = saved_start_line;
      self.start_column = saved_start_column;
    }
    result
  }

  fn try_lex_jsx_tag_inner(&mut self) -> bool {
    let is_close = self.peek() == Some('/');
    if is_close {
      self.advance();
    }

    // Fragment open `<>` or close `</>`. For an opener, require a
    // matching `</>` later in the source -- a stray `<>` without any
    // close is not a fragment, it is two literal angle brackets and
    // should round-trip as `&lt;&gt;` per CM 6.6 (raw HTML element
    // names must start with a letter; `<>` is neither autolink nor
    // valid HTML).
    if self.peek() == Some('>') {
      if !is_close {
        let bytes = self.source.as_bytes();
        let after_open = self.current + 1;
        let mut i = after_open;
        let mut found_close = false;
        while i + 2 < bytes.len() {
          if bytes[i] == b'<' && bytes[i + 1] == b'/' && bytes[i + 2] == b'>' {
            found_close = true;
            break;
          }
          i += 1;
        }
        if !found_close {
          return false;
        }
      }
      self.advance();
      self.emit(if is_close { TokenKind::JsxFragmentClose } else { TokenKind::JsxFragmentOpen });
      return true;
    }

    if !matches!(self.peek(), Some(c) if c.is_ascii_alphabetic()) {
      return false;
    }

    // Reject URL-scheme-like prefixes (`https:`, `http:`, etc.). When
    // the would-be tag name ends in `:` and is followed by `/` the
    // construct is an attempted autolink that lex_angle_autolink
    // already rejected (e.g., contains a space) so we must keep it as
    // literal text rather than swallow it as a JSX tag.
    {
      let bytes = self.source.as_bytes();
      let mut i = self.current;
      while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'_' | b'-' | b'.')) {
        i += 1;
      }
      if i < bytes.len() && bytes[i] == b':' && bytes.get(i + 1) == Some(&b'/') {
        return false;
      }
    }

    // CM 6.6 raw HTML tagname: `[a-zA-Z][a-zA-Z0-9-]*`. JSX additionally
    // permits `.` (member access) and `:` (namespace), but those forms
    // are uppercase JSX components -- a lowercase first char combined
    // with `.` or `:` (`<m:abc>`, `<foo.bar.baz>`) is neither a valid
    // HTML element nor a valid React component, so reject them and let
    // the dispatcher fall back to literal text.
    {
      let bytes = self.source.as_bytes();
      let first = bytes[self.current];
      if first.is_ascii_lowercase() {
        let mut i = self.current;
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-') {
          i += 1;
        }
        if i < bytes.len() && matches!(bytes[i], b'.' | b':') {
          // Make sure the next char after `.`/`:` is part of a tag-name
          // continuation (alpha) -- if it's punctuation/whitespace we'd
          // already terminate the name.
          if matches!(bytes.get(i + 1), Some(c) if c.is_ascii_alphabetic()) {
            return false;
          }
        }
      }
    }

    self.emit(if is_close { TokenKind::JsxCloseTagStart } else { TokenKind::JsxOpenTagStart });

    // Tag name: identifier chars + `.` (member) + `:` (namespace) + `-`.
    while let Some(c) = self.peek() {
      if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' || c == ':' {
        self.advance();
      } else {
        break;
      }
    }
    self.emit(TokenKind::JsxTagName);

    // Closing tags don't take attributes.
    if is_close {
      self.skip_jsx_whitespace();
      return self.consume_jsx_close('>', TokenKind::JsxCloseTagEnd);
    }

    // Attribute loop.
    loop {
      self.skip_jsx_whitespace();
      match self.peek() {
        Some('/') if self.peek_next() == Some('>') => {
          self.advance();
          self.advance();
          self.emit(TokenKind::JsxSelfClosingEnd);
          return true;
        },
        Some('>') => {
          self.advance();
          self.emit(TokenKind::JsxOpenTagEnd);
          return true;
        },
        Some('{') => {
          if !self.lex_jsx_spread() {
            return false;
          }
        },
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {
          if !self.lex_jsx_attribute() {
            return false;
          }
        },
        _ => return false,
      }
    }
  }

  /// Skip whitespace inside a tag and reset start (in-tag whitespace has
  /// no semantic meaning).
  fn skip_jsx_whitespace(&mut self) {
    self.skip_while_ascii(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r'));
    self.start = self.current;
    self.start_line = self.line;
    self.start_column = self.column;
  }

  fn consume_jsx_close(&mut self, expect: char, kind: TokenKind) -> bool {
    if self.peek() != Some(expect) {
      return false;
    }
    self.advance();
    self.emit(kind);
    true
  }

  fn lex_jsx_attribute(&mut self) -> bool {
    // Attribute name: ident chars + `-` + `:` (namespaced like xml:lang).
    while let Some(c) = self.peek() {
      if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == ':' {
        self.advance();
      } else {
        break;
      }
    }
    self.emit(TokenKind::JsxAttributeName);

    // Boolean attribute (no `=`).
    if self.peek() != Some('=') {
      return true;
    }
    self.advance();
    self.emit(TokenKind::JsxAttrEq);

    match self.peek() {
      Some(q @ ('"' | '\'')) => {
        let kind = if q == '"' { QuoteKind::Double } else { QuoteKind::Single };
        self.advance();
        self.emit(TokenKind::JsxAttrStringOpen(kind));

        // Body until matching quote, handling `\` escapes. Reject when
        // the body would span a block-construct line (CM 6.6: raw HTML
        // tags cannot cross block boundaries).
        let bytes = self.source.as_bytes();
        while let Some(c) = self.peek() {
          if c == '\\' {
            self.advance();
            if self.peek().is_some() {
              self.advance();
            }
            continue;
          }
          if c == q {
            break;
          }
          if c == '\n' {
            // Peek the next line. If it's a thematic break or setext
            // underline (3+ same `-`/`=`/`_`/`*` then end-of-line), the
            // tag construct must abort so the block-level pass sees
            // the underline.
            let after = self.current + 1;
            let mut p = after;
            while p < bytes.len() && (bytes[p] == b' ' || bytes[p] == b'\t') {
              p += 1;
            }
            if p < bytes.len() {
              let marker = bytes[p];
              if matches!(marker, b'-' | b'=' | b'_' | b'*') {
                let mut q2 = p;
                let mut count = 0usize;
                while q2 < bytes.len() {
                  match bytes[q2] {
                    b if b == marker => {
                      count += 1;
                      q2 += 1;
                    },
                    b' ' | b'\t' => q2 += 1,
                    b'\n' => break,
                    _ => {
                      count = 0;
                      break;
                    },
                  }
                }
                if count >= 3 {
                  return false;
                }
              }
            }
            self.advance();
            continue;
          }
          self.advance();
        }
        if self.current > self.start {
          self.emit(TokenKind::JsxAttrString);
        }
        if self.peek() != Some(q) {
          return false;
        }
        self.advance();
        self.emit(TokenKind::JsxAttrStringClose(kind));
        true
      },
      Some('{') => {
        self.advance();
        self.lex_mdx_expression();
        true
      },
      _ => false,
    }
  }

  /// Spread attribute `{...rest}`. Emits ExpressionStart, then the body
  /// as JsxAttributeSpread, then ExpressionEnd.
  fn lex_jsx_spread(&mut self) -> bool {
    if self.peek() != Some('{') {
      return false;
    }
    self.advance();
    self.emit(TokenKind::ExpressionStart);

    let mut depth = 1;
    let mut in_string: Option<char> = None;
    let mut in_template = false;
    while let Some(c) = self.peek() {
      if let Some(q) = in_string {
        match c {
          '\\' => {
            self.advance();
            self.advance();
          },
          _ if c == q => {
            self.advance();
            in_string = None;
          },
          _ => {
            self.advance();
          },
        }
        continue;
      }
      if in_template {
        match c {
          '\\' => {
            self.advance();
            self.advance();
          },
          '`' => {
            self.advance();
            in_template = false;
          },
          _ => {
            self.advance();
          },
        }
        continue;
      }
      match c {
        '"' | '\'' => {
          in_string = Some(c);
          self.advance();
        },
        '`' => {
          in_template = true;
          self.advance();
        },
        '{' => {
          depth += 1;
          self.advance();
        },
        '}' => {
          depth -= 1;
          if depth == 0 {
            if self.current > self.start {
              self.emit(TokenKind::JsxAttributeSpread);
            }
            self.advance();
            self.emit(TokenKind::ExpressionEnd);
            return true;
          }
          self.advance();
        },
        _ => {
          self.advance();
        },
      }
    }
    if self.current > self.start {
      self.emit(TokenKind::JsxAttributeSpread);
    }
    false
  }
}
