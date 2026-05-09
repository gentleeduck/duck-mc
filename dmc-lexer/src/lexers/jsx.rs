use crate::{Lexer, token::TokenKind};

impl<'eng, 'src: 'eng> Lexer<'eng, 'src> {
  /// Entry for `<` followed by an alphabetic char or `/`. Emits the open/close
  /// markers, tag name, attributes, and end marker (regular or self-closing).
  pub(crate) fn lex_jsx_tag(&mut self) {
    let mut is_close_tag = false;

    if self.peek() == Some('/') {
      self.advance();
      is_close_tag = true;
      self.emit(TokenKind::JsxCloseTagStart);
    } else {
      self.emit(TokenKind::JsxOpenTagStart);
    }

    self.consume_whitespaces();
    self.skip_while_ascii(|b| b.is_ascii_alphanumeric() || b == b'.');
    self.emit(TokenKind::JsxTagName);

    self.skip_jsx_tag_ws();
    while let Some(cc) = self.current_char()
      && (cc.is_alphabetic() || cc == '_')
    {
      self.lex_jsx_attribute();
      self.skip_jsx_tag_ws();
    }

    if self.current_char() == Some('/') {
      self.advance();
      if self.current_char() == Some('>') {
        self.advance();
      }
      return self.emit(TokenKind::JsxSelfClosingEnd);
    }

    self.advance();
    if is_close_tag {
      return self.emit(TokenKind::JsxCloseTagEnd);
    }
    self.emit(TokenKind::JsxOpenTagEnd)
  }

  fn skip_jsx_tag_ws(&mut self) {
    while let Some(c) = self.current_char() {
      match c {
        ' ' | '\t' => {
          self.advance();
        },
        '\n' => {
          self.advance();
          self.line += 1;
          self.column = 0;
        },
        _ => break,
      }
    }
    self.start = self.current;
  }

  /// Lex one `name`, `name=value` or `name={expr}` attribute inside a JSX tag.
  pub(crate) fn lex_jsx_attribute(&mut self) {
    self.skip_while_ascii(|b| b.is_ascii_alphanumeric() || b == b'-');
    self.emit(TokenKind::JsxAttributeName);

    if self.current_char() != Some('=') {
      return;
    }
    self.advance();
    self.emit(TokenKind::Eq);

    match self.current_char() {
      Some(kind) if kind == '"' || kind == '\'' => {
        self.advance();
        self.emit(TokenKind::Quote);
        self.consume_until(kind);
        self.emit(TokenKind::String);
        if let Some(c) = self.current_char()
          && c == kind
        {
          self.advance();
          self.emit(TokenKind::Quote);
        }
      },
      Some('{') => {
        self.advance();
        self.emit(TokenKind::ExpressionStart);
        let mut depth = 1usize;
        let mut quote: Option<char> = None;
        while let Some(c) = self.current_char() {
          if let Some(q) = quote {
            match c {
              '\\' => {
                self.advance();
                if self.current_char().is_some() {
                  self.advance();
                }
              },
              c if c == q => {
                quote = None;
                self.advance();
              },
              '\n' => {
                self.advance();
                self.line += 1;
                self.column = 0;
              },
              _ => {
                self.advance();
              },
            }
            continue;
          }
          match c {
            '"' | '\'' | '`' => {
              quote = Some(c);
              self.advance();
            },
            '{' => {
              self.advance();
              depth += 1;
            },
            '}' => {
              depth -= 1;
              if depth == 0 {
                self.emit(TokenKind::Text);
                self.advance();
                self.emit(TokenKind::ExpressionEnd);
                break;
              }
              self.advance();
            },
            '\n' => {
              break;
            },
            _ => {
              self.advance();
            },
          }
        }
      },
      _ => {},
    }
  }

  /// Lex an MDX-style comment `{/* ... */}`. Caller has consumed the opening `{`.
  pub(crate) fn lex_md_comment(&mut self) {
    self.emit(TokenKind::MarkdownCommentStart);
    self.advance();
    self.advance();
    self.start = self.current;

    loop {
      if self.is_eof() {
        self.emit(TokenKind::Text);
        return;
      }
      if self.peek() == Some('*') && self.peek_next() == Some('/') {
        let content_end = self.current;
        self.advance();
        self.advance();
        if self.peek() == Some('}') {
          self.advance();
          let saved_current = self.current;
          self.current = content_end;
          self.emit(TokenKind::Text);
          self.start = content_end;
          self.current = saved_current;
          self.emit(TokenKind::MarkdownCommentEnd);
          return;
        }
        continue;
      }
      let c = self.advance();
      if c == '\n' {
        self.line += 1;
        self.column = 0;
      }
    }
  }

  /// Lex a top-level `{ ... }` JSX expression node. Tracks brace depth so
  /// nested object literals don't close the outer expression.
  pub(crate) fn lex_expression(&mut self) {
    self.emit(TokenKind::ExpressionStart);
    let mut depth = 1usize;
    let mut quote: Option<char> = None;

    while !self.is_eof() {
      if let Some(q) = quote {
        match self.peek() {
          Some('\\') => {
            self.advance();
            if self.peek().is_some() {
              self.advance();
            }
            continue;
          },
          Some(c) if c == q => {
            quote = None;
            self.advance();
            continue;
          },
          Some('\n') => {
            self.advance();
            self.line += 1;
            self.column = 0;
            continue;
          },
          Some(_) => {
            self.advance();
            continue;
          },
          None => break,
        }
      }
      match self.peek() {
        Some('"') | Some('\'') | Some('`') => {
          quote = self.peek();
          self.advance();
        },
        Some('{') => {
          self.advance();
          depth += 1;
        },
        Some('}') => {
          depth -= 1;
          if depth == 0 {
            self.emit(TokenKind::Text);
            self.advance();
            return self.emit(TokenKind::ExpressionEnd);
          }
          self.advance();
        },
        Some('\n') => {
          self.advance();
          self.line += 1;
          self.column = 0;
        },
        Some(_) => {
          self.advance();
        },
        None => break,
      }
    }
  }
}
