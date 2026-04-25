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

#[napi]
pub fn compile_many(sources: Vec<String>) -> Result<Vec<Value>> {
    sources.into_iter()
        .map(|s| {
            let out = duck_md::compile(&s);
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
    pub theme_light: Option<String>,
    pub theme_dark: Option<String>,
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
        markdown_remark_plugins: input.markdown_remark_plugins,
        markdown_rehype_plugins: input.markdown_rehype_plugins,
        mdx_remark_plugins: input.mdx_remark_plugins,
        mdx_rehype_plugins: input.mdx_rehype_plugins,
        copy_linked_files: input.copy_linked_files.unwrap_or(false),
        mdx_output_format: input.mdx_output_format,
        mdx_minify: input.mdx_minify.unwrap_or(false),
        markdown_gfm: input.markdown_gfm.unwrap_or(true),
        include_html: input.include_html.unwrap_or(false),
        theme_light: input.theme_light,
        theme_dark: input.theme_dark,
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
