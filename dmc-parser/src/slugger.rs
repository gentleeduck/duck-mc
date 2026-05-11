//! GitHub-style heading-anchor slug generator.
//!
//! Mirrors `github-slugger` (the npm package used by `rehype-slug`):
//! lowercase, drop ASCII punctuation/symbols, drop control chars, replace
//! whitespace runs with `-`, collapse repeated `-`, trim. Punctuation is
//! stripped (NOT replaced), so `0.4.3` -> `043`, `It's` -> `its` -
//! matching velite output.
//!
//! For dedupe (`#patch-changes`, `#patch-changes-1`, `#patch-changes-2`)
//! use [`Slugger`], which threads a count map across one document's
//! headings.

use std::collections::HashMap;

/// Compute the GitHub-style slug for a single heading, ignoring dedupe.
/// For document-wide dedupe, use [`Slugger::slug`].
pub fn github_slugify(input: &str) -> String {
  let lower = input.trim().to_lowercase();
  let mut out = String::with_capacity(lower.len());
  let mut prev_dash = false;
  for ch in lower.chars() {
    if ch.is_control() {
      continue;
    }
    if ch.is_whitespace() {
      if !prev_dash && !out.is_empty() {
        out.push('-');
        prev_dash = true;
      }
      continue;
    }
    // Keep letters, digits, '_', '-'. Drop everything else (`.`, `'`,
    // `:`, etc) - github-slugger's "strip, don't replace" semantics.
    if ch.is_alphanumeric() || ch == '_' || ch == '-' {
      // Treat existing '-' the same as ws so we collapse runs.
      if ch == '-' {
        if prev_dash {
          continue;
        }
        out.push('-');
        prev_dash = true;
      } else {
        out.push(ch);
        prev_dash = false;
      }
    }
  }
  // Trim trailing '-' (leading '-' was avoided by the empty-out guard).
  while out.ends_with('-') {
    out.pop();
  }
  out
}

/// Document-scoped slugger. Tracks how many times each base slug has
/// already been emitted; collisions get a `-1`, `-2`, ... suffix.
#[derive(Debug, Default)]
pub struct Slugger {
  seen: HashMap<String, u32>,
}

impl Slugger {
  pub fn new() -> Self {
    Self { seen: HashMap::new() }
  }

  /// Compute the slug for `text`, suffixing with `-N` when the base slug
  /// has been emitted `N` times before. Empty input -> empty string;
  /// dedupe still applies, so two empty headings get `""` and `"-1"`.
  pub fn slug(&mut self, text: &str) -> String {
    let base = github_slugify(text);
    let count = self.seen.entry(base.clone()).or_insert(0);
    let out = if *count == 0 { base.clone() } else { format!("{}-{}", base, *count) };
    *count += 1;
    out
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn strips_dots() {
    assert_eq!(github_slugify("0.4.3"), "043");
  }

  #[test]
  fn strips_apostrophes() {
    assert_eq!(github_slugify("How It's Built"), "how-its-built");
  }

  #[test]
  fn replaces_spaces_with_dash() {
    assert_eq!(github_slugify("Patch Changes"), "patch-changes");
  }

  #[test]
  fn collapses_runs() {
    assert_eq!(github_slugify("Hello -- World"), "hello-world");
    assert_eq!(github_slugify("foo   bar"), "foo-bar");
  }

  #[test]
  fn dedupes() {
    let mut s = Slugger::new();
    assert_eq!(s.slug("Patch Changes"), "patch-changes");
    assert_eq!(s.slug("Patch Changes"), "patch-changes-1");
    assert_eq!(s.slug("Patch Changes"), "patch-changes-2");
  }

  #[test]
  fn keeps_underscores_and_existing_dashes() {
    assert_eq!(github_slugify("foo_bar-baz"), "foo_bar-baz");
  }
}
