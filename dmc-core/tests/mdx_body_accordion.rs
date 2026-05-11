//! Regression: a column-0 (UNINDENTED) JSX block --
//! `<Accordion> / <AccordionItem> / <AccordionContent>` -- whose
//! `<AccordionContent>` body contains a paragraph with an inline
//! lowercase HTML span (`<code className="...">x</code>`) must emit
//! every enclosing JSX call site in the MDX body. Previously the
//! `MdxBodyEmitter` had no explicit arm for `Node::Html` in `enter()`;
//! the node fell into the default `_ => open_frame` branch, but
//! `leave -> close_frame` short-circuited via `is_container` and never
//! popped the frame. The leaked frame swallowed every following
//! sibling's expression, and eventually the entire `<Accordion>`
//! subtree never made it into the root frame's children list.

use dmc::engine::compile::{CompileConfig, Compiler};
use duck_diagnostic::DiagnosticEngine;
use std::path::Path;

fn compile_body(src: &str) -> String {
  let mut diag = DiagnosticEngine::new();
  Compiler::compile_with_pipeline(src, Path::new("<test>"), &CompileConfig::default(), &mut diag).body
}

/// Count occurrences of an actual `jsx(<Name>,` / `jsxs(<Name>,` call
/// site in the emitted body. The component name appearing only in
/// `_missingMdxReference("X")` or in the `const { X } = _components`
/// destructure header does NOT count.
fn count_calls(body: &str, name: &str) -> usize {
  let mut total = 0usize;
  for needle in [&format!("jsx({}, ", name), &format!("jsxs({}, ", name)] {
    let mut from = 0usize;
    while let Some(idx) = body[from..].find(needle) {
      total += 1;
      from += idx + needle.len();
    }
  }
  total
}

#[test]
fn unindented_accordion_with_inline_code_keeps_jsx_call_sites() {
  let mdx = "# H\n\n<Accordion type=\"multiple\" collapsible className=\"w-full\">\n<AccordionItem value=\"x\">\n<AccordionTrigger>Q?</AccordionTrigger>\n\n<AccordionContent className=\"text-muted-foreground\">\nNo. <code className=\"rounded bg-muted px-2 py-1\">react</code>, more text.\n</AccordionContent>\n</AccordionItem>\n</Accordion>\n";
  let body = compile_body(mdx);
  assert!(count_calls(&body, "Accordion") >= 1, "no `jsx(Accordion,` in MDX body:\n{}", body);
  assert!(count_calls(&body, "AccordionItem") >= 1, "no `jsx(AccordionItem,` in MDX body:\n{}", body);
  assert!(count_calls(&body, "AccordionTrigger") >= 1, "no `jsx(AccordionTrigger,` in MDX body:\n{}", body);
  assert!(count_calls(&body, "AccordionContent") >= 1, "no `jsx(AccordionContent,` in MDX body:\n{}", body);
}
