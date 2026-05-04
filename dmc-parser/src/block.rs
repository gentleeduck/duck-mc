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
            let span = self.current_span();
            let inline = self.collect_inline_until_break();
            if inline.is_empty() {
              self.pos = saved;
              break;
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

  /// If `item`'s body is still raw inline content, wrap it in a `Paragraph`
  /// so loose-list formatting (`<li><p>...</p><p>...</p></li>`) applies
  /// once a continuation paragraph appears.
  fn ensure_loose_item(item: &mut Node, span: &duck_diagnostic::Span) {
    let take_kids = |kids: &mut Vec<Node>| {
      if kids.iter().any(|n| matches!(n, Node::Paragraph(_))) {
        return None;
      }
      let inline = std::mem::take(kids);
      Some(inline)
    };
    match item {
      Node::ListItem(li) => {
        if let Some(inline) = take_kids(&mut li.children) {
          li.children.push(Node::Paragraph(Paragraph { children: inline, span: span.clone() }));
        }
      },
      Node::TaskListItem(t) => {
        if let Some(inline) = take_kids(&mut t.children) {
          t.children.push(Node::Paragraph(Paragraph { children: inline, span: span.clone() }));
        }
      },
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
      let inline = self.collect_inline_until_break();
      let break_kind = self.peek_kind().cloned();
      if matches!(break_kind, Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
        self.advance();
      }
      let top = children.len() - 1;
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
    let children = self.collect_inline_until_break();
    Node::Heading(Heading { level, children, span })
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
  fn parse_paragraph(&mut self) -> Node {
    let span = self.current_span();
    let children = self.collect_inline_until_break();
    if matches!(self.peek_kind(), Some(TokenKind::SoftBreak)) {
      let saved = self.pos;
      self.advance();
      if let Some(lvl) = self.setext_underline_level() {
        self.eat_setext_underline();
        return Node::Heading(Heading { level: lvl, children, span });
      }
      self.pos = saved;
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
