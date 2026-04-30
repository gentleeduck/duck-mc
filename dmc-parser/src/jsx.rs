use crate::ast::*;
use crate::parser::Parser;
use dmc_diagnostic::Code;
use dmc_lexer::token::TokenKind;

impl<'eng, 'tokens> Parser<'eng, 'tokens> {
  /// Caller is positioned at `JsxOpenTagStart`. Consumes through the matching
  /// close (or self-close), returns either a JsxElement, JsxSelfClosing, or
  /// JsxFragment node.
  pub(crate) fn parse_jsx(&mut self) -> Node {
    let span = self.current_span();
    self.advance();
    let name = if let Some(t) = self.peek() {
      if matches!(t.kind, TokenKind::JsxTagName) {
        let n = t.raw.to_string();
        self.advance();
        n
      } else {
        String::new()
      }
    } else {
      String::new()
    };

    let attrs = self.parse_jsx_attrs();

    match self.peek_kind() {
      Some(TokenKind::JsxSelfClosingEnd) => {
        self.advance();
        return Node::JsxSelfClosing(JsxSelfClosing { name, attrs, span });
      },
      Some(TokenKind::JsxOpenTagEnd) => {
        self.advance();
      },
      _ => {
        self.warn(
          Code::RecoveredUnterminatedJsx,
          format!("unterminated JSX open tag <{name}> — synthesizing self-close"),
        );
        return Node::JsxSelfClosing(JsxSelfClosing { name, attrs, span });
      },
    }

    let mut children = Vec::new();
    loop {
      match self.peek_kind() {
        Some(TokenKind::JsxCloseTagStart) => {
          self.advance();
          if matches!(self.peek_kind(), Some(TokenKind::JsxTagName)) {
            self.advance();
          }
          if matches!(self.peek_kind(), Some(TokenKind::JsxCloseTagEnd)) {
            self.advance();
          }
          break;
        },
        Some(TokenKind::Eof) | None => break,
        _ => {
          let before = self.pos;
          if let Some(node) = self.parse_block() {
            children.push(node);
          }
          if self.pos == before {
            self.advance();
          }
        },
      }
    }

    if name.is_empty() {
      Node::JsxFragment(JsxFragment { children, span })
    } else {
      Node::JsxElement(JsxElement { name, attrs, children, span })
    }
  }

  /// Consume a run of `name`, `name="str"`, `name={expr}` attributes. Bare
  /// names map to `JsxAttrValue::Boolean`. Stops at the first non-attribute
  /// token (typically `>` / `/>`).
  fn parse_jsx_attrs(&mut self) -> Vec<JsxAttr> {
    let mut out = Vec::new();
    while let Some(TokenKind::JsxAttributeName) = self.peek_kind() {
      let span = self.current_span();
      let name = self.peek().unwrap().raw.to_string();
      self.advance();
      let value = if matches!(self.peek_kind(), Some(TokenKind::Eq)) {
        self.advance();
        match self.peek_kind() {
          Some(TokenKind::String) => {
            let s = self.peek().unwrap().raw.to_string();
            self.advance();
            JsxAttrValue::String(s)
          },
          Some(TokenKind::ExpressionStart) => {
            self.advance();
            let mut s = String::new();
            while let Some(t) = self.peek() {
              match &t.kind {
                TokenKind::ExpressionEnd | TokenKind::Eof => break,
                _ => {
                  s.push_str(t.raw);
                  self.advance();
                },
              }
            }
            if matches!(self.peek_kind(), Some(TokenKind::ExpressionEnd)) {
              self.advance();
            }
            JsxAttrValue::Expression(s)
          },
          _ => JsxAttrValue::Boolean,
        }
      } else {
        JsxAttrValue::Boolean
      };
      out.push(JsxAttr { name, value, span });
    }
    out
  }

  /// Standalone `{expr}` expression. Caller positioned at ExpressionStart.
  pub(crate) fn parse_jsx_expression(&mut self) -> Node {
    let span = self.current_span();
    self.advance();
    let mut s = String::new();
    while let Some(t) = self.peek() {
      match &t.kind {
        TokenKind::ExpressionEnd | TokenKind::Eof => break,
        _ => {
          s.push_str(t.raw);
          self.advance();
        },
      }
    }
    if matches!(self.peek_kind(), Some(TokenKind::ExpressionEnd)) {
      self.advance();
    }
    Node::JsxExpression(JsxExpression { value: s, span })
  }

  /// Skip a markdown comment `{/* ... */}`. Caller is positioned at MarkdownCommentStart.
  pub(crate) fn skip_md_comment(&mut self) {
    self.advance();
    while let Some(t) = self.peek() {
      match &t.kind {
        TokenKind::MarkdownCommentEnd => {
          self.advance();
          break;
        },
        TokenKind::Eof => break,
        _ => {
          self.advance();
        },
      }
    }
  }
}
