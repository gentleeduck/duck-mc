//! GitHub Flavored Markdown spec test runner. Vendored from
//! https://github.com/github/cmark-gfm/blob/master/test/spec.txt
//! (670 examples), exported to JSON via the same state machine used
//! by `cmark-gfm/test/spec_tests.py`.
//!
//! Strategy mirrors `commonmark_spec.rs`: parse + render, normalize
//! whitespace, diff. Track current pass count in `gfm_baseline.txt`.
//! The test fails only when pass count regresses; bumping the baseline
//! as GFM features land lets the suite move forward without flapping.

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct Example {
  markdown: String,
  html: String,
  example: u32,
  section: String,
  #[serde(default)]
  extensions: Vec<String>,
}

fn fixture_path() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gfm_spec.json")
}

fn baseline_path() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gfm_baseline.txt")
}

fn load_examples() -> Vec<Example> {
  let json = std::fs::read_to_string(fixture_path()).expect("vendored GFM spec.json");
  serde_json::from_str(&json).expect("spec.json is well-formed")
}

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
fn gfm_spec_no_regression() {
  let examples = load_examples();
  let total = examples.len();
  let mut pass = 0usize;
  let mut first_failures: Vec<u32> = Vec::new();

  for ex in &examples {
    let gfm_autolinks = ex.extensions.iter().any(|e| e == "autolink");
    let gfm_tagfilter = ex.extensions.iter().any(|e| e == "tagfilter");
    let doc = dmc_parser::parse_with(
      &ex.markdown,
      dmc_parser::parser::ParseOptions { cm_strict_html_blocks: true, gfm_autolinks },
    );
    let html =
      dmc_codegen::render_html_with(&doc, dmc_codegen::RenderOptions { gfm_disallowed_raw_html: gfm_tagfilter });
    if normalize(&html) == normalize(&ex.html) {
      pass += 1;
    } else if first_failures.len() < 8 {
      first_failures.push(ex.example);
    }
  }

  let baseline = read_baseline();
  println!("GFM spec: {pass}/{total} pass (baseline {baseline}).");
  if pass < baseline {
    panic!("regression: was {baseline}, now {pass}. Sample failing examples: {first_failures:?}");
  }
  if pass > baseline {
    println!(
      "improvement: pass count moved from {baseline} -> {pass}; bump tests/fixtures/gfm_baseline.txt to {pass}."
    );
  }
}

/// Dump the first `N` failures (`--ignored --nocapture`).
#[test]
#[ignore]
fn gfm_spec_dump_failures() {
  let examples = load_examples();
  let mut shown = 0usize;
  let limit: usize = std::env::var("DMC_DUMP_LIMIT").ok().and_then(|s| s.parse().ok()).unwrap_or(20);

  for ex in &examples {
    let gfm_autolinks = ex.extensions.iter().any(|e| e == "autolink");
    let gfm_tagfilter = ex.extensions.iter().any(|e| e == "tagfilter");
    let doc = dmc_parser::parse_with(
      &ex.markdown,
      dmc_parser::parser::ParseOptions { cm_strict_html_blocks: true, gfm_autolinks },
    );
    let html =
      dmc_codegen::render_html_with(&doc, dmc_codegen::RenderOptions { gfm_disallowed_raw_html: gfm_tagfilter });
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
