#![deny(clippy::all)]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::Value;
use std::path::PathBuf;

#[napi]
pub fn compile(source: String) -> Result<Value> {
    let out = duck_md::compile(&source);
    serde_json::to_value(&out).map_err(|e| Error::from_reason(e.to_string()))
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
}

#[napi(object)]
pub struct BuildCollectionReport {
    pub name: String,
    pub records: u32,
    pub output_path: String,
}

#[napi(object)]
pub struct BuildErrorReport {
    pub file: String,
    pub message: String,
}

#[napi(object)]
pub struct BuildReport {
    pub collections: Vec<BuildCollectionReport>,
    pub errors: Vec<BuildErrorReport>,
}

#[napi]
pub fn build(input: BuildInput) -> Result<BuildReport> {
    let cfg = duck_md::EngineConfig {
        output_dir: PathBuf::from(input.output_dir),
        root: PathBuf::from(input.root.unwrap_or_else(|| ".".into())),
        strict: input.strict.unwrap_or(false),
        clean: input.clean.unwrap_or(false),
        output_assets: input.output_assets.map(PathBuf::from),
        output_base: input.output_base,
        output_name: input.output_name,
        output_format: input.output_format,
        collections: input
            .collections
            .into_iter()
            .map(|c| duck_md::CollectionConfig {
                name: c.name,
                pattern: c.pattern,
                base_dir: PathBuf::from(c.base_dir),
                schema: c.schema,
                single: c.single.unwrap_or(false),
            })
            .collect(),
    };
    let rep = duck_md::run(&cfg).map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(BuildReport {
        collections: rep
            .collections
            .into_iter()
            .map(|c| BuildCollectionReport {
                name: c.name,
                records: c.records as u32,
                output_path: c.output_path.to_string_lossy().to_string(),
            })
            .collect(),
        errors: rep
            .errors
            .into_iter()
            .map(|e| BuildErrorReport {
                file: e.file.to_string_lossy().to_string(),
                message: e.message,
            })
            .collect(),
    })
}
