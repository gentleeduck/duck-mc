use std::path::PathBuf;
use duck_md::{run, CollectionConfig, EngineConfig};

fn tmp_workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let docs = dir.path().join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(docs.join("a.mdx"),
        "---\ntitle: A\ndescription: Alpha doc\n---\n# Alpha\n\nbody one\n").unwrap();
    std::fs::write(docs.join("b.mdx"),
        "---\ntitle: B\ndescription: Beta doc\n---\n# Beta\n\nbody two\n").unwrap();
    dir
}

fn cfg_for(out_dir: PathBuf, base: PathBuf) -> EngineConfig {
    EngineConfig {
        output_dir: out_dir,
        root: base.clone(),
        collections: vec![CollectionConfig {
            name: "docs".into(),
            pattern: "docs/**/*.mdx".into(),
            base_dir: base,
            ..Default::default()
        }],
        ..Default::default()
    }
}

#[test]
fn engine_writes_json_for_each_collection() {
    let dir = tmp_workspace();
    let out_dir = dir.path().join(".velite");
    let rep = run(&cfg_for(out_dir.clone(), dir.path().to_path_buf())).expect("run");
    assert_eq!(rep.collections.len(), 1);
    assert_eq!(rep.collections[0].records, 2);
    let json_path = out_dir.join("docs.json");
    assert!(json_path.exists());
    let content = std::fs::read_to_string(json_path).unwrap();
    assert!(content.contains("Alpha"));
    assert!(content.contains("Beta"));
    assert!(out_dir.join("index.js").exists());
    assert!(out_dir.join("index.d.ts").exists());
}

#[test]
fn engine_records_have_velite_fields() {
    let dir = tmp_workspace();
    let out_dir = dir.path().join(".velite");
    let _ = run(&cfg_for(out_dir.clone(), dir.path().to_path_buf())).unwrap();
    let json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(out_dir.join("docs.json")).unwrap()
    ).unwrap();
    let arr = json.as_array().unwrap();
    let first = &arr[0];
    for field in &[
        "body", "content", "excerpt", "metadata", "toc",
        "contentType", "flattenedPath", "permalink", "slug",
        "sourceFileDir", "sourceFileName", "sourceFilePath",
        "title", "description",
    ] {
        assert!(first.get(field).is_some(), "missing field {}: {}", field, first);
    }
    let meta = first.get("metadata").unwrap();
    assert!(meta.get("readingTime").is_some());
    assert!(meta.get("wordCount").is_some());
    for absent in &["html", "frontmatter", "frontmatterRaw", "imports", "exports"] {
        assert!(first.get(absent).is_none(), "field {} should be absent", absent);
    }
    let dts = std::fs::read_to_string(out_dir.join("index.d.ts")).unwrap();
    assert!(dts.contains("DocRecord"));
    assert!(dts.contains("export declare const docs: DocRecord[]"));
}

#[test]
fn engine_validates_frontmatter_against_schema() {
    let dir = tempfile::tempdir().unwrap();
    let docs = dir.path().join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(docs.join("ok.mdx"), "---\ntitle: Hi\n---\n# A\n").unwrap();
    std::fs::write(docs.join("bad.mdx"),
        "---\ntitle: this title is way too long for our 5-char max constraint\n---\n# B\n").unwrap();
    let out_dir = dir.path().join(".velite");

    let schema = serde_json::json!({
        "kind": "object",
        "fields": {
            "title": { "kind": "string", "max": 5 },
        }
    });

    let cfg = EngineConfig {
        output_dir: out_dir.clone(),
        root: dir.path().to_path_buf(),
        collections: vec![CollectionConfig {
            name: "docs".into(),
            pattern: "docs/**/*.mdx".into(),
            base_dir: dir.path().to_path_buf(),
            schema: Some(schema),
            ..Default::default()
        }],
        ..Default::default()
    };
    let rep = run(&cfg).unwrap();
    assert_eq!(rep.errors.len(), 1, "expected 1 schema error, got {}", rep.errors.len());
    assert!(rep.errors[0].message.contains("title"));
    assert!(rep.errors[0].message.contains("too long"));
}

#[test]
fn engine_strict_mode_fails_on_validation_error() {
    let dir = tempfile::tempdir().unwrap();
    let docs = dir.path().join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(docs.join("bad.mdx"), "---\ntitle: ok\n---\n# B\n").unwrap();
    let out_dir = dir.path().join(".velite");

    let schema = serde_json::json!({
        "kind": "object",
        "fields": {
            "title": { "kind": "number" },
        }
    });

    let cfg = EngineConfig {
        output_dir: out_dir,
        root: dir.path().to_path_buf(),
        strict: true,
        collections: vec![CollectionConfig {
            name: "docs".into(),
            pattern: "docs/**/*.mdx".into(),
            base_dir: dir.path().to_path_buf(),
            schema: Some(schema),
            ..Default::default()
        }],
        ..Default::default()
    };
    assert!(run(&cfg).is_err());
}
