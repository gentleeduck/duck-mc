use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use crate::compile;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CollectionConfig {
    pub name: String,
    pub pattern: String,
    pub base_dir: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<Value>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub single: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EngineConfig {
    pub collections: Vec<CollectionConfig>,
    pub output_dir: PathBuf,
    #[serde(default)]
    pub root: PathBuf,
    #[serde(default)]
    pub strict: bool,
    #[serde(default)]
    pub clean: bool,
    #[serde(default)]
    pub output_assets: Option<PathBuf>,
    #[serde(default)]
    pub output_base: Option<String>,
    #[serde(default)]
    pub output_name: Option<String>,
}

#[derive(Debug, Default)]
pub struct EngineReport {
    pub collections: Vec<CollectionReport>,
    pub errors: Vec<EngineError>,
}

#[derive(Debug)]
pub struct EngineError {
    pub file: PathBuf,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct CollectionReport {
    pub name: String,
    pub records: usize,
    pub output_path: PathBuf,
}

pub fn run(cfg: &EngineConfig) -> std::io::Result<EngineReport> {
    if cfg.clean && cfg.output_dir.exists() {
        std::fs::remove_dir_all(&cfg.output_dir)?;
    }
    std::fs::create_dir_all(&cfg.output_dir)?;
    let mut report = EngineReport::default();
    for c in &cfg.collections {
        let r = process_collection(c, cfg, &mut report.errors)?;
        report.collections.push(r);
    }
    write_index(&cfg.output_dir, &report)?;
    if cfg.strict && !report.errors.is_empty() {
        let first = &report.errors[0];
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("validation failed in strict mode: {}: {}", first.file.display(), first.message),
        ));
    }
    Ok(report)
}

fn process_collection(
    c: &CollectionConfig,
    cfg: &EngineConfig,
    errors: &mut Vec<EngineError>,
) -> std::io::Result<CollectionReport> {
    let mut records: Vec<Value> = Vec::new();
    let walker = globwalk::GlobWalkerBuilder::from_patterns(&c.base_dir, &[c.pattern.as_str()])
        .build()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?
        .filter_map(|r| r.ok());

    let collection_schema = c.schema.as_ref()
        .and_then(|d| duck_md_schema::compile_descriptor(d).map_err(|e| {
            errors.push(EngineError {
                file: PathBuf::from(format!("<schema for {}>", c.name)),
                message: e,
            });
        }).ok());

    for entry in walker {
        let path = entry.path().to_path_buf();
        if !path.is_file() { continue; }
        let source = std::fs::read_to_string(&path)?;
        let compiled = compile(&source);

        let validated_frontmatter = match (&collection_schema, &compiled.frontmatter) {
            (Some(schema), fm) if !fm.is_null() => {
                let ctx = build_schema_ctx(&path, &cfg.root, &compiled, cfg);
                match schema.parse(fm, &ctx) {
                    Ok(v) => v,
                    Err(e) => {
                        errors.push(EngineError {
                            file: path.clone(),
                            message: e.to_string(),
                        });
                        compiled.frontmatter.clone()
                    }
                }
            }
            _ => compiled.frontmatter.clone(),
        };

        records.push(build_velite_record(
            compiled,
            validated_frontmatter,
            &path,
            &c.base_dir,
            &c.name,
        ));
    }

    let out_path = cfg.output_dir.join(format!("{}.json", c.name));
    let json = if c.single {
        let single = records.into_iter().next().unwrap_or(Value::Null);
        serde_json::to_string_pretty(&single).unwrap()
    } else {
        serde_json::to_string_pretty(&records).unwrap()
    };
    let count = if c.single { 1 } else { json.matches("\"sourceFilePath\"").count() };
    std::fs::write(&out_path, json)?;
    Ok(CollectionReport {
        name: c.name.clone(),
        records: count,
        output_path: out_path,
    })
}

fn build_schema_ctx(
    path: &Path,
    root: &Path,
    compiled: &crate::CompileOutput,
    cfg: &EngineConfig,
) -> duck_md_schema::Ctx {
    let mut ctx = duck_md_schema::Ctx::new(
        path.to_path_buf(),
        root.to_path_buf(),
        compiled.content.clone(),
    );
    ctx.html = Some(compiled.html.clone());
    ctx.mdx_body = Some(compiled.body.clone());
    ctx.toc = Some(serde_json::to_value(&compiled.toc).unwrap_or(Value::Array(vec![])));
    ctx.plain_text = Some(compiled.excerpt.clone());
    if let (Some(dir), Some(base)) = (&cfg.output_assets, &cfg.output_base) {
        let mut p = duck_md_schema::AssetPipeline::new(dir.clone(), base.clone());
        if let Some(t) = &cfg.output_name { p.name_template = t.clone(); }
        ctx.assets = Some(p);
    }
    ctx
}

fn write_index(out_dir: &Path, report: &EngineReport) -> std::io::Result<()> {
    let mut js = String::new();
    for c in &report.collections {
        js.push_str(&format!(
            "export {{ default as {name} }} from './{name}.json' with {{ type: 'json' }}\n",
            name = c.name
        ));
    }
    std::fs::write(out_dir.join("index.js"), js)?;

    let mut dts = String::from(
        "export interface TocItem { title: string; url: string; items: TocItem[] }\n\
         export interface Metadata { readingTime: number; wordCount: number }\n\
         export interface DocRecord {\n\
         \u{20}\u{20}body: string\n\
         \u{20}\u{20}content: string\n\
         \u{20}\u{20}excerpt: string\n\
         \u{20}\u{20}metadata: Metadata\n\
         \u{20}\u{20}toc: TocItem[]\n\
         \u{20}\u{20}contentType: string\n\
         \u{20}\u{20}flattenedPath: string\n\
         \u{20}\u{20}permalink: string\n\
         \u{20}\u{20}slug: string\n\
         \u{20}\u{20}sourceFileDir: string\n\
         \u{20}\u{20}sourceFileName: string\n\
         \u{20}\u{20}sourceFilePath: string\n\
         \u{20}\u{20}[frontmatterField: string]: unknown\n\
         }\n",
    );
    for c in &report.collections {
        dts.push_str(&format!(
            "export declare const {name}: DocRecord[]\n",
            name = c.name
        ));
    }
    std::fs::write(out_dir.join("index.d.ts"), dts)?;
    Ok(())
}

fn build_velite_record(
    compiled: crate::CompileOutput,
    frontmatter: Value,
    path: &Path,
    base: &Path,
    collection: &str,
) -> Value {
    let rel = path.strip_prefix(base).unwrap_or(path);
    let rel_str = rel.to_string_lossy().to_string();
    let source_file_path = path.to_string_lossy().to_string();
    let source_file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let source_file_dir = path
        .parent()
        .map(|p| {
            let mut comps: Vec<String> = p
                .components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect();
            if comps.len() >= 2 {
                let last2 = comps.split_off(comps.len() - 2);
                last2.join("/")
            } else {
                comps.join("/")
            }
        })
        .unwrap_or_default();
    let content_type = path
        .extension()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let permalink = velite_permalink(&source_file_path, &rel_str, collection);
    let flattened_path = permalink.clone();
    let slug = if permalink.is_empty() {
        collection.to_lowercase()
    } else {
        format!("{}/{}", collection.to_lowercase(), permalink)
    };

    let mut map: Map<String, Value> = Map::new();
    if let Value::Object(fm) = frontmatter {
        for (k, v) in fm { map.insert(k, v); }
    }

    map.insert("body".into(), Value::String(compiled.body));
    map.insert("content".into(), Value::String(compiled.content));
    map.insert("excerpt".into(), Value::String(compiled.excerpt));
    map.insert("metadata".into(), serde_json::to_value(&compiled.metadata).unwrap_or(json!({})));
    map.insert("toc".into(), serde_json::to_value(&compiled.toc).unwrap_or(Value::Array(vec![])));
    map.insert("contentType".into(), Value::String(content_type));
    map.insert("flattenedPath".into(), Value::String(flattened_path));
    map.insert("permalink".into(), Value::String(permalink));
    map.insert("slug".into(), Value::String(slug));
    map.insert("sourceFileDir".into(), Value::String(source_file_dir));
    map.insert("sourceFileName".into(), Value::String(source_file_name));
    map.insert("sourceFilePath".into(), Value::String(source_file_path));

    Value::Object(map)
}

// matches velite: `path.replace(/^.*<collection_dir>\//, '').replace(/\.mdx?$/, '')`
// where <collection_dir> defaults to the lowercased collection name.
fn velite_permalink(abs: &str, rel: &str, collection: &str) -> String {
    let lc = collection.to_lowercase();
    let needle = format!("/{lc}/");
    let after = if let Some(idx) = abs.rfind(&needle) {
        &abs[idx + needle.len()..]
    } else {
        rel
    };
    after
        .trim_end_matches(".mdx")
        .trim_end_matches(".md")
        .to_string()
}
