use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::{compile, CompileOutput};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocRecord {
    #[serde(flatten)]
    pub compiled: CompileOutput,
    pub source_file_path: String,
    pub source_file_name: String,
    pub source_file_dir: String,
    pub flattened_path: String,
    pub permalink: String,
    pub slug: String,
    pub content_type: String,
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
    let mut records: Vec<DocRecord> = Vec::new();
    let walker = globwalk::GlobWalkerBuilder::from_patterns(&c.base_dir, &[c.pattern.as_str()])
        .build()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?
        .filter_map(|r| r.ok());
    for entry in walker {
        let path = entry.path().to_path_buf();
        if !path.is_file() { continue; }
        let source = std::fs::read_to_string(&path)?;
        let compiled = compile(&source);
        let rec = build_record(compiled, &path, &c.base_dir);
        records.push(rec);
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
    // index.js
    let mut js = String::new();
    for c in &report.collections {
        js.push_str(&format!(
            "export {{ default as {name} }} from './{name}.json' with {{ type: 'json' }}\n",
            name = c.name
        ));
    }
    std::fs::write(out_dir.join("index.js"), js)?;
    // index.d.ts (very minimal — proper types are out of scope here)
    let mut dts = String::new();
    for c in &report.collections {
        dts.push_str(&format!(
            "export declare const {name}: any[]\n",
            name = c.name
        ));
    }
    std::fs::write(out_dir.join("index.d.ts"), dts)?;
    Ok(())
}

fn build_record(compiled: CompileOutput, path: &Path, base: &Path) -> DocRecord {
    let rel = path.strip_prefix(base).unwrap_or(path);
    let rel_str = rel.to_string_lossy().to_string();
    let file_name = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let parent = path.parent().map(|p| {
        let mut comps: Vec<String> = p.components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect();
        if comps.len() >= 2 {
            let last2 = comps.split_off(comps.len() - 2);
            last2.join("/")
        } else {
            comps.join("/")
        }
    }).unwrap_or_default();

    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let permalink = rel_str.trim_end_matches(".mdx").trim_end_matches(".md").to_string();
    let slug = if permalink.is_empty() { "docs".to_string() } else { format!("docs/{}", permalink) };
    let content_type = path.extension().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();

    let _ = stem;

    DocRecord {
        compiled,
        source_file_path: path.to_string_lossy().to_string(),
        source_file_name: file_name,
        source_file_dir: parent,
        flattened_path: rel_str.trim_end_matches(".mdx").trim_end_matches(".md").to_string(),
        permalink,
        slug,
        content_type,
    }
}
