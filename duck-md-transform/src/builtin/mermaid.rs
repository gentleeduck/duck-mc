use crate::pipeline::Transformer;
use crate::visit::{NodeAction, Visitor, walk_root};
use duck_diagnostic::{Diagnostic, Label};
use duck_md_diagnostic::Code;
use duck_md_diagnostic::metadata::SourceMeta;
use duck_md_parser::ast::*;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

/// Render `mermaid` code blocks to inline SVG via the external `mmdc` CLI
/// (`@mermaid-js/mermaid-cli`). On success the `CodeBlock` is replaced with a
/// `<MermaidSvg svg="..." />` JsxSelfClosing.
///
/// Diagnostics: [`Code::MmdcUnavailable`] (warning, no-op when CLI absent),
/// [`Code::MermaidRenderFailed`] (per-block error with captured stderr).
#[derive(Default)]
pub struct Mermaid {
  /// Reserved for a future "write SVGs to disk + reference them" mode.
  pub output_dir: Option<PathBuf>,
  /// Cache of rendered SVGs, keyed by `code_block.hash`.
  cache: Mutex<HashMap<u64, String>>,
}

/// Statically guards the CLI availability check.
static MMDC_AVAILABLE: OnceLock<bool> = OnceLock::new();

impl Mermaid {
  pub fn with_output(p: impl Into<PathBuf>) -> Self {
    Self { output_dir: Some(p.into()), cache: Mutex::new(HashMap::new()) }
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

  pub fn render_cached(&self, source: &str) -> Result<String, String> {
    let key = {
      use std::hash::{Hash, Hasher};
      let mut hasher = std::collections::hash_map::DefaultHasher::new();
      source.hash(&mut hasher);
      hasher.finish()
    };

    // L1: Check the in-memory cache
    if let Some(svg) = self.cache.lock().unwrap().get(&key) {
      return Ok(svg.clone());
    }

    // L2: Check the disk cache
    if let Some(dir) = &self.output_dir {
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

    let svg = Self::render_mmdc(source)?;
    self.cache.lock().unwrap().insert(key, svg.clone());
    if let Some(dir) = &self.output_dir {
      let path = dir.join(format!("{key}.svg"));
      let _ = std::fs::write(&path, &svg).map_err(|e| e.to_string());
    }

    Ok(svg)
  }

  /// Returns captured stderr (or a synthesised reason) on failure.
  /// TODO: supprot for `--background` and `--theme` flags and `--configFile` so the user
  /// can customize the output.
  fn render_mmdc(source: &str) -> Result<String, String> {
    let mut child = Command::new("mmdc")
      .args(["--input", "-", "--output", "-", "--outputFormat", "svg"])
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
    String::from_utf8(out.stdout).map_err(|e| format!("non-utf8 svg: {e}"))
  }
}

impl Transformer for Mermaid {
  fn name(&self) -> &str {
    "mermaid"
  }
  fn transform(
    &self,
    doc: &mut Document,
    _meta: &SourceMeta,
    engine: &mut duck_diagnostic::DiagnosticEngine<Code>,
  ) {
    if !Self::mmdc_available() {
      engine.emit(Diagnostic::new(
        Code::MmdcUnavailable,
        "mermaid: `mmdc` is not on PATH; mermaid blocks left as code (install with `npm i -g @mermaid-js/mermaid-cli`)",
      ));
      return;
    }
    let mut v = Apply { pending: Vec::new(), mermaid: self };
    walk_root(&mut doc.children, &mut v);
    for d in v.pending.drain(..) {
      engine.emit(d);
    }
  }
}

struct Apply<'a> {
  pending: Vec<Diagnostic<Code>>,
  mermaid: &'a Mermaid,
}

impl<'a> Visitor for Apply<'a> {
  fn visit_node(&mut self, node: &mut Node) -> NodeAction {
    let Node::CodeBlock(cb) = node else { return NodeAction::Keep };
    if cb.lang.as_deref() != Some("mermaid") {
      return NodeAction::Keep;
    }
    let span = cb.span.clone();
    match self.mermaid.render_cached(&cb.value) {
      Ok(svg) => {
        *node = Node::JsxSelfClosing(JsxSelfClosing {
          name: "MermaidSvg".into(),
          attrs: vec![JsxAttr {
            name: "svg".into(),
            value: JsxAttrValue::String(svg),
            span: span.clone(),
          }],
          span,
        });
        NodeAction::KeepSkipChildren
      },
      Err(err) => {
        self.pending.push(
          Diagnostic::new(
            Code::MermaidRenderFailed,
            format!("mermaid: mmdc failed — {}", err.trim()),
          )
          .with_label(Label::primary(span, Some("for this mermaid block".into()))),
        );
        NodeAction::Keep
      },
    }
  }
}
