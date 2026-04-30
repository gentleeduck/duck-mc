use dmc_diagnostic::Code;
use duck_diagnostic::{DiagnosticEngine, diag};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::engine::config::EngineConfig;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Collection {
  pub name: String,
  pub pattern: String,
  pub base_dir: PathBuf,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub schema: Option<Value>,
  #[serde(skip_serializing_if = "std::ops::Not::not")]
  pub single: bool,
}

impl Collection {
  /// Compile every file matched by `c.pattern` (in parallel via rayon),
  /// validate frontmatter against the collection's schema, optionally run
  /// JS sidecars + MDX module wrap + minify, then write the
  /// velite-compatible index file (`{name}.json`).
  pub(crate) fn process(
    &self,
    cfg: &EngineConfig,
    d_engine: &mut DiagnosticEngine<Code>,
  ) -> std::io::Result<()> {
    let walker =
      globwalk::GlobWalkerBuilder::from_patterns(&self.base_dir, &["\\"]).build().map_err(|e| {
        d_engine.emit(diag!(Code::EmptyFrontMatter, format!("globwalk error: {}", e)));
      });

    // let collection_schema = c.schema.as_ref().and_then(|d| {
    //   duck_md_schema::compile_descriptor(d)
    //     .map_err(|e| {
    //       errors.push(EngineError {
    //         file: PathBuf::from(format!("<schema for {}>", c.name)),
    //         message: e,
    //       });
    //     })
    //     .ok()
    // });
    //
    //

    // let paths: Vec<PathBuf> =
    //   walker.map(|e| e.path().to_path_buf()).filter(|p| p.is_file()).collect();
    //
    // println!("walker: {:?}", paths);
    //
    // let outcomes: Vec<Value> = paths
    //   .par_iter()
    //   .map(|path| {
    //     let source = match std::fs::read_to_string(path) {
    //       Ok(s) => s,
    //       Err(_e) => {
    //         return Value::Null;
    //       },
    //     };
    //     println!("source: {:?}", source);
    //
    //     Value::Null
    //   })
    //   .collect();

    // let outcomes: Vec<(Value, Option<EngineError>)> = paths
    //   .par_iter()
    //   .map(|path| {
    //     let source = match std::fs::read_to_string(path) {
    //       Ok(s) => s,
    //       Err(e) => {
    //         return (Value::Null, Some(EngineError { file: path.clone(), message: e.to_string() }));
    //       },
    //     };
    //     let mut compiled = {
    //       let mut pipeline = dmc_transform::Pipeline::with_defaults();
    //       if !cfg.markdown_gfm {
    //         pipeline = pipeline.add(dmc_transform::DisableGfm);
    //       }
    //       if cfg.copy_linked_files && cfg.output_assets.is_some() && cfg.output_base.is_some() {
    //         pipeline = pipeline.add(dmc_transform::CopyLinkedFiles::new(
    //           path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf(),
    //           cfg.output_assets.clone().unwrap(),
    //           cfg.output_base.clone().unwrap(),
    //         ));
    //       }
    //       crate::compile_with_pipeline(&source, &pipeline)
    //     };
    //     if has_js_plugins(cfg) {
    //       if let Some(html) = run_sidecar(&compiled.content, cfg) {
    //         compiled.html = html;
    //       }
    //     }
    //     if cfg.mdx_output_format.as_deref() == Some("module") {
    //       compiled.body = wrap_mdx_module(&compiled.body, &compiled.imports);
    //     }
    //     if cfg.mdx_minify {
    //       compiled.body = minify_js(&compiled.body);
    //     }
    //     let (validated_frontmatter, err) = match (&collection_schema, &compiled.frontmatter) {
    //       (Some(schema), fm) if !fm.is_null() => {
    //         let ctx = build_schema_ctx(path, &cfg.root, &compiled, cfg);
    //         match schema.parse(fm, &ctx) {
    //           Ok(v) => (v, None),
    //           Err(e) => (
    //             compiled.frontmatter.clone(),
    //             Some(EngineError { file: path.clone(), message: e.to_string() }),
    //           ),
    //         }
    //       },
    //       _ => (compiled.frontmatter.clone(), None),
    //     };
    //     let include_html = cfg.include_html || has_js_plugins(cfg);
    //     let rec = build_velite_record(
    //       compiled,
    //       validated_frontmatter,
    //       path,
    //       &c.base_dir,
    //       &c.name,
    //       include_html,
    //     );
    //     (rec, err)
    //   })
    //   .collect();
    //
    // let mut records: Vec<Value> = Vec::with_capacity(outcomes.len());
    // for (rec, err) in outcomes {
    //   if let Some(e) = err {
    //     errors.push(e);
    //   }
    //   if !rec.is_null() {
    //     records.push(rec);
    //   }
    // }
    //
    // let out_path = cfg.output_dir.join(format!("{}.json", c.name));
    // let count = if c.single { if records.is_empty() { 0 } else { 1 } } else { records.len() };
    // let json = if c.single {
    //   let single = records.into_iter().next().unwrap_or(Value::Null);
    //   serde_json::to_string_pretty(&single).unwrap()
    // } else {
    //   serde_json::to_string_pretty(&records).unwrap()
    // };
    // std::fs::write(&out_path, json)?;
    // Ok(CollectionReport { name: c.name.clone(), records: count, output_path: out_path })
    Ok(())
  }
}
