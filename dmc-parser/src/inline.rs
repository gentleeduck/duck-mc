use crate::ast::*;
use crate::parser::Parser;
use dmc_lexer::token::TokenKind;

fn utf8_char_len(b: u8) -> usize {
  if b < 0x80 {
    1
  } else if b < 0xC0 {
    1
  } else if b < 0xE0 {
    2
  } else if b < 0xF0 {
    3
  } else {
    4
  }
}

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

  /// Inline body of one list item. Same stop set as
  /// `collect_inline_until_break`, but skips the single leading
  /// `Whitespace` token that follows the marker (`- foo` vs `-foo`).
  pub(crate) fn collect_inline_for_list_item(&mut self) -> Vec<Node> {
    if matches!(self.peek_kind(), Some(TokenKind::Whitespace)) {
      self.advance();
    }
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
          let raw = Self::unescape_markdown(t.raw);
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
          // `***x***` (Bold(3)) means strong+em combined per CommonMark:
          // wrap as <em><strong>x</strong></em>.
          if open_n == 3 {
            let strong = Node::Bold(Inline { children: inner, span: span.clone() });
            out.push(Node::Italic(Inline { children: vec![strong], span }));
          } else {
            out.push(Node::Bold(Inline { children: inner, span }));
          }
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
          let mut paren_body = String::new();
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
                  paren_body.push_str(tok.raw);
                  self.advance();
                },
              }
            }
          }
          let (href, title) = Self::split_destination_title(&paren_body);
          out.push(Node::Link(Link { href, title, children: inner, span }));
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
          let mut paren_body = String::new();
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
                  paren_body.push_str(tok.raw);
                  self.advance();
                },
              }
            }
          }
          let (src, title) = Self::split_destination_title(&paren_body);
          out.push(Node::Image(Image { src, alt, title, span }));
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

  /// Split the body of a `(...)` link/image destination into
  /// `(href, title)`. CommonMark allows an optional trailing
  /// `"title"` / `'title'` / `(title)` separated from the destination
  /// by whitespace. Unterminated/missing title returns `(body, None)`.
  fn split_destination_title(body: &str) -> (String, Option<String>) {
    let trimmed = body.trim();
    if trimmed.is_empty() {
      return (String::new(), None);
    }
    // Walk back from the end looking for a balanced quoted title.
    let bytes = trimmed.as_bytes();
    let last = bytes[bytes.len() - 1];
    let close = match last {
      b'"' => Some(b'"'),
      b'\'' => Some(b'\''),
      b')' => Some(b'('),
      _ => None,
    };
    let Some(open) = close else {
      return (trimmed.to_string(), None);
    };
    // Find the matching opener, ensuring a whitespace separator before it.
    let mut i = bytes.len() - 1;
    let mut depth = 1;
    while i > 0 {
      i -= 1;
      let b = bytes[i];
      if b == last && b != open {
        depth += 1;
      }
      if b == open {
        depth -= 1;
        if depth == 0 {
          break;
        }
      }
    }
    if depth != 0 {
      return (trimmed.to_string(), None);
    }
    // Need at least one whitespace between dest and the opener.
    if i == 0 || !bytes[i - 1].is_ascii_whitespace() {
      return (trimmed.to_string(), None);
    }
    let dest = trimmed[..i].trim_end().to_string();
    let title = trimmed[i + 1..bytes.len() - 1].to_string();
    (dest, Some(title))
  }

  /// Strip `\X` -> `X` for the standard CommonMark escapable set so
  /// authors can write `\*literal\*` without the asterisks turning into
  /// emphasis. The lexer keeps the backslash in `Text` raw to preserve
  /// source spans; this collapses it for the rendered text.
  fn unescape_markdown(s: &str) -> String {
    if !s.contains('\\') {
      return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
      if bytes[i] == b'\\' && i + 1 < bytes.len() {
        let nx = bytes[i + 1];
        if matches!(
          nx,
          b'\\'
            | b'*'
            | b'_'
            | b'`'
            | b'<'
            | b'>'
            | b'{'
            | b'}'
            | b'['
            | b']'
            | b'('
            | b')'
            | b'!'
            | b'#'
            | b'-'
            | b'$'
            | b'~'
        ) {
          out.push(nx as char);
          i += 2;
          continue;
        }
      }
      let ch_len = utf8_char_len(bytes[i]);
      out.push_str(&s[i..i + ch_len]);
      i += ch_len;
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
