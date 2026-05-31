use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::{Origin, SourceMeta};
use dmc_lexer::Lexer;
use dmc_parser::Parser;
use duck_diagnostic::{DiagnosticCode, DiagnosticEngine};
use pretty_assertions::assert_eq;
use std::sync::Arc;

fn parse_with_diagnostics(src: &str) -> DiagnosticEngine<Code> {
  let meta = Arc::new(SourceMeta { path: Arc::from("<test>"), origin: Origin::Inline("<test>") });
  let mut lex_diag = DiagnosticEngine::new();
  let mut lexer = Lexer::new(src, meta.clone(), &mut lex_diag);
  let _ = lexer.scan_tokens();
  let tokens = std::mem::take(&mut lexer.tokens);
  drop(lexer);

  let mut parse_diag = DiagnosticEngine::new();
  let mut parser = Parser::new(tokens, meta, &mut parse_diag);
  let _ = parser.parse();
  parse_diag
}

#[test]
fn malformed_link_emits_one_diagnostic() {
  let diag = parse_with_diagnostics("[text](\n");
  assert_eq!(diag.iter().count(), 1);
  assert_eq!(diag.get_diagnostics()[0].code.code(), "P001");
}

#[test]
fn unterminated_fence_emits_one_diagnostic() {
  let diag = parse_with_diagnostics("```\n");
  assert_eq!(diag.iter().count(), 1);
  assert_eq!(diag.get_diagnostics()[0].code.code(), "P004");
}

#[test]
fn orphan_close_tag_emits_one_diagnostic() {
  let diag = parse_with_diagnostics("</div>\n");
  assert_eq!(diag.iter().count(), 1);
  assert_eq!(diag.get_diagnostics()[0].code.code(), "P010");
}

#[test]
fn unterminated_jsx_attr_emits_one_diagnostic() {
  let diag = parse_with_diagnostics("<Foo bar=\n");
  assert_eq!(diag.iter().count(), 1);
  assert_eq!(diag.get_diagnostics()[0].code.code(), "P013");
}

#[test]
fn diagnostic_output_snapshots_are_stable() {
  let link = parse_with_diagnostics("[text](\n").format_all_plain("[text](\n");
  assert!(link.contains(
    "error: [P001]: inline link destination did not close before the end of the line; treating it as literal text"
  ));
  assert!(link.contains("--> <test>:1:1"));
  assert!(
    link.contains("= help: add a closing `)` to finish `[text](...)`, or escape the `[` if this should stay literal")
  );

  let fence = parse_with_diagnostics("```\n").format_all_plain("```\n");
  assert!(fence.contains(
    "error: [P004]: fenced code block never found a matching closing fence; treating the rest of the file as code"
  ));
  assert!(fence.contains("--> <test>:1:1"));
  assert!(fence.contains("= help: add a closing fence with at least 3 ``` characters"));

  let jsx = parse_with_diagnostics("<Foo bar=\n").format_all_plain("<Foo bar=\n");
  assert!(jsx.contains(
    "error: [P013]: JSX attribute `bar` is missing a value before the tag ended; preserving the text literally"
  ));
  assert!(jsx.contains("--> <test>:1:6"));
  assert!(jsx.contains("= help: add a quoted string, `{expression}`, or remove the trailing `=`"));
}
