use crate::ast::*;
use crate::parser::Parser;
use dmc_diagnostic::Code;
use dmc_lexer::token::{QuoteKind, TokenKind};

impl<'eng, 'tokens> Parser<'eng, 'tokens> {
  /// Lowercase / kebab-case JSX tag names are routed through the
  /// CommonMark raw-HTML path. Keep uppercase / namespaced / member
  /// names on the JSX path for MDX component semantics.
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

  /// CommonMark raw HTML does not use JS-style quote escaping inside
  /// attribute strings. Reject those cases so malformed tags stay text.
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

  /// Reconstruct one JSX-tokenized lowercase HTML tag as either raw HTML
  /// (valid per the lightweight CM checks above) or literal text (for
  /// malformed tags that must escape on output).
  pub(crate) fn parse_inline_raw_html_tag(&mut self) -> Option<Node> {
    let kind = self.peek_kind()?.clone();
    if !matches!(kind, TokenKind::JsxOpenTagStart | TokenKind::JsxCloseTagStart) || !self.is_plain_html_jsx_tag() {
      return None;
    }

    let span = self.current_span();
    let valid = self.jsx_raw_html_tag_is_valid();
    let start_ptr = self.tokens.get(self.pos)?.raw.as_ptr() as usize;
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

    let end_ptr = self.tokens.get(end_idx).map(|t| t.raw.as_ptr() as usize + t.raw.len()).unwrap_or(start_ptr);
    let value = if end_ptr > start_ptr {
      // SAFETY: every Token.raw points into the same source buffer.
      let len = end_ptr - start_ptr;
      let slice = unsafe { std::slice::from_raw_parts(start_ptr as *const u8, len) };
      std::str::from_utf8(slice).map(|s| s.to_string()).unwrap_or_default()
    } else {
      String::new()
    };
    self.pos = end_idx + 1;

    Some(if valid {
      Node::Html(Html { value, span })
    } else {
      Node::Text(Text { value: Self::unescape_markdown(&value), span })
    })
  }

  /// Skip the inter-token whitespace the lexer now keeps for inline
  /// spacing. JSX tag-internal whitespace is structural noise; the parser
  /// drops it so attribute / closing-tag tokens line up the way they did
  /// before whitespace tokens were preserved.
  fn skip_jsx_ws(&mut self) {
    while matches!(self.peek_kind(), Some(TokenKind::Whitespace(_))) {
      self.advance();
    }
  }

  /// Cursor at `JsxOpenTagStart`. Consumes through the matching close (or
  /// self-close) and returns a `JsxElement`, `JsxSelfClosing`, or `JsxFragment`.
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

    let children = unwrap_jsx_only_paragraphs(children);

    if name.is_empty() {
      Node::JsxFragment(JsxFragment { children, span })
    } else {
      Node::JsxElement(JsxElement { name, attrs, children, span })
    }
  }

  /// Consume `name`, `name="str"`, `name={expr}` attributes. Bare names map
  /// to `JsxAttrValue::Boolean`. Stops at the first non-attribute token.
  /// Skips inter-attribute whitespace.
  fn parse_jsx_attrs(&mut self) -> Vec<JsxAttr> {
    let mut out = Vec::new();
    self.skip_jsx_ws();
    loop {
      // Spread attribute `{...rest}` -- lexer wraps the body in
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

  /// JSX fragment `<>...</>`. Cursor at `JsxFragmentOpen`.
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
    Node::JsxFragment(JsxFragment { children, span })
  }

  /// Standalone `{expr}`. Cursor at `ExpressionStart`.
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

  /// Skip a markdown comment `{/* ... */}`. Cursor at `MarkdownCommentStart`.
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

/// Indented block JSX inside a `<Tag>...</Tag>` body looks like
///
///   <TabsList>
///     <TabsTrigger value="cli">CLI</TabsTrigger>
///   </TabsList>
///
/// to the block parser, which sees the leading two-space indent + the
/// JSX opener as inline content and wraps the whole line in a
/// `Paragraph`. That makes the emitted React tree
/// `<TabsList><p>  <TabsTrigger>…</p></TabsList>`, which is wrong both
/// semantically and visually.
///
/// MDX's rule: a JSX element that is the only non-whitespace content
/// on a line is a block child of its enclosing element, *not* a
/// paragraph child. Implement the rule as a post-pass: for each
/// `Paragraph` child, drop pure-whitespace `Text` nodes; if the
/// remainder is one or more JSX nodes only, splice them in as direct
/// children. Otherwise the paragraph stays.
fn unwrap_jsx_only_paragraphs(children: Vec<Node>) -> Vec<Node> {
  // Single-paragraph children unwrap: a JSX element like `<del>*foo*</del>`
  // (the only block child is one Paragraph) renders as raw HTML around
  // inline content per CM 6.6 -- no nested `<p>`. Keeps multi-paragraph
  // JSX bodies intact.
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
