//! Link- and footnote-reference tables. CM 4.7 + 6.3 + GFM footnotes require
//! a two-pass parse: harvest all definitions first, then resolve `[label]` /
//! `[text][label]` / `[label][]` references during the main parse.

use std::collections::HashMap;

/// Destination URL + optional title.
pub type LinkRef = (String, Option<String>);

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

/// CM 4.7: case-fold + whitespace-collapse, leading/trailing trimmed.
/// Backslash escapes are NOT unescaped here, so `[foo\!]` and `[foo!]`
/// match different labels.
pub fn normalize_label(s: &str) -> String {
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

/// CM 4.7/6.3 use Unicode case folding for label matching. `to_lowercase`
/// matches that for most code points, but `ß` (U+00DF) folds to `ss` and
/// capital `ẞ` (U+1E9E) lowercases to `ß`, so `[ẞ]` would fail to match
/// `[SS]:` without an explicit `ß` -> `ss` fold. Full Unicode case folding
/// would need a dedicated table; this only patches the CM-critical case.
fn push_case_folded(out: &mut String, c: char) {
  for low in c.to_lowercase() {
    if low == '\u{00DF}' {
      out.push_str("ss");
    } else {
      out.push(low);
    }
  }
}

/// Parse a `LinkRefDef` lexeme into `(label, url, title)`. The lexer
/// already validated gross structure; failures here mean missing `]` / `:`.
pub fn parse_link_ref_def(raw: &str) -> Option<(String, String, Option<String>)> {
  let bytes = raw.as_bytes();
  if bytes.first() != Some(&b'[') {
    return None;
  }
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
  let mut j = after + 1;
  while j < bytes.len() && matches!(bytes[j], b' ' | b'\t' | b'\n') {
    j += 1;
  }
  // Destination: `<...>` (spaces allowed) or a bare run to whitespace.
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
  // Optional title: `"..."`, `'...'`, or `(...)` after whitespace.
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
