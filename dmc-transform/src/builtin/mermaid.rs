//! Mermaid pre-renderer. See `transformers/mermaid.md` for full docs.

use crate::config::{MermaidOptions, MermaidThemeMode};
use crate::pipeline::Transformer;
use crate::visit::{NodeAction, Visitor, walk_root};
use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::SourceMeta;
use dmc_parser::ast::*;
use duck_diagnostic::{Diagnostic, Label, diag};
use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

/// Pre-render mermaid diagrams to inline SVG via the external `mmdc` CLI
/// (`@mermaid-js/mermaid-cli`).
///
/// Two input shapes are handled:
///   * ` ```mermaid ` fenced code blocks — replaced with
///     `<MermaidDiagram chart="..." {mode}Svg="<svg…>" ... />`.
///   * Author-written `<MermaidDiagram chart={`…`} />` JSX nodes — the
///     existing JSX node is preserved and `{mode}Svg` attributes are
///     appended.
///
/// Theme behavior is driven by [`MermaidOptions::theme`]:
/// `Single("dark")` renders once and emits a single `chartSvg` attr;
/// `Multi({ light: "default", dark: "dark" })` (the default) renders
/// per-mode and emits `lightSvg` + `darkSvg`.
///
/// Per-block failures emit [`Code::MermaidRenderFailed`]. The CLI
/// availability probe runs once per process; missing CLI → the whole
/// transformer becomes a no-op with [`Code::MmdcUnavailable`].
pub struct Mermaid {
  opts: MermaidOptions,
  /// Rendered-SVG cache keyed by `(theme, source)` hash, dedupes
  /// identical diagrams across a single compile run.
  cache: Mutex<HashMap<u64, String>>,
}

/// One-shot CLI availability probe.
static MMDC_AVAILABLE: OnceLock<bool> = OnceLock::new();

impl Default for Mermaid {
  fn default() -> Self {
    Self::from_options(MermaidOptions::default())
  }
}

impl Mermaid {
  /// Build a Mermaid transformer with the supplied options. Use
  /// `Mermaid::default()` for the bundled defaults.
  pub fn from_options(opts: MermaidOptions) -> Self {
    Self { opts, cache: Mutex::new(HashMap::new()) }
  }

  /// Convenience constructor preserved for backward compat: enables
  /// the disk cache at `dir`.
  pub fn with_output(p: impl Into<PathBuf>) -> Self {
    Self::from_options(MermaidOptions { output_dir: Some(p.into()), ..Default::default() })
  }

  fn mmdc_available() -> bool {
    *MMDC_AVAILABLE.get_or_init(|| {
      Command::new("mmdc")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    })
  }

  /// Iterate the requested modes as `(jsx_attr_name, mermaid_theme)` pairs.
  /// `Single("dark")` → `[("chartSvg", "dark")]`.
  /// `Multi({"light":"default","dark":"dark"})` → `[("lightSvg","default"), ("darkSvg","dark")]`.
  fn theme_renders(&self) -> Vec<(String, String)> {
    match &self.opts.theme {
      MermaidThemeMode::Single(name) => vec![("chartSvg".to_string(), name.clone())],
      MermaidThemeMode::Multi(map) => map.iter().map(|(k, v)| (format!("{k}Svg"), v.clone())).collect(),
    }
  }

  fn render_cached(&self, source: &str, theme: &str) -> Result<String, String> {
    let key = {
      use std::hash::{Hash, Hasher};
      let mut hasher = std::collections::hash_map::DefaultHasher::new();
      theme.hash(&mut hasher);
      source.hash(&mut hasher);
      hasher.finish()
    };

    if let Some(svg) = self.cache.lock().unwrap().get(&key) {
      return Ok(svg.clone());
    }

    if let Some(dir) = &self.opts.output_dir {
      let path = dir.join(format!("{key}.svg"));
      match std::fs::read_to_string(&path) {
        Ok(s) => return Ok(s),
        Err(e) => {
          if e.kind() != std::io::ErrorKind::NotFound {
            return Err(e.to_string());
          }
        },
      }
    }

    let svg = self.render_mmdc(source, theme)?;
    self.cache.lock().unwrap().insert(key, svg.clone());
    if let Some(dir) = &self.opts.output_dir {
      let _ = std::fs::create_dir_all(dir);
      let path = dir.join(format!("{key}.svg"));
      let _ = std::fs::write(&path, &svg).map_err(|e| e.to_string());
    }

    Ok(svg)
  }

  /// Build the mermaid `initialize` config that goes to
  /// `mmdc --configFile`. dmc defaults: `htmlLabels:false` (root + nested
  /// flowchart for safety), flowchart spacing knobs. User-supplied
  /// initialize fields overlay these defaults via shallow merge.
  fn build_mermaid_config(&self) -> serde_json::Value {
    let html_labels = self.opts.html_labels.unwrap_or(false);
    let mut base = serde_json::json!({
      "htmlLabels": html_labels,
      "flowchart": {
        "htmlLabels": html_labels,
        "useMaxWidth": true,
        "nodeSpacing": 50,
        "rankSpacing": 60,
        "padding": 20,
      }
    });
    // Serialise the full options struct, then strip dmc-side keys
    // (`theme`, `responsiveSvg`, `centerLabels`, `outputDir`,
    // `puppeteerConfigFile`, `backgroundColor`) — every remaining field
    // is part of the typed `mermaid.initialize()` surface.
    if let Ok(serde_json::Value::Object(mut user)) = serde_json::to_value(&self.opts) {
      for k in ["theme", "responsiveSvg", "centerLabels", "outputDir", "puppeteerConfigFile", "backgroundColor"] {
        user.remove(k);
      }
      if !user.is_empty() {
        shallow_merge(&mut base, &serde_json::Value::Object(user));
      }
    }
    base
  }

  /// Run `mmdc` once for the given mermaid `source` + `theme`. Captures
  /// stdout (the SVG markup); maps non-zero exit / stderr to an error.
  fn render_mmdc(&self, source: &str, theme: &str) -> Result<String, String> {
    let cfg_json = self.build_mermaid_config();
    let cfg_str = cfg_json.to_string();
    let cfg_dir = std::env::temp_dir();
    // Hash the config so concurrent compiles with different options
    // don't clobber each other's config file.
    let cfg_hash = {
      use std::hash::{Hash, Hasher};
      let mut hasher = std::collections::hash_map::DefaultHasher::new();
      cfg_str.hash(&mut hasher);
      hasher.finish()
    };
    let cfg_path = cfg_dir.join(format!("dmc-mermaid-config-{}-{cfg_hash:x}.json", std::process::id()));
    if !cfg_path.exists() {
      std::fs::write(&cfg_path, &cfg_str).map_err(|e| format!("config write failed: {e}"))?;
    }

    let bg = self.opts.background_color.as_deref().unwrap_or("transparent");
    let cfg_path_str = cfg_path.to_str().unwrap_or("").to_string();

    let mut args: Vec<String> = vec![
      "--input".into(),
      "-".into(),
      "--output".into(),
      "-".into(),
      "--outputFormat".into(),
      "svg".into(),
      "--theme".into(),
      theme.to_string(),
      "--backgroundColor".into(),
      bg.to_string(),
      "--configFile".into(),
      cfg_path_str,
      "--quiet".into(),
    ];
    if let Some(p) = &self.opts.puppeteer_config_file {
      args.push("--puppeteerConfigFile".into());
      args.push(p.to_string_lossy().into_owned());
    }

    let mut child = Command::new("mmdc")
      .args(&args)
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .spawn()
      .map_err(|e| format!("spawn failed: {e}"))?;
    child
      .stdin
      .as_mut()
      .ok_or_else(|| "no stdin handle".to_string())?
      .write_all(source.as_bytes())
      .map_err(|e| format!("stdin write failed: {e}"))?;
    let out = child.wait_with_output().map_err(|e| format!("wait failed: {e}"))?;
    if !out.status.success() {
      let err = String::from_utf8_lossy(&out.stderr).into_owned();
      return Err(if err.is_empty() { format!("exit {}", out.status) } else { err });
    }
    let svg = String::from_utf8(out.stdout).map_err(|e| format!("non-utf8 svg: {e}"))?;
    Ok(self.post_process(&svg))
  }

  /// Apply optional SVG post-processing: responsive width, centered
  /// labels. Both default-on; togglable via `MermaidOptions`.
  fn post_process(&self, svg: &str) -> String {
    let mut out = svg.to_string();
    if self.opts.responsive_svg.unwrap_or(true) {
      out = make_responsive(&out);
    }
    if self.opts.center_labels.unwrap_or(true) {
      // Only meaningful with htmlLabels:false. Cheap no-op otherwise.
      out = center_labels(&out);
    }
    out
  }

  /// Render every requested theme for `chart`, returning a map of
  /// `{ jsx_attr_name -> svg_string }`. `None` if any theme errors out
  /// (caller emits a diagnostic then).
  fn render_all(
    &self,
    chart: &str,
    span: &duck_diagnostic::Span,
    pending: &mut Vec<Diagnostic<Code>>,
  ) -> Option<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for (attr, theme) in self.theme_renders() {
      match self.render_cached(chart, &theme) {
        Ok(s) => {
          out.insert(attr, s);
        },
        Err(err) => {
          pending.push(
            diag!(Code::MermaidRenderFailed, format!("mermaid ({theme}): mmdc failed - {}", err.trim()))
              .with_label(Label::primary(span.clone(), Some("for this mermaid block".into()))),
          );
          return None;
        },
      }
    }
    Some(out)
  }
}

/// Shallow-merge `extra` into `base` when both are JSON objects: keys in
/// `extra` overwrite keys in `base`. Non-object `extra` is ignored. We
/// intentionally don't recurse — mermaid's nested config (`flowchart`,
/// `themeVariables`, ...) is small enough that a user passing a partial
/// `flowchart` block should fully override our defaults for that block.
fn shallow_merge(base: &mut serde_json::Value, extra: &serde_json::Value) {
  use serde_json::Value;
  if let (Value::Object(b), Value::Object(e)) = (base, extra) {
    for (k, v) in e {
      b.insert(k.clone(), v.clone());
    }
  }
}

/// Rewrite the first `width="…"` on the root `<svg>` element to
/// `width="100%"` so the rendered diagram fluidly scales to its
/// container.
fn make_responsive(svg: &str) -> String {
  if let Some(idx) = svg.find("width=\"")
    && let Some(end) = svg[idx + "width=\"".len()..].find('"')
  {
    let head = &svg[..idx];
    let tail = &svg[idx + "width=\"".len() + end + 1..];
    return format!("{head}width=\"100%\"{tail}");
  }
  svg.to_string()
}

/// With `htmlLabels:false` mermaid 11 emits node `<text>` tags with no
/// `text-anchor`, and an inner `<tspan x="0">` that pins itself to the
/// label's local origin — i.e. node center. Result: label text starts
/// at the rect's mid-point and bleeds off the right edge ("Accordion" →
/// "Accordio"). Inject `text-anchor="middle"` on the outer text/tspans
/// so the `x="0"` becomes the *midpoint* of the line.
fn center_labels(svg: &str) -> String {
  let mut out = svg.replace("<text y=\"-10.1\"", "<text y=\"-10.1\" text-anchor=\"middle\"");
  out = out.replace(
    "<tspan class=\"text-outer-tspan row\" x=\"0\"",
    "<tspan class=\"text-outer-tspan row\" x=\"0\" text-anchor=\"middle\"",
  );
  out
}

impl Transformer for Mermaid {
  fn name(&self) -> &str {
    "mermaid"
  }
  fn transform(
    &self,
    doc: &mut Document,
    _meta: &SourceMeta,
    diag_engine: &mut duck_diagnostic::DiagnosticEngine<Code>,
  ) {
    if !Self::mmdc_available() {
      diag_engine.emit(diag!(
        Code::MmdcUnavailable,
        "mermaid: `mmdc` is not on PATH; mermaid blocks left as code (install with `npm i -g @mermaid-js/mermaid-cli`)"
      ));
      return;
    }
    let mut v = Apply { pending: Vec::new(), mermaid: self };
    walk_root(&mut doc.children, &mut v);
    for d in v.pending.drain(..) {
      diag_engine.emit(d);
    }
  }
}

struct Apply<'a> {
  pending: Vec<Diagnostic<Code>>,
  mermaid: &'a Mermaid,
}

impl<'a> Apply<'a> {
  /// Build the JSX attr list for a `<MermaidDiagram>` node: keep author
  /// extras (className, etc), but always (re)set `chart` and every
  /// rendered `${mode}Svg` from `svgs`.
  fn jsx_attrs_with_svgs(
    chart: String,
    svgs: BTreeMap<String, String>,
    span: &duck_diagnostic::Span,
    extra: Vec<JsxAttr>,
  ) -> Vec<JsxAttr> {
    let svg_keys: std::collections::HashSet<&str> = svgs.keys().map(String::as_str).collect();
    let mut out: Vec<JsxAttr> =
      extra.into_iter().filter(|a| a.name != "chart" && !svg_keys.contains(a.name.as_str())).collect();
    out.push(JsxAttr { name: "chart".into(), value: JsxAttrValue::String(chart), span: span.clone() });
    for (k, v) in svgs {
      out.push(JsxAttr { name: k, value: JsxAttrValue::String(v), span: span.clone() });
    }
    out
  }
}

impl<'a> Visitor for Apply<'a> {
  fn visit_node(&mut self, node: &mut Node) -> NodeAction {
    match node {
      // ```mermaid ... ``` -> <MermaidDiagram chart {modeKey}Svg ... />
      Node::CodeBlock(cb) if cb.lang.as_deref() == Some("mermaid") => {
        let span = cb.span.clone();
        let chart = cb.value.clone();
        let Some(svgs) = self.mermaid.render_all(&chart, &span, &mut self.pending) else {
          return NodeAction::Keep;
        };
        let attrs = Apply::jsx_attrs_with_svgs(chart, svgs, &span, Vec::new());
        *node = Node::JsxSelfClosing(JsxSelfClosing { name: "MermaidDiagram".into(), attrs, span });
        NodeAction::KeepSkipChildren
      },
      // <MermaidDiagram chart={`…`} /> -> same, with {modeKey}Svg appended.
      Node::JsxSelfClosing(jsc) if jsc.name == "MermaidDiagram" => {
        let span = jsc.span.clone();
        let Some(chart) = extract_chart_attr(&jsc.attrs) else { return NodeAction::Keep };
        let Some(svgs) = self.mermaid.render_all(&chart, &span, &mut self.pending) else {
          return NodeAction::Keep;
        };
        let extra = std::mem::take(&mut jsc.attrs);
        jsc.attrs = Apply::jsx_attrs_with_svgs(chart, svgs, &span, extra);
        NodeAction::KeepSkipChildren
      },
      Node::JsxElement(je) if je.name == "MermaidDiagram" => {
        let span = je.span.clone();
        let Some(chart) = extract_chart_attr(&je.attrs) else { return NodeAction::Keep };
        let Some(svgs) = self.mermaid.render_all(&chart, &span, &mut self.pending) else {
          return NodeAction::Keep;
        };
        let extra = std::mem::take(&mut je.attrs);
        je.attrs = Apply::jsx_attrs_with_svgs(chart, svgs, &span, extra);
        NodeAction::KeepSkipChildren
      },
      _ => NodeAction::Keep,
    }
  }
}

/// Pull the `chart` attribute value out as a plain string. Handles both
/// `chart="…"` (string literal) and `chart={`…`}` /
/// `chart={"…"}` (expression carrying a single string / template). The
/// expression branch trims the surrounding `"…"` or `` `…` `` so the
/// extracted text is mermaid source ready for `mmdc`.
fn extract_chart_attr(attrs: &[JsxAttr]) -> Option<String> {
  let attr = attrs.iter().find(|a| a.name == "chart")?;
  match &attr.value {
    JsxAttrValue::String(s) => Some(s.clone()),
    JsxAttrValue::Expression(e) => {
      let t = e.trim();
      if (t.starts_with('`') && t.ends_with('`'))
        || (t.starts_with('"') && t.ends_with('"'))
        || (t.starts_with('\'') && t.ends_with('\''))
      {
        Some(t[1..t.len() - 1].to_string())
      } else {
        None
      }
    },
    JsxAttrValue::Boolean => None,
  }
}
