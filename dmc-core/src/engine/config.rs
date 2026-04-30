use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

use crate::engine::collection::Collection;

/// Top-level engine config. Drives [`run`] — collections to compile, where
/// to write output, schema strictness, JS plugin hooks (remark/rehype that
/// run via a Node sidecar), and feature flags such as GFM toggling.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EngineConfig {
  pub collections: Vec<Collection>,
  pub output_dir: PathBuf,
  pub root: PathBuf,
  pub strict: bool,
  pub clean: bool,
  pub output_assets: Option<PathBuf>,
  pub output_base: Option<String>,
  pub output_name: Option<String>,
  pub output_format: Option<String>,
  pub markdown_remark_plugins: Option<Value>,
  pub markdown_rehype_plugins: Option<Value>,
  pub mdx_remark_plugins: Option<Value>,
  pub mdx_rehype_plugins: Option<Value>,
  pub copy_linked_files: bool,
  pub mdx_output_format: Option<String>,
  pub mdx_minify: bool,
  pub markdown_gfm: bool,
  pub include_html: bool,
}

impl Default for EngineConfig {
  fn default() -> Self {
    Self {
      collections: Vec::new(),
      output_dir: PathBuf::new(),
      root: PathBuf::new(),
      strict: false,
      clean: false,
      output_assets: None,
      output_base: None,
      output_name: None,
      output_format: None,
      markdown_remark_plugins: None,
      markdown_rehype_plugins: None,
      mdx_remark_plugins: None,
      mdx_rehype_plugins: None,
      copy_linked_files: false,
      mdx_output_format: None,
      mdx_minify: false,
      markdown_gfm: true,
      include_html: false,
    }
  }
}
