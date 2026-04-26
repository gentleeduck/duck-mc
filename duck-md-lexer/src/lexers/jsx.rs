use duck_diagnostic::{Diagnostic, Label, Span};

use crate::{Lexer, diagnostic::Code, token::TokenKind};

impl<'engine> Lexer<'engine> {
  pub(crate) fn lex_jsx_tag(&mut self) {
    let mut is_close_tag = false;

    if self.peek() == Some('/') {
      self.advance();
      is_close_tag = true;
      self.emit(TokenKind::JsxCloseTagStart); //  </
    } else {
      self.emit(TokenKind::JsxOpenTagStart); // <
    }

    self.consume_whitespaces();

    // lex tag name e.g. Button, MyCard
    self.consume_while(|c, _| c.is_alphanumeric() || c == '.');
    self.emit(TokenKind::JsxTagName);

    self.consume_whitespaces();

    while let Some(cc) = self.get_current_char()
      && (cc.is_alphabetic() || cc == '_')
    {
      self.lex_jsx_attribute();
      if self.get_current_char() == Some(' ') {
        self.consume_whitespaces();
      }
    }

    // self-close: />
    if self.get_current_char() == Some('/') {
      self.advance(); // /
      if self.get_current_char() == Some('>') {
        self.advance(); // >
      }
      return self.emit(TokenKind::JsxSelfClosingEnd);
    }

    // regular close: >
    self.advance();
    if is_close_tag {
      return self.emit(TokenKind::JsxCloseTagEnd);
    }
    self.emit(TokenKind::JsxOpenTagEnd)
  }

  pub(crate) fn lex_jsx_attribute(&mut self) {
    self.consume_while(|c, _| c.is_alphanumeric() || c == '-');
    self.emit(TokenKind::JsxAttributeName);

    // If the next char is not `=`, this is a boolean attribute — nothing more to do.
    let next = self.get_current_char();
    if next != Some('=') {
      return;
    }

    self.advance(); // consume the =
    self.emit(TokenKind::Eq);

    match self.get_current_char() {
      Some(kind) if kind == '"' || kind == '\'' => {
        self.advance(); // consume the ' | "
        self.emit(TokenKind::Quote);

        self.consume_till(kind);
        self.emit(TokenKind::String);

        if let Some(c) = self.get_current_char()
          && c == kind
        {
          self.advance();
          self.emit(TokenKind::Quote);
        }
      },
      Some('{') => {
        self.advance(); // consume the {
        self.emit(TokenKind::ExpressionStart);

        let mut depth = 1usize;
        let mut terminated = false;
        while let Some(c) = self.get_current_char() {
          match c {
            '{' => {
              self.advance();
              depth += 1;
            },
            '}' => {
              depth -= 1;
              if depth == 0 {
                // emit the inner expression text before consuming the closing brace
                self.emit(TokenKind::Text);
                self.advance(); // consume closing }
                self.emit(TokenKind::ExpressionEnd);
                terminated = true;
                break;
              }
              self.advance();
            },
            '\n' => {
              self.emit_diagnostic(Diagnostic::new(
                Code::UnterminatedExpression,
                "unterminated expression",
              ));
              break;
            },
            _ => {
              self.advance();
            },
          }
        }

        if !terminated && self.is_eof() {
          self.emit_diagnostic(Diagnostic::new(
            Code::UnterminatedExpression,
            "unterminated expression",
          ));
        }
      },
      _ => {
        self.emit_diagnostic(Diagnostic::new(Code::InvalidJsxAttribute, "invalid jsx attribute"));
      },
    }
  }

  pub(crate) fn lex_md_comment(&mut self) {
    // caller already consumed '{'. current points at '/'.
    // Emit '{' as MarkdownCommentStart.
    self.emit(TokenKind::MarkdownCommentStart);

    // advance past '/' and '*'
    self.advance(); // /
    self.advance(); // *
    // reset start so next emit's lexeme begins at the inner content
    self.start = self.current;

    // consume until `*/}` (in that order)
    loop {
      if self.is_eof() {
        // emit the inner text we have so far
        self.emit(TokenKind::Text);
        self.emit_diagnostic(
          Diagnostic::new(Code::UnterminatedExpression, "unterminated markdown comment")
            .with_label(Label::primary(
              Span::new("", self.line, self.column, 1),
              Some("markdown comment not closed before end of file".to_string()),
            ))
            .with_help("close with `*/}`"),
        );
        return;
      }

      if self.peek() == Some('*') && self.peek_next() == Some('/') {
        let content_end = self.current;
        self.advance(); // *
        self.advance(); // /
        if self.peek() == Some('}') {
          self.advance(); // }

          // emit the inner content (before */})
          let saved_current = self.current;
          self.current = content_end;
          self.emit(TokenKind::Text);

          // emit the closing */}
          self.start = content_end;
          self.current = saved_current;
          self.emit(TokenKind::MarkdownCommentEnd);
          return;
        }
        // not */}, keep going
        continue;
      }

      let c = self.advance();
      if c == '\n' {
        self.line += 1;
        self.column = 0;
      }
    }
  }

  pub(crate) fn lex_expression(&mut self) {
    // opening '{' already consumed by caller
    self.emit(TokenKind::ExpressionStart);

    // track nesting depth for expressions like { a ? { b } : c }
    let mut depth = 1usize;

    while !self.is_eof() {
      match self.peek() {
        Some('{') => {
          self.advance();
          depth += 1;
        },
        Some('}') => {
          depth -= 1;
          if depth == 0 {
            // emit the raw expression content before consuming '}'
            self.emit(TokenKind::Text);
            self.advance(); // consume closing '}'
            return self.emit(TokenKind::ExpressionEnd);
          }
          self.advance();
        },
        Some('\n') => {
          // expressions cannot span multiple lines unless inside nested braces
          if depth == 1 {
            self.emit_diagnostic(
              Diagnostic::new(Code::UnterminatedExpression, "unterminated expression")
                .with_label(Label::primary(
                  Span::new("", self.line, self.column, 1),
                  Some("expression not closed before end of line".to_string()),
                ))
                .with_help("close the expression with `}`"),
            );
          }
          self.advance();
        },
        Some(_) => {
          self.advance();
        },
        None => break,
      }
    }

    self.emit_diagnostic(
      Diagnostic::new(Code::UnterminatedExpression, "unterminated expression")
        .with_label(Label::primary(
          Span::new("", self.line, self.column, 1),
          Some("reached end of file before closing `}`".to_string()),
        ))
        .with_help("close the expression with `}`"),
    );
  }
}
