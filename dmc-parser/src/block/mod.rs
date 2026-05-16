use crate::ast::*;
use crate::parser::Parser;
use dmc_diagnostic::Code;
use dmc_lexer::token::TokenKind;

mod blockquote;
mod code;
mod heading;
mod html;
mod list;

/// CM 4.6 type-1: closes on matching `</tag>`. Forces a raw HTML block even
/// if the lexer routed the opener through JSX.
const HTML_BLOCK_TYPE1_TAGS: &[&str] = &["script", "pre", "style", "textarea"];

/// CM 4.6 type-6: closes on next blank line. Type-7 is conditionally
/// handled at the call site - MDX treats capital / namespaced tags as
/// components, not type-7 HTML.
const HTML_BLOCK_TYPE6_TAGS: &[&str] = &[
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

enum HtmlBlockMode {
  /// Closes on matching `</tag>`. Carries lowercased tag name.
  Type1(String),
  /// Closes on blank line.
  Type6,
  /// Type-7: any other tag at col 0. Closes on blank line. Skipped for MDX
  /// capital / namespaced tags so `<MyComponent>` stays JSX.
  Type7,
}

impl<'eng, 'tokens> Parser<'eng, 'tokens> {
  /// Top-down block dispatch. `None` means the cursor advanced but emitted
  /// no node (a stray break, a markdown comment, ...).
  pub(crate) fn parse_block(&mut self) -> Option<Node> {
    // Lexer pre-classifies col-0 indented code as `Whitespace + IndentedCodeLine`.
    let is_indented = matches!(self.peek_kind(), Some(TokenKind::Whitespace(_)))
      && matches!(self.tokens.get(self.pos + 1).map(|t| &t.kind), Some(TokenKind::IndentedCodeLine));

    if is_indented {
      return Some(self.parse_indented_code());
    }

    // CM 4.8: `Whitespace + break` at col 0 is a blank-with-whitespace line.
    if matches!(self.peek_kind(), Some(TokenKind::Whitespace(_)))
      && self.peek().is_some_and(|t| t.span.column == 1)
      && matches!(
        self.tokens.get(self.pos + 1).map(|t| &t.kind),
        Some(TokenKind::SoftBreak)
          | Some(TokenKind::HardBreak)
          | Some(TokenKind::BlankLine)
          | Some(TokenKind::Eof)
          | None
      )
    {
      self.advance();
      self.advance();
      return None;
    }

    // Fallback indented code: lexer didn't classify (prev token was inline
    // content, eg a bq paragraph) but at top-level dispatch we know we're
    // between blocks. `Whitespace(>=4)` + non-block content is indented code.
    if matches!(self.peek_kind(), Some(TokenKind::Whitespace(w)) if (*w as usize) >= 4)
      && self.peek().is_some_and(|t| t.span.column == 1)
    {
      let next_kind = self.tokens.get(self.pos + 1).map(|t| &t.kind);
      let next_is_inline = matches!(
        next_kind,
        Some(TokenKind::Text)
          | Some(TokenKind::Emphasis(_, _))
          | Some(TokenKind::Strikethrough)
          | Some(TokenKind::CodeInlineOpen(_))
          | Some(TokenKind::Whitespace(_))
          | Some(TokenKind::Autolink(_))
          | Some(TokenKind::EntityRef)
          | Some(TokenKind::LinkOpen)
          | Some(TokenKind::ImageMarker)
          | Some(TokenKind::JsxOpenTagStart)
          | Some(TokenKind::HtmlBlockOpen(_))
          | Some(TokenKind::HtmlCommentOpen)
          | Some(TokenKind::Heading(_))
          | Some(TokenKind::ThematicBreak)
          | Some(TokenKind::CodeFenceOpen(_, _))
          | Some(TokenKind::UnorderedListMarker)
          | Some(TokenKind::OrderedListMarker(_))
          | Some(TokenKind::BlockQuoteMarker)
      );
      // CM 4.4 indented code does NOT apply to JSX children: nested-child
      // lines route through `parse_jsx` with a leading `Whitespace(>=4)`
      // and the fallback would otherwise sweep them into one code block,
      // dropping inner JSX from the AST. Real indented code inside a JSX
      // body still works via fenced fences or the lexer-classified
      // `Whitespace + IndentedCodeLine` pair handled above.
      if next_is_inline && self.jsx_open_stack.is_empty() {
        return Some(self.parse_indented_code_fallback());
      }
    }

    // MDX: a lowercase JSX tag with JSX-syntax attrs (`className`, an
    // expression value, `{...spread}`) or sitting inside another JSX element
    // routes through `parse_jsx` instead of becoming a raw-HTML blob. Keeps
    // `<div className="grid"><LinkedCard>...` as nested `jsx(...)` calls
    // instead of one verbatim `<div>` string.
    {
      let has_ws = matches!(self.peek_kind(), Some(TokenKind::Whitespace(_)));
      let tag_at = if has_ws { self.pos + 1 } else { self.pos };
      if matches!(self.tokens.get(tag_at).map(|t| &t.kind), Some(TokenKind::JsxOpenTagStart)) {
        let saved = self.pos;
        if has_ws {
          self.advance();
        }
        if self.lowercase_jsx_tag_is_mdx_element() {
          return Some(self.parse_jsx());
        }
        self.pos = saved;
      }
    }

    // CM allows up to 3 leading spaces before any block marker.
    if matches!(self.peek_kind(), Some(TokenKind::Whitespace(w)) if (*w as usize) <= 3)
      && let Some(next) = self.tokens.get(self.pos + 1)
    {
      match next.kind {
        TokenKind::UnorderedListMarker => {
          self.advance();
          return Some(self.parse_list(false, 0));
        },
        TokenKind::OrderedListMarker(_) => {
          self.advance();
          return Some(self.parse_list(true, 0));
        },
        TokenKind::ThematicBreak => {
          self.advance();
          let span = self.current_span();
          self.advance();
          return Some(Node::HorizontalRule(HorizontalRule { span }));
        },
        TokenKind::Heading(_) => {
          self.advance();
          return Some(self.parse_heading());
        },
        TokenKind::CodeFenceOpen(_, _) => {
          self.advance();
          return Some(self.parse_code_block());
        },
        TokenKind::BlockQuoteMarker => {
          self.advance();
          return Some(self.parse_blockquote());
        },
        TokenKind::HtmlCommentOpen => {
          self.advance();
          return Some(self.parse_html_comment_block());
        },
        TokenKind::JsxOpenTagStart | TokenKind::JsxCloseTagStart => {
          // Peek past the leading whitespace to check Type-1 / Type-6 raw
          // HTML. Don't consume the Whitespace - CM 4.6 keeps the indent
          // in the rendered output.
          let saved = self.pos;
          self.pos += 1;
          if let Some(mode) = self.jsx_html_block_mode() {
            self.pos = saved;
            return Some(self.parse_html_block_from_jsx(mode));
          }
          self.pos = saved;
        },
        TokenKind::LinkRefDef => {
          let valid_ref_def = crate::refs::parse_link_ref_def(next.raw).is_some();
          self.advance();
          if valid_ref_def {
            self.advance();
            return None;
          }
        },
        TokenKind::BlankLine | TokenKind::SoftBreak | TokenKind::HardBreak => {
          self.advance();
          self.advance();
          return None;
        },
        TokenKind::Text | TokenKind::ImageMarker | TokenKind::LinkOpen | TokenKind::Emphasis(_, _) => {
          self.advance();
        },
        _ => {},
      }
    }

    match self.peek_kind()? {
      TokenKind::FrontmatterStart(_) => Some(self.parse_frontmatter()),
      TokenKind::Import => Some(self.import_node()),
      TokenKind::Export => Some(self.export_node()),
      TokenKind::Heading(_) => Some(self.parse_heading()),
      TokenKind::CodeFenceOpen(_, _) => Some(self.parse_code_block()),
      TokenKind::JsxOpenTagStart => {
        if let Some(mode) = self.jsx_html_block_mode() {
          Some(self.parse_html_block_from_jsx(mode))
        } else if self.is_lowercase_jsx_tag() {
          // Lowercase HTML-ish tag with no block type -- inline raw HTML.
          Some(self.parse_paragraph())
        } else {
          Some(self.parse_jsx())
        }
      },
      TokenKind::JsxCloseTagStart => {
        if let Some(mode) = self.jsx_html_block_mode() {
          Some(self.parse_html_block_from_jsx(mode))
        } else if self.is_plain_html_jsx_tag() {
          Some(self.parse_plain_html_close_paragraph())
        } else {
          self.advance();
          None
        }
      },
      TokenKind::JsxFragmentOpen => Some(self.parse_jsx_fragment()),
      TokenKind::HtmlBlockOpen(_) => Some(self.parse_html_block()),
      TokenKind::HtmlCommentOpen if self.peek().is_some_and(|t| t.span.column == 1) => {
        Some(self.parse_html_comment_block())
      },
      TokenKind::ExpressionStart => Some(self.parse_jsx_expression()),
      TokenKind::MdxCommentOpen => {
        self.skip_md_comment();
        None
      },
      TokenKind::UnorderedListMarker => Some(self.parse_list(false, 0)),
      TokenKind::OrderedListMarker(_) => Some(self.parse_list(true, 0)),
      TokenKind::BlockQuoteMarker => Some(self.parse_blockquote()),
      TokenKind::ThematicBreak => {
        let span = self.current_span();
        self.advance();
        Some(Node::HorizontalRule(HorizontalRule { span }))
      },
      TokenKind::HardBreak | TokenKind::SoftBreak | TokenKind::BlankLine => {
        self.advance();
        None
      },
      // Ref-defs were harvested in the pre-pass; emit nothing here.
      TokenKind::LinkRefDef => {
        let span = self.current_span();
        let raw = self.peek_raw().unwrap_or_default().to_string();
        self.advance();
        if crate::refs::parse_link_ref_def(&raw).is_some() {
          None
        } else {
          Some(Node::Paragraph(Paragraph { children: vec![Node::Text(Text { value: raw, span: span.clone() })], span }))
        }
      },
      TokenKind::FootnoteDefMarker => Some(self.parse_footnote_def()),
      _ => {
        if let Some(n) = self.try_parse_table() {
          return Some(n);
        }
        Some(self.parse_paragraph())
      },
    }
  }

  /// Consume `FrontmatterStart .. FrontmatterEnd`. Body is left raw.
  fn parse_frontmatter(&mut self) -> Node {
    let span = self.current_span();
    self.advance();
    let raw = match self.peek() {
      Some(t) if matches!(t.kind, TokenKind::FrontmatterContent) => {
        let raw = t.raw.to_string();
        self.advance();
        raw
      },
      _ => String::new(),
    };
    if matches!(self.peek_kind(), Some(TokenKind::FrontmatterEnd(_))) {
      self.advance();
    }
    Node::Frontmatter(Frontmatter { raw, span })
  }

  /// Visual width of the leading `Whitespace` token. CM 2.2: tabs snap to
  /// the next 4-col stop, computed from the token's column so a post-marker
  /// tab (eg `> \t`) lands on the right multiple.
  fn peek_leading_indent(&self) -> Option<usize> {
    match self.peek() {
      Some(t) if matches!(t.kind, TokenKind::Whitespace(_)) => {
        let mut col: usize = t.span.column.saturating_sub(1);
        let start_col = col;
        for c in t.raw.chars() {
          if c == '\t' {
            col += 4 - (col % 4);
          } else {
            col += 1;
          }
        }
        Some(col - start_col)
      },
      _ => None,
    }
  }

  fn is_block_node(node: &Node) -> bool {
    matches!(
      node,
      Node::Paragraph(_)
        | Node::List(_)
        | Node::Blockquote(_)
        | Node::CodeBlock(_)
        | Node::Heading(_)
        | Node::HorizontalRule(_)
        | Node::Table(_)
        | Node::Html(_)
    )
  }

  /// `>` after a list marker can be lexed as plain text; rewrite it back to
  /// `BlockQuoteMarker` so the list item can start with a blockquote.
  fn try_promote_text_blockquote_marker(&mut self) -> bool {
    let Some(tok) = self.peek() else {
      return false;
    };
    if !matches!(tok.kind, TokenKind::Text) || tok.raw != ">" {
      return false;
    }
    let has_space_after =
      matches!(self.tokens.get(self.pos + 1).map(|t| &t.kind), Some(TokenKind::Whitespace(w)) if *w > 0);
    if !has_space_after {
      return false;
    }
    self.tokens[self.pos].kind = TokenKind::BlockQuoteMarker;
    true
  }

  /// Append `child` to the children of a `ListItem` / `TaskListItem`.
  fn append_to_item(item: &mut Node, child: Node) {
    match item {
      Node::ListItem(li) => li.children.push(child),
      Node::TaskListItem(t) => t.children.push(child),
      _ => {},
    }
  }

  /// Continuation without a blank line stays inside the current paragraph;
  /// does NOT immediately promote the item to loose.
  fn append_inline_continuation(item: &mut Node, inline: &mut Vec<Node>, span: &duck_diagnostic::Span) -> bool {
    let kids = match item {
      Node::ListItem(li) => &mut li.children,
      Node::TaskListItem(t) => &mut t.children,
      _ => return false,
    };
    if let Some(last_block) = kids.iter().rposition(Self::is_block_node) {
      if !matches!(kids.get(last_block), Some(Node::Heading(_))) {
        return false;
      }
      if kids[last_block + 1..].iter().any(Self::is_block_node) {
        return false;
      }
      if last_block + 1 < kids.len() {
        kids.push(Node::SoftBreak(BreakNode { span: span.clone() }));
      }
      kids.append(inline);
      return true;
    }
    if !kids.is_empty() {
      kids.push(Node::SoftBreak(BreakNode { span: span.clone() }));
    }
    kids.append(inline);
    true
  }

  /// Wrap the leading inline run of `item` in a `Paragraph` so loose-list
  /// formatting (`<li><p>...</p><p>...</p></li>`) kicks in. Only contiguous
  /// inline-typed children at the front are promoted; existing block
  /// children stay put - otherwise `[Text, List]` would collapse into one
  /// paragraph that swallows the inner list.
  fn ensure_loose_item(item: &mut Node, span: &duck_diagnostic::Span) {
    let promote = |kids: &mut Vec<Node>, span: &duck_diagnostic::Span| {
      if kids.first().is_some_and(|n| matches!(n, Node::Paragraph(_))) {
        return;
      }
      let split = kids.iter().position(Self::is_block_node).unwrap_or(kids.len());
      if split == 0 {
        return;
      }
      let trailing: Vec<Node> = kids.drain(split..).collect();
      let leading: Vec<Node> = std::mem::take(kids);
      kids.push(Node::Paragraph(Paragraph { children: leading, span: span.clone() }));
      kids.extend(trailing);
    };
    match item {
      Node::ListItem(li) => promote(&mut li.children, span),
      Node::TaskListItem(t) => promote(&mut t.children, span),
      _ => {},
    }
  }

  /// Convert an inline-only list item into a setext heading. Only the
  /// leading inline run is folded; tight-list trailing paragraph text stays
  /// as raw siblings.
  fn promote_item_to_setext_heading(item: &mut Node, level: u8, span: &duck_diagnostic::Span) -> bool {
    let kids = match item {
      Node::ListItem(li) => &mut li.children,
      Node::TaskListItem(t) => &mut t.children,
      _ => return false,
    };
    if kids.is_empty() || kids.iter().any(Self::is_block_node) {
      return false;
    }
    while let Some(Node::Text(t)) = kids.last_mut() {
      let trimmed = t.value.trim_end_matches([' ', '\t']).to_string();
      if trimmed.is_empty() {
        kids.pop();
      } else if trimmed.len() != t.value.len() {
        t.value = trimmed;
        break;
      } else {
        break;
      }
    }
    if kids.is_empty() {
      return false;
    }
    let heading_children = std::mem::take(kids);
    kids.push(Node::Heading(Heading { level, children: heading_children, span: span.clone(), id: None }));
    true
  }

  fn import_node(&mut self) -> Node {
    let span = self.current_span();
    let raw = self.peek().map(|t| t.raw.to_string()).unwrap_or_default();
    self.advance();
    Node::Import(Import { raw, span })
  }

  fn export_node(&mut self) -> Node {
    let span = self.current_span();
    let raw = self.peek().map(|t| t.raw.to_string()).unwrap_or_default();
    self.advance();
    Node::Export(Export { raw, span })
  }

  /// GFM footnote definition. Marker raw is `[^id]: ` (trailing space
  /// included by the lexer); body is the inline run up to the next break.
  fn parse_footnote_def(&mut self) -> Node {
    let span = self.current_span();
    let raw = self.peek().map(|t| t.raw.to_string()).unwrap_or_default();
    self.advance();
    let id = raw.trim_start_matches('[').trim_start_matches('^').split(']').next().unwrap_or("").to_string();
    let children = self.collect_inline_until_break();
    Node::FootnoteDef(FootnoteDef { id, children, span })
  }

  /// Default fallback. Also folds a trailing `=`/`-` underline into a
  /// setext heading. Soft breaks stay inside the paragraph; only a blank
  /// line or block-starter closes it.
  fn parse_paragraph(&mut self) -> Node {
    let span = self.current_span();
    let mut children: Vec<Node> = Vec::new();
    let mut delims: Vec<crate::inline::DelimRecord> = Vec::new();
    let para_stop = |k: &TokenKind| {
      matches!(
        k,
        TokenKind::BlankLine
          | TokenKind::SoftBreak
          | TokenKind::Eof
          | TokenKind::Heading(_)
          | TokenKind::FrontmatterStart(_)
          | TokenKind::Import
          | TokenKind::Export
      )
    };
    self.maybe_diag_unterminated_text_jsx();
    self.collect_inline_into(&para_stop, &mut children, &mut delims);
    // Setext H2 retro-fold: HardBreak + all-`-` Text -> `<h2>`. CM 4.3
    // allows trailing whitespace before the underline (captured in the
    // hard break).
    if children.len() >= 2
      && let Some(Node::Text(t)) = children.last()
      && !t.value.is_empty()
      && t.value.chars().all(|c| c == '-')
      && matches!(children.get(children.len() - 2), Some(Node::HardBreak(_)))
    {
      children.pop();
      children.pop();
      while matches!(children.last(), Some(Node::HardBreak(_)) | Some(Node::Text(_))) {
        if let Some(Node::Text(t)) = children.last()
          && t.value.chars().any(|c| !c.is_whitespace())
        {
          break;
        }
        children.pop();
      }
      if !delims.is_empty() {
        crate::inline::resolve_emphasis_delims(&mut children, &mut delims);
        if self.options.legacy_gfm_emphasis {
          crate::inline::normalize_legacy_gfm_emphasis(&mut children);
        }
      }
      return Node::Heading(Heading { level: 2, children, span, id: None });
    }
    loop {
      if !matches!(self.peek_kind(), Some(TokenKind::SoftBreak)) {
        break;
      }
      let saved = self.pos;
      self.advance();
      // Setext: `=`/`-` underline after the soft break (1-3 leading spaces ok).
      let ws_skip = matches!(self.peek_kind(), Some(TokenKind::Whitespace(w)) if (*w as usize) <= 3);
      if ws_skip {
        self.advance();
      }
      if let Some(lvl) = self.setext_underline_level() {
        self.eat_setext_underline();
        while matches!(children.last(), Some(Node::HardBreak(_))) {
          children.pop();
        }
        if !delims.is_empty() {
          crate::inline::resolve_emphasis_delims(&mut children, &mut delims);
          if self.options.legacy_gfm_emphasis {
            crate::inline::normalize_legacy_gfm_emphasis(&mut children);
          }
        }
        while let Some(Node::Text(t)) = children.last_mut() {
          let trimmed = t.value.trim_end_matches([' ', '\t']).to_string();
          if trimmed.is_empty() {
            children.pop();
          } else if trimmed.len() != t.value.len() {
            t.value = trimmed;
            break;
          } else {
            break;
          }
        }
        return Node::Heading(Heading { level: lvl, children, span, id: None });
      }
      if ws_skip {
        // Restore so the lazy-continuation path sees the whitespace.
        self.pos -= 1;
      }
      if matches!(
        self.peek_kind(),
        Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak) | Some(TokenKind::Eof) | None
      ) {
        self.pos = saved;
        break;
      }
      // CM 5.2: an ordered list with start != 1 cannot interrupt a paragraph.
      let next_is_ol_interrupting = match self.peek() {
        Some(t) if matches!(t.kind, TokenKind::OrderedListMarker(_)) => {
          let digits: String = t.raw.chars().take_while(|c| c.is_ascii_digit()).collect();
          let starts_at_one = digits.parse::<u32>().map(|n| n == 1).unwrap_or(false);
          // CM 5.2: empty-content list item cannot interrupt a paragraph.
          let has_content = !matches!(
            self.tokens.get(self.pos + 1).map(|t| &t.kind),
            Some(TokenKind::SoftBreak)
              | Some(TokenKind::HardBreak)
              | Some(TokenKind::BlankLine)
              | Some(TokenKind::Eof)
              | None
          );
          starts_at_one && has_content
        },
        _ => false,
      };
      let next_is_ul_interrupting = match self.peek_kind() {
        Some(TokenKind::UnorderedListMarker) => !matches!(
          self.tokens.get(self.pos + 1).map(|t| &t.kind),
          Some(TokenKind::SoftBreak)
            | Some(TokenKind::HardBreak)
            | Some(TokenKind::BlankLine)
            | Some(TokenKind::Eof)
            | None
        ),
        _ => false,
      };
      // CM 4.6: only HTML block types 1-6 interrupt a paragraph; type-7
      // stays inline. MDX components always start a JSX block.
      let next_is_jsx_block = match self.peek_kind() {
        Some(TokenKind::JsxOpenTagStart) => {
          if let Some(name_tok) = self.tokens.get(self.pos + 1)
            && matches!(name_tok.kind, TokenKind::JsxTagName)
          {
            let name = name_tok.raw;
            let lower = name.to_ascii_lowercase();
            if !self.is_plain_html_jsx_tag() {
              true
            } else {
              HTML_BLOCK_TYPE1_TAGS.contains(&lower.as_str()) || HTML_BLOCK_TYPE6_TAGS.contains(&lower.as_str())
            }
          } else {
            true
          }
        },
        Some(TokenKind::JsxCloseTagStart) => {
          if let Some(name_tok) = self.tokens.get(self.pos + 1)
            && matches!(name_tok.kind, TokenKind::JsxTagName)
          {
            let lower = name_tok.raw.to_ascii_lowercase();
            self.is_plain_html_jsx_tag() && HTML_BLOCK_TYPE6_TAGS.contains(&lower.as_str())
          } else {
            false
          }
        },
        _ => false,
      };
      let next_is_block = matches!(
        self.peek_kind(),
        Some(TokenKind::Heading(_))
          | Some(TokenKind::BlockQuoteMarker)
          | Some(TokenKind::CodeFenceOpen(_, _))
          | Some(TokenKind::ThematicBreak)
          | Some(TokenKind::FrontmatterStart(_))
          | Some(TokenKind::Import)
          | Some(TokenKind::Export)
      ) || next_is_ol_interrupting
        || next_is_ul_interrupting
        || next_is_jsx_block;
      if next_is_block {
        self.pos = saved;
        break;
      }
      // Same-paragraph continuation: keep the soft break as a literal
      // newline and collect the next line.
      let break_span = self.current_span();
      children.push(Node::SoftBreak(BreakNode { span: break_span }));
      let pre_len = children.len();
      self.maybe_diag_unterminated_text_jsx();
      self.collect_inline_into(&para_stop, &mut children, &mut delims);
      if children.len() == pre_len {
        // Nothing useful followed; rewind so the eaten soft break can act
        // as a separate empty-line marker.
        self.pos = saved;
        children.pop();
        break;
      }
    }
    if !delims.is_empty() {
      crate::inline::resolve_emphasis_delims(&mut children, &mut delims);
      if self.options.legacy_gfm_emphasis {
        crate::inline::normalize_legacy_gfm_emphasis(&mut children);
      }
    }
    Self::finalize_inline_breaks(&mut children);
    Node::Paragraph(Paragraph { children, span })
  }

  /// Surface one actionable diagnostic for `<Foo bar=` -shaped lines that
  /// failed JSX lex and would otherwise silently render as text.
  fn maybe_diag_unterminated_text_jsx(&mut self) {
    let Some(first) = self.peek() else {
      return;
    };
    if !matches!(first.kind, TokenKind::Text) {
      return;
    }
    let Some(tag_name) = first.raw.strip_prefix('<') else {
      return;
    };
    if tag_name.is_empty() || !tag_name.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
      return;
    }
    if !tag_name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':')) {
      return;
    }

    let start = self.pos;
    let mut i = start;
    let mut saw_close = first.raw.contains('>');
    let mut missing_attr: Option<(usize, String)> = None;

    while let Some(tok) = self.tokens.get(i) {
      match tok.kind {
        TokenKind::SoftBreak | TokenKind::HardBreak | TokenKind::BlankLine | TokenKind::Eof => break,
        _ => {},
      }
      if tok.raw.contains('>') {
        saw_close = true;
        break;
      }
      if matches!(tok.kind, TokenKind::Text)
        && let Some(attr) = tok.raw.strip_suffix('=')
        && !attr.is_empty()
        && attr.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | ':'))
      {
        missing_attr = Some((i, attr.to_string()));
      }
      i += 1;
    }

    if saw_close {
      return;
    }

    if let Some((attr_pos, attr)) = missing_attr {
      let diagnostic = duck_diagnostic::diag!(
        Code::MissingJsxAttributeValue,
        self.span_at(attr_pos),
        format!("JSX attribute `{attr}` is missing a value before the tag ended; preserving the text literally")
      )
      .with_help("add a quoted string, `{expression}`, or remove the trailing `=`");
      self.emit_diagnostic(diagnostic);
      return;
    }

    let diagnostic = duck_diagnostic::diag!(
      Code::UnterminatedJsxOpenTag,
      self.span_at(start),
      format!("JSX open tag `<{tag_name}>` never reached a closing `>`; preserving the text literally")
    )
    .with_help("close the tag with `>` or `/>`, or escape the leading `<` if this should stay text");
    self.emit_diagnostic(diagnostic);
  }

  /// CM 6.7: a hard break needs content after it. Strip `\` from prev Text
  /// for mid-paragraph breaks; drop a trailing `HardBreak` so `foo\`
  /// renders literal.
  fn finalize_inline_breaks(children: &mut Vec<Node>) {
    for i in 0..children.len() {
      if !matches!(children.get(i), Some(Node::HardBreak(_))) {
        continue;
      }
      let is_last = i + 1 == children.len();
      if is_last {
        continue;
      }
      if let Some(Node::Text(t)) = children.get_mut(i.saturating_sub(1))
        && t.value.ends_with('\\')
      {
        t.value.pop();
      }
    }
    while let Some(Node::HardBreak(_)) = children.last() {
      children.pop();
      while let Some(Node::Text(t)) = children.last() {
        let trimmed = t.value.trim_end_matches([' ', '\t']);
        if trimmed.is_empty() {
          children.pop();
          continue;
        }
        if trimmed.len() != t.value.len() {
          let len = trimmed.len();
          if let Some(Node::Text(t)) = children.last_mut() {
            t.value.truncate(len);
          }
        }
        break;
      }
    }
    // CM 4.8: trailing whitespace-only Text + SoftBreak runs render as
    // nothing.
    loop {
      match children.last() {
        Some(Node::Text(t)) if t.value.chars().all(|c| c == ' ' || c == '\t') => {
          children.pop();
        },
        Some(Node::SoftBreak(_)) => {
          children.pop();
        },
        _ => break,
      }
    }
  }
}
