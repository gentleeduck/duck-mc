#![cfg(feature = "pretty-code")]

//! End-to-end smoke for the native syntax highlighter.

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
  assert!(html.contains("data-dmc-fragment"), "missing fragment wrapper:\n{html}");
  assert!(html.contains("<pre"), "missing <pre> wrapper:\n{html}");
  assert!(html.contains("data-language=\"rust\""), "missing data-language:\n{html}");
  assert!(html.contains("__dmcRaw__="), "missing __dmcRaw__ for Copy support:\n{html}");
  assert!(html.contains("<code "), "missing <code> wrapper:\n{html}");
  assert!(html.contains("class=\"line\""), "missing per-line span:\n{html}");
  assert!(html.contains("style=\"color:#"), "missing solid-color token style:\n{html}");
  assert!(html.contains("fn"), "highlighted text missing source content:\n{html}");
}

#[test]
fn title_meta_renders_as_figcaption() {
  let html = compile("```rust title=\"hello.rs\"\nfn x() {}\n```\n");
  assert!(html.contains("data-dmc-fragment"), "missing fragment:\n{html}");
  assert!(html.contains("data-dmc-title"), "missing title attr:\n{html}");
  assert!(html.contains(">hello.rs<"), "missing title text:\n{html}");
}

#[test]
fn line_marks_emit_data_highlighted_line() {
  let html = compile("```rust {2}\nfn a() {}\nfn b() {}\nfn c() {}\n```\n");
  assert!(html.contains("data-dmc-line-highlighted"), "missing data-dmc-line-highlighted:\n{html}");
}

#[test]
fn default_compileconfig_yields_split_one_pre_per_mode() {
  // Default `Split`: one `<pre data-theme>` per theme, solid colours.
  let html = compile("```rust\nfn main() {}\n```\n");
  assert!(html.contains("data-theme=\"light\""), "split: missing light <pre>:\n{html}");
  assert!(html.contains("data-theme=\"dark\""), "split: missing dark <pre>:\n{html}");
  assert_eq!(html.matches("<pre ").count(), 2, "split: two <pre>:\n{html}");
  assert!(!html.contains("--dmc-"), "split: leaked CSS var:\n{html}");
}

#[test]
fn css_vars_strategy_emits_single_pre_with_dmc_vars() {
  use dmc_transform::MultiThemeStrategy;
  let cfg = CompileConfig {
    pretty_code: Some(PrettyCodeOptions {
      multi_theme_strategy: Some(MultiThemeStrategy::CssVars),
      ..Default::default()
    }),
    ..CompileConfig::default()
  };
  let html = compile_with("```rust\nfn main() {}\n```\n", &cfg);
  assert_eq!(html.matches("<pre ").count(), 1, "css-vars: one <pre>:\n{html}");
  assert!(html.contains("--dmc-light"), "css-vars: missing --dmc-light:\n{html}");
  assert!(html.contains("--dmc-dark"), "css-vars: missing --dmc-dark:\n{html}");
}

#[test]
fn explicit_single_theme_via_compileconfig_emits_one_pre() {
  let cfg = CompileConfig {
    pretty_code: Some(PrettyCodeOptions { theme: PrettyCodeTheme::Single("Nord".into()), ..Default::default() }),
    ..CompileConfig::default()
  };
  let html = compile_with("```rust\nfn main() {}\n```\n", &cfg);
  assert!(html.contains("color:#"), "missing token color:\n{html}");
  assert!(!html.contains("--dmc-"), "single-theme leaked CSS var:\n{html}");
  assert!(html.matches("<pre ").count() == 1, "expected exactly one <pre>:\n{html}");
}

#[test]
fn explicit_multi_theme_via_compileconfig_overrides_default_modes() {
  use dmc_transform::MultiThemeStrategy;
  let mut map = BTreeMap::new();
  map.insert("day".into(), "Catppuccin Latte".into());
  map.insert("night".into(), "Catppuccin Mocha".into());
  let cfg = CompileConfig {
    pretty_code: Some(PrettyCodeOptions {
      theme: PrettyCodeTheme::Multi(map),
      default_mode: Some("day".into()),
      multi_theme_strategy: Some(MultiThemeStrategy::CssVars),
      ..Default::default()
    }),
    ..CompileConfig::default()
  };
  let html = compile_with("```rust\nfn main() {}\n```\n", &cfg);
  assert!(html.contains("--dmc-day"), "missing `day` CSS var:\n{html}");
  assert!(html.contains("--dmc-night"), "missing `night` CSS var:\n{html}");
  assert!(html.matches("<pre ").count() == 1, "css-vars: one <pre>:\n{html}");
}

#[test]
fn split_strategy_emits_one_pre_per_theme() {
  use dmc_transform::MultiThemeStrategy;
  let mut map = BTreeMap::new();
  map.insert("light".into(), "Catppuccin Latte".into());
  map.insert("dark".into(), "Catppuccin Mocha".into());
  let cfg = CompileConfig {
    pretty_code: Some(PrettyCodeOptions {
      theme: PrettyCodeTheme::Multi(map),
      multi_theme_strategy: Some(MultiThemeStrategy::Split),
      ..Default::default()
    }),
    ..CompileConfig::default()
  };
  let html = compile_with("```rust\nfn main() {}\n```\n", &cfg);
  assert!(html.contains("data-theme=\"light\""), "split: missing light pre:\n{html}");
  assert!(html.contains("data-theme=\"dark\""), "split: missing dark pre:\n{html}");
  assert_eq!(html.matches("<pre ").count(), 2, "split: two <pre>:\n{html}");
}
