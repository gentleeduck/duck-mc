//! Link- and footnote-reference tables. CM 4.7 + 6.3 + GFM footnotes
//! require a two-pass parse: first walk the token stream to harvest all
//! definitions, then resolve `[label]` / `[text][label]` / `[label][]`
//! references against the table during the main parse.

use std::collections::HashMap;

/// Resolved link reference: destination URL plus optional title.
pub type LinkRef = (String, Option<String>);

/// Lookup table built once per parse.
#[derive(Debug, Default, Clone)]
pub struct RefMap {
  links: HashMap<String, LinkRef>,
}

impl RefMap {
  pub fn new() -> Self {
    Self::default()
  }

  /// First definition wins (CM 4.7).
  pub fn insert(&mut self, label: &str, url: String, title: Option<String>) {
    let key = normalize_label(label);
    if !key.is_empty() {
      self.links.entry(key).or_insert((url, title));
    }
  }

  pub fn get(&self, label: &str) -> Option<&LinkRef> {
    self.links.get(&normalize_label(label))
  }

  pub fn is_empty(&self) -> bool {
    self.links.is_empty()
  }
}

/// CM 4.7: case-insensitive comparison, internal whitespace collapsed
/// to single spaces, leading/trailing whitespace trimmed. Backslash
/// escapes resolve before comparison so `[Foo\]]` and `Foo]` match.
pub fn normalize_label(s: &str) -> String {
  // CM 4.7: normalize by case-fold + ws-collapse only. Backslash
  // escapes are NOT unescaped during label matching, so `[foo\!]` and
  // `[foo!]` match different labels.
  let mut out = String::with_capacity(s.len());
  let mut prev_ws = true;
  for c in s.chars() {
    if c.is_whitespace() {
      if !prev_ws {
        out.push(' ');
        prev_ws = true;
      }
    } else {
      push_case_folded(&mut out, c);
      prev_ws = false;
    }
  }
  if out.ends_with(' ') {
    out.pop();
  }
  out
}

/// CM 4.7 / 6.3: link reference labels are matched after Unicode
/// case folding. `to_lowercase` matches that for most code points, but
/// `ß` (U+00DF) folds to `ss` and capital `ẞ` (U+1E9E) lowercases to
/// `ß` -- so a `[ẞ]` reference fails to match a `[SS]:` definition
/// unless we explicitly fold `ß` -> `ss` here.
///
/// Broader full-Unicode case folding still needs a dedicated mapping
/// table or crate; for now the parser keeps the lightweight in-tree
/// approximation plus the CommonMark-critical sharp-s special case.
fn push_case_folded(out: &mut String, c: char) {
  for low in c.to_lowercase() {
    if low == '\u{00DF}' {
      out.push_str("ss");
    } else {
      out.push(low);
    }
  }
}

/// Parse the raw lexeme of a `LinkRefDef` token into
/// `(label, url, title)`. Returns `None` on malformed input; the lexer
/// already validated the gross structure (`[label]:` plus a non-empty
/// destination), so failures here mostly mean a missing `]` or `:`.
pub fn parse_link_ref_def(raw: &str) -> Option<(String, String, Option<String>)> {
  let bytes = raw.as_bytes();
  if bytes.first() != Some(&b'[') {
    return None;
  }
  // Find the unescaped closing `]`.
  let mut i = 1usize;
  while i < bytes.len() && bytes[i] != b']' {
    if bytes[i] == b'\\' && i + 1 < bytes.len() {
      i += 2;
      continue;
    }
    i += 1;
  }
  if i >= bytes.len() {
    return None;
  }
  let label = raw[1..i].to_string();
  let after = i + 1;
  if bytes.get(after) != Some(&b':') {
    return None;
  }
  // Skip whitespace after the colon.
  let mut j = after + 1;
  while j < bytes.len() && matches!(bytes[j], b' ' | b'\t' | b'\n') {
    j += 1;
  }
  // Destination: bracketed `<...>` form (allows spaces) or bare run
  // up to the next whitespace.
  let (url, mut k) = if bytes.get(j) == Some(&b'<') {
    let start = j + 1;
    let mut p = start;
    while p < bytes.len() && bytes[p] != b'>' && bytes[p] != b'\n' {
      p += 1;
    }
    if p >= bytes.len() || bytes[p] != b'>' {
      return None;
    }
    (raw[start..p].to_string(), p + 1)
  } else {
    let start = j;
    let mut p = start;
    while p < bytes.len() && !matches!(bytes[p], b' ' | b'\t' | b'\n') {
      p += 1;
    }
    if start == p {
      return None;
    }
    (raw[start..p].to_string(), p)
  };
  // Optional title: rest of line after whitespace, wrapped in
  // matched `"..."`, `'...'`, or `(...)`.
  while k < bytes.len() && matches!(bytes[k], b' ' | b'\t' | b'\n') {
    k += 1;
  }
  let title = if k >= bytes.len() {
    None
  } else {
    let rest = raw[k..].trim_end();
    if rest.is_empty() {
      None
    } else {
      let bs = rest.as_bytes();
      let first = *bs.first()?;
      let last = *bs.last()?;
      let starts_title = matches!(first, b'"' | b'\'' | b'(');
      let matched =
        (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') || (first == b'(' && last == b')');
      if starts_title {
        if matched && rest.len() >= 2 {
          Some(rest[1..rest.len() - 1].to_string())
        } else {
          return None;
        }
      } else {
        return None;
      }
    }
  };
  Some((label, url, title))
}
