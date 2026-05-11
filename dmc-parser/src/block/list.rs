use crate::ast::*;
use crate::parser::Parser;
use dmc_lexer::token::TokenKind;

impl<'eng, 'tokens> Parser<'eng, 'tokens> {
  /// Collect consecutive list items of one flavor (ordered or unordered) into
  /// a `List` node. `indent` is the column of the marker on its line; nested
  /// lists pass a larger `indent` so deeper sub-items keep recursing.
  pub(super) fn parse_list(&mut self, ordered: bool, indent: usize) -> Node {
    let span = self.current_span();
    let mut items: Vec<Node> = Vec::new();
    let start: Option<u32> = if ordered {
      self.peek().and_then(|t| {
        let digits: String = t.raw.chars().take_while(|c| c.is_ascii_digit()).collect();
        digits.parse::<u32>().ok()
      })
    } else {
      None
    };
    let bullet_char: Option<char> = if !ordered { self.peek().and_then(|t| t.raw.chars().next()) } else { None };
    let ordered_sep: Option<dmc_lexer::token::OrderedSep> = if ordered {
      self.peek().and_then(|t| match t.kind {
        TokenKind::OrderedListMarker(s) => Some(s),
        _ => None,
      })
    } else {
      None
    };

    let mut first = true;
    let mut saw_blank_between_items = false;
    loop {
      let mut item_indent = if first {
        match self.pos.checked_sub(1).and_then(|i| self.tokens.get(i)) {
          Some(t) if matches!(t.kind, TokenKind::Whitespace(_)) => t.raw.chars().count(),
          _ => indent,
        }
      } else {
        indent
      };
      if !first {
        if indent > 0 {
          let aligned = matches!(self.peek(), Some(t) if matches!(t.kind, TokenKind::Whitespace(_)) && t.raw.chars().count() == indent);
          if !aligned {
            break;
          }
          let next = self.tokens.get(self.pos + 1);
          let next_is_marker = matches!(
            next.map(|t| t.kind.clone()),
            Some(TokenKind::UnorderedListMarker) | Some(TokenKind::OrderedListMarker(_))
          );
          if !next_is_marker {
            break;
          }
          self.advance();
        } else if let Some(w) = match self.peek_kind() {
          Some(TokenKind::Whitespace(w)) if (*w as usize) <= 3 => Some(*w as usize),
          _ => None,
        } {
          let next = self.tokens.get(self.pos + 1);
          let next_is_marker = matches!(
            next.map(|t| t.kind.clone()),
            Some(TokenKind::UnorderedListMarker) | Some(TokenKind::OrderedListMarker(_))
          );
          if !next_is_marker {
            break;
          }
          self.advance();
          item_indent = w;
        }
      }
      first = false;

      let kind = match self.peek_kind() {
        Some(k) => k,
        None => break,
      };
      let want_marker = if ordered {
        match kind {
          TokenKind::OrderedListMarker(s) => ordered_sep.is_none() || ordered_sep == Some(*s),
          _ => false,
        }
      } else {
        match kind {
          TokenKind::UnorderedListMarker => {
            let this_char = self.peek().and_then(|t| t.raw.chars().next());
            bullet_char.is_none() || this_char == bullet_char
          },
          _ => false,
        }
      };
      if !want_marker {
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
      self.advance();

      let extra_ws = self
        .peek_leading_indent()
        .map(|n| if !marker_raw_has_ws && marker_raw_width > 0 { n.saturating_sub(1) } else { n })
        .unwrap_or(0);
      let item_content_extra = if extra_ws >= 4 { 0 } else { extra_ws };

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

      let code_item = if extra_ws >= 4 {
        let item_span = self.current_span();
        if matches!(self.peek_kind(), Some(TokenKind::Whitespace(_))) {
          self.advance();
        }
        let mut buf = String::new();
        if extra_ws > 4 {
          buf.push_str(&" ".repeat(extra_ws - 4));
        }
        while let Some(t) = self.peek() {
          match &t.kind {
            TokenKind::SoftBreak | TokenKind::HardBreak | TokenKind::BlankLine | TokenKind::Eof => break,
            _ => {
              buf.push_str(t.raw);
              self.advance();
            },
          }
        }
        if !buf.is_empty() {
          buf.push('\n');
          Some(Node::ListItem(ListItem {
            children: vec![Node::CodeBlock(CodeBlock { lang: None, meta: None, value: buf, span: item_span.clone() })],
            span: item_span,
          }))
        } else {
          None
        }
      } else {
        None
      };

      let mut item = if let Some(item) = code_item { item } else { self.parse_one_list_item(ordered) };

      if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
        self.advance();
      }

      loop {
        let lazy_eligible = !self.line_starts_other_block()
          && matches!(
            self.peek_kind(),
            Some(TokenKind::Text)
              | Some(TokenKind::Emphasis(_, _))
              | Some(TokenKind::Strikethrough)
              | Some(TokenKind::CodeInlineOpen(_))
              | Some(TokenKind::Autolink(_))
              | Some(TokenKind::EntityRef)
              | Some(TokenKind::LinkOpen)
              | Some(TokenKind::ImageMarker)
          );
        if !lazy_eligible {
          break;
        }
        let para_span = self.current_span();
        let mut inline = self.collect_inline_until_break();
        if inline.is_empty() || !Self::append_inline_continuation(&mut item, &mut inline, &para_span) {
          break;
        }
        if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
          self.advance();
        } else {
          break;
        }
      }

      let content_floor =
        item_indent + (if ordered { marker_width.max(3) } else { marker_width.max(2) }) + item_content_extra;
      while let Some(child_indent) = self.peek_leading_indent() {
        if child_indent <= indent {
          break;
        }
        let saved = self.pos;
        self.advance();
        self.try_promote_text_blockquote_marker();
        self.try_promote_text_list_marker();
        if child_indent.saturating_sub(content_floor) <= 3
          && let Some(lvl) = self.setext_underline_level()
          && Self::promote_item_to_setext_heading(&mut item, lvl, &span)
        {
          self.eat_setext_underline();
          if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
            self.advance();
          }
          continue;
        }
        match self.peek_kind() {
          Some(TokenKind::UnorderedListMarker) if child_indent < content_floor => {
            self.pos = saved;
            if indent == 0 && child_indent > 3 {
              self.advance();
              let para_span = self.current_span();
              let mut inline = self.collect_inline_until_break();
              if !inline.is_empty() {
                if !Self::append_inline_continuation(&mut item, &mut inline, &para_span) {
                  Self::ensure_loose_item(&mut item, &para_span);
                  Self::append_to_item(&mut item, Node::Paragraph(Paragraph { children: inline, span: para_span }));
                }
                if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
                  self.advance();
                }
                continue;
              }
              self.pos = saved;
            }
            break;
          },
          Some(TokenKind::OrderedListMarker(_)) if child_indent < content_floor => {
            self.pos = saved;
            if indent == 0 && child_indent > 3 {
              self.advance();
              let para_span = self.current_span();
              let mut inline = self.collect_inline_until_break();
              if !inline.is_empty() {
                if !Self::append_inline_continuation(&mut item, &mut inline, &para_span) {
                  Self::ensure_loose_item(&mut item, &para_span);
                  Self::append_to_item(&mut item, Node::Paragraph(Paragraph { children: inline, span: para_span }));
                }
                if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
                  self.advance();
                }
                continue;
              }
              self.pos = saved;
            }
            break;
          },
          Some(TokenKind::UnorderedListMarker) => {
            let sub = self.parse_list(false, child_indent);
            Self::append_to_item(&mut item, sub);
          },
          Some(TokenKind::OrderedListMarker(_)) => {
            let sub = self.parse_list(true, child_indent);
            Self::append_to_item(&mut item, sub);
          },
          Some(TokenKind::BlockQuoteMarker) => {
            let bq = self.parse_blockquote();
            Self::append_to_item(&mut item, bq);
          },
          Some(TokenKind::CodeFenceOpen(_, _)) => {
            let code = self.parse_code_block();
            Self::append_to_item(&mut item, code);
          },
          Some(_) if child_indent >= content_floor + 4 => {
            self.pos = saved;
            let span = self.current_span();
            let mut buf = String::new();
            loop {
              let aligned = matches!(self.peek(), Some(t) if matches!(t.kind, TokenKind::Whitespace(_)) && t.raw.chars().count() >= content_floor + 4);
              if !aligned {
                break;
              }
              let ws_len = self.peek().map(|t| t.raw.chars().count()).unwrap_or(0);
              self.advance();
              let visible = ws_len.saturating_sub(content_floor + 4);
              if visible > 0 {
                buf.push_str(&" ".repeat(visible));
              }
              while let Some(t) = self.peek() {
                match &t.kind {
                  TokenKind::SoftBreak | TokenKind::HardBreak | TokenKind::BlankLine | TokenKind::Eof => break,
                  _ => {
                    buf.push_str(t.raw);
                    self.advance();
                  },
                }
              }
              buf.push('\n');
              let saved2 = self.pos;
              let mut blanks = 0usize;
              loop {
                match self.peek_kind() {
                  Some(TokenKind::SoftBreak) => {
                    self.advance();
                    blanks += 1;
                    break;
                  },
                  Some(TokenKind::BlankLine) => {
                    self.advance();
                    blanks += 2;
                  },
                  _ => break,
                }
              }
              if blanks == 0 {
                break;
              }
              let next_aligned = matches!(self.peek(), Some(t) if matches!(t.kind, TokenKind::Whitespace(_)) && t.raw.chars().count() >= content_floor + 4);
              if !next_aligned {
                self.pos = saved2;
                break;
              }
              for _ in 1..blanks {
                buf.push('\n');
              }
            }
            Self::ensure_loose_item(&mut item, &span);
            Self::append_to_item(&mut item, Node::CodeBlock(CodeBlock { lang: None, meta: None, value: buf, span }));
          },
          Some(_) => {
            let span = self.current_span();
            let mut inline = self.collect_inline_until_break();
            if inline.is_empty() {
              self.pos = saved;
              break;
            }
            loop {
              if !matches!(self.peek_kind(), Some(TokenKind::SoftBreak)) {
                break;
              }
              let sb_saved = self.pos;
              self.advance();
              let aligned_indent = match self.peek() {
                Some(t) if matches!(t.kind, TokenKind::Whitespace(_)) => {
                  let n = t.raw.chars().count();
                  if n >= child_indent { Some(()) } else { None }
                },
                _ => None,
              };
              if aligned_indent.is_none() {
                self.pos = sb_saved;
                break;
              }
              let after = self.tokens.get(self.pos + 1).map(|t| t.kind.clone());
              let next_is_block = matches!(
                after,
                Some(TokenKind::UnorderedListMarker)
                  | Some(TokenKind::OrderedListMarker(_))
                  | Some(TokenKind::Heading(_))
                  | Some(TokenKind::BlockQuoteMarker)
                  | Some(TokenKind::CodeFenceOpen(_, _))
                  | Some(TokenKind::ThematicBreak)
                  | Some(TokenKind::SoftBreak)
                  | Some(TokenKind::HardBreak)
                  | None
              );
              if next_is_block {
                self.pos = sb_saved;
                break;
              }
              self.advance();
              let break_span = self.current_span();
              inline.push(Node::SoftBreak(BreakNode { span: break_span }));
              let more = self.collect_inline_until_break();
              if more.is_empty() {
                inline.pop();
                self.pos = sb_saved;
                break;
              }
              inline.extend(more);
            }
            if !Self::append_inline_continuation(&mut item, &mut inline, &span) {
              Self::ensure_loose_item(&mut item, &span);
              Self::append_to_item(&mut item, Node::Paragraph(Paragraph { children: inline, span }));
            }
          },
          None => {
            self.pos = saved;
            break;
          },
        }
        if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
          self.advance();
        }
      }

      items.push(item);

      'after_item: while matches!(self.peek_kind(), Some(TokenKind::BlankLine)) {
        let saved = self.pos;
        self.advance();
        let leading = self.peek_leading_indent();
        let last_item_is_empty = items.last().is_some_and(|item| match item {
          Node::ListItem(li) => li.children.is_empty(),
          Node::TaskListItem(t) => t.children.is_empty(),
          _ => false,
        });
        let content_indent = content_floor;
        if let Some(n) = leading
          && !last_item_is_empty
          && n >= content_indent
        {
          if n >= content_indent + 4 {
            let span_code = self.current_span();
            let mut buf = String::new();
            loop {
              let aligned = self.peek_leading_indent().is_some_and(|n| n >= content_indent + 4);
              if !aligned {
                break;
              }
              let ws_len = self.peek_leading_indent().unwrap_or(0);
              self.advance();
              let visible = ws_len.saturating_sub(content_indent + 4);
              if visible > 0 {
                buf.push_str(&" ".repeat(visible));
              }
              while let Some(t) = self.peek() {
                match &t.kind {
                  TokenKind::SoftBreak | TokenKind::HardBreak | TokenKind::BlankLine | TokenKind::Eof => break,
                  _ => {
                    buf.push_str(t.raw);
                    self.advance();
                  },
                }
              }
              buf.push('\n');
              let saved2 = self.pos;
              let mut blanks = 0usize;
              loop {
                match self.peek_kind() {
                  Some(TokenKind::SoftBreak) => {
                    self.advance();
                    blanks += 1;
                    break;
                  },
                  Some(TokenKind::BlankLine) => {
                    self.advance();
                    blanks += 2;
                  },
                  _ => break,
                }
              }
              if blanks == 0 {
                break;
              }
              let next_aligned = self.peek_leading_indent().is_some_and(|n| n >= content_indent + 4);
              if !next_aligned {
                self.pos = saved2;
                break;
              }
              for _ in 1..blanks {
                buf.push('\n');
              }
            }
            if !buf.is_empty()
              && let Some(last) = items.last_mut()
            {
              Self::ensure_loose_item(last, &span);
              Self::append_to_item(
                last,
                Node::CodeBlock(CodeBlock { lang: None, meta: None, value: buf, span: span_code }),
              );
            }
            self.try_continue_same_list_after_blankline(
              ordered,
              indent,
              &mut items,
              &span,
              &mut saw_blank_between_items,
            );
            if matches!(self.peek_kind(), Some(TokenKind::UnorderedListMarker) | Some(TokenKind::OrderedListMarker(_)))
            {
              break 'after_item;
            }
            continue 'after_item;
          }
          let next_after_ws = self.tokens.get(self.pos + 1).map(|t| t.kind.clone());
          if matches!(next_after_ws, Some(TokenKind::UnorderedListMarker) | Some(TokenKind::OrderedListMarker(_))) {
            self.advance();
            let nested = match next_after_ws {
              Some(TokenKind::OrderedListMarker(_)) => self.parse_list(true, n),
              _ => self.parse_list(false, n),
            };
            if let Some(last) = items.last_mut() {
              Self::ensure_loose_item(last, &span);
              Self::append_to_item(last, nested);
            }
            continue 'after_item;
          }
          if let Some(TokenKind::IndentedCodeLine) = next_after_ws.as_ref()
            && let Some(raw) = self.tokens.get(self.pos + 1).map(|t| t.raw)
            && let Some((_marker_char, _rest)) = raw
              .strip_prefix(['-', '*', '+'])
              .and_then(|r| r.strip_prefix([' ', '\t']))
              .map(|r| (raw.as_bytes()[0] as char, r))
          {
            let pos = self.pos + 1;
            let original = self.tokens[pos].raw;
            let marker_byte = original.as_bytes()[0];
            let content = &original[2..];
            self.tokens[pos].raw = match marker_byte {
              b'-' => "-",
              b'*' => "*",
              b'+' => "+",
              _ => "-",
            };
            self.tokens[pos].kind = TokenKind::UnorderedListMarker;
            let text_tok =
              dmc_lexer::token::Token { kind: TokenKind::Text, raw: content, span: self.tokens[pos].span.clone() };
            self.tokens.insert(pos + 1, text_tok);
            self.advance();
            let nested = self.parse_list(false, n);
            if let Some(last) = items.last_mut() {
              Self::ensure_loose_item(last, &span);
              Self::append_to_item(last, nested);
            }
            continue 'after_item;
          }
          if let Some(TokenKind::IndentedCodeLine) = next_after_ws.as_ref()
            && let Some(raw) = self.tokens.get(self.pos + 1).map(|t| t.raw)
            && raw.starts_with('>')
            && raw[1..].chars().next().is_none_or(|c| c == ' ' || c == '\t')
          {
            let pos = self.pos + 1;
            let original = self.tokens[pos].raw;
            let rest = original[1..].trim_start_matches([' ', '\t']);
            self.tokens[pos].raw = ">";
            self.tokens[pos].kind = TokenKind::BlockQuoteMarker;
            if !rest.is_empty() {
              let text_tok =
                dmc_lexer::token::Token { kind: TokenKind::Text, raw: rest, span: self.tokens[pos].span.clone() };
              self.tokens.insert(pos + 1, text_tok);
            }
            self.advance();
            let bq = self.parse_blockquote();
            if let Some(last) = items.last_mut() {
              Self::ensure_loose_item(last, &span);
              Self::append_to_item(last, bq);
            }
            continue 'after_item;
          }
          if let Some(TokenKind::IndentedCodeLine) = next_after_ws.as_ref()
            && let Some(code) = self.parse_same_indent_fenced_code_in_list(content_indent)
          {
            if let Some(last) = items.last_mut() {
              Self::ensure_loose_item(last, &span);
              Self::append_to_item(last, code);
            }
            continue 'after_item;
          }
          if n == content_indent && matches!(next_after_ws, Some(TokenKind::IndentedCodeLine)) {
            self.tokens[self.pos + 1].kind = TokenKind::Text;
          }
          self.advance();
          self.try_promote_text_blockquote_marker();
          self.try_promote_text_list_marker();
          let appended = match self.peek_kind() {
            Some(TokenKind::UnorderedListMarker) => {
              let nested = self.parse_list(false, n);
              if let Some(last) = items.last_mut() {
                Self::ensure_loose_item(last, &span);
                Self::append_to_item(last, nested);
              }
              true
            },
            Some(TokenKind::OrderedListMarker(_)) => {
              let nested = self.parse_list(true, n);
              if let Some(last) = items.last_mut() {
                Self::ensure_loose_item(last, &span);
                Self::append_to_item(last, nested);
              }
              true
            },
            Some(TokenKind::BlockQuoteMarker) => {
              let bq = self.parse_blockquote();
              if let Some(last) = items.last_mut() {
                Self::ensure_loose_item(last, &span);
                Self::append_to_item(last, bq);
              }
              true
            },
            Some(TokenKind::CodeFenceOpen(_, _)) => {
              let code = self.parse_code_block();
              if let Some(last) = items.last_mut() {
                Self::ensure_loose_item(last, &span);
                Self::append_to_item(last, code);
              }
              true
            },
            Some(TokenKind::LinkRefDef) => {
              if let Some(last) = items.last_mut() {
                Self::ensure_loose_item(last, &span);
              }
              self.advance();
              true
            },
            _ => false,
          };
          if appended {
            if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
              self.advance();
            }
            self.try_continue_same_list_after_blankline(
              ordered,
              indent,
              &mut items,
              &span,
              &mut saw_blank_between_items,
            );
            if matches!(self.peek_kind(), Some(TokenKind::UnorderedListMarker) | Some(TokenKind::OrderedListMarker(_)))
            {
              break 'after_item;
            }
            continue 'after_item;
          }
          let para_span = self.current_span();
          let inline = self.collect_inline_until_break();
          if !inline.is_empty() {
            if let Some(last) = items.last_mut() {
              Self::ensure_loose_item(last, &span);
              Self::append_to_item(last, Node::Paragraph(Paragraph { children: inline, span: para_span }));
            }
            if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
              self.advance();
            }
            self.try_continue_same_list_after_blankline(
              ordered,
              indent,
              &mut items,
              &span,
              &mut saw_blank_between_items,
            );
            if matches!(self.peek_kind(), Some(TokenKind::UnorderedListMarker) | Some(TokenKind::OrderedListMarker(_)))
            {
              break 'after_item;
            }
            continue 'after_item;
          } else {
            self.pos = saved;
            break 'after_item;
          }
        } else {
          let ws_pos = self.pos;
          let ws_w = match self.peek_kind() {
            Some(TokenKind::Whitespace(_)) => Some(self.pos),
            _ => None,
          };
          if ws_w.is_some() {
            self.advance();
          }
          let next_is_marker = match self.peek_kind() {
            Some(TokenKind::UnorderedListMarker) if !ordered => true,
            Some(TokenKind::OrderedListMarker(_)) if ordered => true,
            _ => false,
          };
          let next_indent =
            if ws_w.is_some() { self.tokens.get(ws_pos).map(|t| t.raw.chars().count()).unwrap_or(0) } else { 0 };
          let same_list_indent = if indent > 0 { next_indent == indent } else { next_indent <= 3 };
          if !next_is_marker || !same_list_indent {
            self.pos = saved;
          } else {
            if ws_w.is_some() {
              self.pos = ws_pos;
            }
            saw_blank_between_items = true;
            if let Some(last) = items.last_mut() {
              Self::ensure_loose_item(last, &span);
            }
          }
        }
        break 'after_item;
      }
    }

    let any_loose = saw_blank_between_items
      || items.iter().any(|n| match n {
        Node::ListItem(li) => li.children.iter().any(|c| matches!(c, Node::Paragraph(_))),
        Node::TaskListItem(t) => t.children.iter().any(|c| matches!(c, Node::Paragraph(_))),
        _ => false,
      });
    if any_loose {
      for n in items.iter_mut() {
        Self::ensure_loose_item(n, &span);
      }
    }

    Node::List(List { ordered, start, children: items, span })
  }

  /// Parse the body of one list item. Promotes to `TaskListItem` if a GFM
  /// `[ ]` / `[x]` checkbox prefix follows the marker.
  pub(super) fn parse_one_list_item(&mut self, ordered: bool) -> Node {
    let span = self.current_span();
    if !ordered {
      let pre = self.pos;
      if matches!(self.peek_kind(), Some(TokenKind::Whitespace(_))) {
        self.advance();
      }
      if matches!(self.peek_kind(), Some(TokenKind::TaskMarker(_))) {
        let checked = matches!(self.peek_kind(), Some(TokenKind::TaskMarker(true)));
        self.advance();
        let inline = self.collect_inline_for_list_item();
        return Node::TaskListItem(TaskListItem { checked, children: inline, span });
      }
      if matches!(self.peek_kind(), Some(TokenKind::LinkOpen)) {
        self.advance();
        let text_raw = self.peek().map(|t| t.raw.to_string()).unwrap_or_default();
        let kind = self.peek_kind().cloned();
        if matches!(kind, Some(TokenKind::Text)) && (text_raw == " " || text_raw.eq_ignore_ascii_case("x")) {
          self.advance();
          if matches!(self.peek_kind(), Some(TokenKind::LinkClose)) {
            self.advance();
            let checked = text_raw.eq_ignore_ascii_case("x");
            let inline = self.collect_inline_for_list_item();
            return Node::TaskListItem(TaskListItem { checked, children: inline, span });
          }
        }
        self.pos = pre;
      } else {
        self.pos = pre;
      }
    }

    let content_start = self.pos;
    if matches!(self.peek_kind(), Some(TokenKind::Whitespace(_))) {
      self.advance();
    }
    self.try_promote_text_blockquote_marker();
    self.try_promote_text_list_marker();
    match self.peek_kind() {
      Some(TokenKind::UnorderedListMarker) => {
        let nested = self.parse_list(false, 0);
        return Node::ListItem(ListItem { children: vec![nested], span });
      },
      Some(TokenKind::OrderedListMarker(_)) => {
        let nested = self.parse_list(true, 0);
        return Node::ListItem(ListItem { children: vec![nested], span });
      },
      Some(TokenKind::BlockQuoteMarker) => {
        let bq = self.parse_blockquote();
        return Node::ListItem(ListItem { children: vec![bq], span });
      },
      Some(TokenKind::CodeFenceOpen(_, _)) => {
        let code = self.parse_code_block();
        return Node::ListItem(ListItem { children: vec![code], span });
      },
      Some(TokenKind::Heading(_)) => {
        let h = self.parse_heading();
        return Node::ListItem(ListItem { children: vec![h], span });
      },
      Some(TokenKind::ThematicBreak) => {
        let hr_span = self.current_span();
        self.advance();
        return Node::ListItem(ListItem {
          children: vec![Node::HorizontalRule(HorizontalRule { span: hr_span })],
          span,
        });
      },
      _ => {
        self.pos = content_start;
      },
    }

    let inline = self.collect_inline_for_list_item();
    Node::ListItem(ListItem { children: inline, span })
  }
}
