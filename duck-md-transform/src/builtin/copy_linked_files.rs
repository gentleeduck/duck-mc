use crate::pipeline::Transformer;
use crate::visit::{VisitFlow, Visitor, walk_mut};
use duck_md_parser::ast::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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

impl Transformer for CopyLinkedFiles {
  fn name(&self) -> &str {
    "copy-linked-files"
  }
  fn transform(&self, doc: &mut Document) {
    let mut v = Apply { config: self };
    for c in &mut doc.children {
      walk_mut(c, &mut v);
    }
  }
}

struct Apply<'a> {
  config: &'a CopyLinkedFiles,
}

impl<'a> Visitor for Apply<'a> {
  fn visit_node(&mut self, node: &mut Node) -> VisitFlow {
    match node {
      Node::Image(i) => {
        if let Some(url) = self.config.publish(&i.src) {
          i.src = url;
        }
      },
      Node::Link(l) => {
        if l.href.starts_with("./") || l.href.starts_with("../") {
          if let Some(url) = self.config.publish(&l.href) {
            l.href = url;
          }
        }
      },
      _ => {},
    }
    VisitFlow::Continue
  }
}

impl CopyLinkedFiles {
  fn publish(&self, raw: &str) -> Option<String> {
    if raw.starts_with("http://")
      || raw.starts_with("https://")
      || raw.starts_with("//")
      || raw.starts_with('/')
    {
      return None;
    }
    if raw.starts_with('#') {
      return None;
    }
    {
      let map = self.map.lock().unwrap();
      if let Some(u) = map.get(raw) {
        return Some(u.clone());
      }
    }
    let path = self.source_dir.join(raw);
    let bytes = std::fs::read(&path).ok()?;
    let hash = blake3::hash(&bytes);
    let hash8 = &hash.to_hex().to_string()[..8];
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("asset");
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("bin");
    let filename =
      self.name_template.replace("[name]", stem).replace("[hash:8]", hash8).replace("[ext]", ext);
    let dest = self.assets_dir.join(&filename);
    std::fs::create_dir_all(&self.assets_dir).ok()?;
    if !dest.exists() {
      std::fs::write(&dest, &bytes).ok()?;
    }
    let mut url = self.base_url.clone();
    if !url.ends_with('/') {
      url.push('/');
    }
    url.push_str(&filename);
    self.map.lock().unwrap().insert(raw.to_string(), url.clone());
    Some(url)
  }
}
