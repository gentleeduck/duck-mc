use dmc_diagnostic::Code;
use duck_diagnostic::{DiagnosticEngine, diag};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::engine::{
  config::EngineConfig,
  sidecar::run_sidecar,
  utils::{CollectionReport, build_schema_ctx, build_velite_record, has_js_plugins},
};

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
    diag_engine: &mut DiagnosticEngine<Code>,
  ) -> Result<CollectionReport, ()> {
    let walker = globwalk::GlobWalkerBuilder::from_patterns(&self.base_dir, &[&self.pattern])
      .build()
      .map_err(|e| {
        diag_engine.emit(diag!(Code::EmptyFrontMatter, format!("globwalk error: {}", e)));
      })?;

    let paths =
      walker.filter_map(|e| e.ok()).map(|e| e.path().to_path_buf()).collect::<Vec<PathBuf>>();

    let collection_schema = self.schema.as_ref().and_then(|d| {
      dmc_schema::compile_descriptor(d)
        .map_err(|e| {
          diag_engine.emit(diag!(Code::EmptyFrontMatter, format!("schema error: {}", e)));
        })
        .ok()
    });

    // NOTE: we can consider some other options like [`SIMD`]
    // let diag_engine = std::sync::Mutex::new(diag_engine);

    let outcomes: Vec<(Option<Value>, DiagnosticEngine<Code>)> = paths
      .par_iter()
      .map(|path| {
        let mut local_diag_engine = DiagnosticEngine::<Code>::new();

        let source = match std::fs::read_to_string(path) {
          Ok(s) => s,
          Err(e) => {
            local_diag_engine.emit(diag!(Code::EmptyFrontMatter, format!("error: {}", e)));
            return (None, local_diag_engine);
          },
        };

        let mut compiled = {
          let mut pipeline = dmc_transform::Pipeline::with_defaults();

          // TODO: refactor the transfomers below later on
          if !cfg.markdown_gfm {
            pipeline = pipeline.add(dmc_transform::DisableGfm);
          }

          if cfg.copy_linked_files && cfg.output_assets.is_some() && cfg.output_base.is_some() {
            pipeline = pipeline.add(dmc_transform::CopyLinkedFiles::new(
              path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf(),
              cfg.output_assets.clone().unwrap(),
              cfg.output_base.clone().unwrap(),
            ));
          }

          crate::compile_with_pipeline(&source, &pipeline, &mut local_diag_engine)
        };

        if has_js_plugins(cfg) {
          if let Some(html) = run_sidecar(&compiled.content, cfg) {
            compiled.html = html;
          }
        }
        // TODO:
        // if cfg.mdx_output_format.as_deref() == Some("module") {
        //   compiled.body = wrap_mdx_module(&compiled.body, &compiled.imports);
        // }
        // if cfg.mdx_minify {
        //   compiled.body = minify_js(&compiled.body);
        // }

        let validated_frontmatter = match (&collection_schema, &compiled.frontmatter) {
          (Some(schema), fm) if !fm.is_null() => {
            let ctx = build_schema_ctx(path, &cfg.root, &compiled, cfg);
            match schema.parse(fm, &ctx) {
              Ok(v) => v,
              Err(e) => {
                local_diag_engine
                  .emit(diag!(Code::EmptyFrontMatter, format!("schema error: {}", e)));
                compiled.frontmatter.clone()
              },
            }
          },
          _ => compiled.frontmatter.clone(),
        };

        let include_html = cfg.include_html || has_js_plugins(cfg);
        let rec = build_velite_record(
          compiled,
          validated_frontmatter,
          path,
          &self.base_dir,
          &self.name,
          include_html,
        );

        (Some(rec), local_diag_engine)
      })
      .collect();

    let mut records: Vec<Value> = Vec::with_capacity(outcomes.len());
    for (rec, local_diag_engine) in outcomes {
      diag_engine.extend(local_diag_engine);
      if rec.is_some() {
        records.push(rec.unwrap());
      }
    }

    let out_path = cfg.output_dir.join(format!("{}.json", self.name));
    let count = if self.single { if records.is_empty() { 0 } else { 1 } } else { records.len() };
    let json = if self.single {
      let single = records.into_iter().next().unwrap_or(Value::Null);
      serde_json::to_string_pretty(&single).unwrap()
    } else {
      serde_json::to_string_pretty(&records).unwrap()
    };

    std::fs::write(&out_path, json)
      .map_err(|e| diag_engine.emit(diag!(Code::EmptyFrontMatter, format!("error: {}", e))))?;

    Ok(CollectionReport { name: self.name.clone(), records: count, output_path: out_path })
  }
}
