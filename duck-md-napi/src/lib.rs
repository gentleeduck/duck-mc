#![deny(clippy::all)]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json::Value;
use std::path::PathBuf;

/// Compile a single MDX source string. Returns the full CompileOutput as a JS object:
/// { body, content, html, excerpt, metadata, toc, frontmatter, frontmatter_raw, imports, exports }
/// Field names are snake_case (matching serde Serialize on the Rust struct).
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
}

#[napi(object)]
pub struct BuildInput {
    pub output_dir: String,
    pub collections: Vec<CollectionInput>,
}

#[napi(object)]
pub struct BuildCollectionReport {
    pub name: String,
    pub records: u32,
    pub output_path: String,
}

#[napi(object)]
pub struct BuildReport {
    pub collections: Vec<BuildCollectionReport>,
}

/// Run the full engine: walk globs, compile each MDX file, write velite-shape JSON.
/// Equivalent to `duck-md build` CLI but driven by a JS object — no toml needed.
#[napi]
pub fn build(input: BuildInput) -> Result<BuildReport> {
    let cfg = duck_md::EngineConfig {
        output_dir: PathBuf::from(input.output_dir),
        collections: input
            .collections
            .into_iter()
            .map(|c| duck_md::CollectionConfig {
                name: c.name,
                pattern: c.pattern,
                base_dir: PathBuf::from(c.base_dir),
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
    })
}
