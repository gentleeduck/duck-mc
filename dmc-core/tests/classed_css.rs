#![cfg(feature = "pretty-code")]

//! End-to-end: a build with `prettyCode.classed = true` and a
//! light+dark theme map writes `dmc.dark.css` + `dmc.light.css` to the
//! output dir, and the emitted collection JSON carries class-based
//! `<span class="dmc-...">` tokens with no inline `style="color:#..."`.

use dmc::Engine;
use dmc::engine::collection::Collection;
use dmc::engine::compile::CompileConfig;
use dmc::engine::config::EngineConfig;
use dmc_diagnostic::Code;
use dmc_transform::{PrettyCodeOptions, PrettyCodeTheme};
use duck_diagnostic::DiagnosticEngine;
use std::collections::BTreeMap;
use std::fs;
use tempfile::TempDir;

#[test]
fn classed_build_writes_per_theme_css_and_classed_json() {
  let tmp = TempDir::new().expect("tempdir");
  let root = tmp.path();
  let docs = root.join("docs");
  fs::create_dir_all(&docs).expect("mkdir docs");
  fs::write(docs.join("a.mdx"), "---\ntitle: \"A\"\n---\n\n```rust\nfn main() {}\n```\n").expect("write fixture");

  let out_dir = root.join("out");

  let mut theme_map: BTreeMap<String, String> = BTreeMap::new();
  theme_map.insert("light".into(), "Catppuccin Latte".into());
  theme_map.insert("dark".into(), "Catppuccin Mocha".into());

  let compile = CompileConfig {
    pretty_code: Some(PrettyCodeOptions {
      classed: Some(true),
      theme: PrettyCodeTheme::Multi(theme_map),
      ..Default::default()
    }),
    ..CompileConfig::default()
  };

  let cfg = EngineConfig {
    root: root.to_path_buf(),
    output_dir: out_dir.clone(),
    output_name: None,
    output_format: None,
    clean: true,
    strict: false,
    collections: vec![Collection {
      name: "docs".into(),
      pattern: "docs/**/*.mdx".into(),
      base_dir: root.to_path_buf(),
      schema: None,
      single: false,
    }],
    // Emit the rendered HTML into the collection record so the test can
    // assert on the class-based markup directly (same shape the real
    // `duckUi.json` carries via velite's `include_html`).
    include_html: true,
    cache_enabled: false,
    compile,
  };

  let mut diag = DiagnosticEngine::<Code>::new();
  Engine::run(&cfg, None, &mut diag).expect("engine run");

  let dark_css = out_dir.join("dmc.dark.css");
  let light_css = out_dir.join("dmc.light.css");
  let dark = fs::read_to_string(&dark_css).expect("dmc.dark.css written");
  let light = fs::read_to_string(&light_css).expect("dmc.light.css written");
  assert!(!dark.is_empty(), "dmc.dark.css is empty");
  assert!(!light.is_empty(), "dmc.light.css is empty");
  assert!(dark.contains("[data-theme=\"dark\"]"), "dmc.dark.css missing data-theme scope:\n{dark}");
  assert!(light.contains("[data-theme=\"light\"]"), "dmc.light.css missing data-theme scope:\n{light}");
  assert!(dark.contains(".dmc-pre"), "dmc.dark.css missing .dmc-pre root:\n{dark}");
  assert!(light.contains(".dmc-pre"), "dmc.light.css missing .dmc-pre root:\n{light}");

  // At least one color-tuple token rule (a `[data-theme=...] .dmc-<hex>`
  // selector that is not the `.dmc-pre` root) carrying a hex color.
  fn has_token_rule(css: &str) -> bool {
    css.lines().any(|l| {
      let l = l.trim();
      l.starts_with("[data-theme=") && l.contains("] .dmc-") && !l.contains(".dmc-pre") && l.contains("color:#")
    })
  }
  assert!(has_token_rule(&dark), "dmc.dark.css missing a .dmc-<hex> token rule with a hex color:\n{dark}");
  assert!(has_token_rule(&light), "dmc.light.css missing a .dmc-<hex> token rule with a hex color:\n{light}");

  let json = fs::read_to_string(out_dir.join("docs.json")).expect("docs.json written");
  // The HTML field is JSON-encoded, so `class="dmc-...` appears as
  // `class=\"dmc-` inside the serialized string.
  assert!(json.contains("class=\\\"dmc-"), "collection JSON missing class-based tokens:\n{json}");
  assert!(!json.contains("style=\\\"color:#"), "collection JSON unexpectedly has inline color styles:\n{json}");
}
