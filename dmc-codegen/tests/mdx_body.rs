use dmc_codegen::render_mdx_body;
use dmc_parser::parse;

fn body(src: &str) -> String {
  render_mdx_body(&parse(src))
}

#[test]
fn produces_factory_function() {
  let s = body("# Hi");
  assert!(s.contains("function _createMdxContent(props)"), "got:\n{}", s);
  assert!(s.contains("jsxs(Fragment"), "got:\n{}", s);
  assert!(s.contains("return _createMdxContent(arguments[0]);"));
}

#[test]
fn heading_has_id_and_jsx() {
  let s = body("# Hello");
  assert!(s.contains("jsxs(\"h1\""), "got:\n{}", s);
  assert!(s.contains("id: \"hello\""), "got:\n{}", s);
}

#[test]
fn jsx_self_closing_renders() {
  let s = body("<Btn color=\"red\" />");
  assert!(s.contains("jsx(Btn, {"), "got:\n{}", s);
  assert!(s.contains("\"color\": \"red\""), "got:\n{}", s);
}

#[test]
fn jsx_element_with_children() {
  let s = body("<Card>hi</Card>");
  assert!(s.contains("jsxs(Card, {"), "got:\n{}", s);
}

#[test]
fn imports_appear_in_prelude() {
  let s = body("import X from 'x'\n# H");
  assert!(s.starts_with("import X from"), "got start:\n{}", &s[..40.min(s.len())]);
  assert!(s.contains("function _createMdxContent"));
}

#[test]
fn expression_passed_through() {
  let s = body("Hello {name}");
  assert!(s.contains("name"), "got:\n{}", s);
}
