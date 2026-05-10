use crate::ast::*;
use crate::parser::Parser;
use dmc_lexer::token::{AutolinkKind, EmphasisChar, TokenKind};

/// One emphasis delimiter run captured during `collect_inline`. The
/// post-pass walks these and pairs openers with closers per CM 6.4
/// to produce `<em>` / `<strong>` nodes.
pub(crate) struct DelimRecord {
  c: EmphasisChar,
  run: u8,
  can_open: bool,
  can_close: bool,
  out_idx: usize,
  span: duck_diagnostic::Span,
}

/// Resolve emphasis delimiter pairs in `out` per CM 6.4 stack
/// algorithm. Each consumed delimiter is replaced with the resulting
/// `<em>` / `<strong>` (or further nested) node.
pub(crate) fn resolve_emphasis_delims(out: &mut Vec<Node>, delims: &mut [DelimRecord]) {
  // CM 6.4 process_emphasis: walk delims left-to-right; for each
  // closer, scan back for the latest matching opener.
  let mut i = 0usize;
  while i < delims.len() {
    if !delims[i].can_close || delims[i].run == 0 {
      i += 1;
      continue;
    }
    // Scan back for a matching opener.
    let mut j: Option<usize> = None;
    let mut k = i;
    while k > 0 {
      k -= 1;
      let d = &delims[k];
      if d.run == 0 || !d.can_open || d.c != delims[i].c {
        continue;
      }
      // CM rule 9 / 10: combined-length-not-multiple-of-3 unless
      // both lengths are multiples of 3. Apply only when the
      // current delimiter is BOTH a potential opener and closer.
      let combined = (d.run + delims[i].run) as usize;
      let both_open_close = (d.can_open && d.can_close) || (delims[i].can_open && delims[i].can_close);
      if both_open_close && combined % 3 == 0 && d.run as usize % 3 != 0 {
        continue;
      }
      j = Some(k);
      break;
    }
    if let Some(open_idx) = j {
      let open_run = delims[open_idx].run;
      let close_run = delims[i].run;
      let use_n: u8 = if open_run >= 2 && close_run >= 2 { 2 } else { 1 };
      let open_out_idx = delims[open_idx].out_idx;
      let close_out_idx = delims[i].out_idx;
      let span = delims[open_idx].span.clone();
      let lo = open_out_idx + 1;
      let hi = close_out_idx;
      // CM 6.4 process_emphasis: remove any delimiters between the
      // opener and closer from the delimiter stack -- they are now
      // inside the wrapped run and can no longer pair with anything
      // outside it. Without this, an unmatched marker like the `_` in
      // `*foo _bar* baz_` gets paired across the `<em>` boundary.
      for d in delims.iter_mut().skip(open_idx + 1).take(i - open_idx - 1) {
        d.run = 0;
        d.can_open = false;
        d.can_close = false;
      }
      let inner: Vec<Node> = out.drain(lo..hi).collect();
      let node = if use_n == 1 {
        Node::Italic(Inline { children: inner, span })
      } else {
        Node::Bold(Inline { children: inner, span })
      };
      out[open_out_idx] = node;
      // Reduce both delim runs; keep placeholders for remaining
      // chars by truncating raw, otherwise drop.
      delims[open_idx].run -= use_n;
      delims[i].run -= use_n;
      let close_consumed = delims[i].run == 0;
      let open_consumed = delims[open_idx].run == 0;
      if close_consumed {
        out.remove(lo);
      } else {
        // Shrink closer placeholder text to remaining run length.
        let remaining = delims[i].run as usize;
        let truncated_raw = match delims[i].c {
          EmphasisChar::Asterisk => "*".repeat(remaining),
          EmphasisChar::Underscore => "_".repeat(remaining),
        };
        if let Some(Node::Text(t)) = out.get_mut(lo) {
          t.value = truncated_raw;
        }
      }
      if open_consumed {
        delims[open_idx].can_open = false;
      } else {
        // Truncate opener placeholder. (It now lives at open_out_idx
        // again -- but we replaced it with `node`. Need to insert a
        // new placeholder before the node.)
        let remaining = delims[open_idx].run as usize;
        let truncated_raw = match delims[open_idx].c {
          EmphasisChar::Asterisk => "*".repeat(remaining),
          EmphasisChar::Underscore => "_".repeat(remaining),
        };
        out.insert(open_out_idx, Node::Text(Text { value: truncated_raw, span: delims[open_idx].span.clone() }));
      }
      // Net removed slots = (drained inner) + (closer or 0) - (opener insert or 0).
      let inserted = if open_consumed { 0i64 } else { 1 };
      let removed_closer = if close_consumed { 1i64 } else { 0 };
      let removed_total = (hi as i64 - lo as i64) + removed_closer - inserted;
      for d in delims.iter_mut() {
        if d.out_idx > open_out_idx {
          d.out_idx = (d.out_idx as i64 - removed_total).max(0) as usize;
        }
      }
      // If opener still has runs left, the next closer may pair with
      // it (don't disable can_open). If closer still has runs, retry
      // pairing at i.
      if !close_consumed {
        continue;
      }
      i += 1;
      continue;
    }
    // No opener found -- the placeholder stays as literal text.
    i += 1;
  }
}

/// CM 6.4 "Unicode punctuation": ASCII punctuation plus Unicode
/// general categories Pc, Pd, Pe, Pf, Pi, Po, Ps and Sc, Sk, Sm, So.
/// Approximated with common ranges; full Unicode classification would
/// need a table.
fn is_unicode_punct(c: char) -> bool {
  if c.is_ascii_punctuation() {
    return true;
  }
  matches!(
    c,
    '\u{00A1}'..='\u{00BF}'
      | '\u{2010}'..='\u{205E}'
      | '\u{20A0}'..='\u{20CF}'
      | '\u{2200}'..='\u{22FF}'
      | '\u{2300}'..='\u{23FF}'
      | '\u{2600}'..='\u{26FF}'
      | '\u{2700}'..='\u{27BF}'
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

/// Flatten an inline node into a label string that preserves emphasis
/// markers and link / image bracketing -- used to reconstruct the
/// label string for ref-def lookup.
fn push_node_label(out: &mut String, node: &Node) {
  match node {
    Node::Text(t) => out.push_str(&t.value),
    Node::Bold(i) => {
      out.push_str("**");
      for c in &i.children {
        push_node_label(out, c);
      }
      out.push_str("**");
    },
    Node::Italic(i) => {
      out.push('*');
      for c in &i.children {
        push_node_label(out, c);
      }
      out.push('*');
    },
    Node::Strikethrough(i) => {
      out.push_str("~~");
      for c in &i.children {
        push_node_label(out, c);
      }
      out.push_str("~~");
    },
    Node::Link(l) => {
      out.push('[');
      for c in &l.children {
        push_node_label(out, c);
      }
      out.push(']');
      out.push('(');
      out.push_str(&l.href);
      out.push(')');
    },
    Node::Image(img) => {
      out.push_str("![");
      out.push_str(&img.alt);
      out.push(']');
      out.push('(');
      out.push_str(&img.src);
      out.push(')');
    },
    Node::InlineCode(c) => {
      out.push('`');
      out.push_str(&c.value);
      out.push('`');
    },
    _ => {},
  }
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
      Node::Image(img) => s.push_str(&img.alt),
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
    // CM 5.2: a list item with only whitespace content (e.g. `-   \n`
    // -> the trailing spaces produce a HardBreak token) is empty.
    // Consume the line break so the item renders as `<li></li>` and
    // the next marker on the following line is the next sibling item.
    if matches!(self.peek_kind(), Some(TokenKind::HardBreak)) {
      self.advance();
      return Vec::new();
    }
    self.collect_inline_until_break()
  }

  /// Collect inline nodes until `stop(kind)` returns true. The stopping token
  /// is left on the stream.
  pub(crate) fn collect_inline(&mut self, stop: &dyn Fn(&TokenKind) -> bool) -> Vec<Node> {
    let mut out = Vec::new();
    let mut delims: Vec<DelimRecord> = Vec::new();
    self.collect_inline_into(stop, &mut out, &mut delims);
    if !delims.is_empty() {
      resolve_emphasis_delims(&mut out, &mut delims);
    }
    out
  }

  /// Variant of `collect_inline` that pushes into caller-provided
  /// buffers and does NOT run emphasis resolution. Lets multi-segment
  /// callers (parse_paragraph across soft breaks) accumulate one delim
  /// stack and resolve once at the end.
  pub(crate) fn collect_inline_into(
    &mut self,
    stop: &dyn Fn(&TokenKind) -> bool,
    out: &mut Vec<Node>,
    delims: &mut Vec<DelimRecord>,
  ) {
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
          let dc: EmphasisChar = *c;
          let dn = *n;
          let raw = t.raw.to_string();
          let dspan = span.clone();
          // CM 6.4 flanking rules. Compute can_open / can_close from
          // the current cursor context; resolution into <em>/<strong>
          // happens after collect_inline returns via the delimiter
          // stack walk below.
          let next_tok = self.tokens.get(self.pos + 1);
          let next_ws = match next_tok.map(|t| &t.kind) {
            Some(TokenKind::SoftBreak)
            | Some(TokenKind::HardBreak)
            | Some(TokenKind::BlankLine)
            | Some(TokenKind::Eof)
            | None => true,
            Some(TokenKind::Whitespace(_)) => true,
            _ => next_tok.is_some_and(|t| t.raw.chars().next().is_some_and(|c| c.is_whitespace())),
          };
          let next_punct = next_tok.is_some_and(|t| t.raw.chars().next().is_some_and(is_unicode_punct));
          let next_alnum = next_tok.is_some_and(|t| t.raw.chars().next().is_some_and(|c| c.is_alphanumeric()));
          let prev_char: Option<char> =
            self.pos.checked_sub(1).and_then(|i| self.tokens.get(i)).and_then(|t| t.raw.chars().last());
          let prev_ws = prev_char.map(|c| c.is_whitespace()).unwrap_or(true);
          let prev_punct = prev_char.is_some_and(is_unicode_punct);
          let prev_alnum = prev_char.is_some_and(|c| c.is_alphanumeric());
          // Left-flanking: not followed by ws AND (not followed by
          // punct OR preceded by ws/punct).
          let left_flank = !next_ws && (!next_punct || prev_ws || prev_punct);
          // Right-flanking: not preceded by ws AND (not preceded by
          // punct OR followed by ws/punct).
          let right_flank = !prev_ws && (!prev_punct || next_ws || next_punct);
          let mut can_open = left_flank;
          let mut can_close = right_flank;
          if dc == EmphasisChar::Underscore {
            // Intra-word `_` rule: can't open when preceded by alnum,
            // can't close when followed by alnum (unless flanking
            // requirement compensates per CM rule 7-8).
            if prev_alnum && next_alnum {
              can_open = false;
              can_close = false;
            } else if prev_alnum {
              can_open = false;
            } else if next_alnum {
              can_close = false;
            }
          }
          self.advance();
          let idx = out.len();
          out.push(Node::Text(Text { value: raw, span: dspan.clone() }));
          delims.push(DelimRecord { c: dc, run: dn, can_open, can_close, out_idx: idx, span: dspan });
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
          // Capture raw label text via pointer arithmetic on token raws
          // so emphasis/inline markers survive for ref-def lookup
          // (`[*foo* bar]: /url` matches `[*foo* bar]`).
          let label_start_ptr = self.peek().map(|t| t.raw.as_ptr() as usize).unwrap_or(0);
          let inner = self.collect_inline(&|k| {
            matches!(k, TokenKind::LinkClose | TokenKind::BlankLine | TokenKind::SoftBreak | TokenKind::Eof)
          });
          if !matches!(self.peek_kind(), Some(TokenKind::LinkClose)) {
            self.pos = start;
            out.push(Node::Text(Text { value: "[".into(), span }));
            self.advance();
            continue;
          }
          let label_end_ptr = self
            .tokens
            .get(self.pos.saturating_sub(1))
            .map(|t| t.raw.as_ptr() as usize + t.raw.len())
            .unwrap_or(label_start_ptr);
          let raw_inner_label = if label_end_ptr > label_start_ptr {
            let len = label_end_ptr - label_start_ptr;
            // SAFETY: every Token.raw is a slice of the same source.
            let slice = unsafe { std::slice::from_raw_parts(label_start_ptr as *const u8, len) };
            std::str::from_utf8(slice).map(|s| s.to_string()).unwrap_or_default()
          } else {
            String::new()
          };
          self.advance(); // consume the closing `]`
          // CommonMark 6.3: a link cannot contain another link. When
          // inner contains a `Link` (recursively, e.g. wrapped in
          // emphasis), abandon the outer link parse and emit
          // `[inner]...` as text.
          fn contains_link(nodes: &[Node]) -> bool {
            for n in nodes {
              match n {
                Node::Link(_) => return true,
                Node::Bold(i) | Node::Italic(i) | Node::Strikethrough(i) => {
                  if contains_link(&i.children) {
                    return true;
                  }
                },
                _ => {},
              }
            }
            false
          }
          let inner_has_link = contains_link(&inner);
          // CommonMark 6.3: classify the link form by what follows the
          // closing `]`.
          //   `(...)`     -> inline link
          //   `[...]`     -> full reference `[text][label]` or
          //                  collapsed `[label][]`
          //   nothing     -> shortcut reference `[label]`
          // Reference forms resolve against the ref-def map populated in
          // the pre-pass; unresolved refs fall back to literal text.
          if inner_has_link {
            // Emit `[`, inner..., `]` as raw output and let the next
            // iteration handle whatever follows (it might itself be a
            // valid link reference).
            out.push(Node::Text(Text { value: "[".into(), span: span.clone() }));
            for n in inner {
              out.push(n);
            }
            out.push(Node::Text(Text { value: "]".into(), span }));
            continue;
          }
          match self.peek_kind() {
            Some(TokenKind::LinkTargetOpen) => {
              self.advance(); // consume `(`
              let body_start_ptr = self.peek().map(|t| t.raw.as_ptr() as usize).unwrap_or(0);
              // CM 6.3: bare destinations allow balanced parens. Track
              // depth so `[link](foo(and(bar)))` keeps both inner pairs
              // before the matching outer close. Stop at blank lines
              // and end-of-stream so an unbalanced run can't swallow
              // following content. A `<...>` bracketed destination is
              // exempt: inside `<...>`, `(` and `)` are literal so the
              // depth counter must skip them.
              let mut depth = 0i32;
              let mut in_angle = false;
              while let Some(tok) = self.peek() {
                if in_angle {
                  // Walk until we see a token whose raw ends with `>`.
                  let raw = tok.raw;
                  if raw.contains('>') {
                    in_angle = false;
                  }
                  match &tok.kind {
                    TokenKind::Eof | TokenKind::BlankLine => break,
                    TokenKind::SoftBreak | TokenKind::HardBreak => {
                      // Newline inside `<...>` invalidates the bracketed
                      // form; abort so the malformed branch fires.
                      in_angle = false;
                      self.advance();
                    },
                    _ => {
                      self.advance();
                    },
                  }
                  continue;
                }
                match &tok.kind {
                  TokenKind::LinkTargetClose if depth == 0 => break,
                  TokenKind::Eof | TokenKind::BlankLine => break,
                  TokenKind::SoftBreak | TokenKind::HardBreak => {
                    self.advance();
                  },
                  TokenKind::LinkTargetOpen => {
                    depth += 1;
                    self.advance();
                  },
                  TokenKind::LinkTargetClose => {
                    depth -= 1;
                    self.advance();
                  },
                  _ => {
                    // Detect a `<` opener for a bracketed destination.
                    // The lexer may emit it as Text or escape-pair; we
                    // recognize the leading `<` byte in the raw lexeme.
                    if tok.raw.starts_with('<') && depth == 0 && !tok.raw.contains('>') {
                      in_angle = true;
                    }
                    self.advance();
                  },
                }
              }
              // Reconstruct paren body verbatim from source. Lexer's JSX
              // path normalizes whitespace inside `<...>`, so per-token
              // concat would lose spaces; pointer arithmetic preserves
              // them since every `Token.raw` borrows from the same
              // source string.
              let body_end_ptr = self
                .tokens
                .get(self.pos.saturating_sub(1))
                .map(|t| t.raw.as_ptr() as usize + t.raw.len())
                .unwrap_or(body_start_ptr);
              let paren_body = if body_end_ptr > body_start_ptr {
                let len = body_end_ptr - body_start_ptr;
                // SAFETY: every `Token.raw` is a slice of the same
                // `&'src str`; pointer subtraction stays in-bounds.
                let slice = unsafe { std::slice::from_raw_parts(body_start_ptr as *const u8, len) };
                std::str::from_utf8(slice).map(|s| s.to_string()).unwrap_or_default()
              } else {
                String::new()
              };
              let has_close = matches!(self.peek_kind(), Some(TokenKind::LinkTargetClose));
              if has_close {
                self.advance();
              }
              let well_formed = has_close && depth == 0;
              match if well_formed { Self::split_destination_title(&paren_body) } else { None } {
                Some((href, title)) => {
                  let href = decode_entities_in(&Self::unescape_markdown(&href));
                  let title = title.map(|t| decode_entities_in(&Self::unescape_markdown(&t)));
                  out.push(Node::Link(Link { href, title, children: inner, span }));
                },
                None => {
                  // CM 6.3: malformed `[label](destination)` falls back to
                  // shortcut reference resolution -- if `[label]` matches
                  // a definition, render the link and leave the failed
                  // paren body as literal text after it.
                  let label_raw = raw_inner_label.clone();
                  let label_plain = plain_text(&inner);
                  let resolved = self.refs.get(&label_raw).cloned().or_else(|| self.refs.get(&label_plain).cloned());
                  if let Some((href, title)) = resolved {
                    out.push(Node::Link(Link { href, title, children: inner, span: span.clone() }));
                    let close_str = if has_close { ")" } else { "" };
                    out.push(Node::Text(Text { value: format!("({}{}", paren_body, close_str), span }));
                  } else {
                    out.push(Node::Text(Text { value: "[".into(), span: span.clone() }));
                    for n in inner {
                      out.push(n);
                    }
                    let close_str = if has_close { ")" } else { "" };
                    out.push(Node::Text(Text { value: format!("]({}{}", paren_body, close_str), span }));
                  }
                },
              }
            },
            Some(TokenKind::LinkOpen) => {
              // Reference form: peek the second `[..]` to distinguish
              // collapsed (`[]`) from full (`[label]`).
              let second_bracket_pos = self.pos;
              self.advance(); // consume `[`
              if matches!(self.peek_kind(), Some(TokenKind::LinkClose)) {
                self.advance();
                let label_raw = raw_inner_label.clone();
                let label_plain = plain_text(&inner);
                let resolved = self.refs.get(&label_raw).cloned().or_else(|| self.refs.get(&label_plain).cloned());
                if let Some((href, title)) = resolved {
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
                let label_raw = raw_inner_label.clone();
                let label_plain = plain_text(&inner);
                let resolved = self.refs.get(&label_raw).cloned().or_else(|| self.refs.get(&label_plain).cloned());
                if let Some((href, title)) = resolved {
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
              // CM 6.3: an unresolved full reference `[a][b]` leaves the
              // first label literal and rewinds so the second `[...]` can be
              // reparsed as a fresh shortcut/full reference.
              out.push(Node::Text(Text { value: "[".into(), span: span.clone() }));
              for n in inner {
                out.push(n);
              }
              out.push(Node::Text(Text { value: "]".into(), span }));
              self.pos = second_bracket_pos;
            },
            _ => {
              // Shortcut `[label]`. Resolve via the ref-def map; falls
              // back to bracketed text when no matching definition.
              let label_raw = raw_inner_label.clone();
              let label_plain = plain_text(&inner);
              let resolved = self.refs.get(&label_raw).cloned().or_else(|| self.refs.get(&label_plain).cloned());
              if let Some((href, title)) = resolved {
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
          // CM 6.4: image alt is parsed as an inline run -- nested
          // `[link]` and `![image]` are recursively flattened, and the
          // rendered alt is plain text (no markup). Parse the body
          // then derive both a raw label (for ref lookups) and a
          // plain-text alt.
          let alt_inner = self.collect_inline(&|k| {
            matches!(k, TokenKind::LinkClose | TokenKind::BlankLine | TokenKind::SoftBreak | TokenKind::Eof)
          });
          let mut alt_raw = String::new();
          {
            // Rebuild raw label by walking the original token range. We
            // reuse the cursor; for simplicity, just stringify the inner
            // tree. This is fine for ref-def lookup as labels normalize.
            for n in &alt_inner {
              push_node_label(&mut alt_raw, n);
            }
          }
          if matches!(self.peek_kind(), Some(TokenKind::LinkClose)) {
            self.advance();
          }
          let alt = plain_text(&alt_inner);
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
            match Self::split_destination_title(&paren_body) {
              Some((src, title)) => {
                let src = decode_entities_in(&Self::unescape_markdown(&src));
                let title = title.map(|t| decode_entities_in(&Self::unescape_markdown(&t)));
                out.push(Node::Image(Image { src, alt, title, span }));
              },
              None => {
                out.push(Node::Text(Text { value: format!("![{}]({})", alt, paren_body), span }));
              },
            }
            continue;
          }
          // Reference forms: `[label]` (full / collapsed) or shortcut.
          let mut label = alt_raw.clone();
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
              label = second;
            }
            // collapsed `[...][]` keeps `label = alt_raw`.
          }
          if let Some((href, title)) = self.refs.get(&label).cloned() {
            out.push(Node::Image(Image { src: href, alt, title, span }));
            continue;
          }
          // Unresolved reference -- fall back to literal text.
          out.push(Node::Text(Text { value: format!("![{}]", alt), span }));
        },
        TokenKind::HtmlCommentOpen | TokenKind::HtmlBlockOpen(_) => {
          let close_kind = match kind {
            TokenKind::HtmlCommentOpen => TokenKind::HtmlCommentClose,
            _ => TokenKind::HtmlBlockClose,
          };
          let mut value = t.raw.to_string();
          let html_span = span.clone();
          self.advance();
          loop {
            match self.peek_kind() {
              Some(k) if std::mem::discriminant(k) == std::mem::discriminant(&close_kind) => {
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
          out.push(Node::Html(Html { value, span: html_span }));
          continue;
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
          // Backslash-induced hard break: strip the trailing `\` from
          // the preceding text node IFF the break is followed by more
          // inline content. CM 6.7 keeps the `\` literal when the
          // break sits at the end of the inline run (`foo\` with no
          // continuation -> `<p>foo\</p>`).
          let has_following_inline = !matches!(
            self.peek_kind(),
            Some(TokenKind::BlankLine)
              | Some(TokenKind::Eof)
              | Some(TokenKind::SoftBreak)
              | Some(TokenKind::HardBreak)
              | Some(TokenKind::ThematicBreak)
              | Some(TokenKind::SetextUnderline(_))
              | None
          );
          if has_following_inline
            && let Some(Node::Text(t)) = out.last_mut()
            && t.value.ends_with('\\')
          {
            t.value.pop();
            if t.value.is_empty() {
              out.pop();
            }
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
  }

  /// Split the body of a `(...)` link/image destination into
  /// `(href, title)`. CommonMark allows an optional trailing
  /// `"title"` / `'title'` / `(title)` separated from the destination
  /// by whitespace. Unterminated/missing title returns `(body, None)`.
  fn split_destination_title(body: &str) -> Option<(String, Option<String>)> {
    fn is_link_space(c: char) -> bool {
      matches!(c, ' ' | '\t' | '\n')
    }

    fn parse_link_title(rest: &str) -> Option<String> {
      let bytes = rest.as_bytes();
      let (open, close) = match bytes.first().copied()? {
        b'"' => (b'"', b'"'),
        b'\'' => (b'\'', b'\''),
        b'(' => (b'(', b')'),
        _ => return None,
      };
      if bytes.len() < 2 || *bytes.last()? != close {
        return None;
      }
      let inner = &rest[1..rest.len() - 1];
      let mut escaped = false;
      let mut paren_depth = 0usize;
      for ch in inner.bytes() {
        if escaped {
          escaped = false;
          continue;
        }
        if ch == b'\\' {
          escaped = true;
          continue;
        }
        if open == b'(' {
          match ch {
            b'(' => paren_depth += 1,
            b')' => {
              if paren_depth == 0 {
                return None;
              }
              paren_depth -= 1;
            },
            _ => {},
          }
        } else if ch == close {
          return None;
        }
      }
      if open == b'(' && paren_depth != 0 {
        return None;
      }
      Some(inner.to_string())
    }

    let trimmed = body.trim();
    if trimmed.is_empty() {
      return Some((String::new(), None));
    }
    // CM 6.3 link destination: `<...>` (no spaces / line breaks
    // inside) or a bare run with no whitespace.
    let (dest_end, raw_dest) = if let Some(rest) = trimmed.strip_prefix('<') {
      if let Some(end) = rest.find('>') {
        let inside = &rest[..end];
        if inside.contains('\n') || inside.contains('<') {
          return None;
        }
        (1 + end + 1, inside.to_string())
      } else {
        return None;
      }
    } else {
      // Bare destination: only ASCII space / tab / line endings split off
      // an optional title. Other Unicode whitespace (e.g. NBSP) stays in
      // the destination per CM 6.3.
      let dest_end = trimmed.find(is_link_space).unwrap_or(trimmed.len());
      (dest_end, trimmed[..dest_end].to_string())
    };
    let rest = trimmed[dest_end..].trim_start_matches(is_link_space);
    if rest.is_empty() {
      return Some((raw_dest, None));
    }
    // Title: `"..."`, `'...'`, or `(...)`. Trailing junk after the
    // destination (without a valid title pair) makes the link
    // malformed per CM 6.3.
    parse_link_title(rest).map(|title| (raw_dest, Some(title)))
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
