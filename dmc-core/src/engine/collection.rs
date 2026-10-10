use dmc_diagnostic::Code;
use duck_diagnostic::{DiagnosticEngine, diag};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

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
  /// Component name -> schema descriptor. Matching JSX in a body is validated and emitted onto
  /// the record, so the interactive parts of a document are readable as data.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub components: Option<Value>,
  #[serde(skip_serializing_if = "std::ops::Not::not")]
  pub single: bool,
}

impl Collection {
  /// Compile matched files in parallel, validate frontmatter, optionally
  /// run JS sidecars + MDX module wrap + minify, write `{name}.json`.
  pub(crate) fn process(
    &self,
    cfg: &EngineConfig,
    diag_engine: &mut DiagnosticEngine<Code>,
  ) -> Result<CollectionReport, ()> {
    let walker = globwalk::GlobWalkerBuilder::from_patterns(&self.base_dir, &[&self.pattern]).build().map_err(|e| {
      diag_engine.emit(diag!(Code::IoRead, format!("globwalk {}: {}", self.pattern, e)));
    })?;

    let paths = walker.filter_map(|e| e.ok()).map(|e| e.path().to_path_buf()).collect::<Vec<PathBuf>>();

    let collection_schema = self.schema.as_ref().and_then(|d| {
      dmc_schema::compile_descriptor(d)
        .map_err(|e| {
          diag_engine.emit(diag!(Code::JsonDeserialize, format!("schema descriptor for `{}`: {}", self.name, e)));
        })
        .ok()
    });

    let component_schemas = self.component_schemas(diag_engine);
    let component_names: HashSet<String> = component_schemas.keys().cloned().collect();

    let cache = if cfg.cache_enabled { FileCache::open(cfg.output_dir.join(".cache").join("dmc")) } else { None };
    // the component schemas change what a record holds, so they belong in the cache key
    let cfg_fp =
      fingerprint(&(&cfg.compile, &cfg.include_html, &self.name, &self.schema, &cfg.output_format, &self.components));

    let outcomes: Vec<Option<Value>> = paths
      .par_iter()
      .map(|path| {
        let mut local_diag_engine = DiagnosticEngine::<Code>::new();

        let source = match std::fs::read_to_string(path) {
          Ok(s) => s,
          Err(e) => {
            local_diag_engine.emit(diag!(Code::IoRead, format!("read source at {}: {}", path.display(), e)));
            local_diag_engine.print_all_compact();
            return None;
          },
        };

        let cache_key = cache.as_ref().map(|_| FileCache::key(source.as_bytes(), path, &cfg_fp));
        if let (Some(c), Some(k)) = (cache.as_ref(), cache_key.as_ref())
          && let Some(hit) = c.get(k)
        {
          local_diag_engine.print_all(&source);
          return Some(hit);
        }

        let local_compiler_cfg = cfg.compile.for_render();
        let use_sidecar = cfg.compile.has_js_plugins();

        let mut compiled =
          Compiler::compile_collecting(&source, path, &local_compiler_cfg, &component_names, &mut local_diag_engine);

        if use_sidecar && let Some(html) = run_sidecar(&compiled.content, cfg) {
          compiled.html = html;
        }

        if cfg.compile.mdx_output_format.as_deref() == Some("module") {
          compiled.body = wrap_mdx_module(&compiled.body, &compiled.imports);
        }
        if cfg.compile.mdx_minify {
          compiled.body = minify_js(&compiled.body);
        }

        // Shared: a component schema may use the same path- and content-derived kinds
        // (`path`, `excerpt`, `toc`) as frontmatter, which an empty context would starve.
        let schema_ctx = build_schema_ctx(path, &cfg.root, &compiled, cfg);

        let validated_frontmatter = match (&collection_schema, &compiled.frontmatter) {
          (Some(schema), fm) if !fm.is_null() => match schema.parse(fm, &schema_ctx) {
            Ok(v) => v,
            Err(e) => {
              local_diag_engine
                .emit(diag!(Code::JsonDeserialize, format!("frontmatter validation at {}: {}", path.display(), e)));
              compiled.frontmatter.clone()
            },
          },
          _ => compiled.frontmatter.clone(),
        };

        let items = validate_components(
          std::mem::take(&mut compiled.components),
          &component_schemas,
          &schema_ctx,
          path,
          &mut local_diag_engine,
        );

        let include_html = cfg.include_html || use_sidecar;
        let mut rec =
          build_velite_record(compiled, validated_frontmatter, path, &self.base_dir, &self.name, include_html);

        if !items.is_empty()
          && let Some(obj) = rec.as_object_mut()
        {
          obj.insert("components".into(), Value::Array(items));
        }

        // Cache only clean runs so diagnostics re-fire until the source is fixed. Frontmatter YAML
        // that did not parse counts although it only warns: the warning is the one sign the page
        // lost every field, and a cache hit would skip the compile that gives it.
        let dirty = local_diag_engine.error_count() + local_diag_engine.bug_count() > 0
          || local_diag_engine.iter().any(|d| matches!(d.code, Code::InvalidFrontmatterYaml));
        if !dirty && let (Some(c), Some(k)) = (cache.as_ref(), cache_key.as_ref()) {
          c.put(k, &rec);
        }
        local_diag_engine.print_all(&source);

        Some(rec)
      })
      .collect();

    let mut records: Vec<Value> = Vec::with_capacity(outcomes.len());
    for r in outcomes.into_iter().flatten() {
      records.push(r);
    }

    let out_path = cfg.output_dir.join(format!("{}.json", self.name));
    let count = if self.single { if records.is_empty() { 0 } else { 1 } } else { records.len() };
    let json = if self.single {
      let single = records.into_iter().next().unwrap_or(Value::Null);
      serde_json::to_string_pretty(&single).unwrap()
    } else {
      serde_json::to_string_pretty(&records).unwrap()
    };

    std::fs::write(&out_path, json).map_err(|e| {
      diag_engine.emit(diag!(Code::IoWrite, format!("collection {} write at {}: {}", self.name, out_path.display(), e)))
    })?;

    Ok(CollectionReport { name: self.name.clone(), records: count, output_path: out_path })
  }
}

impl Collection {
  /// The collection's `components` map compiled into schemas, one per name.
  ///
  /// A descriptor that does not compile is reported and dropped rather than failing the build:
  /// the rest of the collection is still worth emitting, and the diagnostic names the component.
  fn component_schemas(
    &self,
    diag_engine: &mut DiagnosticEngine<Code>,
  ) -> HashMap<String, Box<dyn dmc_schema::Schema>> {
    let mut out = HashMap::new();
    let Some(Value::Object(map)) = self.components.as_ref() else {
      if let Some(v) = self.components.as_ref() {
        diag_engine.emit(diag!(
          Code::InvalidConfig,
          format!(
            "`components` for collection `{}` must be an object of name -> schema, got {}",
            self.name,
            kind_of(v)
          )
        ));
      }
      return out;
    };
    for (name, descriptor) in map {
      match dmc_schema::compile_descriptor(descriptor) {
        Ok(s) => {
          out.insert(name.clone(), s);
        },
        Err(e) => {
          diag_engine.emit(diag!(
            Code::JsonDeserialize,
            format!("component schema `{}` in collection `{}`: {}", name, self.name, e)
          ));
        },
      }
    }
    out
  }
}

fn kind_of(v: &Value) -> &'static str {
  match v {
    Value::Null => "null",
    Value::Bool(_) => "a boolean",
    Value::Number(_) => "a number",
    Value::String(_) => "a string",
    Value::Array(_) => "an array",
    Value::Object(_) => "an object",
  }
}

/// Validate each collected component against its schema.
///
/// A component that fails its schema is reported against its own line and left out of the record,
/// so a consumer never reads props that were never checked.
fn validate_components(
  found: Vec<Value>,
  schemas: &HashMap<String, Box<dyn dmc_schema::Schema>>,
  ctx: &dmc_schema::Ctx,
  path: &Path,
  diag_engine: &mut DiagnosticEngine<Code>,
) -> Vec<Value> {
  if found.is_empty() {
    return Vec::new();
  }
  let mut out = Vec::with_capacity(found.len());
  // `parent` indexes the collected list, and dropping an item shifts everything after it, so each
  // original position is mapped to where it ended up — or to None, when it did not survive.
  let mut moved: Vec<Option<usize>> = Vec::with_capacity(found.len());

  for mut item in found {
    let name = item.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
    let Some(schema) = schemas.get(&name) else {
      moved.push(None);
      continue;
    };
    let props = item.get("props").cloned().unwrap_or(Value::Null);
    let line = item.get("line").and_then(Value::as_u64).unwrap_or(0);
    match schema.parse(&props, ctx) {
      Ok(v) => {
        // the collector pushes a parent before walking its children, so this is always resolved
        let parent = item.get("parent").and_then(Value::as_u64).and_then(|i| moved.get(i as usize).copied().flatten());
        if let Some(obj) = item.as_object_mut() {
          obj.insert("props".into(), v);
          obj.insert("parent".into(), parent.map_or(Value::Null, Value::from));
        }
        moved.push(Some(out.len()));
        out.push(item);
      },
      Err(e) => {
        moved.push(None);
        diag_engine.emit(diag!(Code::JsonDeserialize, format!("<{}> at {}:{}: {}", name, path.display(), line, e)));
      },
    }
  }
  out
}
