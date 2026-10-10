//! A frontmatter block YAML cannot parse used to vanish without a word: the page compiled with no
//! frontmatter, so its record lost every field, and nothing said why. It now warns with PW002 at the
//! line YAML stopped on, and stays out of the cache so the warning comes back until it is fixed.

use dmc::engine::{
  collection::Collection,
  compile::{CompileConfig, Compiler},
  config::EngineConfig,
};
use dmc_diagnostic::Code;
use duck_diagnostic::{DiagnosticEngine, Severity};
use serde_json::Value;
use std::fs;
use std::path::Path;

// An unquoted `: ` inside a value: the shape that cost 25 duck-ui pages their titles.
const BROKEN: &str = "---\ntitle: Broken\ndescription: a: b\n---\n\n# Hi\n";

fn compile(source: &str) -> (Value, DiagnosticEngine<Code>) {
  let mut diag = DiagnosticEngine::<Code>::new();
  let out = Compiler::compile_with_pipeline(source, Path::new("docs/page.mdx"), &CompileConfig::new(), &mut diag);
  (out.frontmatter, diag)
}

#[test]
fn yaml_that_does_not_parse_warns_at_the_line_it_stopped_on() {
  let (frontmatter, diag) = compile(BROKEN);
  assert!(frontmatter.is_null(), "the page still compiles, with no frontmatter");

  let found: Vec<_> = diag.iter().filter(|d| matches!(d.code, Code::InvalidFrontmatterYaml)).collect();
  assert_eq!(found.len(), 1, "{:#?}", diag.iter().collect::<Vec<_>>());
  let d = found[0];
  assert_eq!(d.severity, Severity::Warning);
  assert!(d.message.contains("mapping values are not allowed"), "carries YAML's reason: {}", d.message);
  assert!(
    !d.message.contains(" at line "),
    "YAML counts lines inside the block; the label says where in the file: {}",
    d.message
  );

  let span = &d.labels[0].span;
  assert_eq!(&*span.file, "docs/page.mdx");
  assert_eq!((span.line, span.column), (3, 15), "line 3 is `description: a: b`, column 15 its second `:`");
}

#[test]
fn frontmatter_that_parses_or_is_absent_does_not_warn() {
  for source in ["---\ntitle: Fine\ndescription: \"a: b\"\n---\n\n# Hi\n", "---\ntitle: x\n---\n", "# No frontmatter\n"]
  {
    let (_, diag) = compile(source);
    assert!(diag.iter().all(|d| !matches!(d.code, Code::InvalidFrontmatterYaml)), "{source:?}");
  }
}

#[test]
fn a_page_whose_yaml_did_not_parse_is_not_cached() {
  // A cache hit skips the compile that warns, so a cached page would warn once and then never.
  let dir = std::env::temp_dir().join(format!("dmc-frontmatter-yaml-{}", std::process::id()));
  let _ = fs::remove_dir_all(&dir);
  fs::create_dir_all(dir.join("content")).unwrap();
  fs::write(dir.join("content/broken.mdx"), BROKEN).unwrap();
  fs::write(dir.join("content/fine.mdx"), "---\ntitle: Fine\n---\n\n# Hi\n").unwrap();
  let out = dir.join(".out");
  let cfg = EngineConfig {
    root: dir.clone(),
    output_dir: out.clone(),
    clean: true,
    collections: vec![Collection {
      name: "docs".into(),
      pattern: "content/**/*.mdx".into(),
      base_dir: dir.clone(),
      ..Default::default()
    }],
    cache_enabled: true,
    compile: CompileConfig::new(),
    ..Default::default()
  };

  dmc::Engine::run(&cfg, None, &mut DiagnosticEngine::<Code>::new()).expect("engine run");
  let records: Vec<Value> = serde_json::from_str(&fs::read_to_string(out.join("docs.json")).unwrap()).unwrap();
  assert_eq!(records.len(), 2, "the broken page is still built");
  let cached = fs::read_dir(out.join(".cache/dmc")).unwrap().count();
  assert_eq!(cached, 1, "only fine.mdx is cached");

  let _ = fs::remove_dir_all(&dir);
}
