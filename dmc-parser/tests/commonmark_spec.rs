//! CommonMark 0.31.2 spec test runner. Vendored from
//! https://spec.commonmark.org/0.31.2/spec.json (652 examples).
//!
//! The runner parses each example through `dmc_parser::parse` +
//! `dmc_codegen::render_html`, normalizes both sides of the HTML
//! comparison (collapsing inter-tag whitespace + lower-casing tag
//! names so cosmetic codegen drift doesn't fail the suite), and
//! diffs.
//!
//! Strategy: track the current pass count in `commonmark_baseline.txt`.
//! The test fails only if the pass count regresses; bumping the
//! baseline as fixes land lets the suite move forward without
//! flapping while parser + codegen catch up to 100% spec compliance.

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct Example {
  markdown: String,
  html: String,
  example: u32,
  section: String,
}

fn fixture_path() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/commonmark_spec.json")
}

fn baseline_path() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/commonmark_baseline.txt")
}

fn load_examples() -> Vec<Example> {
  let json = std::fs::read_to_string(fixture_path()).expect("vendored CommonMark spec.json");
  serde_json::from_str(&json).expect("spec.json is well-formed")
}

/// Cheap normalization: lowercase tag names, collapse runs of
/// whitespace, strip leading / trailing whitespace. Good enough to
/// dampen renderer cosmetic noise without masking real correctness
/// bugs.
fn normalize(html: &str) -> String {
  let lowered = html.to_ascii_lowercase();
  let mut out = String::with_capacity(lowered.len());
  let mut prev_ws = true;
  for c in lowered.chars() {
    if c.is_whitespace() {
      if !prev_ws {
        out.push(' ');
        prev_ws = true;
      }
    } else {
      out.push(c);
      prev_ws = false;
    }
  }
  if out.ends_with(' ') {
    out.pop();
  }
  out
}

fn read_baseline() -> usize {
  std::fs::read_to_string(baseline_path()).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0)
}

#[test]
fn commonmark_spec_no_regression() {
  let examples = load_examples();
  let total = examples.len();
  let mut pass = 0usize;
  let mut first_failures: Vec<u32> = Vec::new();

  for ex in &examples {
    let doc = dmc_parser::parse_with(&ex.markdown, dmc_parser::parser::ParseOptions { cm_strict_html_blocks: true, gfm_autolinks: false });
    let html = dmc_codegen::render_html(&doc);
    if normalize(&html) == normalize(&ex.html) {
      pass += 1;
    } else if first_failures.len() < 8 {
      first_failures.push(ex.example);
    }
  }

  let baseline = read_baseline();
  println!("CommonMark spec: {pass}/{total} pass (baseline {baseline}).");
  if pass < baseline {
    panic!("regression: was {baseline}, now {pass}. Sample failing examples: {first_failures:?}");
  }
  if pass > baseline {
    println!(
      "improvement: pass count moved from {baseline} -> {pass}; bump tests/fixtures/commonmark_baseline.txt to {pass}."
    );
  }
}

/// Dump the first `N` failures with section, source, expected vs.
/// actual HTML. Run with `--ignored --nocapture` to see categories.
#[test]
#[ignore]
fn commonmark_spec_dump_failures() {
  let examples = load_examples();
  let mut shown = 0usize;
  let limit: usize = std::env::var("DMC_DUMP_LIMIT").ok().and_then(|s| s.parse().ok()).unwrap_or(20);

  for ex in &examples {
    let doc = dmc_parser::parse_with(&ex.markdown, dmc_parser::parser::ParseOptions { cm_strict_html_blocks: true, gfm_autolinks: false });
    let html = dmc_codegen::render_html(&doc);
    if normalize(&html) != normalize(&ex.html) {
      shown += 1;
      println!("=== example {} ({}) ===", ex.example, ex.section);
      println!("--- markdown ---\n{}", ex.markdown);
      println!("--- expected ---\n{}", ex.html);
      println!("--- actual ---\n{}", html);
      println!();
      if shown >= limit {
        break;
      }
    }
  }
}
