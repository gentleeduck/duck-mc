//! Replace `:shortcode:` patterns in text with the matching Unicode emoji.
//! Mirrors `remark-emoji` in the JS chain. Unknown shortcodes are left
//! untouched so doc text containing colons (`:foo:bar:`) survives intact.
//!
//! Only `Text` nodes are visited - code blocks, inline code, JSX, and
//! attribute values are left alone, matching the JS plugin's scope.

use crate::pipeline::Transformer;
use crate::visit::{NodeAction, Visitor, walk_root};
use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::SourceMeta;
use dmc_parser::ast::*;
use duck_diagnostic::DiagnosticEngine;

#[derive(Default, Debug)]
pub struct Emoji;

impl Transformer for Emoji {
  fn name(&self) -> &str {
    "emoji"
  }
  fn transform(&self, doc: &mut Document, _meta: &SourceMeta, _engine: &mut DiagnosticEngine<Code>) {
    let mut v = Apply;
    walk_root(&mut doc.children, &mut v);
  }
}

struct Apply;

impl Visitor for Apply {
  fn visit_node(&mut self, node: &mut Node) -> NodeAction {
    let Node::Text(t) = node else { return NodeAction::Keep };
    let Some(replaced) = expand_emoji(&t.value) else { return NodeAction::Keep };
    t.value = replaced;
    NodeAction::Keep
  }
}

/// Walk `text` and replace every recognised `:shortcode:` with its emoji.
/// Returns `None` when no shortcode matched, so callers can skip the
/// allocation.
fn expand_emoji(text: &str) -> Option<String> {
  if !text.contains(':') {
    return None;
  }
  let bytes = text.as_bytes();
  let mut out = String::with_capacity(text.len());
  let mut i = 0;
  let mut found_any = false;
  while i < bytes.len() {
    if bytes[i] != b':' {
      let ch_len = utf8_char_len(bytes[i]);
      out.push_str(&text[i..i + ch_len]);
      i += ch_len;
      continue;
    }
    // Look ahead for the closing colon. Shortcodes are short ASCII tokens
    // (`[a-z0-9_+-]+`); cap the search so a colon-pair miles apart never
    // becomes a fake match.
    let max_end = (i + 1 + 64).min(bytes.len());
    let close = (i + 1..max_end).find(|&j| bytes[j] == b':');
    let Some(close) = close else {
      out.push(':');
      i += 1;
      continue;
    };
    let shortcode = &text[i + 1..close];
    if !is_shortcode(shortcode) {
      out.push(':');
      i += 1;
      continue;
    }
    if let Some(emo) = emojis::get_by_shortcode(shortcode) {
      out.push_str(emo.as_str());
      i = close + 1;
      found_any = true;
    } else {
      out.push(':');
      i += 1;
    }
  }
  if found_any { Some(out) } else { None }
}

fn is_shortcode(s: &str) -> bool {
  !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'+' | b'-'))
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn passthrough_when_no_colon() {
    assert!(expand_emoji("hello world").is_none());
  }

  #[test]
  fn known_shortcode_expands() {
    let out = expand_emoji("hi :smile: there").unwrap();
    assert!(out.contains("hi "));
    assert!(out.contains(" there"));
    assert!(!out.contains(":smile:"), "shortcode survived: {out}");
  }

  #[test]
  fn unknown_shortcode_is_kept() {
    assert!(expand_emoji("see :nonexistent_emoji_token: here").is_none());
  }

  #[test]
  fn ratio_text_passes_through() {
    // `:1:2` is not a valid shortcode; must not be munged.
    assert!(expand_emoji("ratio 1:2").is_none());
  }

  #[test]
  fn multiple_shortcodes() {
    let out = expand_emoji(":heart: and :star:").unwrap();
    assert!(!out.contains(':'), "leftover colon: {out}");
  }
}
