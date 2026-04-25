use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Ctx {
  pub file_path: PathBuf,
  pub root: PathBuf,
  pub body: String,
  pub html: Option<String>,
  pub mdx_body: Option<String>,
  pub toc: Option<serde_json::Value>,
  pub plain_text: Option<String>,
  pub unique_cache: Mutex<HashSet<String>>,
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
    }
  }

  pub fn empty() -> Self {
    Self::new(PathBuf::new(), PathBuf::new(), String::new())
  }
}
