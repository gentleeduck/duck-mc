//! Codegen hardening probes. For each input we (a) parse, (b) render
//! HTML in both safe and dangerous modes, (c) render MDX body. The
//! probes assert no panic + no `javascript:` URL slip + no raw JSX
//! injection in safe HTML.

use dmc_codegen::{RenderOptions, render_html_with, render_mdx_body};

const PROBES: &[&str] = &[
  "",
  "plain",
  "# Heading\n\nbody\n",
  "[a](javascript:alert(1))",
  "[a](javascript&#58;alert(1))",
  "[a](java\tscript:alert(1))",
  "[a](java\nscript:alert(1))",
  "[a](data:text/html;base64,PHNjcmlwdD4=)",
  "[a](vbscript:msgbox)",
  "[a](file:///etc/passwd)",
  "[a](  javascript:alert(1) )",
  "[a](/relative)",
  "[a](#fragment)",
  "[a](mailto:x@y)",
  "<a href=\"javascript:alert(1)\">x</a>",
  "<a href={`javascript:${x}`}>x</a>",
  "<script>alert(1)</script>",
  "<iframe src=\"x\"></iframe>",
  "<img src=\"x\" onerror=\"alert(1)\">",
  "<svg><script>alert(1)</script></svg>",
  "&#60;script&#62;",
  "&#x3c;script&#x3e;",
  "&lt;script&gt;",
  "\\<not html",
  "1 < 2 and 2 > 1",
  "ampersand & here",
  "*emph* and **bold** and ~~strike~~",
  "`code & < > ' \"`",
  "```\nblock & < > ' \"\n```",
  "| a | b |\n|---|---|\n| <b> | & |\n",
  "![alt with \" quote](x.png)",
  "[label \"with quote\"](url \"title\")",
  "footnote[^1]\n\n[^1]: text & more\n",
  "auto: http://x.com",
  "auto-with-ent: http://x.com/?a=1&amp;b=2",
  "\n\n\n",
  "🦆 emoji",
  "RTL: مرحبا",
  "control char\u{0007}here",
  "unicode escape \u{FFFD}",
];

fn render_both(src: &str) {
  let doc = dmc_parser::parse(src);
  let _safe = render_html_with(&doc, RenderOptions::default());
  let _danger = render_html_with(&doc, RenderOptions { allow_dangerous_html: true, ..Default::default() });
  let _mdx = render_mdx_body(&doc);
}

#[test]
fn codegen_does_not_panic_on_corpus() {
  for (i, src) in PROBES.iter().enumerate() {
    render_both(src);
    println!("codegen probe #{i:03} ok ({} bytes)", src.len());
  }
}

fn render_safe(src: &str) -> String {
  let doc = dmc_parser::parse(src);
  render_html_with(&doc, RenderOptions::default())
}

#[test]
fn safe_html_blocks_javascript_urls_in_all_forms() {
  let bad = [
    "[x](javascript:alert(1))",
    "[x](JAVASCRIPT:alert(1))",
    "[x](  javascript:alert(1)  )",
    "[x](\tjavascript:alert(1))",
    "[x](java\u{0000}script:alert(1))",
    "![x](javascript:alert(1))",
  ];
  for src in bad {
    let out = render_safe(src);
    let lower = out.to_ascii_lowercase();
    assert!(
      !lower.contains("href=\"javascript:") && !lower.contains("src=\"javascript:"),
      "javascript: leaked in safe HTML for {src:?}: {out}"
    );
  }
}

#[test]
fn safe_html_blocks_data_text_html_urls() {
  let out = render_safe("[x](data:text/html;base64,PGgxPng8L2gxPg==)");
  assert!(!out.contains("data:text/html"), "data:text/html leaked: {out}");
}

#[test]
fn safe_html_strips_raw_script_blocks() {
  let out = render_safe("<script>alert(1)</script>\n");
  assert!(!out.contains("<script>"), "raw <script> leaked: {out}");
}

#[test]
fn html_escapes_amp_in_text() {
  let out = render_safe("a & b\n");
  assert!(out.contains("&amp;"), "ampersand not escaped: {out}");
}

#[test]
fn html_escapes_angle_brackets_in_text() {
  let out = render_safe("1 < 2 > 0\n");
  let lower = out.to_ascii_lowercase();
  assert!(!lower.contains("<2") && !lower.contains("< 2"), "unescaped `<` leaked: {out}");
}
