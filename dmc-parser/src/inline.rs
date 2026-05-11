use crate::ast::*;
use crate::parser::{MAX_LINK_LABEL_DEPTH, Parser};
use dmc_diagnostic::Code;
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
      if both_open_close && combined.is_multiple_of(3) && !(d.run as usize).is_multiple_of(3) {
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

pub(crate) fn normalize_legacy_gfm_emphasis(nodes: &mut [Node]) {
  for node in nodes.iter_mut() {
    match node {
      Node::Bold(inline) => {
        normalize_legacy_gfm_emphasis(&mut inline.children);
        flatten_nested_bold(&mut inline.children);
      },
      Node::Italic(inline) => normalize_legacy_gfm_emphasis(&mut inline.children),
      Node::Strikethrough(inline) => normalize_legacy_gfm_emphasis(&mut inline.children),
      Node::Link(link) => normalize_legacy_gfm_emphasis(&mut link.children),
      _ => {},
    }
  }
}

fn flatten_nested_bold(children: &mut Vec<Node>) {
  let mut flat = Vec::with_capacity(children.len());
  for child in std::mem::take(children) {
    if let Node::Bold(inner) = child {
      flat.extend(inner.children);
    } else {
      flat.push(child);
    }
  }
  *children = flat;
}

/// CM 6.4 "Unicode punctuation": an ASCII punctuation character, or a
/// character in Unicode general category P* (Pc Pd Pe Pf Pi Po Ps) or
/// S* (Sc Sk Sm So). Uses the `unicode-general-category` table for an
/// exact classification instead of the old hand-rolled range list.
fn is_unicode_punct(c: char) -> bool {
  use unicode_general_category::GeneralCategory as GC;
  if c.is_ascii_punctuation() {
    return true;
  }
  matches!(
    unicode_general_category::get_general_category(c),
    GC::ConnectorPunctuation
      | GC::DashPunctuation
      | GC::ClosePunctuation
      | GC::FinalPunctuation
      | GC::InitialPunctuation
      | GC::OtherPunctuation
      | GC::OpenPunctuation
      | GC::CurrencySymbol
      | GC::ModifierSymbol
      | GC::MathSymbol
      | GC::OtherSymbol
  )
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
  // Some HTML5 named refs expand to multiple code points; htmlentity's
  // `Entity::decode` only yields a single `char`, so patch the specific
  // CM 6.6 cases it cannot represent.
  if raw == "&ngE;" {
    return Some("\u{2267}\u{0338}".to_string());
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

fn is_email_local_byte(b: u8) -> bool {
  b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'+' | b'-')
}

fn trailing_email_local_suffix_start(s: &str) -> Option<usize> {
  let mut start = s.len();
  for (idx, ch) in s.char_indices().rev() {
    if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '+' | '-') {
      start = idx;
    } else {
      break;
    }
  }
  (start < s.len()).then_some(start)
}

/// GFM "extended email autolink": `local@domain` where `local` is
/// `[A-Za-z0-9._+-]+`, `domain` is `[A-Za-z0-9-_]+(\.[A-Za-z0-9-_]+)+`,
/// the domain has at least one `.`, and the final domain label does
/// not end with `-` or `_`. Returns `Some(pieces)` when at least one
/// email was found, mixing `Text` and `Link` nodes; `None` otherwise
/// so the caller emits a single plain `Text`.
fn split_email_autolinks(s: &str, span: &duck_diagnostic::Span) -> Option<Vec<Node>> {
  fn is_domain(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_')
  }
  let bytes = s.as_bytes();
  let mut out: Vec<Node> = Vec::new();
  let mut emitted_text = String::new();
  let mut found = false;
  let mut i = 0;
  let flush_text = |text: &mut String, out: &mut Vec<Node>| {
    if !text.is_empty() {
      out.push(Node::Text(Text { value: Parser::unescape_markdown(text), span: span.clone() }));
      text.clear();
    }
  };
  while i < bytes.len() {
    if bytes[i] == b'@' {
      // Walk back over local-part chars.
      let mut local_start = i;
      while local_start > 0 && is_email_local_byte(bytes[local_start - 1]) {
        local_start -= 1;
      }
      // Walk forward over domain chars + `.`.
      let mut domain_end = i + 1;
      while domain_end < bytes.len() && (is_domain(bytes[domain_end]) || bytes[domain_end] == b'.') {
        domain_end += 1;
      }
      let local = &s[local_start..i];
      // Trim trailing `.` (sentence punctuation) from the domain run.
      let raw_domain = &s[i + 1..domain_end];
      let domain = raw_domain.trim_end_matches('.');
      let valid = !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && domain.split('.').all(|lbl| !lbl.is_empty())
        && {
          let last = domain.rsplit('.').next().unwrap_or("");
          !last.ends_with('-') && !last.ends_with('_') && !last.is_empty()
        };
      if valid {
        // Pull any local-part bytes already in emitted_text out of it.
        if emitted_text.ends_with(local) {
          let keep = emitted_text.len() - local.len();
          emitted_text.truncate(keep);
        }
        flush_text(&mut emitted_text, &mut out);
        let email = format!("{}@{}", local, domain);
        out.push(Node::Link(Link {
          href: format!("mailto:{}", email),
          title: None,
          children: vec![Node::Text(Text { value: email.clone(), span: span.clone() })],
          span: span.clone(),
        }));
        found = true;
        i = i + 1 + domain.len();
        continue;
      }
    }
    let n = utf8_char_len(bytes[i]);
    emitted_text.push_str(&s[i..i + n]);
    i += n;
  }
  flush_text(&mut emitted_text, &mut out);
  if found { Some(out) } else { None }
}

fn split_email_autolinks_with_tail_underscore(
  raw_lexeme: &str,
  span: &duck_diagnostic::Span,
  out: &mut Vec<Node>,
  delims: &mut [DelimRecord],
) -> Option<Vec<Node>> {
  let underscore_idx = out.len().checked_sub(1)?;
  let prev_text = match out.get(underscore_idx)? {
    Node::Text(t) if !t.value.is_empty() && t.value.chars().all(|c| c == '_') => t.value.clone(),
    _ => return None,
  };
  let prev_node = out.get(underscore_idx.checked_sub(1)?)?;
  let prev_value = match prev_node {
    Node::Text(t) => t.value.clone(),
    _ => return None,
  };
  let delim_idx =
    delims.iter().rposition(|d| d.run > 0 && d.out_idx == underscore_idx && matches!(d.c, EmphasisChar::Underscore))?;
  let suffix_start = trailing_email_local_suffix_start(&prev_value)?;
  let candidate = format!("{}{}{}", &prev_value[suffix_start..], prev_text, raw_lexeme);
  let pieces = split_email_autolinks(&candidate, span)?;
  out.pop();
  let prefix = prev_value[..suffix_start].to_string();
  if prefix.is_empty() {
    out.pop();
  } else if let Some(Node::Text(t)) = out.last_mut() {
    t.value = prefix;
  }
  delims[delim_idx].run = 0;
  delims[delim_idx].can_open = false;
  delims[delim_idx].can_close = false;
  Some(pieces)
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
      if self.options.legacy_gfm_emphasis {
        normalize_legacy_gfm_emphasis(&mut out);
      }
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
      // A `</name>` close tag for a JSX element we are currently inside
      // belongs to that enclosing `parse_jsx` frame; stop here and leave
      // the close-tag tokens for its children loop instead of emitting
      // them as literal `</`, name, `>` text. (Stray / non-matching
      // close tags still fall through to the dispatch below.)
      if matches!(kind, TokenKind::JsxCloseTagStart) && self.jsx_close_tag_closes_enclosing() {
        break;
      }

      let span = t.span.clone();
      match &kind {
        TokenKind::Text => {
          let raw_lexeme: &'tokens str = self.peek_raw().unwrap_or("");
          let span_clone = span.clone();
          let gfm = self.options.gfm_autolinks;
          let trailing_underscore_to_break = raw_lexeme.contains('@')
            && matches!(
              self.tokens.get(self.pos + 1).map(|t| &t.kind),
              Some(TokenKind::Emphasis(EmphasisChar::Underscore, _))
            )
            && matches!(
              self.tokens.get(self.pos + 2).map(|t| &t.kind),
              Some(TokenKind::SoftBreak | TokenKind::HardBreak | TokenKind::BlankLine | TokenKind::Eof) | None
            );
          self.advance();
          // GFM email autolink extension: scan the text for
          // `local@domain` patterns and split into Text + Link runs.
          // Only active under `gfm_autolinks`; default MDX path leaves
          // the transformer to do it.
          if gfm
            && raw_lexeme.contains('@')
            && !trailing_underscore_to_break
            && let Some(pieces) = split_email_autolinks_with_tail_underscore(raw_lexeme, &span_clone, out, delims)
              .or_else(|| split_email_autolinks(raw_lexeme, &span_clone))
          {
            out.extend(pieces);
          } else {
            out.push(Node::Text(Text { value: Self::unescape_markdown(raw_lexeme), span: span_clone }));
          }
        },
        TokenKind::Autolink(kind) => {
          let kind = *kind;
          let raw = t.raw.to_string();
          self.advance();
          // Resolve display vs href per autolink kind.
          let link = match kind {
            AutolinkKind::AngleUrl => {
              let inner = raw.trim_start_matches('<').trim_end_matches('>').to_string();
              Some((inner.clone(), inner))
            },
            AutolinkKind::AngleEmail => {
              let inner = raw.trim_start_matches('<').trim_end_matches('>').to_string();
              Some((inner.clone(), format!("mailto:{inner}")))
            },
            // Bare URLs / `www.` runs are a GFM extension. Parser-level
            // autolink fires only when `ParseOptions::gfm_autolinks` is
            // on (spec runners, opt-in callers). Default MDX path keeps
            // them as Text so the `BareUrlAutolink` transformer can
            // operate later in the pipeline.
            AutolinkKind::BareUrl if self.options.gfm_autolinks => Some((raw.clone(), raw.clone())),
            AutolinkKind::BareWww if self.options.gfm_autolinks => Some((raw.clone(), format!("http://{}", raw))),
            AutolinkKind::BareUrl | AutolinkKind::BareWww => None,
          };
          if let Some((display, href)) = link {
            out.push(Node::Link(Link {
              href,
              title: None,
              children: vec![Node::Text(Text { value: display, span: span.clone() })],
              span,
            }));
          } else {
            out.push(Node::Text(Text { value: raw, span }));
          }
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
          let mut has_closer = false;
          for tok in &self.tokens[self.pos + 1..] {
            if matches!(tok.kind, TokenKind::Strikethrough) {
              has_closer = true;
              break;
            }
            if Self::is_top_level_break(&tok.kind) {
              break;
            }
          }
          if !has_closer {
            let raw = t.raw.to_string();
            self.advance();
            out.push(Node::Text(Text { value: raw, span }));
            continue;
          }
          self.advance();
          let inner = self.collect_inline(&|k| Self::is_top_level_break(k) || matches!(k, TokenKind::Strikethrough));
          if matches!(self.peek_kind(), Some(TokenKind::Strikethrough)) {
            self.advance();
            out.push(Node::Strikethrough(Inline { children: inner, span }));
          } else {
            out.push(Node::Text(Text { value: "~~".into(), span }));
            out.extend(inner);
          }
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
          if self.link_label_depth >= MAX_LINK_LABEL_DEPTH {
            // Adversarial `[[[[...`: stop recursing and emit `[` literal.
            self.advance();
            out.push(Node::Text(Text { value: "[".into(), span }));
            continue;
          }
          let start = self.pos;
          self.advance();
          let label_start_pos = self.pos;
          // Capture raw label text via pointer arithmetic on token raws
          // so emphasis/inline markers survive for ref-def lookup
          // (`[*foo* bar]: /url` matches `[*foo* bar]`).
          self.link_label_depth += 1;
          let inner = self.collect_inline(&|k| {
            matches!(k, TokenKind::LinkClose | TokenKind::BlankLine | TokenKind::SoftBreak | TokenKind::Eof)
          });
          self.link_label_depth -= 1;
          if !matches!(self.peek_kind(), Some(TokenKind::LinkClose)) {
            // No closing `]`: emit `[` literally and splice the
            // already-parsed inner nodes. Do NOT reset `self.pos` and
            // re-walk the inner tokens -- with N nested unclosed `[`
            // that re-parse is `O(2^N)` (each `[` re-scans the suffix
            // once recursively and once after backtracking). Keeping
            // `self.pos` after the inner makes label parsing linear.
            out.push(Node::Text(Text { value: "[".into(), span }));
            out.extend(inner);
            continue;
          }
          let label_end_pos = self.pos;
          let raw_inner_label = self.raw_source_for_token_range(label_start_pos, label_end_pos);
          self.advance(); // consume the closing `]`
          // CommonMark 6.3: a link cannot contain another link. When
          // inner contains a `Link` (recursively, e.g. wrapped in
          // emphasis), abandon the outer link parse and emit
          // `[inner]...` as text.
          fn contains_link(nodes: &[Node]) -> bool {
            for n in nodes {
              match n {
                Node::Link(_) => return true,
                Node::Bold(i) | Node::Italic(i) | Node::Strikethrough(i) if contains_link(&i.children) => {
                  return true;
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
              let body_start_pos = self.pos;
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
              let has_close = matches!(self.peek_kind(), Some(TokenKind::LinkTargetClose));
              let body_end_pos = self.pos;
              let paren_body = self.raw_source_for_token_range(body_start_pos, body_end_pos);
              if has_close {
                self.advance();
              }
              let well_formed = has_close && depth == 0;
              if !well_formed {
                let diagnostic = duck_diagnostic::diag!(
                  Code::UnterminatedLink,
                  self.span_at(start),
                  "inline link destination did not close before the end of the line; treating it as literal text"
                )
                .with_help("add a closing `)` to finish `[text](...)`, or escape the `[` if this should stay literal");
                self.emit_diagnostic(diagnostic);
              }
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
                  // paren body as literal inline content after it.
                  // Reparse the consumed token slice so raw HTML / entity
                  // refs inside the malformed body keep their normal CM
                  // inline semantics instead of being flattened into one
                  // escaped text blob.
                  let body_nodes = self.parse_literal_inline_slice(body_start_pos, body_end_pos, !has_close);
                  let label_raw = raw_inner_label.clone();
                  let label_plain = plain_text(&inner);
                  let resolved = self.refs.get(&label_raw).cloned().or_else(|| self.refs.get(&label_plain).cloned());
                  if let Some((href, title)) = resolved {
                    out.push(Node::Link(Link { href, title, children: inner, span: span.clone() }));
                    out.push(Node::Text(Text { value: "(".into(), span: span.clone() }));
                    out.extend(body_nodes);
                    if has_close {
                      out.push(Node::Text(Text { value: ")".into(), span }));
                    }
                  } else {
                    out.push(Node::Text(Text { value: "[".into(), span: span.clone() }));
                    for n in inner {
                      out.push(n);
                    }
                    out.push(Node::Text(Text { value: "](".into(), span: span.clone() }));
                    out.extend(body_nodes);
                    if has_close {
                      out.push(Node::Text(Text { value: ")".into(), span }));
                    }
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
              let label2_start_pos = self.pos;
              self.link_label_depth += 1;
              let label_inner = self.collect_inline(&|k| {
                matches!(k, TokenKind::LinkClose | TokenKind::BlankLine | TokenKind::SoftBreak | TokenKind::Eof)
              });
              self.link_label_depth -= 1;
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
              let label2_end_pos = self.pos;
              self.advance(); // consume label-side `]`
              let label_raw_2 = self.raw_source_for_token_range(label2_start_pos, label2_end_pos);
              // CM 4.7: label matching is case-fold + ws-collapse only.
              // Use the raw source slice (with backslash escapes intact)
              // so `[foo\!]` does NOT match `[foo!]: /url` per spec.
              let resolved = self.refs.get(&label_raw_2).cloned();
              if let Some((href, title)) = resolved {
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
              let resolved = self.refs.get(&label_raw).cloned();
              if let Some((href, title)) = resolved {
                out.push(Node::Link(Link { href, title, children: inner, span }));
                continue;
              }
              // Unresolved shortcut `[label]`: emit `[`, then re-parse
              // the label tokens into the *outer* delimiter stack so an
              // emphasis run that opens before `[` can close inside it
              // (CM ex 523: `*foo [bar* baz]`). The re-parse carries the
              // `link_label_depth` so adversarial `[[[...]]]` still hits
              // the recursion cap instead of cascading.
              out.push(Node::Text(Text { value: "[".into(), span: span.clone() }));
              self.replay_inline_slice_into(label_start_pos, label_end_pos, out, delims, false);
              out.push(Node::Text(Text { value: "]".into(), span }));
            },
          }
        },
        TokenKind::ImageMarker => {
          if self.link_label_depth >= MAX_LINK_LABEL_DEPTH {
            self.advance();
            out.push(Node::Text(Text { value: "![".into(), span }));
            continue;
          }
          // Lexer's `ImageMarker` already covers `![`, so the cursor is on
          // the alt-text body. Walk to the closing `]` (`LinkClose`).
          self.advance();
          // CM 6.4: image alt is parsed as an inline run -- nested
          // `[link]` and `![image]` are recursively flattened, and the
          // rendered alt is plain text (no markup). Parse the body
          // then derive both a raw label (for ref lookups) and a
          // plain-text alt.
          self.link_label_depth += 1;
          let alt_inner = self.collect_inline(&|k| {
            matches!(k, TokenKind::LinkClose | TokenKind::BlankLine | TokenKind::SoftBreak | TokenKind::Eof)
          });
          self.link_label_depth -= 1;
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
        TokenKind::JsxOpenTagStart | TokenKind::JsxCloseTagStart => {
          let raw = t.raw.to_string();
          if let Some(raw_html) = self.parse_inline_raw_html_tag() {
            out.push(raw_html);
          } else if matches!(kind, TokenKind::JsxOpenTagStart) {
            out.push(self.parse_jsx());
          } else {
            self.advance();
            out.push(Node::Text(Text { value: raw, span }));
          }
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

  fn parse_literal_inline_slice(&self, start: usize, end: usize, trim_trailing_break: bool) -> Vec<Node> {
    let mut tokens: Vec<_> = self.tokens[start..end].to_vec();
    if trim_trailing_break {
      while matches!(tokens.last().map(|t| &t.kind), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
        tokens.pop();
      }
    }
    let eof_span = tokens.last().map(|t| t.span.clone()).unwrap_or_else(default_span);
    tokens.push(dmc_lexer::token::Token::new(TokenKind::Eof, eof_span, ""));
    let mut diag = duck_diagnostic::DiagnosticEngine::<dmc_diagnostic::Code>::new();
    let mut parser = Parser::new(tokens, self.meta.clone(), &mut diag);
    parser.refs = self.refs.clone();
    parser.source = self.source;
    parser.link_label_depth = self.link_label_depth.saturating_add(1);
    parser.collect_inline(&|k| matches!(k, TokenKind::Eof))
  }

  /// Re-parse `tokens[start..end)` into the caller's `out`/`delims` so
  /// emphasis delimiters in the slice join the outer delimiter run
  /// (used when an unresolved shortcut `[label]` falls back to literal
  /// text but `*`/`_` runs still need to pair across the brackets).
  /// Carries `link_label_depth` so nested `[...]` re-parses still hit
  /// the recursion cap.
  fn replay_inline_slice_into(
    &self,
    start: usize,
    end: usize,
    out: &mut Vec<Node>,
    delims: &mut Vec<DelimRecord>,
    trim_trailing_break: bool,
  ) {
    let mut tokens: Vec<_> = self.tokens[start..end].to_vec();
    if trim_trailing_break {
      while matches!(tokens.last().map(|t| &t.kind), Some(TokenKind::SoftBreak) | Some(TokenKind::HardBreak)) {
        tokens.pop();
      }
    }
    let eof_span = tokens.last().map(|t| t.span.clone()).unwrap_or_else(default_span);
    tokens.push(dmc_lexer::token::Token::new(TokenKind::Eof, eof_span, ""));
    let mut diag = duck_diagnostic::DiagnosticEngine::<dmc_diagnostic::Code>::new();
    let mut parser = Parser::new(tokens, self.meta.clone(), &mut diag);
    parser.refs = self.refs.clone();
    parser.source = self.source;
    parser.link_label_depth = self.link_label_depth.saturating_add(1);
    parser.collect_inline_into(&|k| matches!(k, TokenKind::Eof), out, delims);
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
    // inside) or a bare run with no whitespace. Backslash escapes
    // the closing `>` inside the bracketed form -- scan byte-by-
    // byte so `<foo\>` is treated as unterminated, not as dest "foo\".
    let (dest_end, raw_dest) = if let Some(rest) = trimmed.strip_prefix('<') {
      let bytes = rest.as_bytes();
      let mut i = 0;
      let mut found = false;
      while i < bytes.len() {
        match bytes[i] {
          b'>' => {
            found = true;
            break;
          },
          b'<' | b'\n' => return None,
          b'\\' if i + 1 < bytes.len() => i += 2,
          _ => i += 1,
        }
      }
      if found {
        let inside = &rest[..i];
        (1 + i + 1, inside.to_string())
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
