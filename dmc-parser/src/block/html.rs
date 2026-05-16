use super::{HTML_BLOCK_TYPE1_TAGS, HTML_BLOCK_TYPE6_TAGS, HtmlBlockMode};
use crate::ast::*;
use crate::parser::Parser;
use dmc_diagnostic::Code;
use dmc_lexer::token::TokenKind;

impl<'eng, 'tokens> Parser<'eng, 'tokens> {
  /// True when the upcoming JSX open tag is lowercase HTML-ish so the
  /// surrounding paragraph wraps it as inline raw HTML.
  pub(super) fn is_lowercase_jsx_tag(&self) -> bool {
    matches!(self.peek_kind(), Some(TokenKind::JsxOpenTagStart)) && self.is_plain_html_jsx_tag()
  }

  /// CM 4.6 raw-HTML block detection keyed off a JSX-style open tag.
  /// `Some(mode)` for type 1, 6, or 7; cursor untouched.
  pub(super) fn jsx_html_block_mode(&self) -> Option<HtmlBlockMode> {
    let open = self.tokens.get(self.pos)?;
    // CM 4.6 allows up to 3 leading spaces before a block (1-based col 1-4).
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
      // CM 4.6 Type-7: lowercase/kebab tag alone on the start line. MDX
      // capitals (`<MyComponent>`) and namespaces (`<svg:circle>`) stay on
      // the JSX path so they compile as component invocations.
      Some(HtmlBlockMode::Type7)
    } else if self.options.cm_strict_html_blocks
      && self.is_htmlish_jsx_tag()
      && self.jsx_raw_html_tag_is_valid_htmlish()
      && self.line_after_tag_is_blank()
    {
      // Spec runner only: treat uppercase HTML-ish names as Type-7 too.
      Some(HtmlBlockMode::Type7)
    } else {
      None
    }
  }

  /// CM 4.6 Type-7 precondition: after `>` / `/>`, the rest of the start
  /// line is whitespace-only AND the tag itself fits on that line.
  fn line_after_tag_is_blank(&self) -> bool {
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
    while let Some(t) = self.tokens.get(i) {
      match &t.kind {
        TokenKind::SoftBreak | TokenKind::HardBreak | TokenKind::BlankLine | TokenKind::Eof => return true,
        TokenKind::Whitespace(_) => i += 1,
        _ => return false,
      }
    }
    true
  }

  /// Reconstruct a raw HTML block from a JSX-tokenized stream. Captures
  /// the verbatim source slice so internal whitespace + attribute
  /// formatting survive (the lexer's JSX path normalizes whitespace, so
  /// per-token concat alone would drop it).
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
              // CM 4.6 type-1: block extends to end-of-line of the closer.
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
    // CM 5.1: HTML block inside a blockquote has `>` markers on every
    // continuation line - strip them.
    if value.contains("\n>") {
      let stripped: String = value
        .split_inclusive('\n')
        .enumerate()
        .map(|(i, line)| {
          if i == 0 {
            line.to_string()
          } else {
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

  /// CM 4.6 type-2: HTML comment block. Cursor on `HtmlCommentOpen` at col 0.
  /// Block extends to a blank line if `-->` never fires on the same line.
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
            value.push('\n');
            self.advance();
            continue;
          }
          // CM 4.6 type-2: block ends with the line containing `-->`.
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

  /// Top-level lowercase HTML close tags (`</a></b>`) are inline raw HTML,
  /// not JSX terminators - paragraph collection must not stop on them.
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
