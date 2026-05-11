#![cfg(feature = "pretty-code")]

use dmc_parser::ast::*;
use dmc_transform::{Pipeline, PrettyCode, PrettyCodeOptions, PrettyCodeTheme};
use std::collections::BTreeMap;

/// The fragment wrapper PrettyCode places at the document root.
fn first_jsx(d: &Document) -> &JsxElement {
  d.children
    .iter()
    .find_map(|n| match n {
      Node::JsxElement(e) => Some(e),
      _ => None,
    })
    .expect("expected a JsxElement at the document root after PrettyCode")
}

/// Per-theme `<pre>` children of the fragment. The consumer's `<pre>`
/// override wraps each one in `<div data-theme>` itself, so the
/// transformer emits the `<pre>` siblings directly (no extra wrapper).
fn theme_divs(fragment: &JsxElement) -> Vec<&JsxElement> {
  fragment
    .children
    .iter()
    .filter_map(|n| match n {
      Node::JsxElement(e) if e.name == "pre" => Some(e),
      _ => None,
    })
    .collect()
}

/// Each per-theme element is itself a `<pre>`, so `inner_pre` is a passthrough.
fn inner_pre(theme_pre: &JsxElement) -> &JsxElement {
  theme_pre
}

fn attr<'a>(el: &'a JsxElement, name: &str) -> Option<&'a JsxAttrValue> {
  el.attrs.iter().find(|a| a.name == name).map(|a| &a.value)
}

#[test]
fn default_strategy_emits_one_pre_per_mode() {
  // Default strategy is `Split`: one `<pre data-theme="…">` per
  // configured theme. No CSS custom properties on tokens — solid
  // colours per pre. Consumer flips themes with a single
  // `[data-theme]` selector.
  let mut d = dmc_parser::parse("```rust\nfn main() {}\n```\n");
  Pipeline::new().add(PrettyCode::default()).run_silent(&mut d);

  let fragment = first_jsx(&d);
  assert_eq!(fragment.name, "div");
  assert!(matches!(attr(fragment, "data-dmc-fragment"), Some(JsxAttrValue::String(s)) if s.is_empty()));

  let pres = theme_divs(fragment);
  assert_eq!(pres.len(), 2, "split: two <pre>");
  let modes: Vec<String> = pres
    .iter()
    .filter_map(|p| match attr(p, "data-theme") {
      Some(JsxAttrValue::String(s)) => Some(s.clone()),
      _ => None,
    })
    .collect();
  assert!(modes.iter().any(|m| m == "light"), "split: missing light: {modes:?}");
  assert!(modes.iter().any(|m| m == "dark"), "split: missing dark: {modes:?}");
}

#[test]
fn css_vars_strategy_emits_single_pre_with_dmc_vars() {
  use dmc_transform::MultiThemeStrategy;
  let mut d = dmc_parser::parse("```rust\nfn main() {}\n```\n");
  let pc = PrettyCode::from_options(&PrettyCodeOptions {
    multi_theme_strategy: Some(MultiThemeStrategy::CssVars),
    ..Default::default()
  });
  Pipeline::new().add(pc).run_silent(&mut d);

  let fragment = first_jsx(&d);
  assert_eq!(fragment.name, "div");
  assert!(matches!(attr(fragment, "data-dmc-fragment"), Some(JsxAttrValue::String(s)) if s.is_empty()));

  let pres = theme_divs(fragment);
  assert_eq!(pres.len(), 1, "css-vars: one <pre>");

  let pre = inner_pre(pres[0]);
  assert_eq!(pre.name, "pre");
  assert!(matches!(attr(pre, "data-language"), Some(JsxAttrValue::String(s)) if s == "rust"));
  assert!(
    matches!(attr(pre, "__dmcRaw__"), Some(JsxAttrValue::String(s)) if s.contains("fn main")),
    "missing __dmcRaw__ for Copy support",
  );
  let pre_style = match attr(pre, "style").expect("<pre> has style") {
    JsxAttrValue::String(s) => s.clone(),
    _ => panic!(),
  };
  assert!(pre_style.contains("--dmc-light"), "missing --dmc-light: {pre_style:?}");
  assert!(pre_style.contains("--dmc-dark"), "missing --dmc-dark: {pre_style:?}");

  let code = match pre.children.first().expect("pre has a child") {
    Node::JsxElement(e) => e,
    other => panic!("expected JsxElement <code>, got {other:?}"),
  };
  assert_eq!(code.name, "code");

  let line = match code.children.first().expect("code has a line") {
    Node::JsxElement(e) => e,
    other => panic!("expected JsxElement <span class=\"line\">, got {other:?}"),
  };
  assert_eq!(line.name, "span");
  assert!(matches!(attr(line, "class"), Some(JsxAttrValue::String(s)) if s == "line"));

  let token = match line.children.first().expect("line has tokens") {
    Node::JsxElement(e) => e,
    other => panic!("expected token JsxElement, got {other:?}"),
  };
  assert_eq!(token.name, "span");
  let styled = code
    .children
    .iter()
    .filter_map(|n| match n {
      Node::JsxElement(e) if e.name == "span" => Some(e),
      _ => None,
    })
    .flat_map(|line| line.children.iter())
    .find_map(|n| match n {
      Node::JsxElement(e) if attr(e, "style").is_some() => Some(e),
      _ => None,
    })
    .expect("at least one styled token");
  let style = match attr(styled, "style").unwrap() {
    JsxAttrValue::String(s) => s.clone(),
    _ => panic!(),
  };
  assert!(style.contains("--dmc-"), "css-vars token missing --dmc-: {style:?}");
}

#[test]
fn unknown_language_falls_back_to_plain_text() {
  let mut d = dmc_parser::parse("```not-a-real-lang\nplain text body\n```\n");
  Pipeline::new().add(PrettyCode::default()).run_silent(&mut d);
  let fragment = first_jsx(&d);
  assert_eq!(fragment.name, "div");
  assert!(!theme_divs(fragment).is_empty());
}

#[test]
fn meta_title_renders_as_figcaption_child() {
  let mut d = dmc_parser::parse("```rust title=\"hello.rs\"\nfn x() {}\n```\n");
  Pipeline::new().add(PrettyCode::default()).run_silent(&mut d);
  let fragment = first_jsx(&d);
  let cap = fragment
    .children
    .iter()
    .find_map(|n| match n {
      Node::JsxElement(e) if e.name == "figcaption" => Some(e),
      _ => None,
    })
    .expect("figcaption present when title set");
  assert!(matches!(attr(cap, "data-dmc-title"), Some(JsxAttrValue::String(s)) if s.is_empty()));
  assert!(matches!(cap.children.first(), Some(Node::Text(t)) if t.value == "hello.rs"));
}

#[test]
fn line_marks_set_data_highlighted_line() {
  let mut d = dmc_parser::parse("```rust {2}\nfn a() {}\nfn b() {}\nfn c() {}\n```\n");
  Pipeline::new().add(PrettyCode::default()).run_silent(&mut d);
  let pre = inner_pre(theme_divs(first_jsx(&d))[0]);
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
  assert!(attr(lines[0], "data-dmc-line-highlighted").is_none());
  assert!(matches!(attr(lines[1], "data-dmc-line-highlighted"), Some(JsxAttrValue::String(s)) if s.is_empty()));
}

#[test]
fn split_strategy_emits_one_pre_per_mode() {
  use dmc_transform::MultiThemeStrategy;
  let mut map = BTreeMap::new();
  map.insert("light".into(), "Catppuccin Latte".into());
  map.insert("dark".into(), "Catppuccin Mocha".into());
  let pc = PrettyCode::from_options(&PrettyCodeOptions {
    theme: PrettyCodeTheme::Multi(map),
    default_mode: Some("dark".into()),
    multi_theme_strategy: Some(MultiThemeStrategy::Split),
    ..Default::default()
  });
  let mut d = dmc_parser::parse("```rust\nfn main() {}\n```\n");
  Pipeline::new().add(pc).run_silent(&mut d);

  let blocks = theme_divs(first_jsx(&d));
  assert_eq!(blocks.len(), 2);
  let modes: Vec<String> = blocks
    .iter()
    .filter_map(|d| match attr(d, "data-theme") {
      Some(JsxAttrValue::String(s)) => Some(s.clone()),
      _ => None,
    })
    .collect();
  assert!(modes.contains(&"light".to_string()), "got modes {modes:?}");
  assert!(modes.contains(&"dark".to_string()), "got modes {modes:?}");
}

#[test]
fn single_theme_renders_single_pre() {
  let pc = PrettyCode::new("Catppuccin Mocha");
  let mut d = dmc_parser::parse("```rust\nfn main() {}\n```\n");
  Pipeline::new().add(pc).run_silent(&mut d);

  let blocks = theme_divs(first_jsx(&d));
  assert_eq!(blocks.len(), 1, "single-theme: one <pre>");

  let pre = inner_pre(blocks[0]);
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
  assert!(!tok_style.contains("--shiki-"), "single-theme leaked CSS var: {tok_style:?}");
}

#[test]
fn theme_serde_round_trip() {
  let s: PrettyCodeTheme = serde_json::from_str(r#""Nord""#).unwrap();
  assert!(matches!(s, PrettyCodeTheme::Single(ref n) if n == "Nord"));
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
  let json = r#"{"theme":{"light":"Catppuccin Latte","dark":"Catppuccin Mocha"},"defaultMode":"light"}"#;
  let opts: PrettyCodeOptions = serde_json::from_str(json).unwrap();
  assert!(matches!(opts.theme, PrettyCodeTheme::Multi(_)));
  assert_eq!(opts.default_mode.as_deref(), Some("light"));
}

/// Recursively collect every `<span>` element in the subtree.
fn collect_spans<'a>(el: &'a JsxElement, out: &mut Vec<&'a JsxElement>) {
  if el.name == "span" {
    out.push(el);
  }
  for c in &el.children {
    if let Node::JsxElement(child) = c {
      collect_spans(child, out);
    }
  }
}

/// Recursively count elements by tag name.
fn count_tag(el: &JsxElement, name: &str) -> usize {
  let mut n = if el.name == name { 1 } else { 0 };
  for c in &el.children {
    if let Node::JsxElement(child) = c {
      n += count_tag(child, name);
    }
  }
  n
}

#[test]
fn classed_emits_span_class_not_inline_style() {
  let mut d = dmc_parser::parse("```rust\nfn main() {}\n```\n");
  let pc = PrettyCode::from_options(&PrettyCodeOptions { classed: Some(true), ..Default::default() });
  Pipeline::new().add(pc).run_silent(&mut d);

  let fragment = first_jsx(&d);
  // Exactly one <pre> -- no per-theme duplication.
  assert_eq!(count_tag(fragment, "pre"), 1, "classed: expected exactly one <pre>");

  let pre = theme_divs(fragment).into_iter().next().expect("a <pre>");
  assert!(
    matches!(attr(pre, "class"), Some(JsxAttrValue::String(s)) if s == "dmc-pre"),
    "classed: <pre> should carry class=\"dmc-pre\""
  );

  let mut spans = Vec::new();
  collect_spans(fragment, &mut spans);
  // No span carries an inline `style` attr.
  for s in &spans {
    assert!(attr(s, "style").is_none(), "classed: token <span> must not have an inline style attr");
  }
  // At least one deepest token span has a `class` of the color-tuple
  // shape `dmc-<6 hex digits>...` (not the line class).
  fn is_color_tuple_class(c: &str) -> bool {
    let Some(rest) = c.strip_prefix("dmc-") else { return false };
    rest.len() >= 6 && rest.as_bytes()[..6].iter().all(|b| b.is_ascii_hexdigit())
  }
  let has_token_class =
    spans.iter().any(|s| matches!(attr(s, "class"), Some(JsxAttrValue::String(c)) if is_color_tuple_class(c)));
  assert!(has_token_class, "classed: expected at least one token <span> with a dmc-<hex> class");
}

#[test]
fn mermaid_codeblock_is_left_for_other_transformer() {
  let mut d = dmc_parser::parse("```mermaid\ngraph TD; a-->b\n```\n");
  Pipeline::new().add(PrettyCode::default()).run_silent(&mut d);
  let still_a_codeblock =
    d.children.iter().any(|n| matches!(n, Node::CodeBlock(cb) if cb.lang.as_deref() == Some("mermaid")));
  assert!(still_a_codeblock, "PrettyCode unexpectedly consumed a mermaid block");
}
