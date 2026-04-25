use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct AssetPipeline {
  pub assets_dir: PathBuf,
  pub base_url: String,
  pub name_template: String,
  pub map: Arc<Mutex<HashMap<String, String>>>,
}

impl AssetPipeline {
  pub fn new(assets_dir: PathBuf, base_url: String) -> Self {
    Self {
      assets_dir,
      base_url,
      name_template: "[name]-[hash:8].[ext]".into(),
      map: Arc::new(Mutex::new(HashMap::new())),
    }
  }
}

pub struct Ctx {
  pub file_path: PathBuf,
  pub root: PathBuf,
  pub body: String,
  pub html: Option<String>,
  pub mdx_body: Option<String>,
  pub toc: Option<serde_json::Value>,
  pub plain_text: Option<String>,
  pub unique_cache: Mutex<HashSet<String>>,
  pub assets: Option<AssetPipeline>,
}

impl Ctx {
  pub fn new(file_path: PathBuf, root: PathBuf, body: String) -> Self {
    Self {
      file_path,
      root,
      body,
      html: None,
      mdx_body: None,
      toc: None,
      plain_text: None,
      unique_cache: Mutex::new(HashSet::new()),
      assets: None,
    }
  }

  pub fn empty() -> Self {
    Self::new(PathBuf::new(), PathBuf::new(), String::new())
  }
}
