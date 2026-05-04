use dmc_diagnostic::Code;
use duck_diagnostic::{DiagnosticEngine, diag};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::engine::{
  cache::{FileCache, fingerprint},
  compile::Compiler,
  config::EngineConfig,
  sidecar::run_sidecar,
  utils::{CollectionReport, build_schema_ctx, build_velite_record, minify_js, wrap_mdx_module},
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
  /// Compile every file matched by `pattern` in parallel, validate
  /// frontmatter against `schema`, optionally run JS sidecars + MDX module
  /// wrap + minify, then write `{name}.json`.
  pub(crate) fn process(
    &self,
    cfg: &EngineConfig,
    diag_engine: &mut DiagnosticEngine<Code>,
  ) -> Result<CollectionReport, ()> {
    let walker = globwalk::GlobWalkerBuilder::from_patterns(&self.base_dir, &[&self.pattern]).build().map_err(|e| {
      diag_engine.emit(diag!(Code::EmptyFrontMatter, format!("globwalk error: {}", e)));
    })?;

    let paths = walker.filter_map(|e| e.ok()).map(|e| e.path().to_path_buf()).collect::<Vec<PathBuf>>();

    let collection_schema = self.schema.as_ref().and_then(|d| {
      dmc_schema::compile_descriptor(d)
        .map_err(|e| {
          diag_engine.emit(diag!(Code::EmptyFrontMatter, format!("schema error: {}", e)));
        })
        .ok()
    });

    // Persistent per-file cache. Each record is keyed by
    // (dmc_version, source_bytes, path, full-cfg-fingerprint) so any
    // change to source or relevant config invalidates the entry.
    let cache = if cfg.cache_enabled { FileCache::open(cfg.output_dir.join(".cache").join("dmc")) } else { None };
    let cfg_fp = fingerprint(&(&cfg.compile, &cfg.include_html, &self.name, &self.schema, &cfg.output_format));

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

        // Cache lookup: skip lex/parse/transform/codegen + sidecar when
        // (source + cfg) is unchanged. Hits the disk and returns the
        // already-rendered Value directly.
        let cache_key = cache.as_ref().map(|_| FileCache::key(source.as_bytes(), path, &cfg_fp));
        if let (Some(c), Some(k)) = (cache.as_ref(), cache_key.as_ref())
          && let Some(hit) = c.get(k)
        {
          return (Some(hit), local_diag_engine);
        }

        let local_compiler_cfg = cfg.compile.for_render();
        let use_sidecar = cfg.compile.has_js_plugins();

        let mut compiled = Compiler::compile_with_pipeline(&source, path, &local_compiler_cfg, &mut local_diag_engine);

        if use_sidecar {
          if let Some(html) = run_sidecar(&compiled.content, cfg) {
            compiled.html = html;
          }
        }

        if cfg.compile.mdx_output_format.as_deref() == Some("module") {
          compiled.body = wrap_mdx_module(&compiled.body, &compiled.imports);
        }
        if cfg.compile.mdx_minify {
          compiled.body = minify_js(&compiled.body);
        }

        let validated_frontmatter = match (&collection_schema, &compiled.frontmatter) {
          (Some(schema), fm) if !fm.is_null() => {
            let ctx = build_schema_ctx(path, &cfg.root, &compiled, cfg);
            match schema.parse(fm, &ctx) {
              Ok(v) => v,
              Err(e) => {
                local_diag_engine.emit(diag!(Code::EmptyFrontMatter, format!("schema error: {}", e)));
                compiled.frontmatter.clone()
              },
            }
          },
          _ => compiled.frontmatter.clone(),
        };

        let include_html = cfg.include_html || use_sidecar;
        let rec = build_velite_record(compiled, validated_frontmatter, path, &self.base_dir, &self.name, include_html);

        // Persist into the on-disk cache so next build sees a hit.
        if let (Some(c), Some(k)) = (cache.as_ref(), cache_key.as_ref()) {
          c.put(k, &rec);
        }
        (Some(rec), local_diag_engine)
      })
      .collect();

    let mut records: Vec<Value> = Vec::with_capacity(outcomes.len());
    for (rec, local_diag_engine) in outcomes {
      diag_engine.extend(local_diag_engine);
      if let Some(r) = rec {
        records.push(r);
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
