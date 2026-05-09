use crate::ast::*;
use crate::parser::Parser;
use dmc_lexer::token::TokenKind;

impl<'eng, 'tokens> Parser<'eng, 'tokens> {
  /// Top-down dispatch: peek one token, route to the matching block parser.
  /// `None` means the cursor advanced but emitted no node (e.g. a stray break
  /// or a markdown comment).
  pub(crate) fn parse_block(&mut self) -> Option<Node> {
    // Indented code: 4+ leading spaces AND the next non-whitespace token is
    // not a list marker (otherwise this is nested-list indentation).
    let is_indented = matches!(
        self.peek(),
        Some(t) if matches!(t.kind, TokenKind::Whitespace) && t.raw.starts_with("    ")
    );

    if is_indented {
      let next_kind = self.tokens.get(self.pos + 1).map(|t| t.kind.clone());
      if !matches!(next_kind, Some(TokenKind::UnorderedListItem) | Some(TokenKind::OrderedListItem)) {
        return Some(self.parse_indented_code());
      }
    }

    // Whitespace-then-list-marker: nested list at top level.
    if matches!(self.peek_kind(), Some(TokenKind::Whitespace))
      && let Some(next) = self.tokens.get(self.pos + 1)
    {
      match next.kind {
        TokenKind::UnorderedListItem => {
          self.advance();
          return Some(self.parse_list(false, 0));
        },
        TokenKind::OrderedListItem => {
          self.advance();
          return Some(self.parse_list(true, 0));
        },
        _ => {},
      }
    }

    match self.peek_kind()? {
      TokenKind::FrontmatterStart => Some(self.parse_frontmatter()),
      TokenKind::Import => Some(self.import_node()),
      TokenKind::Export => Some(self.export_node()),
      TokenKind::Heading(_) => Some(self.parse_heading()),
      TokenKind::CodeStart(n) if *n >= 3 => Some(self.parse_code_block()),
      TokenKind::JsxOpenTagStart => Some(self.parse_jsx()),
      TokenKind::ExpressionStart => Some(self.parse_jsx_expression()),
      TokenKind::MarkdownCommentStart => {
        self.skip_md_comment();
        None
      },
      TokenKind::UnorderedListItem => Some(self.parse_list(false, 0)),
      TokenKind::OrderedListItem => Some(self.parse_list(true, 0)),
      TokenKind::BlockQuote => Some(self.parse_blockquote()),
      TokenKind::ThematicBreak => {
        let span = self.current_span();
        self.advance();
        Some(Node::HorizontalRule(HorizontalRule { span }))
      },
      TokenKind::HardBreak | TokenKind::SoftBreak => {
        self.advance();
        None
      },
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
    if matches!(self.peek_kind(), Some(TokenKind::FrontmatterEnd)) {
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
    let start: Option<u32> =
      if ordered { self.peek().and_then(|t| t.raw.trim_end_matches('.').parse::<u32>().ok()) } else { None };

    let mut first = true;
    loop {
      // First iteration: caller has already advanced past any indent
      // whitespace; cursor is on the marker. Subsequent iterations: for
      // nested lists (indent > 0) require a `Whitespace` of width `indent`
      // before the next marker - a marker at a smaller indent belongs to
      // an outer list.
      if !first && indent > 0 {
        let aligned =
          matches!(self.peek(), Some(t) if matches!(t.kind, TokenKind::Whitespace) && t.raw.chars().count() == indent);
        if !aligned {
          break;
        }
        let next = self.tokens.get(self.pos + 1);
        let next_is_marker =
          matches!(next.map(|t| t.kind.clone()), Some(TokenKind::UnorderedListItem) | Some(TokenKind::OrderedListItem));
        if !next_is_marker {
          break;
        }
        self.advance();
      }
      first = false;

      let kind = match self.peek_kind() {
        Some(k) => k,
        None => break,
      };
      let want_marker =
        if ordered { matches!(kind, TokenKind::OrderedListItem) } else { matches!(kind, TokenKind::UnorderedListItem) };
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
      while let Some(child_indent) = self.peek_leading_indent() {
        if child_indent <= indent {
          break;
        }
        let saved = self.pos;
        self.advance();
        match self.peek_kind() {
          Some(TokenKind::UnorderedListItem) => {
            let sub = self.parse_list(false, child_indent);
            Self::append_to_item(&mut item, sub);
          },
          Some(TokenKind::OrderedListItem) => {
            let sub = self.parse_list(true, child_indent);
            Self::append_to_item(&mut item, sub);
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
                Some(t) if matches!(t.kind, TokenKind::Whitespace) => {
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
                Some(TokenKind::UnorderedListItem)
                  | Some(TokenKind::OrderedListItem)
                  | Some(TokenKind::Heading(_))
                  | Some(TokenKind::BlockQuote)
                  | Some(TokenKind::CodeStart(_))
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
      Some(t) if matches!(t.kind, TokenKind::Whitespace) => Some(t.raw.chars().count()),
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
      if matches!(self.peek_kind(), Some(TokenKind::Whitespace)) {
        self.advance();
      }
      if matches!(self.peek_kind(), Some(TokenKind::Bracket)) {
        self.advance();
        let text_raw = self.peek().map(|t| t.raw.to_string()).unwrap_or_default();
        let kind = self.peek_kind().cloned();
        if matches!(kind, Some(TokenKind::Text)) && (text_raw == " " || text_raw.eq_ignore_ascii_case("x")) {
          self.advance();
          if matches!(self.peek_kind(), Some(TokenKind::Bracket)) {
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
        break;
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
      if matches!(after_marker_kind, TokenKind::UnorderedListItem | TokenKind::OrderedListItem) {
        if !paragraphs[top].is_empty() {
          let para = std::mem::take(&mut paragraphs[top]);
          children[top].push(Node::Paragraph(Paragraph { children: para, span: para_span.clone() }));
        }
        let ordered = matches!(after_marker_kind, TokenKind::OrderedListItem);
        let list = self.parse_list_in_blockquote(ordered, line_markers);
        children[top].push(list);
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
    let start: Option<u32> =
      if ordered { self.peek().and_then(|t| t.raw.trim_end_matches('.').parse::<u32>().ok()) } else { None };
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
      let want_marker =
        if ordered { matches!(kind, TokenKind::OrderedListItem) } else { matches!(kind, TokenKind::UnorderedListItem) };
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
        TokenKind::BlockQuote => {
          count += 1;
          i += 1;
        },
        TokenKind::Whitespace => {
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
      if matches!(self.peek_kind(), Some(TokenKind::Whitespace)) {
        self.advance();
      }
      if !matches!(self.peek_kind(), Some(TokenKind::BlockQuote)) {
        break;
      }
      self.advance();
      taken += 1;
    }
    if matches!(self.peek_kind(), Some(TokenKind::Whitespace)) {
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
    if let Some(Node::Text(t)) = children.last_mut() {
      let trimmed = t.value.trim_end_matches([' ', '\t']).to_string();
      if trimmed.is_empty() {
        children.pop();
      } else {
        t.value = trimmed;
      }
    }
    Node::Heading(Heading { level, children, span, id: None })
  }

  /// 4-space indented code block. Strips the leading 4 spaces from each line
  /// and joins with `\n`. Stops at the first non-indented line.
  fn parse_indented_code(&mut self) -> Node {
    let span = self.current_span();
    let mut buf = String::new();
    loop {
      let starts_indent = matches!(
          self.peek(),
          Some(t) if matches!(t.kind, TokenKind::Whitespace) && t.raw.starts_with("    ")
      );
      if !starts_indent {
        break;
      }
      let leading = self.peek().map(|t| t.raw[4..].to_string()).unwrap_or_default();
      self.advance();
      buf.push_str(&leading);
      loop {
        let next_kind = self.peek().map(|t| t.kind.clone());
        match next_kind {
          Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak) | None => break,
          Some(_) => {
            let raw = self.peek().map(|t| t.raw.to_string()).unwrap_or_default();
            buf.push_str(&raw);
            self.advance();
          },
        }
      }
      buf.push('\n');
      let break_kind = self.peek().map(|t| t.kind.clone());
      match break_kind {
        Some(TokenKind::SoftBreak) => {
          let saved = self.pos;
          self.advance();
          let next_is_indent = matches!(
              self.peek(),
              Some(t) if matches!(t.kind, TokenKind::Whitespace) && t.raw.starts_with("    ")
          );
          if !next_is_indent {
            self.pos = saved;
            break;
          }
        },
        _ => break,
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
    let mut children = self.collect_inline_until_break();
    loop {
      if !matches!(self.peek_kind(), Some(TokenKind::SoftBreak)) {
        break;
      }
      let saved = self.pos;
      self.advance(); // consume the SoftBreak
      // Setext heading: `=`/`-` underline directly after the soft break.
      if let Some(lvl) = self.setext_underline_level() {
        self.eat_setext_underline();
        return Node::Heading(Heading { level: lvl, children, span, id: None });
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
      // paragraph — heading, list item, blockquote, code fence, JSX
      // root, frontmatter, etc.
      let next_is_block = matches!(
        self.peek_kind(),
        Some(TokenKind::Heading(_))
          | Some(TokenKind::UnorderedListItem)
          | Some(TokenKind::OrderedListItem)
          | Some(TokenKind::BlockQuote)
          | Some(TokenKind::CodeStart(_))
          | Some(TokenKind::ThematicBreak)
          | Some(TokenKind::JsxOpenTagStart)
          | Some(TokenKind::FrontmatterStart)
          | Some(TokenKind::Import)
          | Some(TokenKind::Export)
      );
      if next_is_block {
        self.pos = saved;
        break;
      }
      // Same-paragraph continuation: keep the soft break as a literal
      // newline inside the text and collect the next line's inlines.
      let break_span = self.current_span();
      children.push(Node::SoftBreak(BreakNode { span: break_span }));
      let more = self.collect_inline_until_break();
      if more.is_empty() {
        // Nothing useful followed; rewind so the soft break we ate
        // becomes a separate empty-line marker.
        self.pos = saved;
        children.pop();
        break;
      }
      children.extend(more);
    }
    Node::Paragraph(Paragraph { children, span })
  }

  /// `Some(1)` for an `=` underline, `Some(2)` for a `-` underline, else
  /// `None`. Cursor is left untouched.
  fn setext_underline_level(&self) -> Option<u8> {
    let t = self.tokens.get(self.pos)?;
    match &t.kind {
      TokenKind::Eq => {
        let mut i = self.pos;
        while let Some(tt) = self.tokens.get(i) {
          if matches!(tt.kind, TokenKind::Eq) {
            i += 1;
          } else {
            break;
          }
        }
        let next = self.tokens.get(i).map(|t| &t.kind);
        if matches!(next, Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak) | Some(TokenKind::Eof) | None) {
          Some(1)
        } else {
          None
        }
      },
      TokenKind::ThematicBreak => {
        if !t.raw.is_empty() && t.raw.chars().all(|c| c == '-') {
          Some(2)
        } else {
          None
        }
      },
      _ => None,
    }
  }

  /// Consume the underline tokens that `setext_underline_level` matched.
  fn eat_setext_underline(&mut self) {
    if let Some(t) = self.tokens.get(self.pos) {
      match t.kind {
        TokenKind::Eq => {
          while matches!(self.peek_kind(), Some(TokenKind::Eq)) {
            self.advance();
          }
        },
        TokenKind::ThematicBreak => {
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
    let fence_n = match self.peek_kind() {
      Some(TokenKind::CodeStart(n)) => *n,
      _ => 3,
    };
    self.advance();

    let info = match self.peek() {
      Some(t) if matches!(t.kind, TokenKind::Text) => {
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
          (Some(l.to_string()), if rest.is_empty() { None } else { Some(rest.to_string()) })
        },
        None => (Some(info_trimmed.to_string()), None),
      }
    };

    let mut value = String::new();
    while let Some(t) = self.peek() {
      match &t.kind {
        TokenKind::CodeEnd(m) if *m == fence_n => {
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

    Node::CodeBlock(CodeBlock { lang, meta, value, span })
  }
}
