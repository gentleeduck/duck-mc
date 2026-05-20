use dmc_codegen::{MdxBodyEmitter, RenderOptions, render_mdx_body};
use dmc_parser::parse;

fn body(src: &str) -> String {
  render_mdx_body(&parse(src))
}

/// Render with raw-HTML passthrough enabled (CommonMark "unsafe" mode).
fn body_unsafe(src: &str) -> String {
  MdxBodyEmitter::render_with(&parse(src), RenderOptions { allow_dangerous_html: true, ..Default::default() }).0
}

#[test]
fn produces_factory_function() {
  let s = body("# Hi");
  assert!(s.contains("function _createMdxContent(props)"), "got:\n{}", s);
  // Module-scope destructure so the fn closes over the runtime.
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
  assert!(s.contains("_components.h1, { id: \"hello\""), "got:\n{}", s);
  assert!(s.contains("h1: \"h1\""), "missing default tag entry:\n{}", s);
}

#[test]
fn jsx_self_closing_renders() {
  let s = body("<Btn color=\"red\" />");
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
  // `new Function(body)` cannot contain top-level `import`/`export`;
  // body must start with the runtime destructure.
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

/// Raw-HTML inline span inside a JSX element body must not leak a frame
/// in the emitter (would otherwise silently drop every following sibling
/// and ancestor).
#[test]
fn inline_raw_html_does_not_drop_enclosing_jsx_element() {
  // `allow_dangerous_html: true` so the raw inline `<code>` HTML is
  // emitted; the regression this guards (dropped sibling/ancestor
  // frames) is independent of the safe/unsafe mode.
  let src = "<Acc>\n<Item>\nNo. <code className=\"x\">react</code>, more.\n</Item>\n</Acc>\n";
  let s = body_unsafe(src);
  assert!(s.contains("jsx(Acc,") || s.contains("jsxs(Acc,"), "no `jsx(Acc,` in body:\n{}", s);
  assert!(s.contains("jsx(Item,") || s.contains("jsxs(Item,"), "no `jsx(Item,` in body:\n{}", s);
  assert!(s.contains("dangerouslySetInnerHTML"), "raw HTML not emitted:\n{}", s);
  assert!(s.contains("\"react\""), "text inside <code> dropped:\n{}", s);
  assert!(s.contains("more."), "trailing text dropped:\n{}", s);
}

/// SEC-010: in safe mode (default) raw HTML must NOT compile to a live
/// `dangerouslySetInnerHTML`. Block-level raw HTML is omitted; inline raw
/// HTML is escaped to visible text.
#[test]
fn raw_html_not_emitted_as_dangerously_set_inner_html_by_default() {
  let s = body("<script>alert(1)</script>");
  assert!(!s.contains("dangerouslySetInnerHTML"), "raw HTML leaked as live HTML in safe mode:\n{}", s);
  assert!(!s.contains("<script>"), "raw <script> leaked verbatim:\n{}", s);
}

/// SEC-010: inline raw HTML in safe mode is escaped to a text node, not
/// dropped and not live.
#[test]
fn inline_raw_html_escaped_to_text_in_safe_mode() {
  let s = body("see <b>this</b> here");
  assert!(!s.contains("dangerouslySetInnerHTML"), "inline raw HTML leaked as live HTML:\n{}", s);
  // The `<b>` markup survives as an escaped string literal (visible text).
  assert!(s.contains("<b>"), "inline raw HTML text dropped:\n{}", s);
  assert!(s.contains("see") && s.contains("here"), "surrounding text dropped:\n{}", s);
}

/// SEC-010: explicit opt-in still produces `dangerouslySetInnerHTML`.
#[test]
fn raw_html_emitted_when_allow_dangerous_html() {
  let s = body_unsafe("<div>raw</div>");
  assert!(s.contains("dangerouslySetInnerHTML"), "opt-in raw HTML not emitted:\n{}", s);
}

#[test]
fn classed_div_with_component_children_compiles_to_nested_jsx() {
  let s = body(
    "\
<div className=\"mt-8 grid gap-4 sm:grid-cols-2\">
  <LinkedCard href=\"/a\">
    <svg viewBox=\"0 0 24 24\" className=\"h-10 w-10\" fill=\"currentColor\">
      <title>Next.js</title>
      <path d=\"M11\" />
    </svg>
    <p className=\"mt-2 font-medium\">Next.js</p>
  </LinkedCard>
</div>
",
  );
  assert!(s.contains("_components.div, { className: \"mt-8 grid gap-4 sm:grid-cols-2\""), "got:\n{}", s);
  assert!(!s.contains("dangerouslySetInnerHTML"), "should not fall back to raw HTML:\n{}", s);
  assert!(s.contains("const { LinkedCard } = _components;"), "got:\n{}", s);
  assert!(s.contains("LinkedCard, { href: \"/a\""), "got:\n{}", s);
  assert!(s.contains("_components.svg, { viewBox: \"0 0 24 24\""), "got:\n{}", s);
  assert!(s.contains("_components.path, { d: \"M11\" }"), "got:\n{}", s);
  assert!(s.contains("_components.p, { className: \"mt-2 font-medium\", children: \"Next.js\" }"), "got:\n{}", s);
  assert!(!s.contains("children: [\"  \""), "stray indentation text leaked:\n{}", s);
}
