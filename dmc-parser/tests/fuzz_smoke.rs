//! Smoke-run each `dmc-fuzz` target on adversarial inputs so CI catches
//! panics / hangs when libFuzzer isn't in the pipeline.

use dmc_parser::{ParseOptions, parse, parse_with};

const ADVERSARIAL: &[&str] = &[
  "",
  "\u{0}\u{0}\u{0}",
  "\n\n\n\n",
  "\t\t\t- x",
  "> > > > > > > > a",
  "********************************************",
  "[[[[[[[[[[a]]]]]]]]]]",
  "![alt](",
  "<a href=\"",
  "<Foo bar={",
  "```\n```\n```",
  "| a | b |\n| - |\n| 1 | 2 | 3 |",
  "<https://",
  "www.x.y)))",
  "a@b@c.d",
  "&#xZZ; &amp &ngE;",
  "\\\\\\\\\\*not emph*",
  "- a\n  - b\n    - c\n      - d\n",
  "~~~~~~~~~~strike",
  "\u{FFFD}\u{FFFD}<x>",
  "<!--\n<!--\n-->",
  "1. a\n1) b\n- c\n* d\n+ e",
  "$$x^2_3$$ {1+2} <X/>",
];

fn replacement_lossy(bytes: &[u8]) -> String {
  String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn fuzz_parse_target_smoke() {
  for s in ADVERSARIAL {
    let _ = parse(s);
  }
  // Lossy byte gate (matches the fuzz target).
  for raw in [&b"\xff\xfe\x00bad"[..], &b"\x80\x80"[..], &[][..]] {
    let _ = parse(&replacement_lossy(raw));
  }
}

#[test]
fn fuzz_parse_strict_target_smoke() {
  let opts = ParseOptions { cm_strict_html_blocks: true, gfm_autolinks: true, legacy_gfm_emphasis: true };
  for s in ADVERSARIAL {
    let _ = parse_with(s, opts);
  }
}

#[test]
fn fuzz_roundtrip_target_smoke() {
  for s in ADVERSARIAL {
    let doc = parse(s);
    let _ = dmc_codegen::render_html(&doc);
  }
}
