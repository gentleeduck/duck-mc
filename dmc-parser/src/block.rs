use crate::ast::*;
use crate::parser::Parser;
use dmc_lexer::token::TokenKind;

impl<'eng, 'tokens> Parser<'eng, 'tokens> {
  /// Top-down dispatch: peek one token, route to the matching block parser.
  /// `None` means the cursor advanced but emitted no node (e.g. a stray break
  /// or a markdown comment).
  pub(crate) fn parse_block(&mut self) -> Option<Node> {
    let is_indented = matches!(
        self.peek(),
        Some(t) if matches!(t.kind, TokenKind::Whitespace) && t.raw.starts_with("    ")
    );

    if is_indented {
      return Some(self.parse_indented_code());
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
      TokenKind::UnorderedListItem => Some(self.parse_list(false)),
      TokenKind::OrderedListItem => Some(self.parse_list(true)),
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
  /// a `List` node.
  fn parse_list(&mut self, ordered: bool) -> Node {
    let span = self.current_span();
    let mut items: Vec<Node> = Vec::new();
    let start: Option<u32> = if ordered {
      self.peek().and_then(|t| t.raw.trim_end_matches('.').parse::<u32>().ok())
    } else {
      None
    };

    while let Some(kind) = self.peek_kind() {
      let want_marker = if ordered {
        matches!(kind, TokenKind::OrderedListItem)
      } else {
        matches!(kind, TokenKind::UnorderedListItem)
      };
      if !want_marker {
        break;
      }
      self.advance();

      // For ordered list items the lexer leaves the trailing `.` (and any
      // following space) inside the next Text token (e.g. ". three"). Trim
      // a single leading `.` and any leading ASCII whitespace from that
      // first Text token so the inline content starts with the actual body.
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

      items.push(self.parse_one_list_item(ordered));

      if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
        self.advance();
      }
    }

    Node::List(List { ordered, start, children: items, span })
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
        if matches!(kind, Some(TokenKind::Text))
          && (text_raw == " " || text_raw.eq_ignore_ascii_case("x"))
        {
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

  /// Each `>` line becomes one inner `Paragraph`. Stops at the first line
  /// without a leading `BlockQuote` token.
  fn parse_blockquote(&mut self) -> Node {
    let span = self.current_span();
    let mut paras: Vec<Node> = Vec::new();
    loop {
      if !matches!(self.peek_kind(), Some(TokenKind::BlockQuote)) {
        break;
      }
      let para_span = self.current_span();
      self.advance();
      let inline = self.collect_inline_until_break();
      if !inline.is_empty() {
        paras.push(Node::Paragraph(Paragraph { children: inline, span: para_span }));
      }
      if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
        self.advance();
      }
    }
    Node::Blockquote(Blockquote { children: paras, span })
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
        if matches!(
          next,
          Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak) | Some(TokenKind::Eof) | None
        ) {
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
