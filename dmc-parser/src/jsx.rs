use crate::ast::*;
use crate::parser::Parser;
use dmc_diagnostic::Code;
use dmc_lexer::token::{QuoteKind, TokenKind};

impl<'eng, 'tokens> Parser<'eng, 'tokens> {
  /// Lowercase / kebab tag names route through the CM raw-HTML path;
  /// uppercase / namespaced / member names stay on the JSX path.
  pub(crate) fn is_plain_html_jsx_tag(&self) -> bool {
    let Some(open) = self.tokens.get(self.pos) else {
      return false;
    };
    if !matches!(open.kind, TokenKind::JsxOpenTagStart | TokenKind::JsxCloseTagStart) {
      return false;
    }
    let Some(name_tok) = self.tokens.get(self.pos + 1) else {
      return false;
    };
    if !matches!(name_tok.kind, TokenKind::JsxTagName) {
      return false;
    }
    name_tok.raw.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
  }

  /// True when the cursor is at a `JsxCloseTagStart` whose name matches an
  /// element in `jsx_open_stack`. Such a tag belongs to an enclosing
  /// `parse_jsx` frame and must end the current inline/block collection.
  pub(crate) fn jsx_close_tag_closes_enclosing(&self) -> bool {
    if self.jsx_open_stack.is_empty() {
      return false;
    }
    let Some(open) = self.tokens.get(self.pos) else {
      return false;
    };
    if !matches!(open.kind, TokenKind::JsxCloseTagStart) {
      return false;
    }
    let Some(name_tok) = self.tokens.get(self.pos + 1) else {
      return false;
    };
    if !matches!(name_tok.kind, TokenKind::JsxTagName) {
      return false;
    }
    self.jsx_open_stack.iter().any(|n| n == name_tok.raw)
  }

  /// Should a lowercase block-level JSX open tag (`<div ...>`) parse as a
  /// `JsxElement` instead of raw HTML?  True when either we're already
  /// inside a `parse_jsx` children loop, or the open tag carries JSX
  /// attribute syntax (`className`, an expression value, `{...spread}`).
  /// Always false under `cm_strict_html_blocks`. Uppercase tag names are
  /// not a signal here because all-caps HTML like `<XMP>` would trip it.
  pub(crate) fn lowercase_jsx_tag_is_mdx_element(&self) -> bool {
    if self.options.cm_strict_html_blocks {
      return false;
    }
    if !matches!(self.peek_kind(), Some(TokenKind::JsxOpenTagStart)) || !self.is_plain_html_jsx_tag() {
      return false;
    }
    if !self.jsx_open_stack.is_empty() {
      // Lowercase descendants of a JSX element are themselves mdast
      // `mdxJsxFlowElement` children.
      return true;
    }
    let mut i = self.pos + 2;
    loop {
      match self.tokens.get(i).map(|t| &t.kind) {
        Some(TokenKind::JsxAttributeName) if self.tokens[i].raw == "className" => return true,
        Some(TokenKind::ExpressionStart) | Some(TokenKind::JsxAttributeSpread) => return true,
        Some(TokenKind::JsxOpenTagEnd) | Some(TokenKind::JsxSelfClosingEnd) | Some(TokenKind::Eof) | None => {
          return false;
        },
        _ => {},
      }
      i += 1;
    }
  }

  pub(crate) fn is_htmlish_jsx_tag(&self) -> bool {
    let Some(open) = self.tokens.get(self.pos) else {
      return false;
    };
    if !matches!(open.kind, TokenKind::JsxOpenTagStart | TokenKind::JsxCloseTagStart) {
      return false;
    }
    let Some(name_tok) = self.tokens.get(self.pos + 1) else {
      return false;
    };
    if !matches!(name_tok.kind, TokenKind::JsxTagName) {
      return false;
    }
    let mut chars = name_tok.raw.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic()) && chars.all(|c| c.is_ascii_alphanumeric() || c == '-')
  }

  /// CM raw HTML rejects JS-style quote escapes inside attribute strings.
  pub(crate) fn jsx_raw_html_tag_is_valid(&self) -> bool {
    self.jsx_raw_html_tag_is_valid_with(self.is_plain_html_jsx_tag())
  }

  pub(crate) fn jsx_raw_html_tag_is_valid_htmlish(&self) -> bool {
    self.jsx_raw_html_tag_is_valid_with(self.is_htmlish_jsx_tag())
  }

  fn jsx_raw_html_tag_is_valid_with(&self, allowed_tag: bool) -> bool {
    let Some(kind) = self.peek_kind() else {
      return false;
    };
    if !allowed_tag {
      return false;
    }
    let is_close = matches!(kind, TokenKind::JsxCloseTagStart);
    let mut i = self.pos + 2;
    let mut valid = true;
    let mut quote: Option<QuoteKind> = None;
    while let Some(tok) = self.tokens.get(i) {
      match tok.kind {
        TokenKind::JsxAttrStringOpen(kind) if !is_close => {
          quote = Some(kind);
        },
        TokenKind::JsxAttrString if !is_close => {
          if let Some(kind) = quote {
            let escaped_quote = match kind {
              QuoteKind::Double => "\\\"",
              QuoteKind::Single => "\\'",
            };
            if tok.raw.contains(escaped_quote) {
              valid = false;
            }
          }
        },
        TokenKind::JsxAttrStringClose(_) if !is_close => {
          quote = None;
        },
        TokenKind::JsxAttributeName | TokenKind::JsxAttrEq if !is_close => {},
        TokenKind::JsxOpenTagEnd | TokenKind::JsxSelfClosingEnd if !is_close => {
          return valid && quote.is_none();
        },
        TokenKind::JsxCloseTagEnd if is_close => return valid,
        TokenKind::ExpressionStart | TokenKind::ExpressionEnd | TokenKind::JsxAttributeSpread => {
          valid = false;
        },
        TokenKind::Eof | TokenKind::BlankLine | TokenKind::SoftBreak | TokenKind::HardBreak => return false,
        _ => {
          valid = false;
        },
      }
      i += 1;
    }
    false
  }

  /// Reconstruct a JSX-tokenized lowercase HTML tag as either raw HTML
  /// (when CM-valid) or literal text (otherwise).
  pub(crate) fn parse_inline_raw_html_tag(&mut self) -> Option<Node> {
    let kind = self.peek_kind()?.clone();
    if !matches!(kind, TokenKind::JsxOpenTagStart | TokenKind::JsxCloseTagStart) || !self.is_plain_html_jsx_tag() {
      return None;
    }

    let span = self.current_span();
    let valid = self.jsx_raw_html_tag_is_valid();
    let start_idx = self.pos;
    let mut end_idx = self.pos;
    let want_close = matches!(kind, TokenKind::JsxCloseTagStart);
    while let Some(tok) = self.tokens.get(end_idx) {
      let done = match tok.kind {
        TokenKind::JsxCloseTagEnd => want_close,
        TokenKind::JsxOpenTagEnd | TokenKind::JsxSelfClosingEnd => !want_close,
        _ => false,
      };
      if done {
        break;
      }
      if matches!(tok.kind, TokenKind::Eof) {
        return None;
      }
      end_idx += 1;
    }

    let value = self.raw_source_for_token_range(start_idx, end_idx + 1);
    self.pos = end_idx + 1;

    Some(if valid {
      Node::Html(Html { value, span })
    } else {
      Node::Text(Text { value: Self::unescape_markdown(&value), span })
    })
  }

  /// Skip JSX tag-internal whitespace tokens (structural noise; the lexer
  /// keeps them for inline spacing).
  fn skip_jsx_ws(&mut self) {
    while matches!(self.peek_kind(), Some(TokenKind::Whitespace(_))) {
      self.advance();
    }
  }

  /// Cursor at `JsxOpenTagStart`. Consumes through the matching close
  /// (or self-close) and returns `JsxElement` / `JsxSelfClosing`.
  pub(crate) fn parse_jsx(&mut self) -> Node {
    let span = self.current_span();
    self.advance();
    self.skip_jsx_ws();
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
    self.skip_jsx_ws();

    let attrs = self.parse_jsx_attrs();

    self.skip_jsx_ws();
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
          format!("unterminated JSX open tag <{name}> -- synthesizing self-close"),
        );
        return Node::JsxSelfClosing(JsxSelfClosing { name, attrs, span });
      },
    }

    // Push so child collection knows `</name>` closes us, not text.
    // Fragments use a distinct close token, so only named elements push.
    if !name.is_empty() {
      self.jsx_open_stack.push(name.clone());
    }

    let mut children = Vec::new();
    loop {
      match self.peek_kind() {
        Some(TokenKind::JsxCloseTagStart) => {
          self.advance();
          self.skip_jsx_ws();
          if matches!(self.peek_kind(), Some(TokenKind::JsxTagName)) {
            self.advance();
          }
          self.skip_jsx_ws();
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

    if !name.is_empty() {
      self.jsx_open_stack.pop();
    }

    let children = unwrap_jsx_only_paragraphs(children);
    let children = strip_jsx_layout_whitespace(children);

    if name.is_empty() {
      Node::JsxFragment(JsxFragment { children, span })
    } else {
      Node::JsxElement(JsxElement { name, attrs, children, span })
    }
  }

  /// Consume `name`, `name="str"`, `name={expr}`, `{...spread}` attributes.
  /// Bare names map to `JsxAttrValue::Boolean`.
  fn parse_jsx_attrs(&mut self) -> Vec<JsxAttr> {
    let mut out = Vec::new();
    self.skip_jsx_ws();
    loop {
      // Spread `{...rest}`: lexer wraps body in
      // ExpressionStart / JsxAttributeSpread / ExpressionEnd.
      if matches!(self.peek_kind(), Some(TokenKind::ExpressionStart)) {
        let span = self.current_span();
        self.advance();
        let body = if matches!(self.peek_kind(), Some(TokenKind::JsxAttributeSpread)) {
          let s = self.peek().unwrap().raw.to_string();
          self.advance();
          s
        } else {
          String::new()
        };
        if matches!(self.peek_kind(), Some(TokenKind::ExpressionEnd)) {
          self.advance();
        }
        out.push(JsxAttr { name: String::new(), value: JsxAttrValue::Spread(body), span });
        self.skip_jsx_ws();
        continue;
      }
      if !matches!(self.peek_kind(), Some(TokenKind::JsxAttributeName)) {
        break;
      }
      let span = self.current_span();
      let name = self.peek().unwrap().raw.to_string();
      self.advance();
      self.skip_jsx_ws();
      let value = if matches!(self.peek_kind(), Some(TokenKind::JsxAttrEq)) {
        self.advance();
        self.skip_jsx_ws();
        match self.peek_kind() {
          Some(TokenKind::JsxAttrStringOpen(_)) => {
            self.advance();
            let s = if matches!(self.peek_kind(), Some(TokenKind::JsxAttrString)) {
              let s = self.peek().unwrap().raw.to_string();
              self.advance();
              s
            } else {
              String::new()
            };
            if matches!(self.peek_kind(), Some(TokenKind::JsxAttrStringClose(_))) {
              self.advance();
            }
            JsxAttrValue::String(s)
          },
          Some(TokenKind::JsxAttrString) => {
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
      self.skip_jsx_ws();
    }
    out
  }

  /// Cursor at `JsxFragmentOpen` (`<>`).
  pub(crate) fn parse_jsx_fragment(&mut self) -> Node {
    let span = self.current_span();
    self.advance();
    let mut children = Vec::new();
    loop {
      match self.peek_kind() {
        Some(TokenKind::JsxFragmentClose) => {
          self.advance();
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
    let children = unwrap_jsx_only_paragraphs(children);
    let children = strip_jsx_layout_whitespace(children);
    Node::JsxFragment(JsxFragment { children, span })
  }

  /// Cursor at `ExpressionStart` (standalone `{expr}`).
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

  /// Skip an MDX comment `{/* ... */}`. Cursor at `MdxCommentOpen`.
  pub(crate) fn skip_md_comment(&mut self) {
    self.advance();
    while let Some(t) = self.peek() {
      match &t.kind {
        TokenKind::MdxCommentClose => {
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

/// Drop indentation / line-break noise between JSX block children, mirroring
/// JSX's own whitespace rules. Without this, `<div>\n  <Card/>\n</div>`
/// emits stray `"  "` / `"\n"` text children that turn into extra flex/grid
/// items. Only runs when every non-blank child is itself an element or block
/// — loose inline content (`<b>hello world</b>`) keeps its whitespace.
fn strip_jsx_layout_whitespace(children: Vec<Node>) -> Vec<Node> {
  let is_flow_child = |n: &Node| {
    is_whitespace_text(n)
      || matches!(
        n,
        Node::JsxElement(_)
          | Node::JsxSelfClosing(_)
          | Node::JsxFragment(_)
          | Node::Paragraph(_)
          | Node::List(_)
          | Node::Blockquote(_)
          | Node::CodeBlock(_)
          | Node::Heading(_)
          | Node::HorizontalRule(_)
          | Node::Table(_)
          | Node::Html(_)
      )
  };
  if !children.iter().all(is_flow_child) {
    return children;
  }
  children
    .into_iter()
    .filter(|n| match n {
      n if is_whitespace_text(n) => false,
      Node::Paragraph(p) => !p.children.iter().all(is_whitespace_text),
      _ => true,
    })
    .collect()
}

/// Unwrap a single-Paragraph child so `<del>*foo*</del>` renders as raw
/// HTML around inline content (CM 6.6), not as `<del><p>...</p></del>`.
/// Also unwraps Paragraphs whose contents are entirely JSX elements +
/// whitespace, splicing the elements into the parent.
fn unwrap_jsx_only_paragraphs(children: Vec<Node>) -> Vec<Node> {
  if children.len() == 1
    && let Some(Node::Paragraph(p)) = children.first()
  {
    return p.children.clone();
  }
  let mut out = Vec::with_capacity(children.len());
  for child in children {
    if let Node::Paragraph(p) = &child {
      let only_jsx = p
        .children
        .iter()
        .filter(|n| !is_whitespace_text(n))
        .all(|n| matches!(n, Node::JsxElement(_) | Node::JsxSelfClosing(_) | Node::JsxFragment(_)));
      let any_jsx =
        p.children.iter().any(|n| matches!(n, Node::JsxElement(_) | Node::JsxSelfClosing(_) | Node::JsxFragment(_)));
      if only_jsx && any_jsx {
        for n in p
          .children
          .iter()
          .filter(|n| matches!(n, Node::JsxElement(_) | Node::JsxSelfClosing(_) | Node::JsxFragment(_)))
        {
          out.push(n.clone());
        }
        continue;
      }
    }
    out.push(child);
  }
  out
}

fn is_whitespace_text(n: &Node) -> bool {
  match n {
    Node::Text(t) => t.value.chars().all(|c| c.is_whitespace()),
    Node::SoftBreak(_) | Node::HardBreak(_) => true,
    _ => false,
  }
}
