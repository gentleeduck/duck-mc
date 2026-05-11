use crate::ast::*;
use crate::parser::Parser;
use dmc_lexer::token::TokenKind;

impl<'eng, 'tokens> Parser<'eng, 'tokens> {
  /// Build a blockquote tree from the cursor (on a `>`). Walks lines
  /// keeping a stack: each line's marker count sets the live nesting
  /// depth. A line with more markers grows the stack; a line with fewer
  /// markers closes the deeper levels and folds them into their parent.
  pub(super) fn parse_blockquote(&mut self) -> Node {
    let span = self.current_span();
    let para_span = self.current_span();
    let mut children: Vec<Vec<Node>> = vec![Vec::new()];
    let mut paragraphs: Vec<Vec<Node>> = vec![Vec::new()];

    loop {
      let line_markers = self.count_line_blockquote_markers();
      if line_markers == 0 {
        let starts_other_block = self.line_starts_other_block();
        let top = children.len() - 1;
        let lazy_eligible = !paragraphs[top].is_empty()
          && !starts_other_block
          && matches!(
            self.peek_kind(),
            Some(TokenKind::Text)
              | Some(TokenKind::Emphasis(_, _))
              | Some(TokenKind::Strikethrough)
              | Some(TokenKind::CodeInlineOpen(_))
              | Some(TokenKind::Whitespace(_))
              | Some(TokenKind::Autolink(_))
              | Some(TokenKind::EntityRef)
              | Some(TokenKind::LinkOpen)
              | Some(TokenKind::ImageMarker)
              | Some(TokenKind::SetextUnderline(_))
              | Some(TokenKind::ThematicBreak)
          );
        if !lazy_eligible {
          break;
        }
        let inline = self.collect_inline_until_break();
        let break_kind = self.peek_kind().cloned();
        if matches!(break_kind, Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
          self.advance();
        }
        if !inline.is_empty() {
          paragraphs[top].push(Node::Text(Text { value: " ".into(), span: para_span.clone() }));
          paragraphs[top].extend(inline);
        }
        continue;
      }
      while children.len() < line_markers {
        children.push(Vec::new());
        paragraphs.push(Vec::new());
      }
      let deepest_para_open = !paragraphs.last().is_some_and(|p| p.is_empty());
      let shrink_blocked = line_markers < children.len() && deepest_para_open;
      if !shrink_blocked {
        while children.len() > line_markers {
          Self::close_blockquote_level(&mut children, &mut paragraphs, &para_span, &span);
        }
      }

      self.consume_blockquote_markers(line_markers);
      let top = children.len() - 1;
      let after_marker_kind = match self.peek_kind() {
        Some(k) => k.clone(),
        None => break,
      };
      if matches!(after_marker_kind, TokenKind::LinkRefDef) {
        self.advance();
        if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
          self.advance();
        }
        continue;
      }
      if matches!(after_marker_kind, TokenKind::UnorderedListMarker | TokenKind::OrderedListMarker(_)) {
        if !paragraphs[top].is_empty() {
          let para = std::mem::take(&mut paragraphs[top]);
          children[top].push(Node::Paragraph(Paragraph { children: para, span: para_span.clone() }));
        }
        let ordered = matches!(after_marker_kind, TokenKind::OrderedListMarker(_));
        let list = self.parse_list_in_blockquote(ordered, line_markers);
        children[top].push(list);
        continue;
      }
      let indented_code: Option<Node> = if matches!(after_marker_kind, TokenKind::Whitespace(_))
        && self.peek_leading_indent().is_some_and(|n| n >= 4)
      {
        Some(self.parse_indented_code_in_bq())
      } else {
        None
      };
      let block_node: Option<Node> = if indented_code.is_some() {
        indented_code
      } else {
        match after_marker_kind {
          TokenKind::Heading(_) => Some(self.parse_heading()),
          TokenKind::ThematicBreak => {
            let hr_span = self.current_span();
            self.advance();
            Some(Node::HorizontalRule(HorizontalRule { span: hr_span }))
          },
          TokenKind::CodeFenceOpen(_, _) => Some(self.parse_code_block_in_blockquote(line_markers)),
          TokenKind::JsxOpenTagStart | TokenKind::JsxCloseTagStart => {
            self.jsx_html_block_mode().map(|m| self.parse_html_block_from_jsx(m))
          },
          TokenKind::HtmlCommentOpen => Some(self.parse_html_comment_block()),
          _ => None,
        }
      };
      if let Some(node) = block_node {
        if !paragraphs[top].is_empty() {
          let para = std::mem::take(&mut paragraphs[top]);
          children[top].push(Node::Paragraph(Paragraph { children: para, span: para_span.clone() }));
        }
        children[top].push(node);
        if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
          self.advance();
        }
        continue;
      }

      let inline = self.collect_inline_until_break();
      let break_kind = self.peek_kind().cloned();
      if matches!(break_kind, Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
        self.advance();
      }
      if inline.is_empty() {
        if !paragraphs[top].is_empty() {
          let para = std::mem::take(&mut paragraphs[top]);
          children[top].push(Node::Paragraph(Paragraph { children: para, span: para_span.clone() }));
        }
        continue;
      }
      if !paragraphs[top].is_empty() {
        paragraphs[top].push(Node::Text(Text { value: " ".into(), span: para_span.clone() }));
      }
      paragraphs[top].extend(inline);
      if matches!(break_kind, Some(TokenKind::HardBreak)) {
        let para = std::mem::take(&mut paragraphs[top]);
        children[top].push(Node::Paragraph(Paragraph { children: para, span: para_span.clone() }));
      }
    }

    while children.len() > 1 {
      Self::close_blockquote_level(&mut children, &mut paragraphs, &para_span, &span);
    }
    let mut root = children.pop().unwrap();
    let last_para = paragraphs.pop().unwrap();
    if !last_para.is_empty() {
      root.push(Node::Paragraph(Paragraph { children: last_para, span: para_span }));
    }
    Node::Blockquote(Blockquote { children: root, span })
  }

  fn parse_list_in_blockquote(&mut self, ordered: bool, bq_depth: usize) -> Node {
    let span = self.current_span();
    let start: Option<u32> = if ordered {
      self.peek().and_then(|t| {
        let digits: String = t.raw.chars().take_while(|c| c.is_ascii_digit()).collect();
        digits.parse::<u32>().ok()
      })
    } else {
      None
    };
    let mut items: Vec<Node> = Vec::new();
    let mut first = true;

    loop {
      let line_start = self.pos;
      if !first {
        let saved = self.pos;
        let next_count = self.count_line_blockquote_markers();
        if next_count < bq_depth {
          self.pos = saved;
          break;
        }
        self.consume_blockquote_markers(bq_depth);
      }
      first = false;

      let kind = match self.peek_kind() {
        Some(k) => k,
        None => break,
      };
      let want_marker = if ordered {
        matches!(kind, TokenKind::OrderedListMarker(_))
      } else {
        matches!(kind, TokenKind::UnorderedListMarker)
      };
      if !want_marker {
        if !first {
          self.pos = line_start;
        }
        break;
      }
      let marker_raw = self.peek_raw().unwrap_or("");
      let marker_raw_width = marker_raw.chars().count();
      let marker_raw_has_ws = marker_raw.ends_with([' ', '\t']);
      let following_ws = match self.tokens.get(self.pos + 1) {
        Some(t) if matches!(t.kind, TokenKind::Whitespace(_)) => t.raw.chars().count(),
        _ => 0,
      };
      let marker_width = if !marker_raw_has_ws && following_ws > 0 { marker_raw_width + 1 } else { marker_raw_width };
      let extra_ws = match self.tokens.get(self.pos + 1) {
        Some(t) if matches!(t.kind, TokenKind::Whitespace(_)) => {
          let ws = t.raw.chars().count();
          if !marker_raw_has_ws && marker_raw_width > 0 { ws.saturating_sub(1) } else { ws }
        },
        _ => 0,
      };
      let item_content_extra = if extra_ws >= 4 { 0 } else { extra_ws };
      let content_floor = (if ordered { marker_width.max(3) } else { marker_width.max(2) }) + item_content_extra;
      self.advance();

      if ordered {
        let is_text = matches!(self.peek_kind(), Some(TokenKind::Text));
        let raw_opt: Option<&'tokens str> = if is_text { self.peek_raw() } else { None };
        if let Some(raw) = raw_opt {
          let trimmed = raw.strip_prefix('.').unwrap_or(raw).trim_start_matches([' ', '\t']);
          if trimmed.is_empty() {
            self.advance();
          } else if trimmed.len() != raw.len() {
            let pos = self.pos;
            self.tokens[pos].raw = trimmed;
          }
        }
      }

      let mut item = self.parse_one_list_item(ordered);
      if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
        self.advance();
      }

      loop {
        let saved = self.pos;
        let line_markers = self.count_line_blockquote_markers();
        if line_markers < bq_depth {
          break;
        }
        self.consume_blockquote_markers(bq_depth);
        if !matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
          self.pos = saved;
          break;
        }
        self.advance();

        let next_saved = self.pos;
        let next_markers = self.count_line_blockquote_markers();
        if next_markers < bq_depth {
          self.pos = saved;
          break;
        }
        self.consume_blockquote_markers(bq_depth);
        let indent = self.peek_leading_indent().unwrap_or(0);
        if indent < content_floor || indent >= content_floor + 4 {
          self.pos = saved;
          break;
        }
        self.advance();
        let para_span = self.current_span();
        let inline = self.collect_inline_until_break();
        if inline.is_empty() {
          self.pos = next_saved;
          break;
        }
        Self::ensure_loose_item(&mut item, &span);
        Self::append_to_item(&mut item, Node::Paragraph(Paragraph { children: inline, span: para_span }));
        if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
          self.advance();
        }
      }
      items.push(item);
    }

    Node::List(List { ordered, start, children: items, span })
  }

  fn close_blockquote_level(
    children: &mut Vec<Vec<Node>>,
    paragraphs: &mut Vec<Vec<Node>>,
    para_span: &duck_diagnostic::Span,
    bq_span: &duck_diagnostic::Span,
  ) {
    let mut inner_children = children.pop().unwrap();
    let pending = paragraphs.pop().unwrap();
    if !pending.is_empty() {
      inner_children.push(Node::Paragraph(Paragraph { children: pending, span: para_span.clone() }));
    }
    let bq = Node::Blockquote(Blockquote { children: inner_children, span: bq_span.clone() });
    let parent_idx = children.len() - 1;
    children[parent_idx].push(bq);
  }

  pub(super) fn count_line_blockquote_markers(&self) -> usize {
    let mut count = 0usize;
    let mut i = self.pos;
    while let Some(t) = self.tokens.get(i) {
      match t.kind {
        TokenKind::BlockQuoteMarker => {
          count += 1;
          i += 1;
        },
        TokenKind::Whitespace(_) => {
          i += 1;
        },
        _ => break,
      }
    }
    count
  }

  pub(super) fn consume_blockquote_markers(&mut self, n: usize) {
    let mut taken = 0usize;
    while taken < n {
      if matches!(self.peek_kind(), Some(TokenKind::Whitespace(_))) {
        self.advance();
      }
      if !matches!(self.peek_kind(), Some(TokenKind::BlockQuoteMarker)) {
        break;
      }
      self.advance();
      taken += 1;
    }
    if self.peek_leading_indent().is_some_and(|n| n < 4) {
      self.advance();
    }
  }

  pub(super) fn line_starts_other_block(&self) -> bool {
    let mut i = self.pos;
    if matches!(self.tokens.get(i).map(|t| &t.kind), Some(TokenKind::Whitespace(_))) {
      i += 1;
    }
    matches!(
      self.tokens.get(i).map(|t| &t.kind),
      Some(TokenKind::Heading(_))
        | Some(TokenKind::ThematicBreak)
        | Some(TokenKind::BlockQuoteMarker)
        | Some(TokenKind::UnorderedListMarker)
        | Some(TokenKind::OrderedListMarker(_))
        | Some(TokenKind::IndentedCodeLine)
        | Some(TokenKind::CodeFenceOpen(_, _))
        | Some(TokenKind::HtmlCommentOpen)
        | Some(TokenKind::HtmlBlockOpen(_))
        | Some(TokenKind::JsxOpenTagStart)
        | Some(TokenKind::JsxCloseTagStart)
        | Some(TokenKind::LinkRefDef)
    )
  }

  pub(super) fn try_promote_text_list_marker(&mut self) -> bool {
    let Some(tok) = self.peek() else {
      return false;
    };
    if !matches!(tok.kind, TokenKind::Text) {
      return false;
    }
    let has_space_after =
      matches!(self.tokens.get(self.pos + 1).map(|t| &t.kind), Some(TokenKind::Whitespace(w)) if *w > 0);
    if !has_space_after {
      return false;
    }
    let raw = tok.raw;
    let kind = match raw {
      "-" | "*" | "+" => Some(TokenKind::UnorderedListMarker),
      _ => {
        let digits_end = raw.find(|c: char| !c.is_ascii_digit()).unwrap_or(raw.len());
        if digits_end == 0 || digits_end + 1 != raw.len() {
          None
        } else {
          match raw.as_bytes()[digits_end] {
            b'.' => Some(TokenKind::OrderedListMarker(dmc_lexer::token::OrderedSep::Period)),
            b')' => Some(TokenKind::OrderedListMarker(dmc_lexer::token::OrderedSep::Paren)),
            _ => None,
          }
        }
      },
    };
    if let Some(kind) = kind {
      self.tokens[self.pos].kind = kind;
      true
    } else {
      false
    }
  }

  pub(super) fn try_continue_same_list_after_blankline(
    &mut self,
    ordered: bool,
    indent: usize,
    items: &mut Vec<Node>,
    span: &duck_diagnostic::Span,
    saw_blank_between_items: &mut bool,
  ) -> bool {
    if !matches!(self.peek_kind(), Some(TokenKind::BlankLine)) {
      return false;
    }
    let blank_pos = self.pos;
    self.advance();
    let ws_pos = match self.peek_kind() {
      Some(TokenKind::Whitespace(_)) => Some(self.pos),
      _ => None,
    };
    if ws_pos.is_some() {
      self.advance();
    }
    let next_is_marker = match self.peek_kind() {
      Some(TokenKind::UnorderedListMarker) if !ordered => true,
      Some(TokenKind::OrderedListMarker(_)) if ordered => true,
      _ => false,
    };
    let next_indent =
      if let Some(pos) = ws_pos { self.tokens.get(pos).map(|t| t.raw.chars().count()).unwrap_or(0) } else { 0 };
    let same_list_indent = if indent > 0 { next_indent == indent } else { next_indent <= 3 };
    if !next_is_marker || !same_list_indent {
      self.pos = blank_pos;
      return false;
    }
    if let Some(pos) = ws_pos {
      self.pos = pos;
    }
    *saw_blank_between_items = true;
    if let Some(last) = items.last_mut() {
      Self::ensure_loose_item(last, span);
    }
    true
  }
}
