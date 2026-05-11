use dmc_codegen::render_mdx_body;
use dmc_parser::parse;

fn body(src: &str) -> String {
  render_mdx_body(&parse(src))
}

#[test]
fn produces_factory_function() {
  let s = body("# Hi");
  assert!(s.contains("function _createMdxContent(props)"), "got:\n{}", s);
  // Runtime destructure lives at module scope so `_createMdxContent`
  // closes over Fragment/jsx/jsxs even when called as a React component.
  assert!(s.contains("const { Fragment, jsx, jsxs } = arguments[0];"), "got:\n{}", s);
  assert!(s.contains("function _createMdxContent(props)"));
  let destructure_at = s.find("const { Fragment, jsx, jsxs } = arguments[0];").unwrap();
  let function_at = s.find("function _createMdxContent").unwrap();
  assert!(destructure_at < function_at, "destructure must precede function decl:\n{}", s);
  assert!(s.contains("jsx(Fragment,") || s.contains("jsxs(Fragment,"), "got:\n{}", s);
  assert!(s.contains("return { default: _createMdxContent };"));
}

#[test]
fn heading_has_id_and_jsx() {
  let s = body("# Hello");
  // Intrinsic tags route through a static `_components` defaults map so
  // consumer overrides via `props.components` win without per-call fallbacks.
  assert!(s.contains("_components.h1, { id: \"hello\""), "got:\n{}", s);
  assert!(s.contains("h1: \"h1\""), "missing default tag entry:\n{}", s);
}

#[test]
fn jsx_self_closing_renders() {
  let s = body("<Btn color=\"red\" />");
  // Capital JSX names destructure off `_components` and validate up front
  // via `_missingMdxReference`.
  assert!(s.contains("const { Btn } = _components;"), "got:\n{}", s);
  assert!(s.contains("if (!Btn) _missingMdxReference(\"Btn\");"), "got:\n{}", s);
  assert!(s.contains("jsx(Btn, { color: \"red\" })"), "got:\n{}", s);
}

#[test]
fn jsx_element_with_children() {
  let s = body("<Card>hi</Card>");
  assert!(s.contains("const { Card } = _components;"), "got:\n{}", s);
  assert!(s.contains("if (!Card) _missingMdxReference(\"Card\");"), "got:\n{}", s);
  assert!(s.contains("jsx(Card,") || s.contains("jsxs(Card,"), "got:\n{}", s);
}

#[test]
fn imports_dropped_from_function_body_output() {
  // The compiled body is consumed via `new Function(body)(runtime)`,
  // which cannot legally contain top-level `import` / `export`. dmc
  // strips them from the prelude (we still record them in the AST so
  // a future module-output mode can re-emit; the `function-body`
  // path drops them). Body must start with the runtime destructure,
  // not with `import`.
  let s = body("import X from 'x'\n# H");
  assert!(!s.contains("import X from 'x'"), "import leaked into body:\n{}", s);
  assert!(s.starts_with("const { Fragment, jsx, jsxs }"), "got start:\n{}", &s[..60.min(s.len())]);
  assert!(s.contains("function _createMdxContent"));
}

#[test]
fn expression_passed_through() {
  let s = body("Hello {name}");
  assert!(s.contains("name"), "got:\n{}", s);
}

/// Regression: a raw-HTML inline span (`Node::Html`) sitting between
/// text nodes inside a JSX-element body must NOT leak a stack frame
/// in the body emitter. Before the fix, `Node::Html` fell through to
/// the default `_ => open_frame` arm in `enter()`, but `close_frame`
/// short-circuited via `is_container` (Html is not a container) and
/// never popped the frame -- so every following sibling, and every
/// enclosing JSX element, drained into a dropped frame and disappeared
/// from the output. The production trigger was an `<AccordionContent>`
/// body containing an inline `<code className="...">x</code>` -- the
/// entire `<Accordion>` subtree silently vanished from the body.
#[test]
fn inline_raw_html_does_not_drop_enclosing_jsx_element() {
  // UNINDENTED -- mirrors the production preMdx pipeline output.
  let src = "<Acc>\n<Item>\nNo. <code className=\"x\">react</code>, more.\n</Item>\n</Acc>\n";
  let s = body(src);
  // The enclosing JSX call sites must be present.
  assert!(s.contains("jsx(Acc,") || s.contains("jsxs(Acc,"), "no `jsx(Acc,` in body:\n{}", s);
  assert!(s.contains("jsx(Item,") || s.contains("jsxs(Item,"), "no `jsx(Item,` in body:\n{}", s);
  // The inline raw HTML is emitted as a `dangerouslySetInnerHTML` div.
  assert!(s.contains("dangerouslySetInnerHTML"), "raw HTML not emitted:\n{}", s);
  // The trailing text after `</code>` survives.
  assert!(s.contains("\"react\""), "text inside <code> dropped:\n{}", s);
  assert!(s.contains("\"more.\""), "trailing text dropped:\n{}", s);
}
