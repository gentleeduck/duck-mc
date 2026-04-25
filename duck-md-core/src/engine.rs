use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use crate::compile;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionConfig {
    pub name: String,
    pub pattern: String,
    pub base_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub collections: Vec<CollectionConfig>,
    pub output_dir: PathBuf,
}

#[derive(Debug, Default)]
pub struct EngineReport {
    pub collections: Vec<CollectionReport>,
}

#[derive(Debug, Default)]
pub struct CollectionReport {
    pub name: String,
    pub records: usize,
    pub output_path: PathBuf,
}

pub fn run(cfg: &EngineConfig) -> std::io::Result<EngineReport> {
    std::fs::create_dir_all(&cfg.output_dir)?;
    let mut report = EngineReport::default();
    for c in &cfg.collections {
        let r = process_collection(c, &cfg.output_dir)?;
        report.collections.push(r);
    }
    write_index(&cfg.output_dir, &report)?;
    Ok(report)
}

fn process_collection(c: &CollectionConfig, out_dir: &Path) -> std::io::Result<CollectionReport> {
    let mut records: Vec<Value> = Vec::new();
    let walker = globwalk::GlobWalkerBuilder::from_patterns(&c.base_dir, &[c.pattern.as_str()])
        .build()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?
        .filter_map(|r| r.ok());
    for entry in walker {
        let path = entry.path().to_path_buf();
        if !path.is_file() { continue; }
        let source = std::fs::read_to_string(&path)?;
        let compiled = compile(&source);
        records.push(build_velite_record(compiled, &path, &c.base_dir, &c.name));
    }
    let out_path = out_dir.join(format!("{}.json", c.name));
    let json = serde_json::to_string_pretty(&records).unwrap();
    std::fs::write(&out_path, json)?;
    Ok(CollectionReport {
        name: c.name.clone(),
        records: records.len(),
        output_path: out_path,
    })
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

    // Minimal but useful: shape the type after the velite schema we emit.
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

/// Velite-parity record: camelCase keys, frontmatter hoisted to top level,
/// `html`/`imports`/`exports`/`frontmatterRaw` dropped (not part of velite output).
fn build_velite_record(
    compiled: crate::CompileOutput,
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
        collection.to_string()
    } else {
        format!("{}/{}", collection, permalink)
    };

    // Hoist frontmatter object keys onto record root.
    let mut map: Map<String, Value> = Map::new();
    if let Value::Object(fm) = compiled.frontmatter {
        for (k, v) in fm {
            map.insert(k, v);
        }
    }

    let metadata = json!({
        "readingTime": compiled.metadata.reading_time,
        "wordCount": compiled.metadata.word_count,
    });

    map.insert("body".into(), Value::String(compiled.body));
    map.insert("content".into(), Value::String(compiled.content));
    map.insert("excerpt".into(), Value::String(compiled.excerpt));
    map.insert("metadata".into(), metadata);
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

/// Velite's permalink formula: `path.replace(/^.*docs\//, '').replace(/\.mdx?$/, '')`.
/// We reproduce that, then fall back to base-relative path stripping if `docs/` isn't found.
fn velite_permalink(abs: &str, rel: &str, collection: &str) -> String {
    let needle = format!("{}/", collection);
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
