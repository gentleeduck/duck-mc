#![cfg(feature = "pretty-code")]

//! End-to-end smoke for the native syntax highlighter: a fenced code block
//! goes in, syntect-highlighted `<pre><code>` HTML comes out. Asserts on
//! shape rather than exact bytes (token output depends on the bundled
//! grammar version, so a byte-for-byte snapshot would churn).

use dmc::engine::compile::{CompileConfig, Compiler};
use dmc_transform::{PrettyCodeOptions, PrettyCodeTheme};
use duck_diagnostic::DiagnosticEngine;
use std::collections::BTreeMap;
use std::path::Path;

fn compile(src: &str) -> String {
  compile_with(src, &CompileConfig::default())
}

fn compile_with(src: &str, cfg: &CompileConfig) -> String {
  let mut diag = DiagnosticEngine::new();
  Compiler::compile_with_pipeline(src, Path::new("<test>"), cfg, &mut diag).html
}

#[test]
fn rust_codeblock_renders_through_pretty_code() {
  let html = compile("```rust\nfn main() {}\n```\n");
  assert!(html.contains("<pre"), "missing <pre> wrapper:\n{html}");
  assert!(html.contains("data-language=\"rust\""), "missing data-language:\n{html}");
  assert!(html.contains("data-theme="), "missing data-theme:\n{html}");
  assert!(html.contains("<code>"), "missing <code> wrapper:\n{html}");
  assert!(html.contains("data-line"), "missing per-line span:\n{html}");
  assert!(html.contains("style=\"color:#"), "missing inline color span:\n{html}");
  assert!(html.contains("fn"), "highlighted text missing source content:\n{html}");
}

#[test]
fn title_meta_renders_as_figcaption() {
  // Match rehype-pretty-code: wrap in <figure data-rehype-pretty-code-figure>
  // and emit the title as a real <figcaption data-rehype-pretty-code-title>
  // node so consumers do not need custom `::before` CSS.
  let html = compile("```rust title=\"hello.rs\"\nfn x() {}\n```\n");
  assert!(html.contains("data-dmc-figure"), "missing figure:\n{html}");
  assert!(html.contains("data-dmc-title"), "missing figcaption:\n{html}");
  assert!(html.contains(">hello.rs<"), "missing title text:\n{html}");
}

#[test]
fn line_marks_emit_data_highlighted_line() {
  let html = compile("```rust {2}\nfn a() {}\nfn b() {}\nfn c() {}\n```\n");
  assert!(html.contains("data-highlighted-line"), "missing data-highlighted-line:\n{html}");
}

#[test]
fn default_compileconfig_yields_multi_theme_css_variables() {
  // No `pretty_code` override -> bundled defaults (Catppuccin pair, dark
  // primary). End-to-end, the rendered HTML should carry shiki-style CSS
  // variables on every token + on the wrapping <pre>.
  let html = compile("```rust\nfn main() {}\n```\n");
  assert!(html.contains("--dmc-light:#"), "missing per-token CSS var:\n{html}");
  assert!(html.contains("--dmc-light-bg:#"), "missing pre-level bg CSS var:\n{html}");
}

#[test]
fn explicit_single_theme_via_compileconfig_drops_css_variables() {
  let cfg = CompileConfig {
    pretty_code: Some(PrettyCodeOptions { theme: PrettyCodeTheme::Single("Nord".into()), default_mode: None }),
    ..CompileConfig::default()
  };
  let html = compile_with("```rust\nfn main() {}\n```\n", &cfg);
  assert!(html.contains("color:#"), "missing color:\n{html}");
  assert!(!html.contains("--dmc-"), "single-theme leaked CSS var:\n{html}");
  assert!(html.contains("data-theme=\"Nord\""), "expected `data-theme=\"Nord\"`:\n{html}");
}

#[test]
fn explicit_multi_theme_via_compileconfig_overrides_default_modes() {
  let mut map = BTreeMap::new();
  map.insert("day".into(), "Catppuccin Latte".into());
  map.insert("night".into(), "Catppuccin Mocha".into());
  let cfg = CompileConfig {
    pretty_code: Some(PrettyCodeOptions { theme: PrettyCodeTheme::Multi(map), default_mode: Some("day".into()) }),
    ..CompileConfig::default()
  };
  let html = compile_with("```rust\nfn main() {}\n```\n", &cfg);
  // Custom modes show up as --dmc-{mode} CSS vars; the chosen primary
  // ("day") fills `color`/`background-color`, the other ("night") fills
  // the `--dmc-night` variable.
  assert!(html.contains("--dmc-night:#"), "missing custom-mode CSS var:\n{html}");
  assert!(!html.contains("--dmc-day:#"), "primary mode should be unprefixed:\n{html}");
  assert!(html.contains("day:Catppuccin Latte"), "expected mode pair in data-theme:\n{html}");
}
