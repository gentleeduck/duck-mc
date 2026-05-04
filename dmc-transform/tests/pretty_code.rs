#![cfg(feature = "pretty-code")]

use dmc_parser::ast::*;
use dmc_transform::{Pipeline, PrettyCode, PrettyCodeOptions, PrettyCodeTheme};
use std::collections::BTreeMap;

/// Top-level `<figure data-dmc-figure>` wrapper. Use `inner_pre` to
/// reach the `<pre>` inside it.
fn first_jsx(d: &Document) -> &JsxElement {
  d.children
    .iter()
    .find_map(|n| match n {
      Node::JsxElement(e) => Some(e),
      _ => None,
    })
    .expect("expected a JsxElement at the document root after PrettyCode")
}

/// Find the `<pre>` child of the figure wrapper. Skips the optional
/// `<figcaption>` if a title was set.
fn inner_pre(figure: &JsxElement) -> &JsxElement {
  figure
    .children
    .iter()
    .find_map(|n| match n {
      Node::JsxElement(e) if e.name == "pre" => Some(e),
      _ => None,
    })
    .expect("figure should contain a <pre>")
}

fn attr(el: &JsxElement, name: &str) -> Option<&JsxAttrValue> {
  el.attrs.iter().find(|a| a.name == name).map(|a| &a.value)
}

#[test]
fn replaces_codeblock_with_pre_code_jsx_tree() {
  let mut d = dmc_parser::parse("```rust\nfn main() {}\n```\n");
  Pipeline::new().add(PrettyCode::default()).run_silent(&mut d);

  let figure = first_jsx(&d);
  assert_eq!(figure.name, "figure");
  let pre = inner_pre(figure);
  assert_eq!(pre.name, "pre");
  assert!(matches!(attr(pre, "data-language"), Some(JsxAttrValue::String(s)) if s == "rust"));
  assert!(matches!(attr(pre, "data-theme"), Some(JsxAttrValue::String(_))));

  let code = match pre.children.first().expect("pre has a child") {
    Node::JsxElement(e) => e,
    other => panic!("expected JsxElement <code>, got {other:?}"),
  };
  assert_eq!(code.name, "code");

  let line = match code.children.first().expect("code has a line") {
    Node::JsxElement(e) => e,
    other => panic!("expected JsxElement <span data-line>, got {other:?}"),
  };
  assert_eq!(line.name, "span");
  assert!(matches!(attr(line, "data-line"), Some(JsxAttrValue::Boolean)));

  let token = match line.children.first().expect("line has tokens") {
    Node::JsxElement(e) => e,
    other => panic!("expected token JsxElement, got {other:?}"),
  };
  assert_eq!(token.name, "span");
  let style = attr(token, "style").expect("token has style attr");
  match style {
    JsxAttrValue::String(s) => assert!(s.starts_with("color:#"), "unexpected style {s:?}"),
    other => panic!("expected string style, got {other:?}"),
  }
}

#[test]
fn unknown_language_falls_back_to_plain_text() {
  let mut d = dmc_parser::parse("```not-a-real-lang\nplain text body\n```\n");
  Pipeline::new().add(PrettyCode::default()).run_silent(&mut d);
  let figure = first_jsx(&d);
  assert_eq!(figure.name, "figure");
  assert_eq!(inner_pre(figure).name, "pre");
}

#[test]
fn meta_title_renders_as_figcaption_child() {
  let mut d = dmc_parser::parse("```rust title=\"hello.rs\"\nfn x() {}\n```\n");
  Pipeline::new().add(PrettyCode::default()).run_silent(&mut d);
  let figure = first_jsx(&d);
  let cap = figure
    .children
    .iter()
    .find_map(|n| match n {
      Node::JsxElement(e) if e.name == "figcaption" => Some(e),
      _ => None,
    })
    .expect("figcaption present when title set");
  assert!(matches!(attr(cap, "data-dmc-title"), Some(JsxAttrValue::Boolean)));
  assert!(matches!(cap.children.first(), Some(Node::Text(t)) if t.value == "hello.rs"));
}

#[test]
fn line_marks_set_data_highlighted_line() {
  let mut d = dmc_parser::parse("```rust {2}\nfn a() {}\nfn b() {}\nfn c() {}\n```\n");
  Pipeline::new().add(PrettyCode::default()).run_silent(&mut d);
  let pre = inner_pre(first_jsx(&d));
  let code = match pre.children.first().unwrap() {
    Node::JsxElement(e) => e,
    _ => unreachable!(),
  };
  let lines: Vec<&JsxElement> = code
    .children
    .iter()
    .filter_map(|n| match n {
      Node::JsxElement(e) => Some(e),
      _ => None,
    })
    .collect();
  assert!(lines.len() >= 2);
  assert!(attr(lines[0], "data-highlighted-line").is_none());
  assert!(matches!(attr(lines[1], "data-highlighted-line"), Some(JsxAttrValue::Boolean)));
}

#[test]
fn multi_theme_emits_shiki_css_variables_per_token() {
  let mut map = BTreeMap::new();
  map.insert("light".into(), "Catppuccin Latte".into());
  map.insert("dark".into(), "Catppuccin Mocha".into());
  let pc = PrettyCode::from_options(&PrettyCodeOptions {
    theme: PrettyCodeTheme::Multi(map),
    default_mode: Some("dark".into()),
  });
  let mut d = dmc_parser::parse("```rust\nfn main() {}\n```\n");
  Pipeline::new().add(pc).run_silent(&mut d);

  let pre = inner_pre(first_jsx(&d));
  // pre style carries primary background-color + --dmc-light-bg.
  let pre_style = match attr(pre, "style").expect("pre style") {
    JsxAttrValue::String(s) => s.clone(),
    _ => panic!(),
  };
  assert!(pre_style.contains("background-color:#"), "missing primary bg: {pre_style:?}");
  assert!(pre_style.contains("--dmc-light-bg:#"), "missing CSS var bg: {pre_style:?}");

  // data-theme is space-separated `mode:name` pairs.
  let dt = match attr(pre, "data-theme").unwrap() {
    JsxAttrValue::String(s) => s.clone(),
    _ => panic!(),
  };
  assert!(dt.contains("dark:Catppuccin Mocha"), "got {dt:?}");
  assert!(dt.contains("light:Catppuccin Latte"), "got {dt:?}");

  // Per-token style carries both `color` and `--dmc-light`.
  let code = match pre.children.first().unwrap() {
    Node::JsxElement(e) => e,
    _ => unreachable!(),
  };
  let line = match code.children.first().unwrap() {
    Node::JsxElement(e) => e,
    _ => unreachable!(),
  };
  let token = match line.children.first().unwrap() {
    Node::JsxElement(e) => e,
    _ => unreachable!(),
  };
  let tok_style = match attr(token, "style").unwrap() {
    JsxAttrValue::String(s) => s.clone(),
    _ => panic!(),
  };
  assert!(tok_style.contains("color:#"), "missing primary color: {tok_style:?}");
  assert!(tok_style.contains("--dmc-light:#"), "missing CSS var: {tok_style:?}");
}

#[test]
fn single_theme_omits_css_variables() {
  let pc = PrettyCode::new("Catppuccin Mocha");
  let mut d = dmc_parser::parse("```rust\nfn main() {}\n```\n");
  Pipeline::new().add(pc).run_silent(&mut d);
  let pre = inner_pre(first_jsx(&d));
  let code = match pre.children.first().unwrap() {
    Node::JsxElement(e) => e,
    _ => unreachable!(),
  };
  let line = match code.children.first().unwrap() {
    Node::JsxElement(e) => e,
    _ => unreachable!(),
  };
  let token = match line.children.first().unwrap() {
    Node::JsxElement(e) => e,
    _ => unreachable!(),
  };
  let tok_style = match attr(token, "style").unwrap() {
    JsxAttrValue::String(s) => s.clone(),
    _ => panic!(),
  };
  assert!(tok_style.contains("color:#"), "missing color: {tok_style:?}");
  assert!(!tok_style.contains("--dmc-"), "single-theme leaked CSS var: {tok_style:?}");
}

#[test]
fn theme_serde_round_trip() {
  // Single-theme: bare string.
  let s: PrettyCodeTheme = serde_json::from_str(r#""Nord""#).unwrap();
  assert!(matches!(s, PrettyCodeTheme::Single(ref n) if n == "Nord"));
  // Multi-theme: object.
  let m: PrettyCodeTheme = serde_json::from_str(r#"{"light":"Catppuccin Latte","dark":"Nord"}"#).unwrap();
  if let PrettyCodeTheme::Multi(map) = m {
    assert_eq!(map.len(), 2);
    assert_eq!(map.get("dark").unwrap(), "Nord");
  } else {
    panic!("expected Multi");
  }
}

#[test]
fn options_serde_full_round_trip() {
  let json = r#"{"theme":{"light":"Catppuccin Latte","dark":"Catppuccin Mocha"},"default_mode":"light"}"#;
  let opts: PrettyCodeOptions = serde_json::from_str(json).unwrap();
  assert!(matches!(opts.theme, PrettyCodeTheme::Multi(_)));
  assert_eq!(opts.default_mode.as_deref(), Some("light"));
}

#[test]
fn mermaid_codeblock_is_left_for_other_transformer() {
  let mut d = dmc_parser::parse("```mermaid\ngraph TD; a-->b\n```\n");
  Pipeline::new().add(PrettyCode::default()).run_silent(&mut d);
  // PrettyCode must NOT replace mermaid; the dedicated `Mermaid`
  // transformer owns those blocks.
  let still_a_codeblock =
    d.children.iter().any(|n| matches!(n, Node::CodeBlock(cb) if cb.lang.as_deref() == Some("mermaid")));
  assert!(still_a_codeblock, "PrettyCode unexpectedly consumed a mermaid block");
}
