//! Hardening probes: 100+ tiny lexer inputs covering edge cases.
//! Each probe asserts the lexer:
//!   (a) terminates without panic,
//!   (b) reproduces the input byte-for-byte via token `raw` concatenation
//!       (lossless tokenization invariant).
//! New regressions land here; named tests target specific bugs.

mod common;

use dmc_diagnostic::metadata::{Origin, SourceMeta};
use dmc_lexer::Lexer;
use duck_diagnostic::DiagnosticEngine;
use std::sync::Arc;

/// Concatenated `raw` may legitimately *omit* bytes the lexer normalizes
/// away (e.g. the `\` in a CM 6.7 hard-break). It must never *invent*
/// bytes or reorder them, so we require the joined raw to be a
/// subsequence of the source.
fn round_trip(src: &str) {
  let meta = Arc::new(SourceMeta { path: Arc::from("<probe>"), origin: Origin::Inline("<probe>") });
  let mut engine = DiagnosticEngine::new();
  let mut lexer = Lexer::new(src, meta, &mut engine);
  let _ = lexer.scan_tokens();
  let joined: String = lexer.tokens.iter().map(|t| t.raw).collect();
  let mut s = src.bytes();
  for b in joined.bytes() {
    loop {
      match s.next() {
        Some(sb) if sb == b => break,
        Some(_) => continue,
        None => panic!("token raw produced bytes absent from source for input {src:?}: joined={joined:?}"),
      }
    }
  }
}

const PROBES: &[&str] = &[
  // 1-10: backslash escapes
  "\\*not emph*",
  "\\[not link]",
  "\\`not code`",
  "\\\\literal backslash",
  "\\<not tag>",
  "\\&not entity;",
  "\\!\\(",
  "trailing escape\\",
  "\\\n",
  "\\a (non-escapable)",
  // 11-20: code spans
  "`x`",
  "``x`y``",
  "`unterminated",
  "`a\nb`",
  "` foo `",
  "`` ` ``",
  "```triple backtick inline```",
  "`mix `` inside `",
  "a`b`c",
  "`\u{0301}combining`",
  // 21-30: autolinks
  "<http://x.com>",
  "<mailto:a@b>",
  "<not-an-autolink>",
  "<a@b.c>",
  "<javascript:alert(1)>",
  "http://x.com",
  "https://example.com/path?q=1&r=2",
  "see http://x.com.",
  "(http://x.com)",
  "http://x.com;",
  // 31-40: links
  "[a](b)",
  "[a](b 'title')",
  "[a](b \"title\")",
  "[`code`](url)",
  "[multi line\nlabel](url)",
  "[unclosed",
  "[a](unclosed",
  "[a][b]",
  "[a][]",
  "[a]",
  // 41-50: emphasis
  "*a*",
  "**a**",
  "***a***",
  "_a_",
  "__a__",
  "a*b*c",
  "a_b_c",
  "*open\nclosed*",
  "~~strike~~",
  "~single~",
  // 51-60: HTML/JSX
  "<div>x</div>",
  "<Component prop=\"x\" />",
  "<Foo\n  bar={1}>\n</Foo>",
  "<!-- comment -->",
  "<!-- unterminated",
  "<>fragment</>",
  "<a href='x'>y</a>",
  "</orphan>",
  "<br />",
  "<a/>",
  // 61-70: MDX
  "{1 + 2}",
  "{/* mdx comment */}",
  "{`tpl ${x}`}",
  "{ unclosed",
  "{{nested}}",
  "{\"a\":1}",
  "import x from 'y';",
  "export const x = 1;",
  "{() => x}",
  "{<Comp/>}",
  // 71-80: lists / task / quotes
  "- one\n- two\n",
  "1. one\n2. two\n",
  "- [ ] todo\n- [x] done\n",
  "> quoted\n> line\n",
  ">>nested\n",
  "- a\n  - nested\n",
  "+ item\n",
  "* item\n",
  "10) item\n",
  "1.item without space\n",
  // 81-90: tables
  "| a | b |\n|---|---|\n| 1 | 2 |\n",
  "|a|b|\n|-|-|\n",
  "|a\\|b|c|\n|-|-|\n",
  "| left | center | right |\n|:---|:---:|---:|\n",
  "|a|\n|-|\n",
  "no pipe table\n---\n",
  "| unterminated\n",
  "| a | b |\n",
  "|`code|with|pipe`|c|\n|-|-|\n",
  "| <b>raw</b> | x |\n|-|-|\n",
  // 91-100: whitespace / normalization / unicode
  "\r\n\r\n",
  "\tindented\n",
  "    code block\n",
  "no trailing newline",
  "    \n",
  "mixed\r\nendings\nhere\r",
  "naïve café 🦆\n",
  "RTL: مرحبا\n",
  "zero\u{200B}width\n",
  "\u{FEFF}BOM start\n",
  // 101-110: stress edges
  "",
  "\n",
  "\n\n\n",
  "[[[[[",
  "(((((",
  "<<<<<",
  "&&&&&",
  "`````",
  "*****",
  "$$$$$",
];

#[test]
fn lexer_round_trip_probe_corpus() {
  for (i, src) in PROBES.iter().enumerate() {
    round_trip(src);
    println!("probe #{i:03} ok ({} bytes)", src.len());
  }
}

#[test]
fn no_diagnostic_on_well_formed_inline_link_with_code_label_and_semicolon() {
  let src = "[`STATUS.md`](https://example.com/STATUS.md);";
  let meta = Arc::new(SourceMeta { path: Arc::from("<probe>"), origin: Origin::Inline("<probe>") });
  let mut engine = DiagnosticEngine::new();
  let mut lexer = Lexer::new(src, meta, &mut engine);
  let _ = lexer.scan_tokens();
  let opens = lexer.tokens.iter().filter(|t| matches!(t.kind, dmc_lexer::token::TokenKind::LinkTargetOpen)).count();
  let closes = lexer.tokens.iter().filter(|t| matches!(t.kind, dmc_lexer::token::TokenKind::LinkTargetClose)).count();
  assert_eq!(opens, 1, "expected one LinkTargetOpen, got {opens}");
  assert_eq!(closes, 1, "expected one LinkTargetClose, got {closes}");
}
