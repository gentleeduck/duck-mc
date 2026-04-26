use duck_md::{CollectionConfig, EngineConfig, compile, run};
use std::path::PathBuf;

#[test]
fn mdx_minify_preserves_string_literals() {
  use duck_md::compile;
  let out = compile("# title\n\n`code  with  spaces`\n");
  // The body string has the internal "code  with  spaces" preserved within
  // a JS string literal — minify should NOT collapse those spaces.
  let body = out.body;
  assert!(body.contains("code  with  spaces"), "internal spaces in string literal should survive");
}

#[test]
fn mdx_module_format_is_valid_esm_shape() {
  let dir = tempfile::tempdir().unwrap();
  let docs = dir.path().join("docs");
  std::fs::create_dir_all(&docs).unwrap();
  std::fs::write(docs.join("a.mdx"), "---\ntitle: A\n---\n# Hi\n").unwrap();

  let cfg = EngineConfig {
    output_dir: dir.path().join(".out"),
    root: dir.path().to_path_buf(),
    collections: vec![CollectionConfig {
      name: "doc".into(),
      pattern: "docs/**/*.mdx".into(),
      base_dir: dir.path().to_path_buf(),
      ..Default::default()
    }],
    mdx_output_format: Some("module".into()),
    ..Default::default()
  };
  let _ = run(&cfg).unwrap();
  let json: serde_json::Value =
    serde_json::from_str(&std::fs::read_to_string(dir.path().join(".out/doc.json")).unwrap())
      .unwrap();
  let body = json[0]["body"].as_str().unwrap();
  assert!(body.contains("import { Fragment as _Fragment"));
  assert!(body.contains("export default function MDXContent"));
  // arguments[0] should NOT remain in module-mode body
  assert!(!body.contains("arguments[0]"), "arguments[0] must be substituted in module mode");
  let _ = PathBuf::new();
  let _ = compile;
}

#[test]
fn record_count_accurate_for_mdx_w_literal_field_name() {
  // Regression: count was using string-grep on "sourceFilePath" — bogus when
  // mdx body literally contained that string.
  let dir = tempfile::tempdir().unwrap();
  let docs = dir.path().join("docs");
  std::fs::create_dir_all(&docs).unwrap();
  std::fs::write(
    docs.join("a.mdx"),
    "---\ntitle: A\n---\nThe field is `sourceFilePath` and `sourceFilePath` again.\n",
  )
  .unwrap();

  let cfg = EngineConfig {
    output_dir: dir.path().join(".out"),
    root: dir.path().to_path_buf(),
    collections: vec![CollectionConfig {
      name: "doc".into(),
      pattern: "docs/**/*.mdx".into(),
      base_dir: dir.path().to_path_buf(),
      ..Default::default()
    }],
    ..Default::default()
  };
  let rep = run(&cfg).unwrap();
  assert_eq!(rep.collections[0].records, 1, "count must be 1 not 3");
}
