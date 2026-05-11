use crate::ast::*;
use crate::parser::Parser;
use dmc_lexer::token::TokenKind;

mod blockquote;
mod code;
mod heading;
mod html;

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

    // CM 4.8: a `Whitespace + SoftBreak | BlankLine | Eof` pair at col 0
    // is a blank-with-whitespace line. It produces no block; advance
    // and yield None so it acts like a normal blank line.
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

    // Fallback indented code: lexer didn't classify (because the prev
    // token was inline content, eg a bq paragraph) but at top-level
    // dispatch we know we're between blocks. Whitespace(>=4) followed
    // by non-block content is an indented code block here.
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
      if next_is_inline {
        return Some(self.parse_indented_code_fallback());
      }
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
        TokenKind::HtmlCommentOpen => {
          self.advance();
          return Some(self.parse_html_comment_block());
        },
        TokenKind::JsxOpenTagStart | TokenKind::JsxCloseTagStart => {
          // Peek the JSX tag two tokens ahead (after the leading
          // whitespace) to see if it routes to a Type-1 / Type-6 raw
          // HTML block. Don't consume the Whitespace yet so the block's
          // verbatim source slice includes the leading indent (CM 4.6
          // preserves the original spaces in the rendered output).
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
          self.advance(); // skip whitespace
          if valid_ref_def {
            self.advance(); // skip the ref-def itself
            return None;
          }
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
        } else if self.is_lowercase_jsx_tag() {
          // Lowercase HTML-ish tag that doesn't match any block type
          // -- treat as inline raw HTML inside a paragraph.
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
    let mut saw_blank_between_items = false;
    loop {
      // First iteration: caller has already advanced past any indent
      // whitespace; cursor is on the marker. Subsequent iterations: for
      // nested lists (indent > 0) require a `Whitespace` of width `indent`
      // before the next marker - a marker at a smaller indent belongs to
      // an outer list.
      let mut item_indent = if first {
        // Capture the leading whitespace (if any) consumed by the
        // caller right before the marker. parse_block's leading-ws
        // dispatch advances past the Whitespace token before calling
        // parse_list, so tokens[pos-1] holds the actual indent.
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
      let marker_raw = self.peek_raw().unwrap_or("");
      let marker_raw_width = marker_raw.chars().count();
      let marker_raw_has_ws = marker_raw.ends_with([' ', '\t']);
      let following_ws = match self.tokens.get(self.pos + 1) {
        Some(t) if matches!(t.kind, TokenKind::Whitespace(_)) => t.raw.chars().count(),
        _ => 0,
      };
      // Promoted text markers (e.g. nested `-` recovered by the parser)
      // keep the bare marker lexeme, so the first following space is part
      // of the marker width rather than extra item indentation.
      let marker_width = if !marker_raw_has_ws && following_ws > 0 { marker_raw_width + 1 } else { marker_raw_width };
      self.advance();

      // Track extra whitespace between the marker and the first non-blank
      // content. CM 5.2 + 2.2: when 0-4 visual cols of extra, content_indent
      // grows by that count; 5+ folds back to 1 (content becomes an indented
      // code line inside the item). Tabs snap to the 4-col stop, so use
      // visual cols rather than byte count.
      let extra_ws = self
        .peek_leading_indent()
        .map(|n| if !marker_raw_has_ws && marker_raw_width > 0 { n.saturating_sub(1) } else { n })
        .unwrap_or(0);
      let item_content_extra = if extra_ws >= 4 { 0 } else { extra_ws };

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
              self.advance(); // skip the continuation indent
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
              self.advance(); // skip the continuation indent
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

      // CM 5.3 blank-line handling. A list item may keep consuming
      // blankline-separated child blocks until a sibling marker (or a
      // de-dented block) finally breaks it.
      'after_item: while matches!(self.peek_kind(), Some(TokenKind::BlankLine)) {
        let saved = self.pos;
        self.advance();
        let leading = self.peek_leading_indent();
        let last_item_is_empty = items.last().is_some_and(|item| match item {
          Node::ListItem(li) => li.children.is_empty(),
          Node::TaskListItem(t) => t.children.is_empty(),
          _ => false,
        });
        // Indented continuation -- attach a new paragraph to the
        // current item (loose-list with continuation).
        let content_indent = content_floor;
        if let Some(n) = leading
          && !last_item_is_empty
          && n >= content_indent
        {
          // CM 5.2: continuation indented >= content_indent + 4 is an
          // indented code block inside the item.
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
            continue 'after_item;
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
          self.advance(); // skip whitespace
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
          // Peek leading whitespace before the next marker without
          // consuming it: the outer loop's leading-ws check will use
          // that width as the next item's item_indent (for content_floor).
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
            // Rewind to the leading-whitespace position so the next
            // outer-loop iteration captures it as item_indent.
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

    // CM 5.3 loose list: a list is loose when any blank line appears
    // between items (or inside an item between block children). We track
    // the inter-item case during parsing; the per-item case shows up
    // here as items already containing a Paragraph child.
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

  /// Width of the leading `Whitespace` token at the cursor, in spaces. Tabs
  /// count as 1 column for the purposes of comparing against a parent
  /// marker's indent. `None` when the cursor is not on a Whitespace token.
  fn peek_leading_indent(&self) -> Option<usize> {
    match self.peek() {
      Some(t) if matches!(t.kind, TokenKind::Whitespace(_)) => {
        // CM 2.2: tabs snap to the next 4-col stop. Compute visual
        // width starting from the token's actual column so a
        // post-marker tab (eg `> \t`) lands on the right multiple.
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

  /// Some deeper-indent list markers are lexed as plain `Text` + trailing
  /// `Whitespace` once they move past the lexer's top-level block-start fast
  /// path. Reclassify them in-place so nested list parsing can still recurse.
  fn try_promote_text_list_marker(&mut self) -> bool {
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

  /// `>` after a list marker can be lexed as plain text once we're already
  /// inside list-item parsing. Recover it so the item can start with a
  /// blockquote.
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

  /// Append `child` to the children of `item` (works for both `ListItem`
  /// and `TaskListItem`).
  fn append_to_item(item: &mut Node, child: Node) {
    match item {
      Node::ListItem(li) => li.children.push(child),
      Node::TaskListItem(t) => t.children.push(child),
      _ => {},
    }
  }

  /// Continuation lines without an intervening blank line stay inside the
  /// current paragraph; they do not immediately make the list item loose.
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

  /// Wrap any LEADING raw inline content of `item` in a `Paragraph` so
  /// loose-list formatting (`<li><p>...</p><p>...</p></li>`) applies
  /// once a continuation paragraph or nested block appears.
  ///
  /// Only contiguous inline-typed children at the front of the list are
  /// promoted; existing block children (Paragraph, List, Blockquote,
  /// CodeBlock, ...) are left in place. Otherwise a `[Text, List]` item
  /// would collapse into a single paragraph that swallows the list.
  fn ensure_loose_item(item: &mut Node, span: &duck_diagnostic::Span) {
    let promote = |kids: &mut Vec<Node>, span: &duck_diagnostic::Span| {
      // Already loose? Bail.
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

  /// Convert an inline-only list item into a setext heading. Tight-list
  /// items keep following paragraph text as raw inline siblings, so we
  /// only fold the leading inline run here.
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

  /// After consuming a continuation block inside a list item, a
  /// following blank line may still separate this item from the next
  /// marker in the SAME list. Keep the cursor on that next marker so
  /// the outer list loop can continue instead of splitting the list.
  fn try_continue_same_list_after_blankline(
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
          if self.options.legacy_gfm_emphasis {
            crate::inline::normalize_legacy_gfm_emphasis(&mut children);
          }
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
          let starts_at_one = digits.parse::<u32>().map(|n| n == 1).unwrap_or(false);
          // CM 5.2: a list item with no content after the marker
          // cannot interrupt a paragraph (matches the unordered rule).
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
      // CM 4.6: only HTML block types 1-6 can interrupt a paragraph;
      // type-7 (any other lowercase tag) is inline raw HTML and stays
      // in the running paragraph. MDX components (uppercase /
      // namespaced) always start a JSX block.
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
      if self.options.legacy_gfm_emphasis {
        crate::inline::normalize_legacy_gfm_emphasis(&mut children);
      }
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
    // CM 4.8: trailing whitespace-only Text + SoftBreak runs at the end
    // of a paragraph render as nothing. Strip them so blank-line padding
    // before block boundaries doesn't leak into the rendered output.
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
