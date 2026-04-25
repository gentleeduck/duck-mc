use duck_md_parser::ast::*;
use duck_md_parser::parse;
use duck_md_transform::{ComponentSource, Pipeline, Transformer};

#[test]
fn component_source_replaces_jsx_with_code_block() {
  let dir = tempfile::tempdir().unwrap();
  let path = dir.path().join("foo.tsx");
  std::fs::write(&path, "export const Foo = () => null\n").unwrap();

  let src = "<ComponentSource path=\"foo.tsx\" />\n";
  let mut doc = parse(src);
  let p = Pipeline::new().add(ComponentSource::with_base_dir(dir.path()));
  p.run(&mut doc);

  let cb = doc.children.iter().find_map(|n| match n {
    Node::CodeBlock(c) => Some(c),
    _ => None,
  }).expect("expected CodeBlock after ComponentSource transform");
  assert_eq!(cb.lang.as_deref(), Some("tsx"));
  assert!(cb.value.contains("export const Foo"));
  assert!(cb.meta.as_ref().is_some_and(|m| m.contains("foo.tsx")));
}

#[test]
fn component_source_no_op_for_missing_file() {
  let dir = tempfile::tempdir().unwrap();
  let src = "<ComponentSource path=\"missing.tsx\" />\n";
  let mut doc = parse(src);
  let p = Pipeline::new().add(ComponentSource::with_base_dir(dir.path()));
  p.run(&mut doc);
  // original JSX node preserved when file not found
  let still_jsx = doc.children.iter().any(|n| matches!(n, Node::JsxSelfClosing(_) | Node::JsxElement(_)));
  assert!(still_jsx, "missing file should preserve JSX node");
}
