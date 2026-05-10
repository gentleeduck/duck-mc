use crate::ast::*;
use crate::parser::Parser;
use dmc_lexer::token::TokenKind;

/// CommonMark 4.6 type-1: closes on the matching `</tag>`. Tag names
/// here force the surrounding source into a raw HTML block even when
/// the lexer routed them through JSX.
const HTML_BLOCK_TYPE1_TAGS: &[&str] = &["script", "pre", "style", "textarea"];

/// CommonMark 4.6 type-6: block-level HTML tag set. Closes on the next
/// blank line. Type-7 ("any other tag at column 0") is intentionally
/// not handled -- in MDX the same shape means a JSX component, and
/// reclassifying every capital-or-namespaced tag as HTML would break
/// the dialect.
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
  /// Closes on matching `</tag>`. Carries the lowercased tag name.
  Type1(String),
  /// Closes on blank line.
  Type6,
  /// Type-7: any other lowercase tag at col 0. Closes on blank line.
  /// Skipped for capital / namespaced tags (those stay as JSX so the
  /// MDX dialect keeps `<MyComponent>` working).
  Type7,
}

impl<'eng, 'tokens> Parser<'eng, 'tokens> {
  /// Top-down dispatch: peek one token, route to the matching block parser.
  /// `None` means the cursor advanced but emitted no node (e.g. a stray break
  /// or a markdown comment).
  pub(crate) fn parse_block(&mut self) -> Option<Node> {
    // Indented code: lexer emits `Whitespace(_) + IndentedCodeLine` for
    // any column-0 indent that reaches col 4 (4+ spaces, a tab, or a
    // tab/space mix), so the parser just trusts the pair.
    let is_indented = matches!(self.peek_kind(), Some(TokenKind::Whitespace(_)))
      && matches!(self.tokens.get(self.pos + 1).map(|t| &t.kind), Some(TokenKind::IndentedCodeLine));

    if is_indented {
      return Some(self.parse_indented_code());
    }

    // Whitespace-then-block-marker. CM allows up to 3 leading spaces
    // before any block-level marker, so when the lexer emitted a
    // leading Whitespace followed by a known block opener we drop the
    // indent and dispatch on the marker.
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
        TokenKind::JsxOpenTagStart => {
          // Peek the JSX tag two tokens ahead (after the leading
          // whitespace) to see if it routes to a Type-1 / Type-6 raw
          // HTML block.
          let saved = self.pos;
          self.advance();
          if let Some(mode) = self.jsx_html_block_mode() {
            return Some(self.parse_html_block_from_jsx(mode));
          }
          self.pos = saved;
        },
        TokenKind::LinkRefDef => {
          self.advance(); // skip whitespace
          self.advance(); // skip the ref-def itself
          return None;
        },
        // Whitespace followed by an empty-line break -- drop both,
        // they're indent + blank padding around block structure.
        TokenKind::BlankLine | TokenKind::SoftBreak | TokenKind::HardBreak => {
          self.advance();
          self.advance();
          return None;
        },
        // Plain content with 1-3 leading spaces -- strip the indent and
        // dispatch normally so the resulting paragraph doesn't render
        // the leading whitespace.
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
        } else {
          Some(self.parse_jsx())
        }
      },
      TokenKind::JsxCloseTagStart => {
        if let Some(mode) = self.jsx_html_block_mode() {
          Some(self.parse_html_block_from_jsx(mode))
        } else {
          // Stray close tag at top level - treat as text by falling
          // through to paragraph collection.
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
      // Link reference definitions are harvested in the pre-pass; the
      // tokens themselves produce no output node.
      TokenKind::LinkRefDef => {
        self.advance();
        None
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

  /// Consume `FrontmatterStart .. FrontmatterEnd`. Inner YAML is left as raw
  /// text; interpretation is the caller's job.
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

  /// Collect consecutive list items of one flavor (ordered or unordered) into
  /// a `List` node. `indent` is the column of the marker on its line; nested
  /// lists pass a larger `indent` so deeper sub-items keep recursing.
  fn parse_list(&mut self, ordered: bool, indent: usize) -> Node {
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
    // CM 5.2: a list of one type can't continue with a different
    // marker. `- foo\n+ bar` is two lists. Track the bullet char
    // (unordered) or separator (ordered) of the first item and break
    // out when the next marker mismatches.
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
    loop {
      // First iteration: caller has already advanced past any indent
      // whitespace; cursor is on the marker. Subsequent iterations: for
      // nested lists (indent > 0) require a `Whitespace` of width `indent`
      // before the next marker - a marker at a smaller indent belongs to
      // an outer list.
      let mut item_indent = indent;
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
          // Top-level list: allow 1-3 leading spaces before each
          // continuation marker so `- a\n - b\n  - c` parses as one
          // list (CM 5.2 sub-marker indent < 4 keeps them at the same
          // level when no list-content boundary intervenes).
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
      self.advance();

      // Ordered-list items: trim the trailing `.` left in the first Text token.
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

      // Anything indented strictly deeper than the parent marker belongs
      // to this item. Three cases at the deeper indent:
      //   1. list marker -> nested sublist
      //   2. plain text  -> loose-list paragraph continuation
      //   3. anything else -> rewind, parent decides
      // CM 5.2: a sub-list marker only nests when its column is at or
      // past the parent item's content-indent. Approximate that with
      // `indent + 2` (unordered) / `indent + 3` (ordered, 1-digit). A
      // marker at a shallower column breaks out so the outer loop can
      // continue the same list.
      let content_floor = item_indent + if ordered { 3 } else { 2 };
      while let Some(child_indent) = self.peek_leading_indent() {
        if child_indent <= indent {
          break;
        }
        let saved = self.pos;
        self.advance();
        match self.peek_kind() {
          Some(TokenKind::UnorderedListMarker) if child_indent < content_floor => {
            self.pos = saved;
            break;
          },
          Some(TokenKind::OrderedListMarker(_)) if child_indent < content_floor => {
            self.pos = saved;
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
          Some(_) if child_indent >= content_floor + 4 => {
            // CM 5.2: a continuation line indented >= content_floor + 4
            // becomes an indented code block inside the item. Rewind so
            // parse_indented_code can consume the leading whitespace it
            // needs, but trim the item-content portion off the leading
            // run by skipping past `content_floor` cols on each line.
            self.pos = saved;
            let extra = child_indent - content_floor;
            let span = self.current_span();
            let mut buf = String::new();
            loop {
              let aligned = matches!(self.peek(), Some(t) if matches!(t.kind, TokenKind::Whitespace(_)) && t.raw.chars().count() >= content_floor + 4);
              if !aligned {
                break;
              }
              let ws_len = self.peek().map(|t| t.raw.chars().count()).unwrap_or(0);
              self.advance(); // whitespace
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
            let _ = extra;
            Self::ensure_loose_item(&mut item, &span);
            Self::append_to_item(&mut item, Node::CodeBlock(CodeBlock { lang: None, meta: None, value: buf, span }));
          },
          Some(_) => {
            // Loose-list paragraph continuation. Wrap the item's existing
            // inline body as a `Paragraph` (so CommonMark-style loose
            // formatting applies), collect the new inline run as another
            // `Paragraph`, and append.
            //
            // CommonMark: contiguous indented lines (no blank line between
            // them) belong to the SAME paragraph with the soft break kept
            // as a literal newline inside the text. Continue eating soft
            // breaks + properly-indented continuation lines until a blank
            // line, block-starter, or de-indented line stops the run.
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
              self.advance(); // consume the indent whitespace
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
            Self::ensure_loose_item(&mut item, &span);
            Self::append_to_item(&mut item, Node::Paragraph(Paragraph { children: inline, span }));
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

      // CM 5.3 blank-line handling.
      if matches!(self.peek_kind(), Some(TokenKind::BlankLine)) {
        let saved = self.pos;
        self.advance();
        let leading = self.peek_leading_indent();
        // Indented continuation -- attach a new paragraph to the
        // current item (loose-list with continuation).
        let content_indent = indent + 2;
        if let Some(n) = leading
          && n >= content_indent
        {
          // CM 5.2: continuation indented >= content_indent + 4 is an
          // indented code block inside the item.
          if n >= content_indent + 4 {
            let span_code = self.current_span();
            let mut buf = String::new();
            loop {
              let aligned = matches!(self.peek(), Some(t) if matches!(t.kind, TokenKind::Whitespace(_)) && t.raw.chars().count() >= content_indent + 4);
              if !aligned {
                break;
              }
              let ws_len = self.peek().map(|t| t.raw.chars().count()).unwrap_or(0);
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
              let next_aligned = matches!(self.peek(), Some(t) if matches!(t.kind, TokenKind::Whitespace(_)) && t.raw.chars().count() >= content_indent + 4);
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
            continue;
          }
          // Sub-list nest: if the indented line opens with a list marker,
          // recurse into a nested list at this indent. The line may have
          // been pre-classified by the lexer as `Whitespace + IndentedCodeLine`
          // because it sits at col >= 4; sniff the IndentedCodeLine raw
          // for a marker prefix and synthesize the missing Marker token.
          let next_after_ws = self.tokens.get(self.pos + 1).map(|t| t.kind.clone());
          if matches!(next_after_ws, Some(TokenKind::UnorderedListMarker) | Some(TokenKind::OrderedListMarker(_))) {
            self.advance(); // skip whitespace
            let nested = match next_after_ws {
              Some(TokenKind::OrderedListMarker(_)) => self.parse_list(true, n),
              _ => self.parse_list(false, n),
            };
            if let Some(last) = items.last_mut() {
              Self::ensure_loose_item(last, &span);
              Self::append_to_item(last, nested);
            }
            continue;
          }
          if let Some(TokenKind::IndentedCodeLine) = next_after_ws.as_ref()
            && let Some(raw) = self.tokens.get(self.pos + 1).map(|t| t.raw)
            && let Some((_marker_char, _rest)) = raw
              .strip_prefix(['-', '*', '+'])
              .and_then(|r| r.strip_prefix([' ', '\t']))
              .map(|r| (raw.as_bytes()[0] as char, r))
          {
            // Convert the IndentedCodeLine to UnorderedListMarker + content
            // by rewriting the next two tokens. Cheap because we own the
            // token slice.
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
            // Insert a Text token for the content, if any.
            let text_tok =
              dmc_lexer::token::Token { kind: TokenKind::Text, raw: content, span: self.tokens[pos].span.clone() };
            self.tokens.insert(pos + 1, text_tok);
            self.advance(); // skip whitespace
            let nested = self.parse_list(false, n);
            if let Some(last) = items.last_mut() {
              Self::ensure_loose_item(last, &span);
              Self::append_to_item(last, nested);
            }
            continue;
          }
          self.advance(); // skip whitespace
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
            continue;
          } else {
            self.pos = saved;
          }
        } else {
          // Allow optional leading whitespace before the next marker.
          if matches!(self.peek_kind(), Some(TokenKind::Whitespace(_))) {
            self.advance();
          }
          let next_is_marker = match self.peek_kind() {
            Some(TokenKind::UnorderedListMarker) if !ordered => true,
            Some(TokenKind::OrderedListMarker(_)) if ordered => true,
            _ => false,
          };
          if !next_is_marker {
            self.pos = saved;
          } else if let Some(last) = items.last_mut() {
            Self::ensure_loose_item(last, &span);
          }
        }
      }
    }

    // Loose list: when any item has a Paragraph child, every item must
    // also be wrapped in a Paragraph (CommonMark loose-list rule).
    let any_loose = items.iter().any(|n| match n {
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

  /// Width of the leading `Whitespace` token at the cursor, in spaces. Tabs
  /// count as 1 column for the purposes of comparing against a parent
  /// marker's indent. `None` when the cursor is not on a Whitespace token.
  fn peek_leading_indent(&self) -> Option<usize> {
    match self.peek() {
      Some(t) if matches!(t.kind, TokenKind::Whitespace(_)) => Some(t.raw.chars().count()),
      _ => None,
    }
  }

  /// Append `child` to the children of `item` (works for both `ListItem`
  /// and `TaskListItem`).
  fn append_to_item(item: &mut Node, child: Node) {
    match item {
      Node::ListItem(li) => li.children.push(child),
      Node::TaskListItem(t) => t.children.push(child),
      _ => {},
    }
  }

  /// Wrap any LEADING raw inline content of `item` in a `Paragraph` so
  /// loose-list formatting (`<li><p>...</p><p>...</p></li>`) applies
  /// once a continuation paragraph or nested block appears.
  ///
  /// Only contiguous inline-typed children at the front of the list are
  /// promoted; existing block children (Paragraph, List, Blockquote,
  /// CodeBlock, ...) are left in place. Otherwise a `[Text, List]` item
  /// would collapse into a single paragraph that swallows the list.
  fn ensure_loose_item(item: &mut Node, span: &duck_diagnostic::Span) {
    fn is_block_node(n: &Node) -> bool {
      matches!(
        n,
        Node::Paragraph(_)
          | Node::List(_)
          | Node::Blockquote(_)
          | Node::CodeBlock(_)
          | Node::Heading(_)
          | Node::HorizontalRule(_)
          | Node::Table(_)
      )
    }
    let promote = |kids: &mut Vec<Node>, span: &duck_diagnostic::Span| {
      // Already loose? Bail.
      if kids.first().is_some_and(|n| matches!(n, Node::Paragraph(_))) {
        return;
      }
      let split = kids.iter().position(is_block_node).unwrap_or(kids.len());
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

  /// Parse the body of one list item. Promotes to `TaskListItem` if a GFM
  /// `[ ]` / `[x]` checkbox prefix follows the marker.
  fn parse_one_list_item(&mut self, ordered: bool) -> Node {
    let span = self.current_span();
    // GFM task-list prefix `[ ]` / `[x]` / `[X]` for unordered lists.
    if !ordered {
      let pre = self.pos;
      if matches!(self.peek_kind(), Some(TokenKind::Whitespace(_))) {
        self.advance();
      }
      // Lexer now emits `TaskMarker(bool)` directly. Phase C8 will wire
      // it through; for Phase A we keep the legacy bracket-walk fallback
      // so existing behavior holds when the lexer hasn't fired the
      // dedicated marker (e.g. `[?]` in older fixtures).
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

    let inline = self.collect_inline_for_list_item();
    Node::ListItem(ListItem { children: inline, span })
  }

  /// Build a blockquote tree from the cursor (on a `>`). Walks lines
  /// keeping a stack: each line's marker count sets the live nesting
  /// depth. A line with more markers grows the stack; a line with fewer
  /// markers closes the deeper levels and folds them into their parent.
  fn parse_blockquote(&mut self) -> Node {
    let span = self.current_span();
    let para_span = self.current_span();
    // Per-level state. `children[i]` is the accumulated block-level
    // contents of nesting level `i+1`; `paragraphs[i]` is its in-progress
    // paragraph (inline run still being collected).
    let mut children: Vec<Vec<Node>> = vec![Vec::new()];
    let mut paragraphs: Vec<Vec<Node>> = vec![Vec::new()];

    loop {
      let line_markers = self.count_line_blockquote_markers();
      if line_markers == 0 {
        // CM 5.1 lazy continuation: a non-marker line continues the
        // currently-open paragraph when the cursor sits on inline
        // content (not a block-boundary token).
        let top = children.len() - 1;
        let lazy_eligible = !paragraphs[top].is_empty()
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
      // Grow the stack to match this line's depth.
      while children.len() < line_markers {
        children.push(Vec::new());
        paragraphs.push(Vec::new());
      }
      // Shrink the stack when this line has fewer markers than the
      // current open depth (close the deeper blockquotes).
      while children.len() > line_markers {
        Self::close_blockquote_level(&mut children, &mut paragraphs, &para_span, &span);
      }

      self.consume_blockquote_markers(line_markers);
      let top = children.len() - 1;

      // List marker after the blockquote prefix: parse a list as a
      // child of the current blockquote level. Flush any pending
      // paragraph first so the list lands as a sibling of it.
      let after_marker_kind = match self.peek_kind() {
        Some(k) => k.clone(),
        None => break,
      };
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
      // ATX heading / thematic break / fenced code inside blockquote.
      let block_node: Option<Node> = match after_marker_kind {
        TokenKind::Heading(_) => Some(self.parse_heading()),
        TokenKind::ThematicBreak => {
          let hr_span = self.current_span();
          self.advance();
          Some(Node::HorizontalRule(HorizontalRule { span: hr_span }))
        },
        TokenKind::CodeFenceOpen(_, _) => Some(self.parse_code_block()),
        _ => None,
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

    // Close every level we still have open.
    while children.len() > 1 {
      Self::close_blockquote_level(&mut children, &mut paragraphs, &para_span, &span);
    }
    // Flush the root level.
    let mut root = children.pop().unwrap();
    let last_para = paragraphs.pop().unwrap();
    if !last_para.is_empty() {
      root.push(Node::Paragraph(Paragraph { children: last_para, span: para_span }));
    }
    Node::Blockquote(Blockquote { children: root, span })
  }

  /// Parse a list nested inside a blockquote at the given depth. Each
  /// item line is preceded by `depth` `>` markers (already consumed for
  /// the first item by the caller). For subsequent items we skip the
  /// `>` markers ourselves before reading the next list marker.
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
      if !first {
        // Already advanced past softbreak from previous iter; skip
        // the next line's `>` markers, then check for another marker.
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
        break;
      }
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

      let item = self.parse_one_list_item(ordered);
      items.push(item);

      if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
        self.advance();
      }
    }

    Node::List(List { ordered, start, children: items, span })
  }

  /// Pop the deepest blockquote level: flush any pending paragraph,
  /// wrap as a `Blockquote` node, and append to the parent level.
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

  /// Count consecutive `>` markers at the current cursor, skipping over
  /// inter-marker whitespace. Stops at any other token or a line break.
  fn count_line_blockquote_markers(&self) -> usize {
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

  /// Advance past exactly `n` `>` markers (and the whitespace between
  /// each one). Stops early if fewer remain.
  fn consume_blockquote_markers(&mut self, n: usize) {
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
    if matches!(self.peek_kind(), Some(TokenKind::Whitespace(_))) {
      self.advance();
    }
  }

  /// Wrap the upcoming `Import` token's raw lexeme into an `Import` node.
  fn import_node(&mut self) -> Node {
    let span = self.current_span();
    let raw = self.peek().map(|t| t.raw.to_string()).unwrap_or_default();
    self.advance();
    Node::Import(Import { raw, span })
  }

  /// Counterpart to `import_node` for `export ...` statements.
  fn export_node(&mut self) -> Node {
    let span = self.current_span();
    let raw = self.peek().map(|t| t.raw.to_string()).unwrap_or_default();
    self.advance();
    Node::Export(Export { raw, span })
  }

  /// ATX heading. Anchor slug is derived lazily via `Heading::slug()`.
  fn parse_heading(&mut self) -> Node {
    let span = self.current_span();
    let level = match self.peek_kind() {
      Some(TokenKind::Heading(n)) => *n,
      _ => 1,
    };
    self.advance();
    let mut children = self.collect_inline_until_break();
    // CM 4.2: drop trailing HardBreak / whitespace-only text from a
    // heading. Trailing-spaces hard-break detection by the lexer fires
    // on the `   \n` at the end of `# foo   `, which the spec doesn't
    // turn into a `<br />` inside a heading.
    while matches!(children.last(), Some(Node::HardBreak(_))) {
      children.pop();
    }
    // The lexer leaves the post-`#` space in the inline stream as a Text /
    // Whitespace node, so the first heading-text node ends up with a leading
    // space ("` Inline marks`"). Strip it to match velite / rehype output.
    if let Some(Node::Text(t)) = children.first_mut() {
      let trimmed = t.value.trim_start_matches([' ', '\t']).to_string();
      if trimmed.is_empty() {
        children.remove(0);
      } else {
        t.value = trimmed;
      }
    }
    // Strip trailing whitespace-only text nodes (left by the HeadingTrailingHashes
    // skip + spaces between the text and the optional `###`).
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
    Node::Heading(Heading { level, children, span, id: None })
  }

  /// CM 4.6 raw-HTML block detection, keyed off a JSX-style open tag at
  /// column 0. Returns `Some(mode)` when the upcoming tag belongs to
  /// the type-1 or type-6 set; cursor untouched.
  fn jsx_html_block_mode(&self) -> Option<HtmlBlockMode> {
    let open = self.tokens.get(self.pos)?;
    // Span column is 1-based; accept 1-4 (col 0-3 in 0-based) per CM
    // 4.6: up to three leading spaces are allowed before any block.
    if open.span.column > 4 {
      return None;
    }
    if !matches!(open.kind, TokenKind::JsxOpenTagStart | TokenKind::JsxCloseTagStart) {
      return None;
    }
    let name_tok = self.tokens.get(self.pos + 1)?;
    if !matches!(name_tok.kind, TokenKind::JsxTagName) {
      return None;
    }
    let raw_name = name_tok.raw;
    let lower = raw_name.to_ascii_lowercase();
    if HTML_BLOCK_TYPE1_TAGS.contains(&lower.as_str()) {
      Some(HtmlBlockMode::Type1(lower))
    } else if HTML_BLOCK_TYPE6_TAGS.contains(&lower.as_str()) {
      Some(HtmlBlockMode::Type6)
    } else if raw_name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) && self.line_after_tag_is_blank() {
      // CM 4.6 Type-7: any tag at col 0 closes on next blank line --
      // BUT the start line itself must contain only the tag plus
      // whitespace (no inline content after the closing `>`).
      // Restricted to plain lowercase names so MDX components like
      // `<MyComponent>` and namespaces like `<svg:circle>` stay JSX.
      Some(HtmlBlockMode::Type7)
    } else {
      None
    }
  }

  /// After the upcoming JSX tag's `>` / `/>`, is the rest of the line
  /// whitespace-only? Required for CM 4.6 Type-7 trigger.
  fn line_after_tag_is_blank(&self) -> bool {
    // Skip over the open tag tokens until JsxOpenTagEnd / JsxSelfClosingEnd.
    let mut i = self.pos;
    let mut depth = 0i32;
    while let Some(t) = self.tokens.get(i) {
      match t.kind {
        TokenKind::JsxOpenTagStart | TokenKind::JsxCloseTagStart => depth += 1,
        TokenKind::JsxOpenTagEnd | TokenKind::JsxCloseTagEnd | TokenKind::JsxSelfClosingEnd => {
          depth -= 1;
          if depth == 0 {
            i += 1;
            break;
          }
        },
        _ => {},
      }
      i += 1;
    }
    // Now scan to end-of-line; tolerate Whitespace, reject anything else.
    while let Some(t) = self.tokens.get(i) {
      match &t.kind {
        TokenKind::SoftBreak | TokenKind::HardBreak | TokenKind::BlankLine | TokenKind::Eof => return true,
        TokenKind::Whitespace(_) => i += 1,
        _ => return false,
      }
    }
    true
  }

  /// Reconstruct a raw HTML block from a JSX-tokenized stream. Type-1
  /// closes on the matching `</tag>`; Type-6 closes on the next blank
  /// line. Captures the verbatim source span from the first token's
  /// start to the closer's end so internal whitespace and attribute
  /// formatting survive intact (the lexer's JSX path normalizes
  /// whitespace, so per-token concat alone would drop it).
  fn parse_html_block_from_jsx(&mut self, mode: HtmlBlockMode) -> Node {
    let span = self.current_span();
    let start_ptr = self.tokens.get(self.pos).map(|t| t.raw.as_ptr() as usize).unwrap_or(0);
    match mode {
      HtmlBlockMode::Type1(tag) => loop {
        match self.peek_kind() {
          Some(TokenKind::JsxCloseTagStart) => {
            self.advance();
            let close_name = match self.peek() {
              Some(t) if matches!(t.kind, TokenKind::JsxTagName) => {
                let n = t.raw.to_ascii_lowercase();
                self.advance();
                n
              },
              _ => String::new(),
            };
            if matches!(self.peek_kind(), Some(TokenKind::JsxCloseTagEnd)) {
              self.advance();
            }
            if close_name == tag {
              break;
            }
          },
          Some(TokenKind::Eof) | None => break,
          _ => {
            self.advance();
          },
        }
      },
      HtmlBlockMode::Type6 | HtmlBlockMode::Type7 => loop {
        match self.peek_kind() {
          Some(TokenKind::BlankLine) | Some(TokenKind::Eof) | None => break,
          _ => {
            self.advance();
          },
        }
      },
    }
    let end_ptr =
      self.tokens.get(self.pos.saturating_sub(1)).map(|t| t.raw.as_ptr() as usize + t.raw.len()).unwrap_or(start_ptr);
    let value = if end_ptr > start_ptr {
      // SAFETY: every Token.raw borrows from the same `&'src str`
      // source; pointer subtraction stays within that buffer.
      let len = end_ptr - start_ptr;
      let slice = unsafe { std::slice::from_raw_parts(start_ptr as *const u8, len) };
      std::str::from_utf8(slice).map(|s| s.to_string()).unwrap_or_default()
    } else {
      String::new()
    };
    Node::Html(Html { value, span })
  }

  /// GFM footnote definition: cursor at `FootnoteDefMarker`. The marker
  /// token's raw lexeme is `[^id]: ` (trailing space included by the
  /// lexer); body is the inline run that follows up to the next break.
  fn parse_footnote_def(&mut self) -> Node {
    let span = self.current_span();
    let raw = self.peek().map(|t| t.raw.to_string()).unwrap_or_default();
    self.advance();
    let id = raw.trim_start_matches('[').trim_start_matches('^').split(']').next().unwrap_or("").to_string();
    let children = self.collect_inline_until_break();
    Node::FootnoteDef(FootnoteDef { id, children, span })
  }

  /// Raw HTML block (CM 4.6 types 2-5). Lexer flagged the open token
  /// with the type discriminator; we capture the entire span verbatim
  /// (open + body + close) into a single `Html` node.
  /// CM 4.6 type-2: HTML comment as a block (cursor on
  /// `HtmlCommentOpen` at col 0). Slurps tokens through the matching
  /// `HtmlCommentClose` and emits a single `Html` node containing the
  /// verbatim source. The block extends to a blank line if the close
  /// never fires on the same line.
  fn parse_html_comment_block(&mut self) -> Node {
    let span = self.current_span();
    let mut value = String::new();
    if let Some(t) = self.peek() {
      value.push_str(t.raw);
    }
    self.advance();
    let mut closed = false;
    loop {
      match self.peek_kind() {
        Some(TokenKind::HtmlCommentClose) => {
          if let Some(t) = self.peek() {
            value.push_str(t.raw);
          }
          self.advance();
          closed = true;
        },
        Some(TokenKind::BlankLine) | Some(TokenKind::Eof) | None => break,
        Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak) => {
          if !closed {
            // Comment still open: absorb the newline verbatim and keep
            // slurping into the next line.
            value.push('\n');
            self.advance();
            continue;
          }
          // CM 4.6 type-2: block ends at the end of the line that
          // contains `-->`. Stop here so the next line opens a fresh
          // block.
          break;
        },
        _ => {
          if let Some(t) = self.peek() {
            value.push_str(t.raw);
          }
          self.advance();
        },
      }
    }
    Node::Html(Html { value, span })
  }

  fn parse_html_block(&mut self) -> Node {
    let span = self.current_span();
    let mut value = String::new();
    if let Some(t) = self.peek() {
      value.push_str(t.raw);
    }
    self.advance();
    loop {
      match self.peek_kind() {
        Some(TokenKind::HtmlBlockClose) => {
          if let Some(t) = self.peek() {
            value.push_str(t.raw);
          }
          self.advance();
          break;
        },
        Some(TokenKind::Eof) | None => break,
        _ => {
          if let Some(t) = self.peek() {
            value.push_str(t.raw);
          }
          self.advance();
        },
      }
    }
    Node::Html(Html { value, span })
  }

  /// 4-space indented code block (CM 4.4). Lexer pre-classifies a valid
  /// indent line as `Whitespace(>=4) + IndentedCodeLine`; this method
  /// concatenates consecutive pairs, joining with `\n` and stopping at
  /// the first non-indented line.
  fn parse_indented_code(&mut self) -> Node {
    let span = self.current_span();
    let mut buf = String::new();
    loop {
      let starts_indent = matches!(self.peek_kind(), Some(TokenKind::Whitespace(_)))
        && matches!(self.tokens.get(self.pos + 1).map(|t| &t.kind), Some(TokenKind::IndentedCodeLine));
      if !starts_indent {
        break;
      }
      // The lexer's Whitespace covers the entire leading run; CM 4.4
      // strips exactly 4 spaces (or 1 tab) and keeps the rest as part
      // of the rendered body. Compute the leftover indent from the
      // whitespace token's raw byte count (each space = 1 col, tab
      // expands to next 4-stop; a single tab fully consumes the
      // 4-space strip).
      let extra = self
        .peek()
        .map(|t| {
          if t.raw.starts_with('\t') {
            // Tab fills first 4 cols. Remaining chars are extras.
            t.raw.len() - 1
          } else {
            t.raw.len().saturating_sub(4)
          }
        })
        .unwrap_or(0);
      self.advance();
      let mut body = match self.peek() {
        Some(t) if matches!(t.kind, TokenKind::IndentedCodeLine) => {
          let raw = t.raw.to_string();
          self.advance();
          raw
        },
        // Fallback: pre-rewrite path where the lexer didn't pre-classify
        // (paragraph context, mid-list, etc.). Walk inline tokens until
        // the next break.
        _ => {
          let mut s = String::new();
          while let Some(t) = self.peek() {
            match &t.kind {
              TokenKind::SoftBreak | TokenKind::HardBreak | TokenKind::BlankLine | TokenKind::Eof => break,
              _ => {
                s.push_str(t.raw);
                self.advance();
              },
            }
          }
          s
        },
      };
      // Prefix any leftover indent (whitespace beyond the 4-space
      // strip) so deeper-indented code lines render with the visible
      // extra leading spaces.
      if extra > 0 {
        body = " ".repeat(extra) + &body;
      }
      buf.push_str(&body);
      buf.push('\n');
      // Continue across a soft break only if the next line is also
      // indented. CM 4.4 also keeps blank lines inside the block when a
      // later line resumes the indent; pick that up by buffering blanks
      // and only emitting them when an indented line follows.
      let saved = self.pos;
      let mut blanks: usize = 0;
      loop {
        match self.peek_kind() {
          Some(TokenKind::SoftBreak) => {
            self.advance();
            blanks += 1;
            break;
          },
          Some(TokenKind::BlankLine) => {
            self.advance();
            blanks += 2; // BlankLine collapses >=2 newlines
          },
          _ => break,
        }
      }
      if blanks == 0 {
        break;
      }
      let next_is_indent = matches!(self.peek_kind(), Some(TokenKind::Whitespace(_)))
        && matches!(self.tokens.get(self.pos + 1).map(|t| &t.kind), Some(TokenKind::IndentedCodeLine));
      if !next_is_indent {
        self.pos = saved;
        break;
      }
      // Push the buffered blank-line newlines (the body already ended
      // with one `\n` for the previous line, so add `blanks - 1`).
      for _ in 1..blanks {
        buf.push('\n');
      }
    }
    Node::CodeBlock(CodeBlock { lang: None, meta: None, value: buf, span })
  }

  /// Default fallback block. Also handles setext headings: a trailing soft
  /// break followed by a run of `=` or `-` rewrites the paragraph as a
  /// level-1 / level-2 `Heading`.
  ///
  /// CommonMark: a soft break inside a paragraph stays inside the
  /// paragraph (becomes a literal newline in the text). Only a blank
  /// line or a block-starter token closes the paragraph.
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
          | TokenKind::JsxCloseTagStart
      )
    };
    self.collect_inline_into(&para_stop, &mut children, &mut delims);
    // Setext H2 retro-fold: when the inline run ends with a hard break
    // followed by a Text node consisting only of `-` characters, treat
    // the run as the body of an `<h2>`. CM 4.3 allows trailing
    // whitespace before the underline; the hard break captured those
    // spaces.
    if children.len() >= 2
      && let Some(Node::Text(t)) = children.last()
      && !t.value.is_empty()
      && t.value.chars().all(|c| c == '-')
      && matches!(children.get(children.len() - 2), Some(Node::HardBreak(_)))
    {
      children.pop(); // text "----"
      children.pop(); // hard break
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
      }
      return Node::Heading(Heading { level: 2, children, span, id: None });
    }
    loop {
      if !matches!(self.peek_kind(), Some(TokenKind::SoftBreak)) {
        break;
      }
      let saved = self.pos;
      self.advance(); // consume the SoftBreak
      // Setext heading: `=`/`-` underline directly after the soft break,
      // possibly preceded by 1-3 leading spaces.
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
        }
        // Trim trailing whitespace-only text nodes from the heading.
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
      // Blank line (another break right away) closes the paragraph.
      if matches!(
        self.peek_kind(),
        Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak) | Some(TokenKind::Eof) | None
      ) {
        self.pos = saved;
        break;
      }
      // A block-starter token following the soft break also closes the
      // paragraph -- heading, list item, blockquote, code fence, JSX
      // root, frontmatter, etc.
      // CM 5.2: an ordered list with start != 1 cannot interrupt a
      // paragraph, so check the marker's start digit before treating
      // it as a block boundary.
      let next_is_ol_interrupting = match self.peek() {
        Some(t) if matches!(t.kind, TokenKind::OrderedListMarker(_)) => {
          let digits: String = t.raw.chars().take_while(|c| c.is_ascii_digit()).collect();
          digits.parse::<u32>().map(|n| n == 1).unwrap_or(false)
        },
        _ => false,
      };
      // CM 5.2: an unordered list interrupting a paragraph requires
      // non-empty content immediately after the marker.
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
      let next_is_block = matches!(
        self.peek_kind(),
        Some(TokenKind::Heading(_))
          | Some(TokenKind::BlockQuoteMarker)
          | Some(TokenKind::CodeFenceOpen(_, _))
          | Some(TokenKind::ThematicBreak)
          | Some(TokenKind::JsxOpenTagStart)
          | Some(TokenKind::FrontmatterStart(_))
          | Some(TokenKind::Import)
          | Some(TokenKind::Export)
      ) || next_is_ol_interrupting
        || next_is_ul_interrupting;
      if next_is_block {
        self.pos = saved;
        break;
      }
      // Same-paragraph continuation: keep the soft break as a literal
      // newline inside the text and collect the next line's inlines.
      let break_span = self.current_span();
      children.push(Node::SoftBreak(BreakNode { span: break_span }));
      let pre_len = children.len();
      self.collect_inline_into(&para_stop, &mut children, &mut delims);
      if children.len() == pre_len {
        // Nothing useful followed; rewind so the soft break we ate
        // becomes a separate empty-line marker.
        self.pos = saved;
        children.pop();
        break;
      }
    }
    if !delims.is_empty() {
      crate::inline::resolve_emphasis_delims(&mut children, &mut delims);
    }
    Self::finalize_inline_breaks(&mut children);
    Node::Paragraph(Paragraph { children, span })
  }

  /// CM 6.7: a hard line break needs content after it. Strip a stripped
  /// `\` from the prev `Text` for mid-paragraph breaks; drop a trailing
  /// `HardBreak` (plus any trailing whitespace-only text) so paragraphs
  /// like `foo\` render as literal `foo\`.
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
  }

  /// `Some(1)` for an `=` underline, `Some(2)` for a `-` underline, else
  /// `None`. Cursor is left untouched.
  fn setext_underline_level(&self) -> Option<u8> {
    let t = self.tokens.get(self.pos)?;
    match &t.kind {
      TokenKind::SetextUnderline(_) => Some(1),
      TokenKind::ThematicBreak => {
        // CM 4.3: setext H2 = run of `-` plus optional trailing
        // whitespace. Trim trailing ws then verify all-dashes.
        let trimmed = t.raw.trim_end_matches([' ', '\t']);
        if !trimmed.is_empty() && trimmed.chars().all(|c| c == '-') { Some(2) } else { None }
      },
      _ => None,
    }
  }

  /// Consume the underline tokens that `setext_underline_level` matched.
  fn eat_setext_underline(&mut self) {
    if let Some(t) = self.tokens.get(self.pos) {
      match t.kind {
        TokenKind::SetextUnderline(_) | TokenKind::ThematicBreak => {
          self.advance();
        },
        _ => {},
      }
    }
  }

  /// Fenced code block. The first inline `Text` becomes the info string; the
  /// body is concatenated until the matching `CodeEnd(n)`. The info string
  /// splits at the first whitespace into `(lang, meta)`.
  fn parse_code_block(&mut self) -> Node {
    let span = self.current_span();
    let (fence_char, fence_n, fence_indent) = match self.peek() {
      Some(t) => match t.kind {
        TokenKind::CodeFenceOpen(c, n) => (c, n, t.span.column.saturating_sub(1)),
        _ => (dmc_lexer::token::FenceChar::Backtick, 3, 0),
      },
      None => (dmc_lexer::token::FenceChar::Backtick, 3, 0),
    };
    self.advance();

    let info = match self.peek() {
      Some(t) if matches!(t.kind, TokenKind::CodeFenceInfo | TokenKind::Text) => {
        let raw = t.raw.to_string();
        self.advance();
        raw
      },
      _ => String::new(),
    };
    let info_trimmed = info.trim();
    let (lang, meta) = if info_trimmed.is_empty() {
      (None, None)
    } else {
      match info_trimmed.split_once(char::is_whitespace) {
        Some((l, rest)) => {
          let rest = rest.trim();
          (Some(Self::unescape_markdown(l)), if rest.is_empty() { None } else { Some(Self::unescape_markdown(rest)) })
        },
        None => (Some(Self::unescape_markdown(info_trimmed)), None),
      }
    };

    let mut value = String::new();
    while let Some(t) = self.peek() {
      match &t.kind {
        TokenKind::CodeFenceClose(c, m) if *c == fence_char && *m >= fence_n => {
          self.advance();
          break;
        },
        TokenKind::Eof => break,
        TokenKind::Text => {
          value.push_str(t.raw);
          self.advance();
        },
        _ => {
          value.push_str(t.raw);
          self.advance();
        },
      }
    }

    // CM 4.5: fenced code-block content ends with a newline. The lexer
    // strips the newline that precedes the closing fence; restore it
    // so renderers emit `<pre><code>...\n</code></pre>` per spec.
    if !value.is_empty() && !value.ends_with('\n') {
      value.push('\n');
    }
    // CM 4.5: a fence with N leading spaces strips up to N spaces of
    // leading indent from every content line (capped at the actual
    // run, never deeper than the line's own leading whitespace).
    if fence_indent > 0 {
      let stripped = value
        .split_inclusive('\n')
        .map(|line| {
          let mut consumed = 0usize;
          let bytes = line.as_bytes();
          while consumed < fence_indent && consumed < bytes.len() && bytes[consumed] == b' ' {
            consumed += 1;
          }
          &line[consumed..]
        })
        .collect::<String>();
      value = stripped;
    }
    Node::CodeBlock(CodeBlock { lang, meta, value, span })
  }
}
