//! Replace fenced `CodeBlock`s with a pre-highlighted `<pre><code>...</code></pre>`
//! tree. Each token becomes a `<span>` with an inline `style="color:#xxxxxx"`,
//! each line is wrapped in `<span data-line>`, and lines listed in the block's
//! meta (`{1,3-5}`) get `data-highlighted-line`. Unknown languages fall back to
//! the plain-text grammar so build never errors on niche langs.
//!
//! Multi-theme output (e.g. `{ light, dark }`): the primary mode supplies
//! unprefixed `color` / `background-color`, and every other mode emits
//! `--dmc-{mode}` and `--dmc-{mode}-bg` CSS custom properties. Consumer
//! CSS swaps modes by overriding `color` / `background-color` to the
//! matching `--dmc-*` variable inside whichever class or media-query
//! controls the theme.

use crate::config::{PrettyCodeOptions, PrettyCodeTheme};
use crate::pipeline::Transformer;
use crate::visit::{NodeAction, Visitor, walk_root};
use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::SourceMeta;
use dmc_highlight::{SyntaxBundle, highlight_code_multi};
use dmc_parser::ast::*;
use duck_diagnostic::DiagnosticEngine;

/// Code-block syntax highlighter. Holds the resolved theme list (single
/// or multi-mode) and the mode whose colors fill the unprefixed CSS attrs.
#[derive(Debug, Clone)]
pub struct PrettyCode {
  /// Ordered `(mode, bundled_theme_name)` pairs. For single-theme mode,
  /// `mode` is the empty string and the vec has one entry.
  themes: Vec<(String, String)>,
  /// Mode whose colors are emitted as plain `color` / `background-color`.
  /// Empty string for single-theme.
  default_mode: String,
}

impl Default for PrettyCode {
  fn default() -> Self {
    Self::from_options(&PrettyCodeOptions::default())
  }
}

impl PrettyCode {
  /// Single-theme constructor.
  pub fn new(theme: impl Into<String>) -> Self {
    Self::from_options(&PrettyCodeOptions { theme: PrettyCodeTheme::Single(theme.into()), default_mode: None })
  }

  /// Resolve `PrettyCodeOptions` into a runtime `PrettyCode`. Picks
  /// `default_mode` from explicit config, else `"dark"` if present, else
  /// the first key in the theme map.
  pub fn from_options(opts: &PrettyCodeOptions) -> Self {
    match &opts.theme {
      PrettyCodeTheme::Single(name) => {
        Self { themes: vec![(String::new(), name.clone())], default_mode: String::new() }
      },
      PrettyCodeTheme::Multi(map) => {
        let themes: Vec<(String, String)> = map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        let default_mode = opts
          .default_mode
          .clone()
          .filter(|m| map.contains_key(m))
          .or_else(|| if map.contains_key("dark") { Some("dark".into()) } else { None })
          .or_else(|| themes.first().map(|(k, _)| k.clone()))
          .unwrap_or_default();
        Self { themes, default_mode }
      },
    }
  }
}

impl Transformer for PrettyCode {
  fn name(&self) -> &str {
    "pretty-code"
  }
  fn transform(&self, doc: &mut Document, _meta: &SourceMeta, _engine: &mut DiagnosticEngine<Code>) {
    let mut v = Apply { themes: &self.themes, default_mode: &self.default_mode };
    walk_root(&mut doc.children, &mut v);
  }
}

struct Apply<'a> {
  themes: &'a [(String, String)],
  default_mode: &'a str,
}

impl Visitor for Apply<'_> {
  fn visit_node(&mut self, node: &mut Node) -> NodeAction {
    let Node::CodeBlock(cb) = node else { return NodeAction::Keep };
    // Mermaid blocks are owned by the `mermaid` transformer.
    if cb.lang.as_deref() == Some("mermaid") {
      return NodeAction::Keep;
    }
    let meta = cb.meta.as_deref().map(parse_meta).unwrap_or_default();
    let rendered = render_code_block(cb, &meta, self.themes, self.default_mode);
    NodeAction::Replace(vec![Node::JsxElement(rendered)])
  }
}

#[derive(Default, Debug)]
struct CodeMeta {
  title: Option<String>,
  line_marks: Vec<LineMark>,
}

#[derive(Debug, Clone, Copy)]
struct LineMark {
  start: u32,
  end: u32,
}

impl LineMark {
  fn contains(&self, line: u32) -> bool {
    line >= self.start && line <= self.end
  }
}

/// Parse meta strings like `title="x" {1,3-5}` into a typed `CodeMeta`.
/// Tolerant: any malformed segment is silently skipped.
fn parse_meta(raw: &str) -> CodeMeta {
  let mut m = CodeMeta::default();
  if let Some(start) = raw.find("title=\"") {
    let after = &raw[start + 7..];
    if let Some(end) = after.find('"') {
      m.title = Some(after[..end].to_string());
    }
  }
  if let Some(start) = raw.find('{')
    && let Some(end) = raw[start + 1..].find('}')
  {
    let body = &raw[start + 1..start + 1 + end];
    for tok in body.split(',') {
      let tok = tok.trim();
      if tok.is_empty() {
        continue;
      }
      if let Some((a, b)) = tok.split_once('-') {
        if let (Ok(a), Ok(b)) = (a.trim().parse::<u32>(), b.trim().parse::<u32>()) {
          m.line_marks.push(LineMark { start: a.min(b), end: a.max(b) });
        }
      } else if let Ok(n) = tok.parse::<u32>() {
        m.line_marks.push(LineMark { start: n, end: n });
      }
    }
  }
  m
}

fn render_code_block(cb: &CodeBlock, meta: &CodeMeta, themes: &[(String, String)], default_mode: &str) -> JsxElement {
  // Single-tokenize, multi-color: parse + scope-walk happens once; each
  // theme contributes only its color resolution. Cuts per-file syntect cost
  // roughly in half for the default 2-theme config.
  let theme_names: Vec<&str> = themes.iter().map(|(_, n)| n.as_str()).collect();
  let lines = highlight_code_multi(&cb.value, cb.lang.as_deref(), &theme_names);
  let span = cb.span.clone();

  let primary_idx = themes.iter().position(|(m, _)| m == default_mode).unwrap_or(0);

  let bundle = SyntaxBundle::get();
  let backgrounds: Vec<Option<dmc_highlight::Color>> =
    themes.iter().map(|(_, name)| bundle.themes.themes.get(name).and_then(|t| t.settings.background)).collect();

  let mut line_children: Vec<Node> = Vec::with_capacity(lines.len());
  for (line_i, tokens) in lines.iter().enumerate() {
    let line_no = (line_i + 1) as u32;
    let mut tok_children: Vec<Node> = Vec::with_capacity(tokens.len());
    for tok in tokens.iter() {
      let style = token_style(themes, &tok.styles, primary_idx);
      let text_node = Node::Text(Text { value: tok.text.to_string(), span: span.clone() });
      tok_children.push(Node::JsxElement(JsxElement {
        name: "span".into(),
        attrs: vec![JsxAttr { name: "style".into(), value: JsxAttrValue::String(style), span: span.clone() }],
        children: vec![text_node],
        span: span.clone(),
      }));
    }
    let mut line_attrs = vec![JsxAttr { name: "data-line".into(), value: JsxAttrValue::Boolean, span: span.clone() }];
    if meta.line_marks.iter().any(|m| m.contains(line_no)) {
      line_attrs.push(JsxAttr {
        name: "data-highlighted-line".into(),
        value: JsxAttrValue::Boolean,
        span: span.clone(),
      });
    }
    line_children.push(Node::JsxElement(JsxElement {
      name: "span".into(),
      attrs: line_attrs,
      children: tok_children,
      span: span.clone(),
    }));
  }

  let code_el = Node::JsxElement(JsxElement {
    name: "code".into(),
    attrs: Vec::new(),
    children: line_children,
    span: span.clone(),
  });

  let mut pre_attrs: Vec<JsxAttr> = Vec::new();
  let pre_style = pre_style(themes, &backgrounds, primary_idx);
  if !pre_style.is_empty() {
    pre_attrs.push(JsxAttr { name: "style".into(), value: JsxAttrValue::String(pre_style), span: span.clone() });
  }
  if let Some(lang) = &cb.lang {
    pre_attrs.push(JsxAttr {
      name: "data-language".into(),
      value: JsxAttrValue::String(lang.clone()),
      span: span.clone(),
    });
  }
  pre_attrs.push(JsxAttr {
    name: "data-theme".into(),
    value: JsxAttrValue::String(data_theme_attr(themes)),
    span: span.clone(),
  });

  let pre_node =
    Node::JsxElement(JsxElement { name: "pre".into(), attrs: pre_attrs, children: vec![code_el], span: span.clone() });

  // Always wrap in <figure data-dmc-figure> for parity with rehype-pretty-code.
  // <figcaption> only present when a title is set.
  let mut fig_children: Vec<Node> = Vec::with_capacity(2);
  if let Some(title) = &meta.title {
    fig_children.push(Node::JsxElement(JsxElement {
      name: "figcaption".into(),
      attrs: vec![
        JsxAttr { name: "data-dmc-title".into(), value: JsxAttrValue::Boolean, span: span.clone() },
        JsxAttr {
          name: "data-language".into(),
          value: JsxAttrValue::String(cb.lang.clone().unwrap_or_default()),
          span: span.clone(),
        },
      ],
      children: vec![Node::Text(Text { value: title.clone(), span: span.clone() })],
      span: span.clone(),
    }));
  }
  fig_children.push(pre_node);

  JsxElement {
    name: "figure".into(),
    attrs: vec![JsxAttr { name: "data-dmc-figure".into(), value: JsxAttrValue::Boolean, span: span.clone() }],
    children: fig_children,
    span,
  }
}

fn token_style(themes: &[(String, String)], styles: &[dmc_highlight::HlStyle], primary_idx: usize) -> String {
  let mut parts: Vec<String> = Vec::with_capacity(themes.len());
  for (j, (mode, _)) in themes.iter().enumerate() {
    let Some(style) = styles.get(j) else { continue };
    let fg = style.foreground;
    let prop = if j == primary_idx || mode.is_empty() { "color".to_string() } else { format!("--dmc-{mode}") };
    parts.push(format!("{prop}:#{:02x}{:02x}{:02x}", fg.r, fg.g, fg.b));
  }
  parts.join(";")
}

fn pre_style(themes: &[(String, String)], backgrounds: &[Option<dmc_highlight::Color>], primary_idx: usize) -> String {
  let mut parts: Vec<String> = Vec::with_capacity(themes.len());
  for (j, (mode, _)) in themes.iter().enumerate() {
    let Some(bg) = backgrounds.get(j).and_then(|b| *b) else { continue };
    let prop =
      if j == primary_idx || mode.is_empty() { "background-color".to_string() } else { format!("--dmc-{mode}-bg") };
    parts.push(format!("{prop}:#{:02x}{:02x}{:02x}", bg.r, bg.g, bg.b));
  }
  parts.join(";")
}

fn data_theme_attr(themes: &[(String, String)]) -> String {
  if themes.len() == 1 {
    themes[0].1.clone()
  } else {
    themes.iter().map(|(mode, name)| format!("{mode}:{name}")).collect::<Vec<_>>().join(" ")
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::BTreeMap;

  #[test]
  fn parse_meta_title_and_marks() {
    let m = parse_meta(r#"title="hello" {1,3-5}"#);
    assert_eq!(m.title.as_deref(), Some("hello"));
    assert_eq!(m.line_marks.len(), 2);
    assert!(m.line_marks[0].contains(1));
    assert!(!m.line_marks[0].contains(2));
    assert!(m.line_marks[1].contains(3));
    assert!(m.line_marks[1].contains(5));
    assert!(!m.line_marks[1].contains(6));
  }

  #[test]
  fn parse_meta_empty() {
    let m = parse_meta("");
    assert!(m.title.is_none());
    assert!(m.line_marks.is_empty());
  }

  #[test]
  fn parse_meta_malformed_marks_skipped() {
    let m = parse_meta("{1,abc,3-x}");
    assert_eq!(m.line_marks.len(), 1);
    assert!(m.line_marks[0].contains(1));
  }

  #[test]
  fn options_default_resolves_to_catppuccin_pair_with_dark_primary() {
    let pc = PrettyCode::default();
    assert_eq!(pc.themes.len(), 2);
    assert_eq!(pc.default_mode, "dark");
    let modes: BTreeMap<_, _> = pc.themes.iter().cloned().collect();
    assert_eq!(modes.get("light").map(String::as_str), Some("Catppuccin Latte"));
    assert_eq!(modes.get("dark").map(String::as_str), Some("Catppuccin Mocha"));
  }

  #[test]
  fn from_options_picks_explicit_default_mode_when_present() {
    let map: BTreeMap<String, String> =
      [("dim".to_string(), "Nord".to_string()), ("bright".to_string(), "Catppuccin Latte".to_string())]
        .into_iter()
        .collect();
    let opts = PrettyCodeOptions { theme: PrettyCodeTheme::Multi(map), default_mode: Some("bright".into()) };
    let pc = PrettyCode::from_options(&opts);
    assert_eq!(pc.default_mode, "bright");
  }

  #[test]
  fn from_options_falls_back_to_first_when_no_dark_or_explicit() {
    let map: BTreeMap<String, String> =
      [("alpha".into(), "Nord".into()), ("beta".into(), "TwoDark".into())].into_iter().collect();
    let opts = PrettyCodeOptions { theme: PrettyCodeTheme::Multi(map), default_mode: None };
    let pc = PrettyCode::from_options(&opts);
    // BTreeMap orders alphabetically; first key is "alpha".
    assert_eq!(pc.default_mode, "alpha");
  }

  #[test]
  fn theme_serde_accepts_string_and_object() {
    let s: PrettyCodeTheme = serde_json::from_str(r#""Catppuccin Mocha""#).unwrap();
    assert!(matches!(s, PrettyCodeTheme::Single(ref n) if n == "Catppuccin Mocha"));
    let m: PrettyCodeTheme = serde_json::from_str(r#"{"light":"Catppuccin Latte","dark":"Catppuccin Mocha"}"#).unwrap();
    if let PrettyCodeTheme::Multi(map) = m {
      assert_eq!(map.len(), 2);
    } else {
      panic!("expected Multi");
    }
  }
}
