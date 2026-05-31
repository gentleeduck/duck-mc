//! Hardening probes for the engine-level compile pipeline. End-to-end:
//! source → lex → parse → transform → render. The compile() entry point
//! must produce safe HTML by default and never panic.

use dmc::engine::compile::Compiler;
use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;

const SOURCES: &[&str] = &[
  "",
  "plain text\n",
  "# Heading\n\nbody\n",
  "```rust\nfn main() {}\n```\n",
  "- a\n- b\n- c\n",
  "> quote\n",
  "| a | b |\n|---|---|\n| 1 | 2 |\n",
  "[link](https://example.com)\n",
  "[`STATUS.md`](https://example.com/STATUS.md);\n",
  "see https://example.com for info\n",
  "<Comp prop=\"x\" />\n",
  "{1 + 2}\n",
  "{/* comment */}\n",
  "<a href=\"javascript:alert(1)\">x</a>\n",
  "<script>alert(1)</script>\n",
  "footnote[^1]\n\n[^1]: text\n",
  "import X from 'y';\n\n<X/>\n",
  "$$x = y$$\n",
  "$inline$ math\n",
  "🦆 unicode\n",
];

#[test]
fn compile_does_not_panic_on_corpus() {
  for (i, src) in SOURCES.iter().enumerate() {
    let mut diag: DiagnosticEngine<Code> = DiagnosticEngine::new();
    let out = Compiler::compile(src, &mut diag);
    let _ = out;
    println!("compile probe #{i:03} ok ({} bytes)", src.len());
  }
}

#[test]
fn compile_default_blocks_javascript_url() {
  let mut diag: DiagnosticEngine<Code> = DiagnosticEngine::new();
  let out = Compiler::compile("[x](javascript:alert(1))\n", &mut diag);
  let html = out.html.as_str();
  let lower = html.to_ascii_lowercase();
  assert!(!lower.contains("href=\"javascript:"), "javascript: leaked from default compile: {html}");
}

#[test]
fn compile_default_blocks_javascript_url_in_images() {
  let mut diag: DiagnosticEngine<Code> = DiagnosticEngine::new();
  let out = Compiler::compile("![x](javascript:alert(1))\n", &mut diag);
  let html = out.html.as_str();
  let lower = html.to_ascii_lowercase();
  assert!(!lower.contains("src=\"javascript:"), "javascript: leaked in img src from default compile: {html}");
}

#[test]
fn compile_default_strips_raw_script_block() {
  let mut diag: DiagnosticEngine<Code> = DiagnosticEngine::new();
  let out = Compiler::compile("<script>alert(1)</script>\n", &mut diag);
  let html = out.html.as_str();
  assert!(!html.contains("<script>"), "raw <script> leaked from default compile: {html}");
}
