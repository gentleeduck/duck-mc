#![deny(clippy::all)]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::Value;
use std::path::PathBuf;

use dmc::Engine;
use dmc::engine::collection::Collection as CollectionDef;
use dmc::engine::compile::{CompileConfig, Compiler};
use dmc::engine::config::EngineConfig;
use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;

#[napi]
pub fn compile(source: String) -> Result<Value> {
  let mut diag = DiagnosticEngine::<Code>::new();
  let out = Compiler::compile(&source, &mut diag);
  serde_json::to_value(&out).map_err(|e| Error::from_reason(e.to_string()))
}

#[napi]
pub fn compile_many(sources: Vec<String>) -> Result<Vec<Value>> {
  let mut diag = DiagnosticEngine::<Code>::new();
  sources
    .into_iter()
    .map(|s| {
      let out = Compiler::compile(&s, &mut diag);
      serde_json::to_value(&out).map_err(|e| Error::from_reason(e.to_string()))
    })
    .collect()
}

#[napi(object)]
pub struct CollectionInput {
  pub name: String,
  pub pattern: String,
  pub base_dir: String,
  pub schema: Option<Value>,
  pub single: Option<bool>,
}

#[napi(object)]
pub struct BuildInput {
  pub output_dir: String,
  pub collections: Vec<CollectionInput>,
  pub root: Option<String>,
  pub strict: Option<bool>,
  pub clean: Option<bool>,
  pub output_assets: Option<String>,
  pub output_base: Option<String>,
  pub output_name: Option<String>,
  pub output_format: Option<String>,
  pub markdown_remark_plugins: Option<Value>,
  pub markdown_rehype_plugins: Option<Value>,
  pub mdx_remark_plugins: Option<Value>,
  pub mdx_rehype_plugins: Option<Value>,
  pub copy_linked_files: Option<bool>,
  pub mdx_output_format: Option<String>,
  pub mdx_minify: Option<bool>,
  pub markdown_gfm: Option<bool>,
  pub include_html: Option<bool>,
}

#[napi(object)]
pub struct BuildReport {
  pub diagnostics: Vec<String>,
}

fn array_or_default(v: Option<Value>) -> Vec<Value> {
  match v {
    Some(Value::Array(a)) => a,
    _ => Vec::new(),
  }
}

#[napi]
pub fn build(input: BuildInput) -> Result<BuildReport> {
  let compile = CompileConfig {
    markdown_gfm: input.markdown_gfm.unwrap_or(true),
    emit_html: true,
    emit_body: true,
    mdx_minify: input.mdx_minify.unwrap_or(false),
    mdx_output_format: input.mdx_output_format,
    markdown_remark_plugins: array_or_default(input.markdown_remark_plugins),
    markdown_rehype_plugins: array_or_default(input.markdown_rehype_plugins),
    mdx_remark_plugins: array_or_default(input.mdx_remark_plugins),
    mdx_rehype_plugins: array_or_default(input.mdx_rehype_plugins),
    copy_linked_files: input.copy_linked_files.unwrap_or(false),
    output_assets: input.output_assets,
    output_base: input.output_base,
  };

  let cfg = EngineConfig {
    output_dir: PathBuf::from(input.output_dir),
    root: PathBuf::from(input.root.unwrap_or_else(|| ".".into())),
    strict: input.strict.unwrap_or(false),
    clean: input.clean.unwrap_or(false),
    output_name: input.output_name,
    output_format: input.output_format,
    include_html: input.include_html.unwrap_or(false),
    collections: input
      .collections
      .into_iter()
      .map(|c| CollectionDef {
        name: c.name,
        pattern: c.pattern,
        base_dir: PathBuf::from(c.base_dir),
        schema: c.schema,
        single: c.single.unwrap_or(false),
      })
      .collect(),
    compile,
  };

  let mut diag = DiagnosticEngine::<Code>::new();
  Engine::run(&cfg, None, &mut diag).map_err(|e| Error::from_reason(e.to_string()))?;

  Ok(BuildReport { diagnostics: diag.iter().map(|d| format!("{:?}", d)).collect() })
}
