use duck_diagnostic::{Diagnostic, Label, Span};

use crate::{Lexer, token::TokenKind};
use dmc_diagnostic::Code;

impl<'eng, 'src: 'eng> Lexer<'eng, 'src> {
  /// Entry for `<` followed by an alphabetic char or `/`. Emits the open/close
  /// markers, tag name, attributes, and end marker (regular or self-closing).
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
    self.skip_while_ascii(|b| b.is_ascii_alphanumeric() || b == b'.');
    self.emit(TokenKind::JsxTagName);

    self.consume_whitespaces();

    while let Some(cc) = self.current_char()
      && (cc.is_alphabetic() || cc == '_')
    {
      self.lex_jsx_attribute();
      if self.current_char() == Some(' ') {
        self.consume_whitespaces();
      }
    }

    // self-close: />
    if self.current_char() == Some('/') {
      self.advance(); // /
      if self.current_char() == Some('>') {
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

  /// Lex one `name`, `name=value` or `name={expr}` attribute inside a JSX tag.
  pub(crate) fn lex_jsx_attribute(&mut self) {
    self.skip_while_ascii(|b| b.is_ascii_alphanumeric() || b == b'-');
    self.emit(TokenKind::JsxAttributeName);

    // If the next char is not `=`, this is a boolean attribute - nothing more to do.
    let next = self.current_char();
    if next != Some('=') {
      return;
    }

    self.advance(); // consume the =
    self.emit(TokenKind::Eq);

    match self.current_char() {
      Some(kind) if kind == '"' || kind == '\'' => {
        self.advance(); // consume the ' | "
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
        let start_line = self.line;
        let start_col = self.column;
        self.advance(); // consume the {
        self.emit(TokenKind::ExpressionStart);

        let mut depth = 1usize;
        let mut terminated = false;
        while let Some(c) = self.current_char() {
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
              self.diag(
                Diagnostic::new(Code::UnterminatedExpression, "unterminated jsx attribute expression")
                  .with_label(Label::primary(
                    Span::from_zero_based("", start_line, start_col, 1),
                    Some("expression starts here".to_string()),
                  ))
                  .with_label(Label::secondary(
                    Span::from_zero_based("", self.line, self.column, 1),
                    Some("expected `}` before end of line".to_string()),
                  ))
                  .with_help("close the expression with `}`"),
              );
              break;
            },
            _ => {
              self.advance();
            },
          }
        }

        if !terminated && self.is_eof() {
          self.diag(
            Diagnostic::new(Code::UnterminatedExpression, "unterminated jsx attribute expression")
              .with_label(Label::primary(
                Span::from_zero_based("", start_line, start_col, 1),
                Some("expression starts here".to_string()),
              ))
              .with_label(Label::secondary(
                Span::from_zero_based("", self.line, self.column, 1),
                Some("reached end of file".to_string()),
              ))
              .with_help("close the expression with `}`"),
          );
        }
      },
      _ => {
        self.diag(
          Diagnostic::new(Code::InvalidJsxAttribute, "invalid jsx attribute")
            .with_label(Label::primary(
              Span::from_zero_based("", self.line, self.column, 1),
              Some("expected `\"...\"` or `{...}` after `=`".to_string()),
            ))
            .with_help("attribute values must be quoted strings or `{}`-wrapped expressions"),
        );
      },
    }
  }

  /// Lex an MDX-style comment `{/* ... */}`. Caller has consumed the opening `{`.
  pub(crate) fn lex_md_comment(&mut self) {
    // caller already consumed '{'. current points at '/'.
    // Track '{' opener for diagnostic.
    let start_line = self.line;
    let start_col = self.column.saturating_sub(1);
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
        self.diag(
          Diagnostic::new(Code::UnterminatedExpression, "unterminated markdown comment")
            .with_label(Label::primary(
              Span::from_zero_based("", start_line, start_col, 3),
              Some("comment opens here".to_string()),
            ))
            .with_label(Label::secondary(
              Span::from_zero_based("", self.line, self.column, 1),
              Some("end of file reached".to_string()),
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

  /// Lex a top-level `{ ... }` JSX expression node. Tracks brace depth so
  /// nested object literals don't close the outer expression.
  pub(crate) fn lex_expression(&mut self) {
    // opening '{' already consumed by caller - its position is one column back
    let start_line = self.line;
    let start_col = self.column.saturating_sub(1);
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
            self.diag(
              Diagnostic::new(Code::UnterminatedExpression, "unterminated expression")
                .with_label(Label::primary(
                  Span::from_zero_based("", start_line, start_col, 1),
                  Some("expression starts here".to_string()),
                ))
                .with_label(Label::secondary(
                  Span::from_zero_based("", self.line, self.column, 1),
                  Some("expected `}` before end of line".to_string()),
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

    self.diag(
      Diagnostic::new(Code::UnterminatedExpression, "unterminated expression")
        .with_label(Label::primary(
          Span::from_zero_based("", start_line, start_col, 1),
          Some("expression starts here".to_string()),
        ))
        .with_label(Label::secondary(
          Span::from_zero_based("", self.line, self.column, 1),
          Some("reached end of file before closing `}`".to_string()),
        ))
        .with_help("close the expression with `}`"),
    );
  }
}
