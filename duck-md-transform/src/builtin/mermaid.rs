use crate::pipeline::Transformer;
use crate::visit::{VisitFlow, Visitor, walk_mut};
use duck_md_parser::ast::*;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Default)]
pub struct Mermaid {
  pub output_dir: Option<PathBuf>,
}

impl Mermaid {
  pub fn with_output(p: impl Into<PathBuf>) -> Self {
    Self { output_dir: Some(p.into()) }
  }
}

impl Transformer for Mermaid {
  fn name(&self) -> &str {
    "mermaid"
  }
  fn transform(&self, doc: &mut Document) {
    if !mmdc_available() {
      return;
    }
    let mut v = Apply { output_dir: self.output_dir.clone() };
    for c in &mut doc.children {
      walk_mut(c, &mut v);
    }
  }
}

struct Apply {
  output_dir: Option<PathBuf>,
}

impl Visitor for Apply {
  fn visit_node(&mut self, node: &mut Node) -> VisitFlow {
    if let Node::CodeBlock(cb) = node
      && cb.lang.as_deref() == Some("mermaid")
      && cb.highlighted_html.is_none()
    {
      if let Some(svg) = render_mmdc(&cb.value) {
        cb.highlighted_html = Some(format!("<div class=\"mermaid-svg\">{svg}</div>",));
      }
    }
    VisitFlow::Continue
  }
}

fn mmdc_available() -> bool {
  Command::new("mmdc")
    .arg("--version")
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .status()
    .map(|s| s.success())
    .unwrap_or(false)
}

fn render_mmdc(source: &str) -> Option<String> {
  let mut child = Command::new("mmdc")
    .args(["--input", "-", "--output", "-", "--outputFormat", "svg"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .spawn()
    .ok()?;
  child.stdin.as_mut()?.write_all(source.as_bytes()).ok()?;
  let out = child.wait_with_output().ok()?;
  if !out.status.success() {
    return None;
  }
  String::from_utf8(out.stdout).ok()
}
