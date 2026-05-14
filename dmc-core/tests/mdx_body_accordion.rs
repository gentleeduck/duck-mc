//! Column-0 JSX block with an inline raw-HTML span inside its body
//! must still emit every enclosing JSX call site.

use dmc::engine::compile::{CompileConfig, Compiler};
use duck_diagnostic::DiagnosticEngine;
use std::path::Path;

fn compile_body(src: &str) -> String {
  let mut diag = DiagnosticEngine::new();
  Compiler::compile_with_pipeline(src, Path::new("<test>"), &CompileConfig::default(), &mut diag).body
}

/// Count actual `jsx(<Name>,` / `jsxs(<Name>,` call sites only
/// (skips `_missingMdxReference` and the destructure header).
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
