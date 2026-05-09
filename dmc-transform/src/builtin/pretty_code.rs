//! Pre-rendered syntax highlighter. See `transformers/pretty-code.md`
//! for full docs.

use crate::config::{MultiThemeStrategy, PrettyCodeOptions, PrettyCodeTheme};
use crate::pipeline::Transformer;
use crate::visit::{NodeAction, Visitor, walk_root};
use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::SourceMeta;
use dmc_highlight::{SyntaxBundle, highlight_code_multi, list_bundled_themes};
use dmc_parser::ast::*;
use duck_diagnostic::{DiagnosticEngine, diag};
/// Code-block syntax highlighter. Holds the resolved theme list (single
/// or multi-mode), the mode whose colors fill the unprefixed CSS attrs,
/// and every DOM-shape knob from [`PrettyCodeOptions`].
#[derive(Debug, Clone)]
pub struct PrettyCode {
  /// Ordered `(mode, bundled_theme_name)` pairs. For single-theme mode,
  /// `mode` is the empty string and the vec has one entry.
  themes: Vec<(String, String)>,
  /// Mode whose colors are emitted as plain `color` / `background-color`.
  /// Empty string for single-theme.
  default_mode: String,
  /// Resolved DOM-shape options (defaults applied).
  shape: ShapeOpts,
}

/// Process-wide dedupe set for "theme not bundled" warnings. The
/// pipeline rebuilds a fresh `PrettyCode` per document, so a per-
/// instance gate would still fire once per doc (300+ duplicates on a
/// real docs build). Keyed on `theme_name` so each missing theme warns
/// exactly once for the whole process.
fn warn_once_for_unbundled(theme: &str) -> bool {
  use std::collections::HashSet;
  use std::sync::Mutex;
  use std::sync::OnceLock;
  static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
  let mu = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
  let mut g = mu.lock().expect("pretty-code dedupe mutex poisoned");
  g.insert(theme.to_string())
}

/// Resolved DOM-shape options. All fields are post-default so render
/// code can read them without `Option` chasing on every code block.
#[derive(Debug, Clone)]
struct ShapeOpts {
  keep_raw_string: bool,
  fragment_wrapper: bool,
  line_class: String,
  highlighted_line_attr: String,
  default_language: String,
  fallback_to_plaintext: bool,
  render_title: bool,
  include_data_language: bool,
  include_pre_background: bool,
  skip_languages: Vec<String>,
  tab_size: Option<u32>,
  multi_theme_strategy: MultiThemeStrategy,
}

impl ShapeOpts {
  fn from_options(o: &PrettyCodeOptions) -> Self {
    Self {
      keep_raw_string: o.keep_raw_string.unwrap_or(true),
      fragment_wrapper: o.fragment_wrapper.unwrap_or(true),
      line_class: o.line_class.clone().unwrap_or_else(|| "line".into()),
      highlighted_line_attr: o.highlighted_line_attr.clone().unwrap_or_else(|| "data-dmc-line-highlighted".into()),
      default_language: o.default_language.clone().unwrap_or_else(|| "plaintext".into()),
      fallback_to_plaintext: o.fallback_to_plaintext.unwrap_or(true),
      render_title: o.render_title.unwrap_or(true),
      include_data_language: o.include_data_language.unwrap_or(true),
      include_pre_background: o.include_pre_background.unwrap_or(true),
      skip_languages: o.skip_languages.clone(),
      tab_size: o.tab_size,
      multi_theme_strategy: o.multi_theme_strategy.unwrap_or_default(),
    }
  }
}

impl Default for PrettyCode {
  fn default() -> Self {
    Self::from_options(&PrettyCodeOptions::default())
  }
}

impl PrettyCode {
  /// Single-theme constructor.
  pub fn new(theme: impl Into<String>) -> Self {
    Self::from_options(&PrettyCodeOptions { theme: PrettyCodeTheme::Single(theme.into()), ..Default::default() })
  }

  /// Resolve `PrettyCodeOptions` into a runtime `PrettyCode`. Picks
  /// `default_mode` from explicit config, else `"dark"` if present, else
  /// the first key in the theme map.
  pub fn from_options(opts: &PrettyCodeOptions) -> Self {
    let shape = ShapeOpts::from_options(opts);
    match &opts.theme {
      PrettyCodeTheme::Single(name) => {
        Self { themes: vec![(String::new(), name.clone())], default_mode: String::new(), shape }
      },
      PrettyCodeTheme::Multi(map) => {
        // Order modes the way shiki/rehype-pretty-code does: light first,
        // then dark, then any other custom mode in alphabetical order. The
        // user-facing data is theme-key -> theme-name, so sorting at the
        // boundary lets consumers diff against shiki output without having
        // to maintain insertion-ordered config maps in Rust.
        let mut themes: Vec<(String, String)> = map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        themes.sort_by(|a, b| {
          fn rank(k: &str) -> u8 {
            match k {
              "light" => 0,
              "dark" => 1,
              _ => 2,
            }
          }
          rank(&a.0).cmp(&rank(&b.0)).then_with(|| a.0.cmp(&b.0))
        });
        let default_mode = opts
          .default_mode
          .clone()
          .filter(|m| map.contains_key(m))
          .or_else(|| if map.contains_key("dark") { Some("dark".into()) } else { None })
          .or_else(|| themes.first().map(|(k, _)| k.clone()))
          .unwrap_or_default();
        Self { themes, default_mode, shape }
      },
    }
  }
}

impl Transformer for PrettyCode {
  fn name(&self) -> &str {
    "pretty-code"
  }
  fn transform(&self, doc: &mut Document, _meta: &SourceMeta, engine: &mut DiagnosticEngine<Code>) {
    // Warn once per process for every misconfigured theme. The pipeline
    // rebuilds a fresh `PrettyCode` per doc, so a per-instance gate
    // would still fire 300+ times on a typical docs build.
    let bundled = list_bundled_themes();
    for (mode, theme) in &self.themes {
      if bundled.iter().any(|t| t == theme) {
        continue;
      }
      if !warn_once_for_unbundled(theme) {
        continue;
      }
      let mode_label = if mode.is_empty() { "default".to_string() } else { mode.clone() };
      let hint = if bundled.is_empty() {
        "no themes are bundled — run a clean build".to_string()
      } else {
        format!("bundled themes: {}", bundled.join(", "))
      };
      engine.emit(
        diag!(
          Code::ThemeNotBundled,
          format!(
            "pretty-code: theme `{theme}` (mode `{mode_label}`) is not bundled; falling back to the first bundled theme",
          ))
        .with_help(hint),
      );
    }
    let mut v = Apply { themes: &self.themes, default_mode: &self.default_mode, shape: &self.shape };
    walk_root(&mut doc.children, &mut v);
  }
}

struct Apply<'a> {
  themes: &'a [(String, String)],
  default_mode: &'a str,
  shape: &'a ShapeOpts,
}

impl Visitor for Apply<'_> {
  fn visit_node(&mut self, node: &mut Node) -> NodeAction {
    let Node::CodeBlock(cb) = node else { return NodeAction::Keep };
    // Mermaid blocks are owned by the `mermaid` transformer.
    if cb.lang.as_deref() == Some("mermaid") {
      return NodeAction::Keep;
    }
    // Skip user-listed languages (passed through unchanged for downstream
    // consumers). Bare strings or comparisons against `lang` only.
    if let Some(lang) = cb.lang.as_deref()
      && self.shape.skip_languages.iter().any(|l| l == lang)
    {
      return NodeAction::Keep;
    }
    let meta = cb.meta.as_deref().map(parse_meta).unwrap_or_default();
    let rendered = render_code_block(cb, &meta, self.themes, self.default_mode, self.shape);
    match rendered {
      Some(node) => NodeAction::Replace(vec![Node::JsxElement(node)]),
      None => NodeAction::Keep,
    }
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

/// Render a fenced code block as velite's `rehype-pretty-code` shape:
///
/// ```html
/// <div data-dmc-fragment="">
///   <div data-theme="<mode>"> <!-- one per theme -->
///     <pre __dmcRaw__="..." data-language="..." data-theme="<mode>">
///       <code data-language="..." data-theme="<mode>">
///         <span class="line">
///           <span style="color:#XXX">token</span>
///           ...
///         </span>
///       </code>
///     </pre>
///   </div>
///   ...
/// </div>
/// ```
///
/// Each theme gets its own `<pre>` with solid `color:#XXX` per token (no
/// shared CSS-var fallback). `__dmcRaw__` lets the consumer's `<pre>`
/// override render the Copy button. The outer `<div data-theme>` wrapper
/// is what consumer CSS uses to show/hide per active theme.
fn render_code_block(
  cb: &CodeBlock,
  meta: &CodeMeta,
  themes: &[(String, String)],
  default_mode: &str,
  shape: &ShapeOpts,
) -> Option<JsxElement> {
  let theme_names: Vec<&str> = themes.iter().map(|(_, n)| n.as_str()).collect();
  // Resolve language: explicit fence lang > default. Borrow when possible
  // so the common path (lang on fence, no tab expansion) does zero allocs.
  let resolved_lang: &str = match cb.lang.as_deref() {
    Some(l) => l,
    None => shape.default_language.as_str(),
  };
  if !shape.fallback_to_plaintext
    && cb.lang.is_some()
    && dmc_highlight::SyntaxBundle::get().syntaxes.find_syntax_by_token(resolved_lang).is_none()
  {
    return None;
  }
  // Optional tab→spaces expansion before highlighting. Borrow when off
  // (the typical case) so we don't clone every block's body.
  let expanded;
  let source: &str = match shape.tab_size {
    Some(n) if n > 0 => {
      expanded = cb.value.replace('\t', &" ".repeat(n as usize));
      expanded.as_str()
    },
    _ => cb.value.as_str(),
  };
  let lines = highlight_code_multi(source, Some(resolved_lang), &theme_names);
  let span = cb.span.clone();

  let bundle = SyntaxBundle::get();
  let foregrounds: Vec<Option<dmc_highlight::Color>> =
    themes.iter().map(|(_, name)| bundle.themes.themes.get(name).and_then(|t| t.settings.foreground)).collect();
  let backgrounds: Vec<Option<dmc_highlight::Color>> =
    themes.iter().map(|(_, name)| bundle.themes.themes.get(name).and_then(|t| t.settings.background)).collect();

  // Pick layout: single tree with `--dmc-{mode}` CSS vars (default,
  // ~25% faster), or one `<pre>` per theme (velite parity, ~2× nodes).
  // Single-theme always uses the simple per-theme path — there's only
  // one tree either way.
  let theme_blocks: Vec<Node> = if themes.len() > 1 && shape.multi_theme_strategy == MultiThemeStrategy::CssVars {
    vec![Node::JsxElement(render_css_vars_pre(
      cb,
      meta,
      &lines,
      themes,
      default_mode,
      &foregrounds,
      &backgrounds,
      resolved_lang,
      shape,
      span.clone(),
    ))]
  } else {
    themes
      .iter()
      .enumerate()
      .map(|(theme_idx, (mode, _))| {
        let fg_default = foregrounds[theme_idx];
        Node::JsxElement(render_theme_pre(
          cb,
          meta,
          &lines,
          theme_idx,
          mode,
          fg_default,
          resolved_lang,
          shape,
          span.clone(),
        ))
      })
      .collect()
  };

  let mut fragment_children: Vec<Node> = Vec::new();
  if shape.render_title
    && let Some(title) = &meta.title
  {
    let mut figcaption_attrs =
      vec![JsxAttr { name: "data-dmc-title".into(), value: JsxAttrValue::String(String::new()), span: span.clone() }];
    if shape.include_data_language {
      figcaption_attrs.push(JsxAttr {
        name: "data-language".into(),
        value: JsxAttrValue::String(cb.lang.clone().unwrap_or_default()),
        span: span.clone(),
      });
    }
    fragment_children.push(Node::JsxElement(JsxElement {
      name: "figcaption".into(),
      attrs: figcaption_attrs,
      children: vec![Node::Text(Text { value: title.clone(), span: span.clone() })],
      span: span.clone(),
    }));
  }
  fragment_children.extend(theme_blocks);

  // Without the fragment wrapper there's no single root to return —
  // emit a fragment-style `<>` if title is present, else just one
  // synthetic `<div>` wrapping the panes (keeps the AST single-rooted).
  if !shape.fragment_wrapper {
    return Some(JsxElement { name: "div".into(), attrs: Vec::new(), children: fragment_children, span });
  }

  Some(JsxElement {
    name: "div".into(),
    attrs: vec![JsxAttr {
      name: "data-dmc-fragment".into(),
      value: JsxAttrValue::String(String::new()),
      span: span.clone(),
    }],
    children: fragment_children,
    span,
  })
}

/// Build the `<pre><code>...</code></pre>` for one theme. Honors every
/// shape knob: line class, highlighted-line attribute, data-language
/// inclusion, raw-string preservation.
#[allow(clippy::too_many_arguments)]
fn render_theme_pre(
  cb: &CodeBlock,
  meta: &CodeMeta,
  lines: &[Vec<dmc_highlight::MultiToken<'_>>],
  theme_idx: usize,
  mode: &str,
  fg_default: Option<dmc_highlight::Color>,
  resolved_lang: &str,
  shape: &ShapeOpts,
  span: duck_diagnostic::Span,
) -> JsxElement {
  let mut line_children: Vec<Node> = Vec::with_capacity(lines.len());
  for (line_i, tokens) in lines.iter().enumerate() {
    let line_no = (line_i + 1) as u32;
    // Coalesce adjacent tokens with identical style — keeps DOM tight.
    let mut runs: Vec<(String, String)> = Vec::with_capacity(tokens.len());
    for tok in tokens.iter() {
      let style = single_theme_token_style(tok, theme_idx, fg_default);
      match runs.last_mut() {
        Some(last) if last.0 == style => last.1.push_str(tok.text),
        _ => runs.push((style, tok.text.to_string())),
      }
    }
    // Fold leading-whitespace runs into the first styled token so
    // indented lines don't emit an empty `<span>"   "</span>` sibling.
    coalesce_leading_whitespace(&mut runs);
    let mut tok_children: Vec<Node> = Vec::with_capacity(runs.len());
    for (style, text) in runs {
      let text_node = Node::Text(Text { value: text, span: span.clone() });
      let attrs = if style.is_empty() {
        Vec::new()
      } else {
        vec![JsxAttr { name: "style".into(), value: JsxAttrValue::String(style), span: span.clone() }]
      };
      tok_children.push(Node::JsxElement(JsxElement {
        name: "span".into(),
        attrs,
        children: vec![text_node],
        span: span.clone(),
      }));
    }
    let mut line_attrs =
      vec![JsxAttr { name: "class".into(), value: JsxAttrValue::String(shape.line_class.clone()), span: span.clone() }];
    if meta.line_marks.iter().any(|m| m.contains(line_no)) {
      line_attrs.push(JsxAttr {
        name: shape.highlighted_line_attr.clone(),
        value: JsxAttrValue::String(String::new()),
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

  let mut code_attrs: Vec<JsxAttr> = Vec::new();
  if shape.include_data_language {
    code_attrs.push(JsxAttr {
      name: "data-language".into(),
      value: JsxAttrValue::String(resolved_lang.to_string()),
      span: span.clone(),
    });
  }
  code_attrs.push(JsxAttr {
    name: "data-theme".into(),
    value: JsxAttrValue::String(mode.to_string()),
    span: span.clone(),
  });
  let code_el = Node::JsxElement(JsxElement {
    name: "code".into(),
    attrs: code_attrs,
    children: line_children,
    span: span.clone(),
  });

  let mut pre_attrs: Vec<JsxAttr> = Vec::new();
  if shape.keep_raw_string {
    // Consumer `<PreBlock>` reads `__dmcRaw__` for the Copy button.
    pre_attrs.push(JsxAttr {
      name: "__dmcRaw__".into(),
      value: JsxAttrValue::String(cb.value.clone()),
      span: span.clone(),
    });
  }
  if shape.include_data_language {
    pre_attrs.push(JsxAttr {
      name: "data-language".into(),
      value: JsxAttrValue::String(resolved_lang.to_string()),
      span: span.clone(),
    });
  }
  pre_attrs.push(JsxAttr {
    name: "data-theme".into(),
    value: JsxAttrValue::String(mode.to_string()),
    span: span.clone(),
  });

  JsxElement { name: "pre".into(), attrs: pre_attrs, children: vec![code_el], span }
}

/// Multi-theme css-vars renderer. ONE `<pre><code>` tree carrying every
/// theme via `--dmc-{mode}` CSS custom properties. Consumer CSS swaps
/// modes by overriding `color` to `var(--dmc-{active-mode})`.
///
/// The `<pre>` carries default `color` / `background-color` from the
/// primary mode, plus `--dmc-{mode}:#hex` for every mode (foreground)
/// and `--dmc-{mode}-bg:#hex` (background). Each styled token carries
/// `--dmc-{mode}:#hex` for its color, plus per-mode font-style if any.
/// Tokens whose color matches the default foreground emit no style.
#[allow(clippy::too_many_arguments)]
fn render_css_vars_pre(
  cb: &CodeBlock,
  meta: &CodeMeta,
  lines: &[Vec<dmc_highlight::MultiToken<'_>>],
  themes: &[(String, String)],
  default_mode: &str,
  foregrounds: &[Option<dmc_highlight::Color>],
  backgrounds: &[Option<dmc_highlight::Color>],
  resolved_lang: &str,
  shape: &ShapeOpts,
  span: duck_diagnostic::Span,
) -> JsxElement {
  let default_idx = themes.iter().position(|(m, _)| m == default_mode).unwrap_or(0);

  let mut line_children: Vec<Node> = Vec::with_capacity(lines.len());
  for (line_i, tokens) in lines.iter().enumerate() {
    let line_no = (line_i + 1) as u32;
    // Coalesce adjacent tokens with identical multi-theme style runs.
    let mut runs: Vec<(String, String)> = Vec::with_capacity(tokens.len());
    for tok in tokens.iter() {
      let style = css_vars_token_style(tok, themes, foregrounds);
      match runs.last_mut() {
        Some(last) if last.0 == style => last.1.push_str(tok.text),
        _ => runs.push((style, tok.text.to_string())),
      }
    }
    // Fold leading-whitespace runs into the first styled token.
    coalesce_leading_whitespace(&mut runs);
    let mut tok_children: Vec<Node> = Vec::with_capacity(runs.len());
    for (style, text) in runs {
      let text_node = Node::Text(Text { value: text, span: span.clone() });
      let attrs = if style.is_empty() {
        Vec::new()
      } else {
        vec![JsxAttr { name: "style".into(), value: JsxAttrValue::String(style), span: span.clone() }]
      };
      tok_children.push(Node::JsxElement(JsxElement {
        name: "span".into(),
        attrs,
        children: vec![text_node],
        span: span.clone(),
      }));
    }
    let mut line_attrs =
      vec![JsxAttr { name: "class".into(), value: JsxAttrValue::String(shape.line_class.clone()), span: span.clone() }];
    if meta.line_marks.iter().any(|m| m.contains(line_no)) {
      line_attrs.push(JsxAttr {
        name: shape.highlighted_line_attr.clone(),
        value: JsxAttrValue::String(String::new()),
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

  let mut code_attrs: Vec<JsxAttr> = Vec::new();
  if shape.include_data_language {
    code_attrs.push(JsxAttr {
      name: "data-language".into(),
      value: JsxAttrValue::String(resolved_lang.to_string()),
      span: span.clone(),
    });
  }
  let code_el = Node::JsxElement(JsxElement {
    name: "code".into(),
    attrs: code_attrs,
    children: line_children,
    span: span.clone(),
  });

  // `<pre>` style: primary `color` + `background-color`, plus
  // `--dmc-{mode}` and `--dmc-{mode}-bg` per mode for theme switching.
  let pre_style = build_pre_style(themes, foregrounds, backgrounds, default_idx, shape.include_pre_background);
  let mut pre_attrs: Vec<JsxAttr> = Vec::new();
  if shape.keep_raw_string {
    pre_attrs.push(JsxAttr {
      name: "__dmcRaw__".into(),
      value: JsxAttrValue::String(cb.value.clone()),
      span: span.clone(),
    });
  }
  if shape.include_data_language {
    pre_attrs.push(JsxAttr {
      name: "data-language".into(),
      value: JsxAttrValue::String(resolved_lang.to_string()),
      span: span.clone(),
    });
  }
  if !pre_style.is_empty() {
    pre_attrs.push(JsxAttr { name: "style".into(), value: JsxAttrValue::String(pre_style), span: span.clone() });
  }
  JsxElement { name: "pre".into(), attrs: pre_attrs, children: vec![code_el], span }
}

/// Build the multi-theme token-style string. Emits `--dmc-{mode}:#hex`
/// for every mode whose color differs from that mode's default
/// foreground, plus per-mode font-style flags.
/// If the line begins with one or more whitespace-only style runs
/// (empty `style` strings, all chars are whitespace), merge their
/// text into the first styled run that follows. Avoids emitting an
/// empty `<span>   </span>` for indentation while keeping the visual
/// indent intact (the whitespace ends up as the leading text of the
/// first colored token's span).
///
/// Lines that are entirely whitespace are left as-is.
fn coalesce_leading_whitespace(runs: &mut Vec<(String, String)>) {
  let split = runs.iter().position(|(style, text)| !(style.is_empty() && text.chars().all(|c| c.is_whitespace())));
  let Some(idx) = split else { return };
  if idx == 0 {
    return;
  }
  let mut prefix = String::new();
  for (_, text) in runs.drain(..idx) {
    prefix.push_str(&text);
  }
  if let Some(first) = runs.first_mut() {
    first.1.insert_str(0, &prefix);
  }
}

fn css_vars_token_style(
  tok: &dmc_highlight::MultiToken<'_>,
  themes: &[(String, String)],
  foregrounds: &[Option<dmc_highlight::Color>],
) -> String {
  use dmc_highlight::HlFontStyle as FontStyle;
  // Whitespace inherits unconditionally — saves a span attribute on
  // most lines (significant share of token count).
  if tok.text.chars().all(|c| c.is_whitespace()) {
    return String::new();
  }
  let mut parts: Vec<String> = Vec::with_capacity(themes.len() * 2);
  for (j, (mode, _)) in themes.iter().enumerate() {
    let Some(style) = tok.styles.get(j) else { continue };
    let fg = style.foreground;
    let same_as_default =
      foregrounds.get(j).and_then(|c| *c).map(|d| d.r == fg.r && d.g == fg.g && d.b == fg.b).unwrap_or(false);
    if !same_as_default {
      parts.push(format!("--dmc-{mode}:#{:02X}{:02X}{:02X}", fg.r, fg.g, fg.b));
    }
    if style.font_style.contains(FontStyle::ITALIC) {
      parts.push(format!("--dmc-{mode}-fs:italic"));
    }
    if style.font_style.contains(FontStyle::BOLD) {
      parts.push(format!("--dmc-{mode}-fw:bold"));
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
      parts.push(format!("--dmc-{mode}-td:underline"));
    }
  }
  parts.join(";")
}

/// Build the `<pre>` `style` attribute: primary mode's solid color +
/// background, plus every mode's `--dmc-{mode}` and `--dmc-{mode}-bg`.
fn build_pre_style(
  themes: &[(String, String)],
  foregrounds: &[Option<dmc_highlight::Color>],
  backgrounds: &[Option<dmc_highlight::Color>],
  default_idx: usize,
  include_bg: bool,
) -> String {
  let mut parts: Vec<String> = Vec::with_capacity(themes.len() * 2 + 2);
  if let Some(fg) = foregrounds.get(default_idx).and_then(|c| *c) {
    parts.push(format!("color:#{:02x}{:02x}{:02x}", fg.r, fg.g, fg.b));
  }
  // Solid `background-color` from the primary theme. Consumers that
  // want their own chrome around the `<pre>` (and don't want the
  // theme palette bleeding through) opt out via
  // `prettyCode.includePreBackground: false` — the per-mode
  // `--dmc-{mode}-bg` custom properties below stay either way.
  if include_bg {
    if let Some(bg) = backgrounds.get(default_idx).and_then(|c| *c) {
      parts.push(format!("background-color:#{:02x}{:02x}{:02x}", bg.r, bg.g, bg.b));
    }
  }
  for (j, (mode, _)) in themes.iter().enumerate() {
    if let Some(fg) = foregrounds.get(j).and_then(|c| *c) {
      parts.push(format!("--dmc-{mode}:#{:02x}{:02x}{:02x}", fg.r, fg.g, fg.b));
    }
    if let Some(bg) = backgrounds.get(j).and_then(|c| *c) {
      parts.push(format!("--dmc-{mode}-bg:#{:02x}{:02x}{:02x}", bg.r, bg.g, bg.b));
    }
  }
  parts.join(";")
}

/// Per-theme token style: solid `color:#XXX` from that theme alone, plus
/// font-style flags. Tokens whose color matches the theme's default
/// foreground get an empty style so they inherit through `<pre>`.
fn single_theme_token_style(
  tok: &dmc_highlight::MultiToken<'_>,
  theme_idx: usize,
  fg_default: Option<dmc_highlight::Color>,
) -> String {
  use dmc_highlight::HlFontStyle as FontStyle;
  let Some(style) = tok.styles.get(theme_idx) else { return String::new() };
  let fg = style.foreground;
  let plain = tok.text.chars().all(|c| c.is_whitespace());
  if plain {
    return String::new();
  }
  let same_as_default = fg_default.map(|d| d.r == fg.r && d.g == fg.g && d.b == fg.b).unwrap_or(false);
  let mut s = if same_as_default { String::new() } else { format!("color:#{:02X}{:02X}{:02X}", fg.r, fg.g, fg.b) };
  if style.font_style.contains(FontStyle::ITALIC) {
    if !s.is_empty() {
      s.push(';');
    }
    s.push_str("font-style:italic");
  }
  if style.font_style.contains(FontStyle::BOLD) {
    if !s.is_empty() {
      s.push(';');
    }
    s.push_str("font-weight:bold");
  }
  if style.font_style.contains(FontStyle::UNDERLINE) {
    if !s.is_empty() {
      s.push(';');
    }
    s.push_str("text-decoration:underline");
  }
  s
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
    let opts = PrettyCodeOptions {
      theme: PrettyCodeTheme::Multi(map),
      default_mode: Some("bright".into()),
      ..Default::default()
    };
    let pc = PrettyCode::from_options(&opts);
    assert_eq!(pc.default_mode, "bright");
  }

  #[test]
  fn from_options_falls_back_to_first_when_no_dark_or_explicit() {
    let map: BTreeMap<String, String> =
      [("alpha".into(), "Nord".into()), ("beta".into(), "TwoDark".into())].into_iter().collect();
    let opts = PrettyCodeOptions { theme: PrettyCodeTheme::Multi(map), default_mode: None, ..Default::default() };
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
