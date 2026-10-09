//! End-to-end cover for a collection's `components` map: embedded JSX validated at build time
//! and emitted onto the record, so a consumer reads the interactive parts of a document as data.

use dmc::engine::{collection::Collection, compile::CompileConfig, config::EngineConfig};
use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

const LESSON: &str = r#"---
id: build-drivers
title: Build drivers
---

A builder is a BuildKit instance.

<Task id="make-a-builder" title="Create a container builder" node={1}>
  Create a builder named `bk1`.

  <Check name="builder exists" run="docker buildx inspect bk1" within={60} />
  <Check name="it is running" run="docker buildx inspect bk1" />
</Task>

<Quiz id="which-driver" multiple={false}>
  Which driver exports to a registry cache?
</Quiz>
"#;

fn schemas() -> Value {
  json!({
    "Task": { "kind": "object", "fields": {
      "id": { "kind": "string" },
      "title": { "kind": "string" },
      "node": { "kind": "default", "inner": { "kind": "number" }, "fallback": 1 }
    }},
    "Check": { "kind": "object", "fields": {
      "name": { "kind": "string" },
      "run": { "kind": "string" },
      "within": { "kind": "default", "inner": { "kind": "number" }, "fallback": 30 }
    }},
    "Quiz": { "kind": "object", "fields": {
      "id": { "kind": "string" },
      "multiple": { "kind": "default", "inner": { "kind": "boolean" }, "fallback": false }
    }}
  })
}

fn run(dir: &Path, components: Option<Value>) -> Vec<Value> {
  fs::create_dir_all(dir.join("content")).unwrap();
  fs::write(dir.join("content/lesson.mdx"), LESSON).unwrap();
  let out = dir.join(".out");

  let cfg = EngineConfig {
    root: dir.to_path_buf(),
    output_dir: out.clone(),
    clean: true,
    collections: vec![Collection {
      name: "lessons".into(),
      pattern: "content/**/*.mdx".into(),
      base_dir: dir.to_path_buf(),
      components,
      ..Default::default()
    }],
    // a cold cache every run, so a hit never hides what this test is checking
    cache_enabled: false,
    compile: CompileConfig::new(),
    ..Default::default()
  };

  let mut diag = DiagnosticEngine::<Code>::new();
  dmc::Engine::run(&cfg, None, &mut diag).expect("engine run");
  let raw = fs::read_to_string(out.join("lessons.json")).unwrap();
  serde_json::from_str(&raw).unwrap()
}

fn tmp(name: &str) -> PathBuf {
  let d = std::env::temp_dir().join(format!("dmc-components-{}-{}", name, std::process::id()));
  let _ = fs::remove_dir_all(&d);
  d
}

#[test]
fn collects_embedded_components_with_defaults_applied() {
  let dir = tmp("collect");
  let records = run(&dir, Some(schemas()));
  let items = records[0]["components"].as_array().expect("components on the record");

  let names: Vec<&str> = items.iter().map(|i| i["name"].as_str().unwrap()).collect();
  assert_eq!(names, ["Task", "Check", "Check", "Quiz"], "document order, nested ones included");

  assert_eq!(items[0]["props"]["id"], "make-a-builder");
  assert_eq!(items[0]["props"]["node"], 1, "a braced number is a number, not the text \"1\"");
  assert_eq!(items[1]["props"]["within"], 60);
  assert_eq!(items[2]["props"]["within"], 30, "the schema default fills what the body left out");
  assert_eq!(items[3]["props"]["multiple"], false);

  // a nested component points back at the one that encloses it
  assert_eq!(items[1]["parent"], 0);
  assert_eq!(items[2]["parent"], 0);
  assert!(items[0]["parent"].is_null(), "a top-level component has no parent");
  assert!(items[0]["line"].as_u64().unwrap() > 0, "carries its own line for diagnostics");

  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn no_components_config_means_no_extra_key() {
  let dir = tmp("none");
  let records = run(&dir, None);
  assert!(records[0].get("components").is_none(), "a collection that asked for nothing gets nothing");
  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_component_failing_its_schema_is_left_out() {
  let dir = tmp("invalid");
  fs::create_dir_all(dir.join("content")).unwrap();
  // `node` is a string where the schema wants a number
  fs::write(
    dir.join("content/lesson.mdx"),
    "---\nid: x\ntitle: X\n---\n\n<Task id=\"t\" title=\"T\" node=\"not-a-number\" />\n",
  )
  .unwrap();
  let out = dir.join(".out");
  let cfg = EngineConfig {
    root: dir.clone(),
    output_dir: out.clone(),
    clean: true,
    collections: vec![Collection {
      name: "lessons".into(),
      pattern: "content/**/*.mdx".into(),
      base_dir: dir.clone(),
      components: Some(schemas()),
      ..Default::default()
    }],
    cache_enabled: false,
    compile: CompileConfig::new(),
    ..Default::default()
  };
  let mut diag = DiagnosticEngine::<Code>::new();
  dmc::Engine::run(&cfg, None, &mut diag).expect("engine run");
  let records: Vec<Value> = serde_json::from_str(&fs::read_to_string(out.join("lessons.json")).unwrap()).unwrap();
  assert!(
    records[0].get("components").is_none(),
    "an unvalidated component must not reach the record; the build reports it instead"
  );
  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn parent_still_points_at_the_right_component_after_one_is_dropped() {
  // The first Task fails its schema and is withheld. Everything after it shifts down a slot, so a
  // child whose `parent` was recorded against the unfiltered list would point at the wrong one.
  let dir = tmp("remap");
  fs::create_dir_all(dir.join("content")).unwrap();
  fs::write(
    dir.join("content/lesson.mdx"),
    concat!(
      "---\nid: x\ntitle: X\n---\n\n",
      "<Task id=\"bad\" title=\"T\" node=\"not-a-number\" />\n\n",
      "<Task id=\"good\" title=\"T\">\n",
      "  <Check name=\"c1\" run=\"true\" />\n",
      "</Task>\n"
    ),
  )
  .unwrap();
  let out = dir.join(".out");
  let cfg = EngineConfig {
    root: dir.to_path_buf(),
    output_dir: out.clone(),
    clean: true,
    collections: vec![Collection {
      name: "lessons".into(),
      pattern: "content/**/*.mdx".into(),
      base_dir: dir.to_path_buf(),
      components: Some(schemas()),
      ..Default::default()
    }],
    cache_enabled: false,
    compile: CompileConfig::new(),
    ..Default::default()
  };
  let mut diag = DiagnosticEngine::<Code>::new();
  dmc::Engine::run(&cfg, None, &mut diag).expect("engine run");
  let records: Vec<Value> = serde_json::from_str(&fs::read_to_string(out.join("lessons.json")).unwrap()).unwrap();
  let items = records[0]["components"].as_array().expect("the surviving components");

  let names: Vec<&str> = items.iter().map(|i| i["name"].as_str().unwrap()).collect();
  assert_eq!(names, ["Task", "Check"], "the invalid Task is withheld");
  assert_eq!(items[0]["props"]["id"], "good");
  assert_eq!(items[1]["parent"], 0, "the Check points at the Task it is actually inside");

  let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_child_of_a_dropped_component_has_no_parent() {
  let dir = tmp("orphan");
  fs::create_dir_all(dir.join("content")).unwrap();
  fs::write(
    dir.join("content/lesson.mdx"),
    concat!(
      "---\nid: x\ntitle: X\n---\n\n",
      "<Task id=\"bad\" title=\"T\" node=\"not-a-number\">\n",
      "  <Check name=\"c1\" run=\"true\" />\n",
      "</Task>\n"
    ),
  )
  .unwrap();
  let out = dir.join(".out");
  let cfg = EngineConfig {
    root: dir.to_path_buf(),
    output_dir: out.clone(),
    clean: true,
    collections: vec![Collection {
      name: "lessons".into(),
      pattern: "content/**/*.mdx".into(),
      base_dir: dir.to_path_buf(),
      components: Some(schemas()),
      ..Default::default()
    }],
    cache_enabled: false,
    compile: CompileConfig::new(),
    ..Default::default()
  };
  let mut diag = DiagnosticEngine::<Code>::new();
  dmc::Engine::run(&cfg, None, &mut diag).expect("engine run");
  let records: Vec<Value> = serde_json::from_str(&fs::read_to_string(out.join("lessons.json")).unwrap()).unwrap();
  let items = records[0]["components"].as_array().unwrap();

  assert_eq!(items.len(), 1);
  assert_eq!(items[0]["name"], "Check");
  assert!(items[0]["parent"].is_null(), "its parent was withheld, so it reports none rather than an index");

  let _ = fs::remove_dir_all(&dir);
}
