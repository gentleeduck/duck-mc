use super::{HTML_BLOCK_TYPE1_TAGS, HTML_BLOCK_TYPE6_TAGS, HtmlBlockMode};
use crate::ast::*;
use crate::parser::Parser;
use dmc_diagnostic::Code;
use dmc_lexer::token::TokenKind;

impl<'eng, 'tokens> Parser<'eng, 'tokens> {
  /// CM 4.6 raw-HTML block detection, keyed off a JSX-style open tag at
  /// column 0. Returns `Some(mode)` when the upcoming tag belongs to
  /// the type-1 or type-6 set; cursor untouched.
  /// Is the upcoming JSX open tag a lowercase HTML-ish tag (so the
  /// surrounding paragraph wraps it as inline raw HTML)?
  pub(super) fn is_lowercase_jsx_tag(&self) -> bool {
    matches!(self.peek_kind(), Some(TokenKind::JsxOpenTagStart)) && self.is_plain_html_jsx_tag()
  }

  pub(super) fn jsx_html_block_mode(&self) -> Option<HtmlBlockMode> {
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
    } else if self.is_plain_html_jsx_tag() && self.jsx_raw_html_tag_is_valid() && self.line_after_tag_is_blank() {
      // CM 4.6 Type-7: any tag at col 0 closes on next blank line --
      // BUT the start line itself must contain only the tag plus
      // whitespace (no inline content after the closing `>`).
      // Restricted to lowercase / kebab-case names so MDX components
      // like `<MyComponent>` and namespaces like `<svg:circle>` stay
      // on the JSX path and compile to component invocations.
      Some(HtmlBlockMode::Type7)
    } else if self.options.cm_strict_html_blocks
      && self.is_htmlish_jsx_tag()
      && self.jsx_raw_html_tag_is_valid_htmlish()
      && self.line_after_tag_is_blank()
    {
      // CM-strict spec runner: also treat uppercase HTML-ish names
      // (like `<Warning>`) as Type-7 raw HTML blocks. MDX mode keeps
      // these on the JSX path so the component compiles correctly.
      Some(HtmlBlockMode::Type7)
    } else {
      None
    }
  }

  /// After the upcoming JSX tag's `>` / `/>`, is the rest of the line
  /// whitespace-only? Required for CM 4.6 Type-7 trigger.
  fn line_after_tag_is_blank(&self) -> bool {
    // Skip over the open tag tokens until JsxOpenTagEnd / JsxSelfClosingEnd.
    // CM 4.6 type-7 requires the open tag to be a single complete tag on
    // the start line, so reject if any tag-internal token spans a newline.
    let mut i = self.pos;
    let start_line = self.tokens.get(i).map(|t| t.span.line);
    let mut depth = 0i32;
    while let Some(t) = self.tokens.get(i) {
      if t.raw.contains('\n') || start_line.is_some_and(|line| t.span.line != line) {
        return false;
      }
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
  pub(super) fn parse_html_block_from_jsx(&mut self, mode: HtmlBlockMode) -> Node {
    let span = self.current_span();
    let start_idx = self.pos;
    if matches!(self.peek_kind(), Some(TokenKind::JsxCloseTagStart)) {
      let close_name = self
        .tokens
        .get(self.pos + 1)
        .filter(|t| matches!(t.kind, TokenKind::JsxTagName))
        .map(|t| t.raw)
        .unwrap_or_default();
      let diagnostic = duck_diagnostic::diag!(
        Code::MismatchedJsxCloseTag,
        span.clone(),
        format!("orphan close tag `</{close_name}>` has no matching opener in this block; preserving it as raw HTML")
      )
      .with_help(
        "add the matching opening tag earlier in the block, or escape the leading `<` if this should render as text",
      );
      self.emit_diagnostic(diagnostic);
    }
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
              // CM 4.6 type-1: the block extends to the end of the line
              // that contains the matching close tag (everything after
              // `</tag>` on that line stays inside the block).
              while let Some(t) = self.peek() {
                match &t.kind {
                  TokenKind::SoftBreak | TokenKind::HardBreak | TokenKind::BlankLine | TokenKind::Eof => break,
                  _ => {
                    self.advance();
                  },
                }
              }
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
    let mut value = self.raw_source_for_token_range(start_idx, self.pos);
    // CM 5.1: when an HTML block lives inside a blockquote, each
    // continuation line carries its own `>` marker(s). Strip them so
    // the rendered raw HTML matches the spec output.
    if value.contains("\n>") {
      let stripped: String = value
        .split_inclusive('\n')
        .enumerate()
        .map(|(i, line)| {
          if i == 0 {
            line.to_string()
          } else {
            // Strip leading `>` markers (with optional one space each).
            let mut rest = line;
            while let Some(stripped) = rest.strip_prefix('>') {
              rest = stripped.strip_prefix(' ').unwrap_or(stripped);
            }
            rest.to_string()
          }
        })
        .collect();
      value = stripped;
    }
    Node::Html(Html { value, span })
  }

  /// Raw HTML block (CM 4.6 types 2-5). Lexer flagged the open token
  /// with the type discriminator; we capture the entire span verbatim
  /// (open + body + close) into a single `Html` node.
  /// CM 4.6 type-2: HTML comment as a block (cursor on
  /// `HtmlCommentOpen` at col 0). Slurps tokens through the matching
  /// `HtmlCommentClose` and emits a single `Html` node containing the
  /// verbatim source. The block extends to a blank line if the close
  /// never fires on the same line.
  pub(super) fn parse_html_comment_block(&mut self) -> Node {
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

  pub(super) fn parse_html_block(&mut self) -> Node {
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

  /// Top-level lowercase HTML close tags like `</a></b>` are inline raw
  /// HTML, not JSX terminators. Use the normal paragraph break rules but
  /// do not stop on `JsxCloseTagStart`.
  pub(super) fn parse_plain_html_close_paragraph(&mut self) -> Node {
    let span = self.current_span();
    let children = self.collect_inline(&|k| {
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
    });
    if matches!(self.peek_kind(), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
      self.advance();
    }
    Node::Paragraph(Paragraph { children, span })
  }
}
