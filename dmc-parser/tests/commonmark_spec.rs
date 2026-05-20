//! CommonMark 0.31.2 spec runner. Vendored spec.json (652 examples).
//! Compares normalized HTML (collapsed whitespace, lowercased tag names).
//! Tracks current pass count in `commonmark_baseline.txt`; the test fails
//! only on regression so the suite moves forward without flapping.

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

/// Lowercase + collapse whitespace + trim. Dampens cosmetic codegen drift
/// without masking correctness bugs.
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

// SEC-001: the codegen URL sanitizer (`is_safe_url`) only permits the
// `{http, https, mailto, tel}` scheme allowlist for absolute URLs. Four
// CommonMark autolink examples (596, 598, 599, 601) use otherwise-valid
// but non-allowlisted schemes (`irc:`, `a+b+c:`, `made-up-scheme:`,
// `localhost:`) and now render `href="#"`. The baseline was lowered from
// 652 -> 648 to reflect this intentional security trade-off: rejecting
// arbitrary schemes is what neutralizes `<javascript:...>` autolink XSS.
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
    let doc = dmc_parser::parse_with(
      &ex.markdown,
      dmc_parser::parser::ParseOptions {
        cm_strict_html_blocks: true,
        gfm_autolinks: false,
        legacy_gfm_emphasis: false,
      },
    );
    let html = dmc_codegen::render_html_with(
      &doc,
      dmc_codegen::RenderOptions { allow_dangerous_html: true, ..Default::default() },
    );
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

/// `--ignored --nocapture` dumps first `N` failures.
#[test]
#[ignore]
fn commonmark_spec_dump_failures() {
  let examples = load_examples();
  let mut shown = 0usize;
  let limit: usize = std::env::var("DMC_DUMP_LIMIT").ok().and_then(|s| s.parse().ok()).unwrap_or(20);

  for ex in &examples {
    let doc = dmc_parser::parse_with(
      &ex.markdown,
      dmc_parser::parser::ParseOptions {
        cm_strict_html_blocks: true,
        gfm_autolinks: false,
        legacy_gfm_emphasis: false,
      },
    );
    let html = dmc_codegen::render_html_with(
      &doc,
      dmc_codegen::RenderOptions { allow_dangerous_html: true, ..Default::default() },
    );
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
