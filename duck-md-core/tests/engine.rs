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

#[test]
fn engine_writes_json_for_each_collection() {
    let dir = tmp_workspace();
    let out_dir = dir.path().join(".velite");
    let cfg = EngineConfig {
        output_dir: out_dir.clone(),
        collections: vec![CollectionConfig {
            name: "docs".into(),
            pattern: "docs/**/*.mdx".into(),
            base_dir: dir.path().to_path_buf(),
        }],
    };
    let rep = run(&cfg).expect("run");
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
    let cfg = EngineConfig {
        output_dir: out_dir.clone(),
        collections: vec![CollectionConfig {
            name: "docs".into(),
            pattern: "docs/**/*.mdx".into(),
            base_dir: dir.path().to_path_buf(),
        }],
    };
    let _ = run(&cfg).unwrap();
    let json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(out_dir.join("docs.json")).unwrap()
    ).unwrap();
    let arr = json.as_array().unwrap();
    let first = &arr[0];
    // velite-shape fields (camelCase, frontmatter hoisted)
    for field in &[
        "body", "content", "excerpt", "metadata", "toc",
        "contentType", "flattenedPath", "permalink", "slug",
        "sourceFileDir", "sourceFileName", "sourceFilePath",
        // hoisted from frontmatter
        "title", "description",
    ] {
        assert!(first.get(field).is_some(), "missing field {}: {}", field, first);
    }
    // metadata sub-shape
    let meta = first.get("metadata").unwrap();
    assert!(meta.get("readingTime").is_some());
    assert!(meta.get("wordCount").is_some());
    // velite drops these — make sure we did too
    for absent in &["html", "frontmatter", "frontmatterRaw", "imports", "exports"] {
        assert!(first.get(absent).is_none(), "field {} should be absent (velite parity)", absent);
    }
    // index.d.ts includes the typed export
    let dts = std::fs::read_to_string(out_dir.join("index.d.ts")).unwrap();
    assert!(dts.contains("DocRecord"));
    assert!(dts.contains("export declare const docs: DocRecord[]"));
}
