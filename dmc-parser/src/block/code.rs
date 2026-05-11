use crate::ast::*;
use crate::parser::Parser;
use dmc_diagnostic::Code;
use dmc_lexer::token::TokenKind;

impl<'eng, 'tokens> Parser<'eng, 'tokens> {
  /// CM 5.1 + 4.4: an indented code block inside a blockquote. Cursor is
  /// at the leading Whitespace(>=4) immediately after the bq marker(s).
  /// Reads one line of code, then continues across soft breaks when the
  /// next bq line starts with another Whitespace(>=4).
  pub(super) fn parse_indented_code_in_bq(&mut self) -> Node {
    let span = self.current_span();
    let mut buf = String::new();
    loop {
      let ws_visual = match self.peek_leading_indent() {
        Some(n) if n >= 4 => n,
        _ => break,
      };
      self.advance(); // consume whitespace
      let visible = ws_visual.saturating_sub(4);
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
      // Optional soft break + bq marker on next line; if the next non-
      // marker peek isn't another Whitespace(>=4 visual), stop.
      let saved = self.pos;
      if !matches!(self.peek_kind(), Some(TokenKind::SoftBreak)) {
        break;
      }
      self.advance();
      let next_markers = self.count_line_blockquote_markers();
      if next_markers == 0 {
        self.pos = saved;
        break;
      }
      self.consume_blockquote_markers(next_markers);
      let next_aligned = self.peek_leading_indent().is_some_and(|n| n >= 4);
      if !next_aligned {
        self.pos = saved;
        break;
      }
    }
    Node::CodeBlock(CodeBlock { lang: None, meta: None, value: buf, span })
  }

  pub(super) fn parse_same_indent_fenced_code_in_list(&mut self, content_indent: usize) -> Option<Node> {
    if self.peek_leading_indent() != Some(content_indent) {
      return None;
    }
    let raw = match self.tokens.get(self.pos + 1) {
      Some(t) if matches!(t.kind, TokenKind::IndentedCodeLine) => t.raw,
      _ => return None,
    };
    let bytes = raw.as_bytes();
    let fence_byte = match bytes.first().copied() {
      Some(b'`') => b'`',
      Some(b'~') => b'~',
      _ => return None,
    };
    let mut fence_n = 0usize;
    while fence_n < bytes.len() && bytes[fence_n] == fence_byte {
      fence_n += 1;
    }
    if fence_n < 3 {
      return None;
    }

    let span = self.current_span();
    self.advance(); // content indent
    self.advance(); // opener line
    if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
      self.advance();
    }

    let mut value = String::new();
    while let Some(indent) = self.peek_leading_indent() {
      if indent < content_indent {
        break;
      }
      self.advance();
      let Some(t) = self.peek() else {
        break;
      };
      if !matches!(t.kind, TokenKind::IndentedCodeLine) {
        break;
      }
      let raw = t.raw.to_string();
      let close = {
        let b = raw.as_bytes();
        let mut i = 0usize;
        while i < b.len() && b[i] == b' ' && i < 3 {
          i += 1;
        }
        let mut count = 0usize;
        while i + count < b.len() && b[i + count] == fence_byte {
          count += 1;
        }
        let mut j = i + count;
        while j < b.len() && matches!(b[j], b' ' | b'\t') {
          j += 1;
        }
        count >= fence_n && j == b.len()
      };
      self.advance();
      if close {
        if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
          self.advance();
        }
        break;
      }
      value.push_str(&raw);
      value.push('\n');
      if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
        self.advance();
      } else {
        break;
      }
    }

    Some(Node::CodeBlock(CodeBlock { lang: None, meta: None, value, span }))
  }

  /// CM 4.4 fallback: build an indented code block by hand when the
  /// lexer didn't pre-classify (eg between bq lines). Reads contiguous
  /// `Whitespace(>=4) + line content` until the indent breaks.
  pub(super) fn parse_indented_code_fallback(&mut self) -> Node {
    let span = self.current_span();
    let mut buf = String::new();
    loop {
      let ws_len = match self.peek() {
        Some(t) if matches!(t.kind, TokenKind::Whitespace(_)) && t.raw.chars().count() >= 4 => t.raw.chars().count(),
        _ => break,
      };
      // Only at col 0.
      if self.peek().is_none_or(|t| t.span.column != 1) {
        break;
      }
      self.advance();
      let visible = ws_len.saturating_sub(4);
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
      let saved = self.pos;
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
      let next_aligned = matches!(self.peek(), Some(t) if matches!(t.kind, TokenKind::Whitespace(_)) && t.raw.chars().count() >= 4 && t.span.column == 1);
      if !next_aligned {
        self.pos = saved;
        break;
      }
      for _ in 1..blanks {
        buf.push('\n');
      }
    }
    Node::CodeBlock(CodeBlock { lang: None, meta: None, value: buf, span })
  }

  /// 4-space indented code block (CM 4.4). Lexer pre-classifies a valid
  /// indent line as `Whitespace(>=4) + IndentedCodeLine`; this method
  /// concatenates consecutive pairs, joining with `\n` and stopping at
  /// the first non-indented line.
  pub(super) fn parse_indented_code(&mut self) -> Node {
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
      let mut blank_ws_visible: Vec<usize> = Vec::new();
      let mut consumed_softbreak = false;
      loop {
        // CM 4.4: blank-with-whitespace lines between indented code
        // lines stay in the body. Each such line is `Whitespace(N) +
        // SoftBreak`. Capture the visible cols (N - 4) so the body
        // renders the indent past the 4-space strip.
        let ws_then_break = matches!(self.peek_kind(), Some(TokenKind::Whitespace(_)))
          && self.peek().is_some_and(|t| t.span.column == 1)
          && matches!(
            self.tokens.get(self.pos + 1).map(|t| &t.kind),
            Some(TokenKind::SoftBreak) | Some(TokenKind::BlankLine)
          );
        if ws_then_break {
          let ws_n = self.peek().map(|t| t.raw.chars().count()).unwrap_or(0);
          self.advance();
          if matches!(self.peek_kind(), Some(TokenKind::BlankLine)) {
            self.advance();
            blanks += 2;
            blank_ws_visible.push(ws_n.saturating_sub(4));
            blank_ws_visible.push(0);
          } else {
            self.advance();
            blanks += 1;
            blank_ws_visible.push(ws_n.saturating_sub(4));
          }
          continue;
        }
        match self.peek_kind() {
          Some(TokenKind::SoftBreak) if !consumed_softbreak => {
            self.advance();
            blanks += 1;
            consumed_softbreak = true;
            // Don't break -- keep scanning so blank-with-ws lines
            // following the soft break stay in the code body.
          },
          Some(TokenKind::BlankLine) => {
            self.advance();
            blanks += 2;
            blank_ws_visible.push(0);
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
      // Push buffered blank-line content. The body already ended with
      // one `\n` for the previous code line; emit per-blank visible
      // spaces + `\n`. The terminating SoftBreak before the next code
      // line is handled by the outer loop's `\n` push.
      let blanks_to_emit = blanks.saturating_sub(1);
      for i in 0..blanks_to_emit {
        let visible = blank_ws_visible.get(i).copied().unwrap_or(0);
        if visible > 0 {
          buf.push_str(&" ".repeat(visible));
        }
        buf.push('\n');
      }
    }
    Node::CodeBlock(CodeBlock { lang: None, meta: None, value: buf, span })
  }

  /// Fenced code block. The first inline `Text` becomes the info string; the
  /// body is concatenated until the matching `CodeEnd(n)`. The info string
  /// splits at the first whitespace into `(lang, meta)`.
  pub(super) fn parse_code_block(&mut self) -> Node {
    self.parse_code_block_with_blockquote_prefix(0)
  }

  pub(super) fn parse_code_block_in_blockquote(&mut self, bq_depth: usize) -> Node {
    self.parse_code_block_with_blockquote_prefix(bq_depth)
  }

  fn parse_code_block_with_blockquote_prefix(&mut self, bq_depth: usize) -> Node {
    let span = self.current_span();
    let open_line = self.peek().map(|t| t.span.line).unwrap_or(0);
    let (fence_char, fence_n, fence_indent) = match self.peek() {
      Some(t) => match t.kind {
        TokenKind::CodeFenceOpen(c, n) => (c, n, if bq_depth == 0 { t.span.column.saturating_sub(1) } else { 0 }),
        _ => (dmc_lexer::token::FenceChar::Backtick, 3, 0),
      },
      None => (dmc_lexer::token::FenceChar::Backtick, 3, 0),
    };
    self.advance();

    let info = match self.peek() {
      Some(t) if t.span.line == open_line && matches!(t.kind, TokenKind::CodeFenceInfo | TokenKind::Text) => {
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
      // CM 4.5: info-string entity references (`&ouml;`) decode before
      // the value reaches the renderer; backslash escapes resolve too.
      let decode = |s: &str| crate::inline::decode_entities_in(&Self::unescape_markdown(s));
      match info_trimmed.split_once(char::is_whitespace) {
        Some((l, rest)) => {
          let rest = rest.trim();
          (Some(decode(l)), if rest.is_empty() { None } else { Some(decode(rest)) })
        },
        None => (Some(decode(info_trimmed)), None),
      }
    };

    let mut value = String::new();
    let mut closed = false;
    if bq_depth > 0 {
      if let Some(t) = self.peek()
        && matches!(t.kind, TokenKind::CodeFenceContent)
      {
        value.push_str(&Self::strip_blockquote_prefix_from_fence_content(t.raw, bq_depth));
        self.advance();
      }
      if matches!(self.peek_kind(), Some(TokenKind::CodeFenceClose(c, m)) if *c == fence_char && *m >= fence_n) {
        self.advance();
        closed = true;
      }
    } else {
      while let Some(t) = self.peek() {
        match &t.kind {
          TokenKind::CodeFenceClose(c, m) if *c == fence_char && *m >= fence_n => {
            self.advance();
            closed = true;
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
    }

    if !closed {
      let fence = match fence_char {
        dmc_lexer::token::FenceChar::Backtick => "`",
        dmc_lexer::token::FenceChar::Tilde => "~",
      };
      let diagnostic = duck_diagnostic::diag!(
        Code::UnterminatedCodeBlockBlock,
        span.clone(),
        "fenced code block never found a matching closing fence; treating the rest of the file as code"
      )
      .with_help(format!("add a closing fence with at least {fence_n} `{fence}` characters"));
      self.emit_diagnostic(diagnostic);
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

  fn strip_blockquote_prefix_from_fence_content(raw: &str, depth: usize) -> String {
    raw
      .split_inclusive('\n')
      .map(|line| {
        let bytes = line.as_bytes();
        let mut p = 0usize;
        let mut leading = 0usize;
        while p < bytes.len() && bytes[p] == b' ' && leading < 3 {
          p += 1;
          leading += 1;
        }
        for idx in 0..depth {
          if p >= bytes.len() || bytes[p] != b'>' {
            return line.to_string();
          }
          p += 1;
          if idx + 1 < depth {
            while p < bytes.len() && matches!(bytes[p], b' ' | b'\t') {
              p += 1;
            }
          } else if p < bytes.len() && matches!(bytes[p], b' ' | b'\t') {
            p += 1;
          }
        }
        line[p..].to_string()
      })
      .collect()
  }
}
