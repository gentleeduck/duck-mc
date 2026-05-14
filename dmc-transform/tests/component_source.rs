use dmc_parser::ast::*;
use dmc_parser::parse;
use dmc_transform::{ComponentSource, Pipeline};

#[test]
fn component_source_injects_code_block_as_jsx_child() {
  let dir = tempfile::tempdir().unwrap();
  let path = dir.path().join("foo.tsx");
  std::fs::write(&path, "export const Foo = () => null\n").unwrap();

  let src = "<ComponentSource path=\"foo.tsx\" />\n";
  let mut doc = parse(src);
  let p = Pipeline::new().add(ComponentSource::with_base_dir(dir.path()));
  p.run_silent(&mut doc);

  let wrapper = doc
    .children
    .iter()
    .find_map(|n| match n {
      Node::JsxElement(e) if e.name == "ComponentSource" => Some(e),
      _ => None,
    })
    .expect("expected populated <ComponentSource> JsxElement");
  let cb = wrapper
    .children
    .iter()
    .find_map(|n| match n {
      Node::CodeBlock(c) => Some(c),
      _ => None,
    })
    .expect("expected CodeBlock child inside <ComponentSource>");
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
  p.run_silent(&mut doc);
  let still_jsx = doc.children.iter().any(|n| matches!(n, Node::JsxSelfClosing(_) | Node::JsxElement(_)));
  assert!(still_jsx, "missing file should preserve JSX node");
}
