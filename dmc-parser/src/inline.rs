use crate::ast::*;
use crate::parser::Parser;
use dmc_lexer::token::{AutolinkKind, EmphasisChar, TokenKind};

/// CM 6.4 "Unicode punctuation": ASCII punctuation plus Unicode
/// general categories Pc, Pd, Pe, Pf, Pi, Po, Ps. Approximated here as
/// `c.is_ascii_punctuation()` plus a handful of common ranges; full
/// Unicode classification would need a table.
fn is_unicode_punct(c: char) -> bool {
  if c.is_ascii_punctuation() {
    return true;
  }
  matches!(
    c,
    '\u{00A1}'..='\u{00BF}'
      | '\u{2010}'..='\u{205E}'
      | '\u{2E00}'..='\u{2E7F}'
      | '\u{3001}'..='\u{303F}'
      | '\u{FE30}'..='\u{FE6F}'
      | '\u{FF00}'..='\u{FF65}'
  )
}

/// Strip lexer-emitted inline markers (`*`, `_`, backticks) from a
/// raw text run so image alt text renders as plain text per CM 6.3.
fn strip_inline_markers(s: &str) -> String {
  s.chars().filter(|c| !matches!(c, '*' | '_' | '`')).collect()
}

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

/// Decode every `&...;` reference inside a string and return the
/// resolved version. Numeric refs always succeed; named refs hit the
/// HTML5 table via `htmlentity`. Unknown / malformed refs survive
/// verbatim so nothing is lost.
pub(crate) fn decode_entities_in(s: &str) -> String {
  if !s.contains('&') {
    return s.to_string();
  }
  let bytes = s.as_bytes();
  let mut out = String::with_capacity(s.len());
  let mut i = 0;
  while i < bytes.len() {
    if bytes[i] == b'&' {
      // Find the next `;` within a reasonable window (CM caps named
      // entities at 32 chars; numeric at ~10).
      let mut j = i + 1;
      let cap = (i + 33).min(bytes.len());
      while j < cap && bytes[j] != b';' {
        j += 1;
      }
      if j < cap && bytes[j] == b';' {
        let raw = &s[i..=j];
        if let Some(decoded) = decode_entity(raw) {
          out.push_str(&decoded);
          i = j + 1;
          continue;
        }
      }
    }
    out.push(bytes[i] as char);
    i += 1;
  }
  out
}

/// Decode a CommonMark entity reference (`&amp;`, `&#9;`, `&#x2A;`).
/// Returns `None` when the form is malformed or the named entity is
/// not recognized; the caller falls back to the raw lexeme so the
/// output stays lossless.
fn decode_entity(raw: &str) -> Option<String> {
  // Numeric forms: handle ourselves so we can fold NUL -> U+FFFD per
  // CM 6.6.
  if let Some(inner) = raw.strip_prefix('&').and_then(|s| s.strip_suffix(';'))
    && let Some(rest) = inner.strip_prefix('#')
  {
    let cp: u32 = if let Some(hex) = rest.strip_prefix(['x', 'X']) {
      u32::from_str_radix(hex, 16).ok()?
    } else {
      rest.parse().ok()?
    };
    let cp = if cp == 0 { 0xFFFD } else { cp };
    return char::from_u32(cp).map(|c| c.to_string());
  }
  // Named forms via the full HTML5 entity table.
  use htmlentity::entity::{ICodedDataTrait, decode};
  let decoded = decode(raw.as_bytes());
  let s: String = decoded.to_string().ok()?;
  if s == raw { None } else { Some(s) }
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
          // CM 6.4 left-flanking rule: an emphasis run that is followed
          // by whitespace / EOL / EOF can't open. For `_` underscores
          // also block opening when the previous char is alphanumeric
          // (intra-word `_` rule). When can't open, surface the run as
          // text and let the closer's logic decide if anything still
          // pairs.
          let next_tok = self.tokens.get(self.pos + 1);
          let next_is_break = match next_tok.map(|t| &t.kind) {
            Some(TokenKind::SoftBreak)
            | Some(TokenKind::HardBreak)
            | Some(TokenKind::BlankLine)
            | Some(TokenKind::Eof)
            | None => true,
            Some(TokenKind::Whitespace(_)) => true,
            // Catch Unicode whitespace embedded in a Text token (e.g.
            // NBSP `\xa0`) -- CM 6.4 left-flanking rule treats those
            // as whitespace too.
            _ => next_tok.is_some_and(|t| t.raw.chars().next().is_some_and(|c| c.is_whitespace())),
          };
          let next_punct = next_tok.is_some_and(|t| t.raw.chars().next().is_some_and(is_unicode_punct));
          let prev_char: Option<char> = match out.last() {
            Some(Node::Text(t)) => t.value.chars().last(),
            _ => None,
          };
          let prev_is_ws_or_punct =
            prev_char.is_none() || prev_char.is_some_and(|c| c.is_whitespace() || is_unicode_punct(c));
          let prev_alnum = prev_char.is_some_and(|c| c.is_alphanumeric());
          // CM 6.4 left-flanking-delimiter run.
          let mut can_open = !next_is_break;
          if can_open && next_punct {
            can_open = prev_is_ws_or_punct;
          }
          if open_c == EmphasisChar::Underscore && prev_alnum {
            can_open = false;
          }
          if !can_open {
            self.advance();
            out.push(Node::Text(Text { value: raw, span }));
            continue;
          }
          self.advance();
          let inner = self.collect_inline(&|k| {
            Self::is_top_level_break(k)
              || matches!(k, TokenKind::Emphasis(cc, m) if *cc == open_c && *m == open_n)
              || matches!(k, TokenKind::LinkClose)
          });
          // CM 6.4 right-flanking: closer must not be preceded by
          // whitespace. Underscore additionally can't close when
          // immediately followed by an alphanumeric char (intra-word
          // `_`).
          let closed_kind =
            matches!(self.peek_kind(), Some(TokenKind::Emphasis(cc, m)) if *cc == open_c && *m == open_n);
          // Char in source immediately before the would-be closer = last
          // char of the previously-consumed token.
          let prev_at_close =
            self.pos.checked_sub(1).and_then(|i| self.tokens.get(i)).and_then(|t| t.raw.chars().last());
          let after_closer_tok = self.tokens.get(self.pos + 1);
          let after_alnum = after_closer_tok.is_some_and(|t| t.raw.chars().next().is_some_and(|c| c.is_alphanumeric()));
          let prev_at_close_ws = prev_at_close.map(|c| c.is_whitespace()).unwrap_or(true);
          let prev_at_close_punct = prev_at_close.is_some_and(is_unicode_punct);
          let after_punct = after_closer_tok.is_some_and(|t| t.raw.chars().next().is_some_and(is_unicode_punct));
          let after_ws = match after_closer_tok.map(|t| &t.kind) {
            Some(TokenKind::SoftBreak)
            | Some(TokenKind::HardBreak)
            | Some(TokenKind::BlankLine)
            | Some(TokenKind::Eof)
            | None => true,
            Some(TokenKind::Whitespace(_)) => true,
            _ => after_closer_tok.is_some_and(|t| t.raw.chars().next().is_some_and(|c| c.is_whitespace())),
          };
          let mut can_close = closed_kind && !prev_at_close_ws;
          // CM 6.4 rule 4 for `*`: closer is right-flanking; if also
          // left-flanking (preceded by punctuation), closer must be
          // followed by whitespace / punct / EOF for the run to close.
          if can_close && prev_at_close_punct && !(after_ws || after_punct) {
            can_close = false;
          }
          if open_c == EmphasisChar::Underscore && after_alnum {
            can_close = false;
          }
          let closed = closed_kind && can_close;
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
              let href = decode_entities_in(&Self::unescape_markdown(&href));
              let title = title.map(|t| decode_entities_in(&Self::unescape_markdown(&t)));
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
          // Strip lexer-emitted markers from the captured alt so it
          // renders as plain text (CM 6.3 image alt is the inner text).
          let alt = strip_inline_markers(&alt);
          // Inline form: `(...)` follows.
          if matches!(self.peek_kind(), Some(TokenKind::LinkTargetOpen)) {
            self.advance();
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
            let (src, title) = Self::split_destination_title(&paren_body);
            let src = decode_entities_in(&Self::unescape_markdown(&src));
            let title = title.map(|t| decode_entities_in(&Self::unescape_markdown(&t)));
            out.push(Node::Image(Image { src, alt, title, span }));
            continue;
          }
          // Reference forms: `[label]` (full / collapsed) or shortcut.
          let mut label = alt.clone();
          if matches!(self.peek_kind(), Some(TokenKind::LinkOpen)) {
            self.advance();
            let mut second = String::new();
            let mut has_second = false;
            while let Some(tok) = self.peek() {
              match &tok.kind {
                TokenKind::LinkClose => {
                  self.advance();
                  break;
                },
                TokenKind::Eof | TokenKind::BlankLine | TokenKind::SoftBreak => break,
                _ => {
                  second.push_str(tok.raw);
                  has_second = true;
                  self.advance();
                },
              }
            }
            if has_second && !second.is_empty() {
              label = strip_inline_markers(&second);
            }
            // collapsed `[...][]` keeps `label = alt`.
          }
          if let Some((href, title)) = self.refs.get(&label).cloned() {
            out.push(Node::Image(Image { src: href, alt, title, span }));
            continue;
          }
          // Unresolved reference -- fall back to literal text.
          out.push(Node::Text(Text { value: format!("![{}]", alt), span }));
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
        TokenKind::HardBreak => {
          self.advance();
          // CM 6.7: drop trailing whitespace-only text immediately
          // before the hard break -- the spaces / `\` produced the
          // break itself, they shouldn't render in the body.
          while let Some(Node::Text(t)) = out.last()
            && t.value.chars().all(|c| c == ' ' || c == '\t')
          {
            out.pop();
          }
          out.push(Node::HardBreak(BreakNode { span }));
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
    // CM 6.3 link destination: `<...>` (no spaces / line breaks
    // inside) or a bare run with no whitespace.
    let (dest_end, raw_dest) = if let Some(rest) = trimmed.strip_prefix('<') {
      if let Some(end) = rest.find('>') {
        let inside = &rest[..end];
        if inside.contains('\n') || inside.contains('<') {
          // CM rejects line breaks / nested `<` inside angle dest.
          (1 + end + 1, format!("<{}>", inside))
        } else {
          (1 + end + 1, inside.to_string())
        }
      } else {
        // No closing `>`. Whole body is bare dest fallback.
        return (trimmed.to_string(), None);
      }
    } else {
      // Bare destination: stop at first whitespace.
      let dest_end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
      (dest_end, trimmed[..dest_end].to_string())
    };
    let rest = trimmed[dest_end..].trim_start();
    if rest.is_empty() {
      return (raw_dest, None);
    }
    // Title: `"..."`, `'...'`, or `(...)`.
    let bytes = rest.as_bytes();
    let first = bytes[0];
    let last = bytes[bytes.len() - 1];
    let matches_pair =
      (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') || (first == b'(' && last == b')');
    if matches_pair && rest.len() >= 2 {
      (raw_dest, Some(rest[1..rest.len() - 1].to_string()))
    } else {
      // Malformed title -- whole body is dest only? CM falls back to
      // not-a-link, but we don't have the wholesale-rollback path
      // here. Best-effort: keep the raw dest, no title.
      (raw_dest, None)
    }
  }

  /// Strip `\X` -> `X` for the standard CommonMark escapable set so
  /// authors can write `\*literal\*` without the asterisks turning into
  /// emphasis. The lexer keeps the backslash in `Text` raw to preserve
  /// source spans; this collapses it for the rendered text.
  pub(crate) fn unescape_markdown(s: &str) -> String {
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
