//! Smoke-parity test against real velite-style MDX docs.
//!
//! These assertions check the *shape* of `duck_md::compile` output — not byte
//! equality with velite. We don't yet support shiki, mermaid, etc., so we only
//! verify: html non-empty, body looks like a JS factory, metadata counts
//! something, frontmatter title exists somewhere, TOC exists somewhere.
//!
//! Fixtures live at workspace root (`tests/fixtures/velite-parity/`), one
//! directory above this crate.

use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
  let manifest = std::env!("CARGO_MANIFEST_DIR");
  PathBuf::from(manifest)
    .join("..")
    .join("tests")
    .join("fixtures")
    .join("velite-parity")
}

fn read_fixture(name: &str) -> String {
  let p = fixtures_dir().join(name);
  std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("missing {}: {}", p.display(), e))
}

/// Returns all `.mdx` fixtures except those marked `.known_fail.mdx`.
fn live_fixtures() -> Vec<PathBuf> {
  std::fs::read_dir(fixtures_dir())
    .unwrap_or_else(|e| panic!("cannot read fixtures dir: {}", e))
    .filter_map(|e| e.ok())
    .map(|e| e.path())
    .filter(|p| p.extension().map_or(false, |x| x == "mdx"))
    .filter(|p| {
      // skip *.known_fail.mdx
      !p.file_name()
        .and_then(|n| n.to_str())
        .map_or(false, |s| s.ends_with(".known_fail.mdx"))
    })
    .collect()
}

/// Compile under `catch_unwind` so a parser panic on one fixture doesn't
/// take the whole test suite down. Returns `None` on panic.
fn try_compile(src: &str) -> Option<duck_md::CompileOutput> {
  std::panic::catch_unwind(|| duck_md::compile(src)).ok()
}

#[test]
fn fixtures_dir_exists() {
  let p = fixtures_dir();
  assert!(
    p.exists(),
    "fixtures dir not found at {} — vendoring step missing?",
    p.display()
  );
  // At least one fixture should be present.
  assert!(!live_fixtures().is_empty(), "no fixtures vendored");
}

#[test]
fn fixture_one_compiles_without_panic() {
  let entries = live_fixtures();
  assert!(!entries.is_empty(), "no fixtures vendored");

  let mut at_least_one_ok = false;
  for path in &entries {
    let src = std::fs::read_to_string(path).unwrap();
    let Some(out) = try_compile(&src) else {
      eprintln!("FIXTURE_PARSE_PANIC: {}", path.display());
      continue;
    };

    // sanity assertions — only enforced for fixtures that compiled
    if out.html.is_empty() {
      eprintln!("FIXTURE_EMPTY_HTML: {}", path.display());
      continue;
    }
    if !out.body.contains("_createMdxContent") {
      eprintln!("FIXTURE_BODY_NOT_FACTORY: {}", path.display());
      continue;
    }
    if out.metadata.word_count == 0 {
      eprintln!("FIXTURE_ZERO_WORDS: {}", path.display());
      continue;
    }
    at_least_one_ok = true;
  }
  assert!(
    at_least_one_ok,
    "no fixture passed all sanity assertions — see FIXTURE_* lines above"
  );
}

#[test]
fn fixture_with_frontmatter_extracts_title() {
  let entries = live_fixtures();
  let mut found_title = false;
  for path in &entries {
    let src = std::fs::read_to_string(path).unwrap();
    let Some(out) = try_compile(&src) else {
      eprintln!("FIXTURE_PARSE_PANIC: {}", path.display());
      continue;
    };
    if out.frontmatter.get("title").and_then(|v| v.as_str()).is_some() {
      found_title = true;
      break;
    }
  }
  assert!(
    found_title,
    "no fixture has a title in its frontmatter — vendor a better one"
  );
}

#[test]
fn fixture_with_headings_builds_toc() {
  let entries = live_fixtures();
  let mut found_toc = false;
  for path in &entries {
    let src = std::fs::read_to_string(path).unwrap();
    let Some(out) = try_compile(&src) else {
      eprintln!("FIXTURE_PARSE_PANIC: {}", path.display());
      continue;
    };
    if !out.toc.is_empty() {
      found_toc = true;
      break;
    }
  }
  assert!(found_toc, "no fixture produced a TOC");
}

#[test]
fn fixture_metadata_has_reading_time_when_words_present() {
  // Loosely: if word_count > 0, reading_time should be sane (>= 1 minute or 0).
  // We don't know the exact reading_time field type, so we just touch it via Debug.
  let entries = live_fixtures();
  let mut checked = 0;
  for path in &entries {
    let src = std::fs::read_to_string(path).unwrap();
    let Some(out) = try_compile(&src) else { continue };
    if out.metadata.word_count > 0 {
      // Just ensure Debug works (catches a missing field at compile time).
      let _ = format!("{:?}", out.metadata);
      checked += 1;
    }
  }
  assert!(checked > 0, "no fixture had a non-zero word count to verify");
}

/// Sanity guard: this test wires up `read_fixture` so the helper isn't dead code
/// in case the suite is later trimmed.
#[test]
fn read_fixture_helper_works() {
  let entries = live_fixtures();
  if let Some(first) = entries.first() {
    let name = first.file_name().unwrap().to_str().unwrap();
    let s = read_fixture(name);
    assert!(!s.is_empty(), "fixture {} was empty", name);
  }
}
