use crate::ast::*;
use crate::parser::Parser;
use dmc_lexer::token::TokenKind;

impl<'eng, 'tokens> Parser<'eng, 'tokens> {
  /// Accumulate inline nodes until any top-level break token.
  pub(crate) fn collect_inline_until_break(&mut self) -> Vec<Node> {
    self.collect_inline(&|kind| {
      matches!(
        kind,
        TokenKind::HardBreak
          | TokenKind::SoftBreak
          | TokenKind::Eof
          | TokenKind::Heading(_)
          | TokenKind::FrontmatterStart
          | TokenKind::Import
          | TokenKind::Export
          | TokenKind::JsxCloseTagStart
      )
    })
  }

  /// Inline body of one list item. Same stop set as `collect_inline_until_break`.
  pub(crate) fn collect_inline_for_list_item(&mut self) -> Vec<Node> {
    self.collect_inline_until_break()
  }

  /// Collect inline nodes until `stop(kind)` returns true. The stopping token
  /// is left on the stream.
  pub(crate) fn collect_inline(&mut self, stop: &dyn Fn(&TokenKind) -> bool) -> Vec<Node> {
    let mut out = Vec::new();
    while let Some(t) = self.peek() {
      let kind = t.kind.clone();
      if stop(&kind) {
        break;
      }

      let span = t.span.clone();
      match &kind {
        TokenKind::Text => {
          let raw = t.raw.to_string();
          self.advance();
          out.push(Node::Text(Text { value: raw, span }));
        },
        TokenKind::Autolink => {
          let raw = t.raw.to_string();
          self.advance();
          let inner = raw.trim_start_matches('<').trim_end_matches('>').to_string();
          let href = if inner.contains("://") {
            inner.clone()
          } else if inner.contains('@') {
            format!("mailto:{inner}")
          } else {
            inner.clone()
          };
          out.push(Node::Link(Link {
            href,
            title: None,
            children: vec![Node::Text(Text { value: inner, span: span.clone() })],
            span,
          }));
        },
        TokenKind::Whitespace => {
          let raw = t.raw.to_string();
          self.advance();
          out.push(Node::Text(Text { value: raw, span }));
        },
        TokenKind::Bold(n) => {
          let open_n = *n;
          self.advance();
          let inner =
            self.collect_inline(&|k| Self::is_top_level_break(k) || matches!(k, TokenKind::Bold(m) if *m == open_n));
          if matches!(self.peek_kind(), Some(TokenKind::Bold(m)) if *m == open_n) {
            self.advance();
          }
          out.push(Node::Bold(Inline { children: inner, span }));
        },
        TokenKind::Italic(n) => {
          let open_n = *n;
          self.advance();
          let inner =
            self.collect_inline(&|k| Self::is_top_level_break(k) || matches!(k, TokenKind::Italic(m) if *m == open_n));
          if matches!(self.peek_kind(), Some(TokenKind::Italic(m)) if *m == open_n) {
            self.advance();
          }
          out.push(Node::Italic(Inline { children: inner, span }));
        },
        TokenKind::Strike(n) => {
          let open_n = *n;
          self.advance();
          let inner =
            self.collect_inline(&|k| Self::is_top_level_break(k) || matches!(k, TokenKind::Strike(m) if *m == open_n));
          if matches!(self.peek_kind(), Some(TokenKind::Strike(m)) if *m == open_n) {
            self.advance();
          }
          out.push(Node::Strikethrough(Inline { children: inner, span }));
        },
        TokenKind::CodeStart(n) => {
          let open_n = *n;
          self.advance();
          let mut value = String::new();
          while let Some(tok) = self.peek() {
            match &tok.kind {
              TokenKind::CodeEnd(m) if *m == open_n => {
                self.advance();
                break;
              },
              TokenKind::Eof => break,
              _ => {
                value.push_str(tok.raw);
                self.advance();
              },
            }
          }
          out.push(Node::InlineCode(InlineCode { value, span }));
        },
        TokenKind::Bracket => {
          let start = self.pos;
          self.advance();
          let inner = self.collect_inline(&|k| {
            matches!(k, TokenKind::Bracket | TokenKind::HardBreak | TokenKind::SoftBreak | TokenKind::Eof)
          });
          if !matches!(self.peek_kind(), Some(TokenKind::Bracket)) {
            self.pos = start;
            out.push(Node::Text(Text { value: "[".into(), span }));
            self.advance();
            continue;
          }
          self.advance();
          let mut href = String::new();
          if matches!(self.peek_kind(), Some(TokenKind::ParenOpen)) {
            self.advance();
            while let Some(tok) = self.peek() {
              match &tok.kind {
                TokenKind::ParenClose => {
                  self.advance();
                  break;
                },
                TokenKind::Eof => break,
                _ => {
                  href.push_str(tok.raw);
                  self.advance();
                },
              }
            }
          }
          out.push(Node::Link(Link { href, title: None, children: inner, span }));
        },
        TokenKind::Bang => {
          let start = self.pos;
          self.advance();
          if !matches!(self.peek_kind(), Some(TokenKind::Bracket)) {
            self.pos = start;
            out.push(Node::Text(Text { value: "!".into(), span }));
            self.advance();
            continue;
          }
          self.advance();
          let mut alt = String::new();
          while let Some(tok) = self.peek() {
            match &tok.kind {
              TokenKind::Bracket => {
                self.advance();
                break;
              },
              TokenKind::Eof | TokenKind::HardBreak | TokenKind::SoftBreak => break,
              _ => {
                alt.push_str(tok.raw);
                self.advance();
              },
            }
          }
          let mut src = String::new();
          if matches!(self.peek_kind(), Some(TokenKind::ParenOpen)) {
            self.advance();
            while let Some(tok) = self.peek() {
              match &tok.kind {
                TokenKind::ParenClose => {
                  self.advance();
                  break;
                },
                TokenKind::Eof => break,
                _ => {
                  src.push_str(tok.raw);
                  self.advance();
                },
              }
            }
          }
          out.push(Node::Image(Image { src, alt, title: None, span }));
        },
        TokenKind::JsxOpenTagStart => {
          out.push(self.parse_jsx());
          continue;
        },
        TokenKind::ExpressionStart => {
          out.push(self.parse_jsx_expression());
          continue;
        },
        TokenKind::MarkdownCommentStart => {
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
          continue;
        },
        _ => {
          let raw = t.raw.to_string();
          self.advance();
          if !raw.is_empty() {
            out.push(Node::Text(Text { value: raw, span }));
          }
        },
      }
    }
    out
  }

  /// Tokens that terminate inline collection regardless of nesting depth.
  pub(crate) fn is_top_level_break(k: &TokenKind) -> bool {
    matches!(
      k,
      TokenKind::HardBreak
        | TokenKind::SoftBreak
        | TokenKind::Eof
        | TokenKind::Heading(_)
        | TokenKind::FrontmatterStart
        | TokenKind::Import
        | TokenKind::Export
        | TokenKind::JsxCloseTagStart
    )
  }
}
