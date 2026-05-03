use crate::pipeline::Transformer;
use crate::visit::{NodeAction, Visitor, walk_root};
use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::SourceMeta;
use dmc_parser::ast::*;
use duck_diagnostic::{Diagnostic, Label, Span};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Copy referenced asset files (image `src`s and relative `href`s) into
/// `assets_dir`, hash-name them via `name_template`, and rewrite the AST
/// node to point at the published URL under `base_url`. `map` caches
/// `raw -> url` so repeated references hash the file only once.
pub struct CopyLinkedFiles {
  pub source_dir: PathBuf,
  pub assets_dir: PathBuf,
  pub base_url: String,
  pub name_template: String,
  pub map: Arc<Mutex<HashMap<String, String>>>,
}

impl CopyLinkedFiles {
  pub fn new(source_dir: PathBuf, assets_dir: PathBuf, base_url: String) -> Self {
    Self {
      source_dir,
      assets_dir,
      base_url,
      name_template: "[name]-[hash:8].[ext]".into(),
      map: Arc::new(Mutex::new(HashMap::new())),
    }
  }
}

enum Outcome {
  Skip,
  Published(String),
  SourceMissing(PathBuf, std::io::Error),
  CopyFailed(PathBuf, std::io::Error),
}

impl Transformer for CopyLinkedFiles {
  fn name(&self) -> &str {
    "copy-linked-files"
  }
  fn transform(
    &self,
    doc: &mut Document,
    _meta: &SourceMeta,
    diag_engine: &mut duck_diagnostic::DiagnosticEngine<Code>,
  ) {
    let mut v = Apply { config: self, pending: Vec::new() };
    walk_root(&mut doc.children, &mut v);
    for d in v.pending.drain(..) {
      diag_engine.emit(d);
    }
  }
}

struct Apply<'a> {
  config: &'a CopyLinkedFiles,
  pending: Vec<Diagnostic<Code>>,
}

impl<'a> Apply<'a> {
  fn rewrite_slot(&mut self, raw_slot: &mut String, span: Span, kind: &'static str) {
    match self.config.publish(raw_slot) {
      Outcome::Skip => {},
      Outcome::Published(url) => *raw_slot = url,
      Outcome::SourceMissing(path, err) => {
        self.pending.push(
          Diagnostic::new(
            Code::AssetSourceMissing,
            format!("copy-linked-files: cannot read {} source {} ({})", kind, path.display(), err),
          )
          .with_label(Label::primary(span, Some(format!("from this {}", kind)))),
        );
      },
      Outcome::CopyFailed(path, err) => {
        self.pending.push(
          Diagnostic::new(
            Code::AssetCopyFailed,
            format!("copy-linked-files: failed to write asset {} ({})", path.display(), err),
          )
          .with_label(Label::primary(span, Some(format!("for this {}", kind)))),
        );
      },
    }
  }
}

impl<'a> Visitor for Apply<'a> {
  fn visit_node(&mut self, node: &mut Node) -> NodeAction {
    match node {
      Node::Image(i) => {
        let span = i.span.clone();
        self.rewrite_slot(&mut i.src, span, "image");
      },
      Node::Link(l) => {
        if l.href.starts_with("./") || l.href.starts_with("../") {
          let span = l.span.clone();
          self.rewrite_slot(&mut l.href, span, "link");
        }
      },
      _ => {},
    }
    NodeAction::Keep
  }
}

impl CopyLinkedFiles {
  fn publish(&self, raw: &str) -> Outcome {
    if raw.starts_with("http://")
      || raw.starts_with("https://")
      || raw.starts_with("//")
      || raw.starts_with('/')
      || raw.starts_with('#')
    {
      return Outcome::Skip;
    }
    {
      let map = self.map.lock().unwrap();
      if let Some(u) = map.get(raw) {
        return Outcome::Published(u.clone());
      }
    }
    let path = self.source_dir.join(raw);
    let bytes = match std::fs::read(&path) {
      Ok(b) => b,
      Err(e) => return Outcome::SourceMissing(path, e),
    };
    let hash = blake3::hash(&bytes);
    let hash8 = &hash.to_hex().to_string()[..8];
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("asset");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("bin");
    let filename = self.name_template.replace("[name]", stem).replace("[hash:8]", hash8).replace("[ext]", ext);
    let dest = self.assets_dir.join(&filename);
    if let Err(e) = std::fs::create_dir_all(&self.assets_dir) {
      return Outcome::CopyFailed(self.assets_dir.clone(), e);
    }
    if !dest.exists()
      && let Err(e) = std::fs::write(&dest, &bytes)
    {
      return Outcome::CopyFailed(dest, e);
    }
    let mut url = self.base_url.clone();
    if !url.ends_with('/') {
      url.push('/');
    }
    url.push_str(&filename);
    self.map.lock().unwrap().insert(raw.to_string(), url.clone());
    Outcome::Published(url)
  }
}
