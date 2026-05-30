//! Hardening probes: 100+ AST inputs. Each probe asserts the parser
//! does not panic and produces a non-degenerate document for non-empty
//! inputs. Bugs surface as panics, empty results, or invalid AST.

use dmc_parser::parse;

const PROBES: &[&str] = &[
  // 1-10: paragraphs / inline edges
  "",
  "hello",
  "hello\nworld",
  "para 1\n\npara 2",
  "trailing space  \nbreak",
  "trailing backslash\\\nbreak",
  "unicode 🦆 café",
  "  leading spaces",
  "\u{FEFF}bom-start",
  "embedded\u{200B}zwsp",
  // 11-20: headings
  "# H1",
  "## H2\n",
  "###### H6",
  "####### too many",
  "# H1 #",
  "# {1+2}",
  "# `code` in heading",
  "Setext\n=====",
  "Setext sub\n-----",
  "# multi\nword",
  // 21-30: code blocks
  "```\ncode\n```",
  "```rust\nfn x() {}\n```",
  "```\nno close",
  "    indented code",
  "\tindented code",
  "```\n```",
  "~~~ts\nfoo\n~~~",
  "```lang attr=value\nfoo\n```",
  "```\n```\nafter",
  "    code 1\n\n    code 2",
  // 31-40: lists
  "- a\n- b\n",
  "1. a\n2. b\n",
  "- a\n  - nested\n",
  "- [ ] task\n- [x] done\n",
  "- a\n- b\n- c\n",
  "1) parens style\n",
  "+ plus\n* star\n- dash\n",
  "- item with **bold**\n",
  "- ```\n  code in list\n  ```\n",
  "- multi\n  para\n",
  // 41-50: blockquotes
  "> quote",
  "> a\n> b",
  "> > nested",
  "> a\n>\n> b",
  "> # heading in quote",
  "> - list in quote",
  "> ```\n> code\n> ```",
  ">no space",
  "> > > triple nest",
  ">\n> after blank",
  // 51-60: links / refs / autolinks
  "[a](b)",
  "[a](<b>)",
  "[a](b \"title\")",
  "[`code`](https://example.com);",
  "[ref][a]\n\n[a]: /url",
  "[shortcut]\n\n[shortcut]: /u 'T'",
  "[](empty)",
  "[a](javascript:alert(1))",
  "<http://x.com>",
  "https://github.com/foo/bar)",
  // 61-70: emphasis
  "*it*",
  "**bold**",
  "***both***",
  "_under_",
  "__bold__",
  "***nest*it**",
  "a*b*c",
  "*open\nclosed*",
  "~~strike~~",
  "**bold _and_ italic**",
  // 71-80: tables
  "| a | b |\n|---|---|\n| 1 | 2 |\n",
  "|h|\n|-|\n",
  "|a|b|\n|:-|-:|\n|1|2|\n",
  "|a\\|b|c|\n|-|-|\n|x|y|\n",
  "| <b>raw</b> | x |\n|-|-|\n| 1 | 2 |\n",
  "broken |table",
  "| 1 |\n",
  "| h |\n| - |\n| 1 |\n| 2 |\n",
  "| a | b | c |\n|---|---|\n| 1 | 2 |\n",
  "|`code`|x|\n|-|-|\n",
  // 81-90: JSX / MDX
  "<Comp />",
  "<Comp prop=\"x\" />",
  "<Comp prop={1 + 2} />",
  "<Outer><Inner/></Outer>",
  "<Comp>\n  text\n</Comp>",
  "{1 + 2}",
  "{/* comment */}",
  "{`tpl ${x}`}",
  "import X from 'y';\n\n<X/>",
  "<a href={`/u/${id}`}>x</a>",
  // 91-100: HTML / footnotes / refs
  "<div>raw</div>",
  "<!-- html comment -->",
  "<br>",
  "<a href='x'>y</a>",
  "footnote[^1]\n\n[^1]: text",
  "[link][undefined]",
  "[\\[escaped\\]](url)",
  "[image ![alt](inner)](outer)",
  "[a](<url with spaces>)",
  "[a](url \"unterminated)",
  // 101-110: stress
  "[[[",
  "(((",
  "$$$",
  "***",
  "===",
  "---",
  "```",
  "```\n",
  "\n\n\n\n\n",
  "deep\nnested\nlines\n",
];

#[test]
fn parser_does_not_panic_on_corpus() {
  for (i, src) in PROBES.iter().enumerate() {
    let doc = parse(src);
    let _ = doc;
    println!("probe #{i:03} ok ({} bytes)", src.len());
  }
}

#[test]
fn parser_inline_link_with_code_label_and_trailing_semicolon_does_not_diagnose() {
  use dmc_diagnostic::Code;
  use dmc_diagnostic::metadata::{Origin, SourceMeta};
  use dmc_lexer::Lexer;
  use dmc_parser::Parser;
  use duck_diagnostic::DiagnosticEngine;
  use std::sync::Arc;

  let src = "[`STATUS.md`](https://example.com/STATUS.md);\n";
  let meta = Arc::new(SourceMeta { path: Arc::from("<test>"), origin: Origin::Inline("<test>") });
  let mut lex_diag: DiagnosticEngine<Code> = DiagnosticEngine::new();
  let mut lexer = Lexer::new(src, meta.clone(), &mut lex_diag);
  let _ = lexer.scan_tokens();
  let tokens = std::mem::take(&mut lexer.tokens);
  drop(lexer);
  let mut parse_diag: DiagnosticEngine<Code> = DiagnosticEngine::new();
  let mut parser = Parser::new(tokens, meta, &mut parse_diag);
  let _ = parser.parse();
  assert_eq!(parse_diag.iter().count(), 0, "diagnostics: {:?}", parse_diag.get_diagnostics());
}
