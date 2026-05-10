use crate::ast::*;
use crate::parser::Parser;
use dmc_lexer::token::{AutolinkKind, EmphasisChar, TokenKind};

/// Flatten an inline node list to plain text. Used to build the lookup
/// label for shortcut / collapsed / full reference links from the
/// already-parsed inner content.
fn plain_text(nodes: &[Node]) -> String {
  let mut s = String::new();
  for n in nodes {
    match n {
      Node::Text(t) => s.push_str(&t.value),
      Node::Bold(i) | Node::Italic(i) | Node::Strikethrough(i) => s.push_str(&plain_text(&i.children)),
      Node::Link(l) => s.push_str(&plain_text(&l.children)),
      Node::InlineCode(c) => s.push_str(&c.value),
      _ => {},
    }
  }
  s
}

/// Decode a CommonMark entity reference (`&amp;`, `&#9;`, `&#x2A;`).
/// Returns `None` when the form is malformed or the named entity is not in
/// the small subset table; the caller falls back to the raw lexeme so
/// rendering stays lossless.
fn decode_entity(raw: &str) -> Option<String> {
  let inner = raw.strip_prefix('&')?.strip_suffix(';')?;
  if let Some(rest) = inner.strip_prefix('#') {
    let cp: u32 = if let Some(hex) = rest.strip_prefix(['x', 'X']) {
      u32::from_str_radix(hex, 16).ok()?
    } else {
      rest.parse().ok()?
    };
    // CM 6.6: NUL becomes U+FFFD.
    let cp = if cp == 0 { 0xFFFD } else { cp };
    return char::from_u32(cp).map(|c| c.to_string());
  }
  // Small subset of HTML5 named entities — covers the common cases the
  // CommonMark spec exercises (the full table is ~2000 entries; codegen
  // can swap in a generated lookup later).
  let s = match inner {
    "amp" => "&",
    "lt" => "<",
    "gt" => ">",
    "quot" => "\"",
    "apos" => "'",
    "nbsp" => "\u{00A0}",
    "copy" => "\u{00A9}",
    "reg" => "\u{00AE}",
    "trade" => "\u{2122}",
    "hellip" => "\u{2026}",
    "mdash" => "\u{2014}",
    "ndash" => "\u{2013}",
    "lsquo" => "\u{2018}",
    "rsquo" => "\u{2019}",
    "ldquo" => "\u{201C}",
    "rdquo" => "\u{201D}",
    "laquo" => "\u{00AB}",
    "raquo" => "\u{00BB}",
    "middot" => "\u{00B7}",
    "bull" => "\u{2022}",
    _ => return None,
  };
  Some(s.to_string())
}

fn utf8_char_len(b: u8) -> usize {
  if b < 0x80 {
    1
  } else if b < 0xE0 {
    2
  } else if b < 0xF0 {
    3
  } else {
    4
  }
}

impl<'eng, 'tokens> Parser<'eng, 'tokens> {
  /// Accumulate inline nodes until any top-level break token.
  pub(crate) fn collect_inline_until_break(&mut self) -> Vec<Node> {
    self.collect_inline(&|kind| {
      matches!(
        kind,
        TokenKind::BlankLine
          | TokenKind::SoftBreak
          | TokenKind::Eof
          | TokenKind::Heading(_)
          | TokenKind::FrontmatterStart(_)
          | TokenKind::Import
          | TokenKind::Export
          | TokenKind::JsxCloseTagStart
      )
    })
  }

  /// Inline body of one list item. Same stop set as
  /// `collect_inline_until_break`, but skips the single leading
  /// `Whitespace` token that follows the marker (`- foo` vs `-foo`).
  pub(crate) fn collect_inline_for_list_item(&mut self) -> Vec<Node> {
    if matches!(self.peek_kind(), Some(TokenKind::Whitespace(_))) {
      self.advance();
    }
    self.collect_inline_until_break()
  }

  /// Collect inline nodes until `stop(kind)` returns true. The stopping token
  /// is left on the stream.
  pub(crate) fn collect_inline(&mut self, stop: &dyn Fn(&TokenKind) -> bool) -> Vec<Node> {
    let mut out = Vec::new();
    while let Some(t) = self.peek() {
      let kind = t.kind.clone();
      if stop(&kind) {
        break;
      }

      let span = t.span.clone();
      match &kind {
        TokenKind::Text => {
          let raw = Self::unescape_markdown(t.raw);
          self.advance();
          out.push(Node::Text(Text { value: raw, span }));
        },
        TokenKind::Autolink(kind) => {
          let kind = *kind;
          let raw = t.raw.to_string();
          self.advance();
          // Resolve display vs href per autolink kind.
          let (display, href) = match kind {
            AutolinkKind::AngleUrl => {
              let inner = raw.trim_start_matches('<').trim_end_matches('>').to_string();
              (inner.clone(), inner)
            },
            AutolinkKind::AngleEmail => {
              let inner = raw.trim_start_matches('<').trim_end_matches('>').to_string();
              (inner.clone(), format!("mailto:{inner}"))
            },
            AutolinkKind::BareUrl => (raw.clone(), raw),
            AutolinkKind::BareWww => (raw.clone(), format!("https://{raw}")),
          };
          out.push(Node::Link(Link {
            href,
            title: None,
            children: vec![Node::Text(Text { value: display, span: span.clone() })],
            span,
          }));
        },
        TokenKind::Whitespace(_) => {
          let raw = t.raw.to_string();
          self.advance();
          out.push(Node::Text(Text { value: raw, span }));
        },
        TokenKind::Emphasis(c, n) => {
          let open_c: EmphasisChar = *c;
          let open_n = *n;
          let raw = t.raw.to_string();
          self.advance();
          let inner = self.collect_inline(&|k| {
            Self::is_top_level_break(k) || matches!(k, TokenKind::Emphasis(cc, m) if *cc == open_c && *m == open_n)
          });
          let closed = matches!(self.peek_kind(), Some(TokenKind::Emphasis(cc, m)) if *cc == open_c && *m == open_n);
          if !closed {
            // CM: an unmatched emphasis run is literal text. Surface the
            // opener and the already-collected inner as siblings so the
            // delimiters render verbatim.
            out.push(Node::Text(Text { value: raw, span: span.clone() }));
            for n in inner {
              out.push(n);
            }
            continue;
          }
          self.advance();
          // Run-length 1 = italic, 2 = bold, 3 = strong+em combined per
          // CommonMark: <em><strong>x</strong></em>.
          match open_n {
            1 => out.push(Node::Italic(Inline { children: inner, span })),
            2 => out.push(Node::Bold(Inline { children: inner, span })),
            _ => {
              let strong = Node::Bold(Inline { children: inner, span: span.clone() });
              out.push(Node::Italic(Inline { children: vec![strong], span }));
            },
          }
        },
        TokenKind::Strikethrough => {
          self.advance();
          let inner = self.collect_inline(&|k| Self::is_top_level_break(k) || matches!(k, TokenKind::Strikethrough));
          if matches!(self.peek_kind(), Some(TokenKind::Strikethrough)) {
            self.advance();
          }
          out.push(Node::Strikethrough(Inline { children: inner, span }));
        },
        TokenKind::CodeInlineOpen(n) => {
          let open_n = *n;
          self.advance();
          let mut value = String::new();
          while let Some(tok) = self.peek() {
            match &tok.kind {
              TokenKind::CodeInlineClose(m) if *m == open_n => {
                self.advance();
                break;
              },
              TokenKind::Eof => break,
              _ => {
                value.push_str(tok.raw);
                self.advance();
              },
            }
          }
          // CM 6.1: line endings inside become single spaces; if the
          // resulting string both starts and ends with a single space
          // (and isn't all-spaces), strip one from each side.
          let value = value.replace('\n', " ");
          let value = if value.starts_with(' ') && value.ends_with(' ') && value.chars().any(|c| c != ' ') {
            value[1..value.len() - 1].to_string()
          } else {
            value
          };
          out.push(Node::InlineCode(InlineCode { value, span }));
        },
        TokenKind::LinkOpen => {
          let start = self.pos;
          self.advance();
          let inner = self.collect_inline(&|k| {
            matches!(k, TokenKind::LinkClose | TokenKind::BlankLine | TokenKind::SoftBreak | TokenKind::Eof)
          });
          if !matches!(self.peek_kind(), Some(TokenKind::LinkClose)) {
            self.pos = start;
            out.push(Node::Text(Text { value: "[".into(), span }));
            self.advance();
            continue;
          }
          self.advance(); // consume the closing `]`
          // CommonMark 6.3: classify the link form by what follows the
          // closing `]`.
          //   `(...)`     -> inline link
          //   `[...]`     -> full reference `[text][label]` or
          //                  collapsed `[label][]`
          //   nothing     -> shortcut reference `[label]`
          // Reference forms resolve against the ref-def map populated in
          // the pre-pass; unresolved refs fall back to literal text.
          match self.peek_kind() {
            Some(TokenKind::LinkTargetOpen) => {
              self.advance(); // consume `(`
              let mut paren_body = String::new();
              while let Some(tok) = self.peek() {
                match &tok.kind {
                  TokenKind::LinkTargetClose => {
                    self.advance();
                    break;
                  },
                  TokenKind::Eof => break,
                  _ => {
                    paren_body.push_str(tok.raw);
                    self.advance();
                  },
                }
              }
              let (href, title) = Self::split_destination_title(&paren_body);
              out.push(Node::Link(Link { href, title, children: inner, span }));
            },
            Some(TokenKind::LinkOpen) => {
              // Reference form: peek the second `[..]` to distinguish
              // collapsed (`[]`) from full (`[label]`).
              self.advance(); // consume `[`
              if matches!(self.peek_kind(), Some(TokenKind::LinkClose)) {
                self.advance();
                let label = plain_text(&inner);
                if let Some((href, title)) = self.refs.get(&label).cloned() {
                  out.push(Node::Link(Link { href, title, children: inner, span }));
                  continue;
                }
                // Unresolved -- emit literal `[text][]`.
                out.push(Node::Text(Text { value: "[".into(), span: span.clone() }));
                for n in inner {
                  out.push(n);
                }
                out.push(Node::Text(Text { value: "][]".into(), span }));
                continue;
              }
              let label_inner = self.collect_inline(&|k| {
                matches!(k, TokenKind::LinkClose | TokenKind::BlankLine | TokenKind::SoftBreak | TokenKind::Eof)
              });
              if !matches!(self.peek_kind(), Some(TokenKind::LinkClose)) {
                // Treat the trailing `[..` as text and fall through to
                // shortcut behavior on the original inner.
                let label = plain_text(&inner);
                if let Some((href, title)) = self.refs.get(&label).cloned() {
                  out.push(Node::Link(Link { href, title, children: inner, span }));
                  continue;
                }
                out.push(Node::Text(Text { value: "[".into(), span: span.clone() }));
                for n in inner {
                  out.push(n);
                }
                out.push(Node::Text(Text { value: "]".into(), span: span.clone() }));
                out.push(Node::Text(Text { value: "[".into(), span: span.clone() }));
                for n in label_inner {
                  out.push(n);
                }
                continue;
              }
              self.advance(); // consume label-side `]`
              let label = plain_text(&label_inner);
              if let Some((href, title)) = self.refs.get(&label).cloned() {
                out.push(Node::Link(Link { href, title, children: inner, span }));
                continue;
              }
              // Unresolved full ref -- emit `[inner][label]` as text.
              out.push(Node::Text(Text { value: "[".into(), span: span.clone() }));
              for n in inner {
                out.push(n);
              }
              out.push(Node::Text(Text { value: "][".into(), span: span.clone() }));
              for n in label_inner {
                out.push(n);
              }
              out.push(Node::Text(Text { value: "]".into(), span }));
            },
            _ => {
              // Shortcut `[label]`. Resolve via the ref-def map; falls
              // back to bracketed text when no matching definition.
              let label = plain_text(&inner);
              if let Some((href, title)) = self.refs.get(&label).cloned() {
                out.push(Node::Link(Link { href, title, children: inner, span }));
                continue;
              }
              out.push(Node::Text(Text { value: "[".into(), span: span.clone() }));
              for n in inner {
                out.push(n);
              }
              out.push(Node::Text(Text { value: "]".into(), span }));
            },
          }
        },
        TokenKind::ImageMarker => {
          // Lexer's `ImageMarker` already covers `![`, so the cursor is on
          // the alt-text body. Walk to the closing `]` (`LinkClose`).
          self.advance();
          let mut alt = String::new();
          while let Some(tok) = self.peek() {
            match &tok.kind {
              TokenKind::LinkClose => {
                self.advance();
                break;
              },
              TokenKind::Eof | TokenKind::BlankLine | TokenKind::SoftBreak => break,
              _ => {
                alt.push_str(tok.raw);
                self.advance();
              },
            }
          }
          let mut paren_body = String::new();
          if matches!(self.peek_kind(), Some(TokenKind::LinkTargetOpen)) {
            self.advance();
            while let Some(tok) = self.peek() {
              match &tok.kind {
                TokenKind::LinkTargetClose => {
                  self.advance();
                  break;
                },
                TokenKind::Eof => break,
                _ => {
                  paren_body.push_str(tok.raw);
                  self.advance();
                },
              }
            }
          }
          let (src, title) = Self::split_destination_title(&paren_body);
          out.push(Node::Image(Image { src, alt, title, span }));
        },
        TokenKind::JsxOpenTagStart => {
          out.push(self.parse_jsx());
          continue;
        },
        TokenKind::JsxFragmentOpen => {
          out.push(self.parse_jsx_fragment());
          continue;
        },
        TokenKind::ExpressionStart => {
          out.push(self.parse_jsx_expression());
          continue;
        },
        TokenKind::FootnoteRefOpen => {
          // Lexer emits a single token covering `[^id]`; pull the id out.
          let raw = t.raw.to_string();
          self.advance();
          let id = raw.trim_start_matches('[').trim_start_matches('^').trim_end_matches(']').to_string();
          out.push(Node::FootnoteRef(FootnoteRef { id, span }));
        },
        TokenKind::EntityRef => {
          let raw = t.raw.to_string();
          self.advance();
          let value = decode_entity(&raw).unwrap_or(raw);
          out.push(Node::Text(Text { value, span }));
        },
        TokenKind::HeadingTrailingHashes => {
          // Decoration only; lexer flags the trailing `#` run on an ATX
          // heading so the parser can drop it. Surrounding whitespace
          // gets trimmed by `parse_heading`'s end-trim pass.
          self.advance();
          continue;
        },
        TokenKind::MdxCommentOpen => {
          while let Some(t) = self.peek() {
            match &t.kind {
              TokenKind::MdxCommentClose => {
                self.advance();
                break;
              },
              TokenKind::Eof => break,
              _ => {
                self.advance();
              },
            }
          }
          continue;
        },
        _ => {
          let raw = t.raw.to_string();
          self.advance();
          if !raw.is_empty() {
            out.push(Node::Text(Text { value: raw, span }));
          }
        },
      }
    }
    out
  }

  /// Split the body of a `(...)` link/image destination into
  /// `(href, title)`. CommonMark allows an optional trailing
  /// `"title"` / `'title'` / `(title)` separated from the destination
  /// by whitespace. Unterminated/missing title returns `(body, None)`.
  fn split_destination_title(body: &str) -> (String, Option<String>) {
    let trimmed = body.trim();
    if trimmed.is_empty() {
      return (String::new(), None);
    }
    // Walk back from the end looking for a balanced quoted title.
    let bytes = trimmed.as_bytes();
    let last = bytes[bytes.len() - 1];
    let close = match last {
      b'"' => Some(b'"'),
      b'\'' => Some(b'\''),
      b')' => Some(b'('),
      _ => None,
    };
    let Some(open) = close else {
      return (trimmed.to_string(), None);
    };
    // Find the matching opener, ensuring a whitespace separator before it.
    let mut i = bytes.len() - 1;
    let mut depth = 1;
    while i > 0 {
      i -= 1;
      let b = bytes[i];
      if b == last && b != open {
        depth += 1;
      }
      if b == open {
        depth -= 1;
        if depth == 0 {
          break;
        }
      }
    }
    if depth != 0 {
      return (trimmed.to_string(), None);
    }
    // Need at least one whitespace between dest and the opener.
    if i == 0 || !bytes[i - 1].is_ascii_whitespace() {
      return (trimmed.to_string(), None);
    }
    let dest = trimmed[..i].trim_end().to_string();
    let title = trimmed[i + 1..bytes.len() - 1].to_string();
    (dest, Some(title))
  }

  /// Strip `\X` -> `X` for the standard CommonMark escapable set so
  /// authors can write `\*literal\*` without the asterisks turning into
  /// emphasis. The lexer keeps the backslash in `Text` raw to preserve
  /// source spans; this collapses it for the rendered text.
  fn unescape_markdown(s: &str) -> String {
    if !s.contains('\\') {
      return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
      if bytes[i] == b'\\' && i + 1 < bytes.len() {
        let nx = bytes[i + 1];
        // CM appendix: `!"#$%&'()*+,-./:;<=>?@[\]^_`{|}~`.
        if matches!(
          nx,
          b'!'
            | b'"'
            | b'#'
            | b'$'
            | b'%'
            | b'&'
            | b'\''
            | b'('
            | b')'
            | b'*'
            | b'+'
            | b','
            | b'-'
            | b'.'
            | b'/'
            | b':'
            | b';'
            | b'<'
            | b'='
            | b'>'
            | b'?'
            | b'@'
            | b'['
            | b'\\'
            | b']'
            | b'^'
            | b'_'
            | b'`'
            | b'{'
            | b'|'
            | b'}'
            | b'~'
        ) {
          out.push(nx as char);
          i += 2;
          continue;
        }
      }
      let ch_len = utf8_char_len(bytes[i]);
      out.push_str(&s[i..i + ch_len]);
      i += ch_len;
    }
    out
  }

  /// Tokens that terminate inline collection regardless of nesting depth.
  pub(crate) fn is_top_level_break(k: &TokenKind) -> bool {
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
  }
}
