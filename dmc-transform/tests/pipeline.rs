use dmc_parser::ast::*;
use dmc_parser::parse;
use dmc_transform::{AssignHeadingIds, AutolinkHeadings, CodeImport, Pipeline};

fn anchor_href(node: &Node) -> Option<String> {
  match node {
    Node::JsxElement(e) if e.name == "a" => e.attrs.iter().find_map(|a| match (&a.name[..], &a.value) {
      ("href", JsxAttrValue::String(s)) => Some(s.clone()),
      _ => None,
    }),
    _ => None,
  }
}

#[test]
fn pipeline_runs_autolink() {
  let mut d = parse("# Hello");
  Pipeline::new().add(AssignHeadingIds::new()).add(AutolinkHeadings::new()).run_silent(&mut d);
  let h = match &d.children[0] {
    Node::Heading(h) => h,
    n => panic!("expected heading, got {:?}", n),
  };
  assert!(h.children.len() >= 2, "expected anchor + text, got {:?}", h.children);
  assert_eq!(anchor_href(&h.children[0]).as_deref(), Some("#hello"));
}

#[test]
fn idempotent() {
  let mut d = parse("# Hello");
  Pipeline::new().add(AssignHeadingIds::new()).add(AutolinkHeadings::new()).run_silent(&mut d);
  let first_pass_len = match &d.children[0] {
    Node::Heading(h) => h.children.len(),
    _ => unreachable!(),
  };
  Pipeline::new().add(AssignHeadingIds::new()).add(AutolinkHeadings::new()).run_silent(&mut d);
  let h = match &d.children[0] {
    Node::Heading(h) => h,
    _ => unreachable!(),
  };
  assert_eq!(h.children.len(), first_pass_len, "autolink should not double-prepend");
}

#[test]
fn defaults_pipeline_includes_autolink() {
  let mut d = parse("# Foo Bar");
  Pipeline::with_defaults().run_silent(&mut d);
  let h = match &d.children[0] {
    Node::Heading(h) => h,
    n => panic!("expected heading, got {:?}", n),
  };
  assert_eq!(anchor_href(h.children.first().unwrap()).as_deref(), Some("#foo-bar"));
}

#[test]
fn dedupe_assigns_unique_ids() {
  let mut d = parse("## Patch Changes\n\n## Patch Changes\n\n## Patch Changes\n");
  Pipeline::new().add(AssignHeadingIds::new()).run_silent(&mut d);
  let ids: Vec<_> = d
    .children
    .iter()
    .filter_map(|n| match n {
      Node::Heading(h) => h.id.clone(),
      _ => None,
    })
    .collect();
  assert_eq!(ids, vec!["patch-changes", "patch-changes-1", "patch-changes-2"]);
}

#[test]
fn slug_strips_dots() {
  let mut d = parse("## 0.4.3\n");
  Pipeline::new().add(AssignHeadingIds::new()).run_silent(&mut d);
  let h = match &d.children[0] {
    Node::Heading(h) => h,
    _ => unreachable!(),
  };
  assert_eq!(h.id.as_deref(), Some("043"));
}

#[test]
fn code_import_reads_file() {
  let dir = tempfile::tempdir().unwrap();
  let snippet = dir.path().join("snippet.ts");
  std::fs::write(&snippet, "export const x = 1\n").unwrap();
  let src = "```ts file=\"snippet.ts\"\nplaceholder\n```\n".to_string();
  let mut d = dmc_parser::parse(&src);
  dmc_transform::Pipeline::new().add(CodeImport::with_base_dir(dir.path().to_path_buf())).run_silent(&mut d);
  let cb = d
    .children
    .iter()
    .find_map(|n| match n {
      Node::CodeBlock(cb) => Some(cb),
      _ => None,
    })
    .expect("cb");
  assert!(cb.value.contains("export const x = 1"), "got {:?}", cb.value);
}
