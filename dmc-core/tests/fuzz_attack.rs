//! Adversarial inputs run end-to-end. Each fail surfaces a missing-
//! handling bug. As bugs are fixed, inputs stay here as regressions.

use dmc::engine::compile::Compiler;
use dmc_diagnostic::Code;
use duck_diagnostic::{DiagnosticCode, DiagnosticEngine};

fn compile_default(src: &str) -> String {
  let mut diag: DiagnosticEngine<Code> = DiagnosticEngine::new();
  let out = Compiler::compile(src, &mut diag);
  out.html
}

/// Extract substrings between literal `<` and the next `>` (skipping
/// any inner `<...>` boundary). These are the actual HTML tag bodies
/// the browser will parse. Escaped text never produces a `<` byte and
/// is therefore inert.
fn raw_tag_bodies(html: &str) -> Vec<String> {
  let mut out = Vec::new();
  let bytes = html.as_bytes();
  let mut i = 0;
  while i < bytes.len() {
    if bytes[i] == b'<' {
      let start = i + 1;
      let mut j = start;
      while j < bytes.len() && bytes[j] != b'>' && bytes[j] != b'<' {
        j += 1;
      }
      if j < bytes.len() && bytes[j] == b'>' {
        out.push(html[start..j].to_ascii_lowercase());
        i = j + 1;
        continue;
      }
    }
    i += 1;
  }
  out
}

fn assert_no_xss(src: &str) {
  let html = compile_default(src);
  let tags = raw_tag_bodies(&html);
  for t in &tags {
    let name = t.split_ascii_whitespace().next().unwrap_or("");
    let name = name.trim_start_matches('/');
    for bad in ["script", "iframe", "object", "embed", "svg"] {
      assert!(name != bad, "raw <{bad}> tag in safe HTML for src={src:?}\n  html={html}");
    }
    for needle in [
      "href=\"javascript:",
      "href='javascript:",
      "src=\"javascript:",
      "src='javascript:",
      "href=\"data:text/html",
      "src=\"data:text/html",
      "href=\"vbscript:",
    ] {
      assert!(!t.contains(needle), "dangerous attr {needle:?} in tag <{t}> for src={src:?}\n  html={html}");
    }
    for handler in ["onerror=", "onload=", "onclick=", "onmouseover=", "onfocus="] {
      // Only count handler if it sits at an attribute boundary (preceded by ws or tag-name char).
      if let Some(pos) = t.find(handler) {
        let ok_boundary = pos == 0 || t.as_bytes()[pos - 1].is_ascii_whitespace();
        assert!(!ok_boundary, "inline event handler {handler} in tag <{t}> for src={src:?}\n  html={html}");
      }
    }
  }
}

const NO_PANIC: &[&str] = &[
  // Round 1: empty / whitespace / EOF
  "",
  " ",
  "\n",
  "\r",
  "\r\n",
  "\t",
  "\n\n\n\n\n",
  "    ",
  "\u{0000}",
  "\u{FFFD}",
  // Round 2: backslash escapes
  "\\",
  "\\\\",
  "\\\\\\",
  "\\*not emph*",
  "\\[\\]",
  "\\<\\>",
  "\\&amp;",
  "trailing\\",
  "trailing\\\n",
  "\\` not code `",
  // Round 3: code spans
  "`",
  "``",
  "```",
  "````",
  "`````",
  "`x``y`",
  "``x```y``",
  "`unbalanced",
  "`a\nb\nc`",
  "` ` ` `",
  // Round 4: emphasis insanity
  "*",
  "**",
  "***",
  "****",
  "*a",
  "*a**b*c**",
  "_a__b___c_",
  "***a*b**c***",
  "*_*_*_*_*",
  "**_*nest*_**",
  // Round 5: links - all malformed shapes
  "[",
  "]",
  "[]",
  "()",
  "[](",
  "[](x",
  "[a](b",
  "[a](b\\)",
  "[a](b(c)d)",
  "[a](b)c)",
  "[\\[escaped\\]](x)",
  "[![alt](inner)](outer)",
  "[a][b][c][d]",
  "[a]: \n\n[a]",
  "[a](b 'unterminated",
  // Round 6: autolinks attack
  "<>",
  "<x>",
  "<javascript:alert(1)>",
  "<data:text/html,xxx>",
  "<http://>",
  "http://",
  "https://",
  "www.",
  "www.a",
  "see http://x.com),.;:!?\n",
  // Round 7: tables
  "|",
  "||",
  "|a|",
  "|a|\n|-|",
  "|a|b|\n|-|",
  "|a|b|\n|-|-|-|",
  "| a |\n| - |\n|     |\n",
  "|`code|with|pipe`|\n|-|\n",
  "|a\\|b|c|\n|-|-|\n",
  "| a |\n|---|\n| ![im](x) |\n",
  // Round 8: lists
  "-",
  "- ",
  "-a",
  "*",
  "* ",
  "+",
  "1.",
  "1)",
  "999999999999999999. item",
  "- - - - - - - -",
  // Round 9: blockquotes
  ">",
  ">>",
  ">>>",
  "> > > >",
  ">a",
  "> ",
  "> # \n> > \n> ```\n",
  "> | a |\n> |---|\n> | 1 |\n",
  "> - item\n> \n> after",
  "> ```\n> code\n>",
  // Round 10: headings
  "#",
  "##",
  "#######",
  "# ",
  "#h",
  "# h #",
  "# h ##\n",
  "# h \\#\n",
  "Setext\n=",
  "Setext\n===",
  // Round 11: code blocks
  "```",
  "```\n",
  "```\n```",
  "```rust",
  "```\nfoo\n``",
  "~~~",
  "~~~\n~~~",
  "    indented",
  "    a\n        b\n",
  "  ```\n  code\n  ```\n",
  // Round 12: HTML
  "<",
  "<>",
  "<a>",
  "<a/>",
  "<a></a>",
  "<a></b>",
  "<a><b></a></b>",
  "<a href>x</a>",
  "<a href=>x</a>",
  "<a href=x onerror=alert(1)>",
  // Round 13: JSX
  "<X/>",
  "<X />",
  "<X></X>",
  "<X attr={1}/>",
  "<X attr={`tpl${x}`}/>",
  "<X {...spread}/>",
  "<X\n  attr={1}\n  attr2={2}\n/>",
  "<X>{`text`}</X>",
  "<X><Y><Z/></Y></X>",
  "<X><Y></Y></X></X>",
  // Round 14: MDX expressions
  "{}",
  "{ }",
  "{x}",
  "{() => x}",
  "{`a${`b${c}`}d`}",
  "{x => x + 1}",
  "{({a, b}) => a + b}",
  "{\n  let x = 1\n  return x\n}",
  "{/* unterminated",
  "{/*}*/}",
  // Round 15: frontmatter
  "---\ntitle: a\n---\n",
  "---\n---\n",
  "---\n",
  "---\nbad: [unclosed\n---\n",
  "+++\ntitle = 'a'\n+++\n",
  "---\ntitle: |\n  multi\n  line\n---\n",
  "---\nempty:\n---\n",
  "---\n{json}\n---\n",
  "---\ntitle: \"with \\\"quotes\\\"\"\n---\n",
  // Round 16: footnotes
  "[^]",
  "[^a]",
  "[^a]\n\n[^a]: text",
  "[^a]: orphan def",
  "[^a]\n\n[^a]: text\n  more",
  "[^a][^a]\n[^a]: text",
  "[^1]\n\n[^1]: text [^2]\n\n[^2]: nested",
  "[^a]: \n",
  "[^]: x",
  "[^a] [^b] [^c]\n[^a]: 1\n[^b]: 2\n[^c]: 3",
  // Round 17: image edge
  "![]()",
  "![alt]()",
  "![](x.png)",
  "![alt with [ in it](x)",
  "![alt](x.png \"title with \\\"esc\\\"\")",
  "![](\"x\")",
  "![alt](<url with spaces.png>)",
  "![nest ![inner](inner)](outer)",
  // Round 18: math
  "$",
  "$$",
  "$$$$",
  "$x$",
  "$$x$$",
  "$x\ny$",
  "$$\nblock\n$$",
  "$a_b^c$",
  "$\\\\\\\\$",
  "$$\n\\frac{a}{b}\n$$",
  // Round 19: combinations
  "**[link](url)**",
  "*[link](url)*",
  "[**bold link**](url)",
  "[`code link`](url)",
  "[![image link](i.png)](url)",
  "> [link](url)\n",
  "- [task](url)\n",
  "| [link](url) |\n|-|\n",
  "# [link](url)\n",
  "```\n[not link](url)\n```\n",
  // Round 20: unicode tortures
  "\u{200B}\u{200C}\u{200D}\u{FEFF}",
  "\u{2028}",
  "\u{2029}",
  "\u{0301}combining",
  "العربية\n",
  "中文测试",
  "🦆🌟✨🎉",
  "ﬁﬂligatures",
  "ｆｕｌｌｗｉｄｔｈ",
  "𝕌𝕟𝕚𝕔𝕠𝕕𝕖",
  // Round 21: HTML entities
  "&",
  "&amp;",
  "&amp",
  "&#",
  "&#;",
  "&#x;",
  "&#999999999;",
  "&unknown;",
  "&amp;&lt;&gt;",
  "&#65;&#x41;",
];

#[test]
fn compile_does_not_panic_on_adversarial_corpus() {
  for (i, s) in NO_PANIC.iter().enumerate() {
    let _ = compile_default(s);
    if i % 25 == 0 {
      println!("no_panic #{i:04}");
    }
  }
}

#[test]
fn pathological_repeat_inputs_terminate() {
  let cases: &[String] = &[
    "[".repeat(100),
    "(".repeat(100),
    "<".repeat(100),
    "`".repeat(100),
    "*".repeat(100),
    "#".repeat(100),
    ">".repeat(100),
    "- ".repeat(100),
    "a\n".repeat(1000),
    "a*b".repeat(100),
    "[a]".repeat(50),
    "**".repeat(200),
    "```\n".repeat(50),
  ];
  for (i, s) in cases.iter().enumerate() {
    let _ = compile_default(s);
    println!("pathological #{i:02} ok ({} bytes)", s.len());
  }
}

const XSS_INPUTS: &[&str] = &[
  "[x](javascript:alert(1))",
  "[x](JAVASCRIPT:alert(1))",
  "[x](\tjavascript:alert(1))",
  "[x](java\u{0000}script:alert(1))",
  "[x](java&#x09;script:alert(1))",
  "![x](javascript:alert(1))",
  "<a href=\"javascript:alert(1)\">x</a>",
  "<a href=\"  javascript:alert(1)  \">x</a>",
  "<a href='javascript:alert(1)'>x</a>",
  "<img src=\"x\" onerror=\"alert(1)\">",
  "<svg onload=alert(1)>",
  "<script>alert(1)</script>",
  "<iframe srcdoc=\"<script>alert(1)</script>\">",
  "<object data=\"javascript:alert(1)\">",
  "<embed src=\"javascript:alert(1)\">",
  "<a href={`javascript:${x}`}>x</a>",
  "[x](data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==)",
  "[x](vbscript:msgbox)",
  "[<img src=x onerror=alert(1)>](url)",
  "<a href=\"javascript&colon;alert(1)\">x</a>",
];

#[test]
fn xss_corpus_is_neutralized_in_default_safe_mode() {
  for (i, s) in XSS_INPUTS.iter().enumerate() {
    assert_no_xss(s);
    println!("xss-block #{i:02} ok");
  }
}

/// Compile is deterministic: identical input must produce identical
/// output. A diff between runs would point to global mutable state or
/// hash-iteration nondeterminism leaking into HTML.
#[test]
fn compile_is_deterministic() {
  for s in NO_PANIC.iter().chain(XSS_INPUTS.iter()) {
    let a = compile_default(s);
    let b = compile_default(s);
    assert_eq!(a, b, "nondeterministic compile for src={s:?}");
  }
}

const ADVERSARIAL_EXTRA: &[&str] = &[
  // Round 22: smart quotes / typographic
  "\u{201C}quoted\u{201D}",
  "\u{2018}apos\u{2019}",
  "\u{2014}em dash\u{2014}",
  "\u{2026}ellipsis",
  // Round 23: blocks bumping into fences
  "para\n```\ncode\n```\npara\n",
  "- a\n```\ncode in list?\n```\n",
  "> ```\n> code in quote\n> ```\n",
  "# heading\n```\nimmediate fence\n```\n",
  // Round 24: setext after various
  "para\n=====",
  "*emph*\n=====",
  "- list\n=====",
  "> quote\n=====",
  // Round 25: refs edge labels
  "[ ][]\n\n[ ]: /url",
  "[*emph*][a]\n\n[a]: /url",
  "[\\[esc\\]][a]\n\n[a]: /url",
  "[A][a]\n\n[a]: /url 'T'",
  "[a]\n\n[A]: /url",
  "[a][]\n[a]: /url",
  // Round 26: hard breaks in emphasis
  "*a\\\nb*",
  "*a  \nb*",
  "**a\\\nb**",
  "[a\\\nb](url)",
  "[a  \nb](url)",
  // Round 27: fence info strings
  "```rust , no_run\ncode\n```\n",
  "```{.cpp .highlight}\ncode\n```\n",
  "```` with ``` inside ````\n",
  "```rust\u{200B}\ncode\n```\n",
  // Round 28: entities in URLs
  "[a](http://x?q=&amp;r=1)",
  "[a](http://x?q=&#62;)",
  "[a](http://x#&amp;)",
  // Round 29: CR-only line endings
  "para\ranother\rline",
  "# H\r\nbody",
  "- a\r- b\r",
  // Round 30: tabs and indent edge
  "\tcode\n\tline2\n",
  "  \tmixed indent\n",
  "-\tlist marker tab\n",
  ">\tquote tab\n",
  // Round 31: link in different containers
  "1. [link](url)\n2. [other](url2)\n",
  "- - [nested link](url)\n",
  "> > [deep quote link](url)\n",
  // Round 32: empty constructs in containers
  "- \n- \n- \n",
  "> \n> \n> \n",
  "```\n\n```\n",
  // Round 33: footnote edge in containers
  "- foot[^1]\n\n[^1]: text\n",
  "> foot[^a]\n\n[^a]: text\n",
  "| f[^x] |\n|-|\n\n[^x]: text\n",
  // Round 34: math edge
  "$\\frac{a}{b}$",
  "$$\n\\begin{align}\nx &= y \\\\\nz &= w\n\\end{align}\n$$",
  "$x$ and $y$ in one line",
  "${not math because no closing",
];

#[test]
fn extra_adversarial_corpus_does_not_panic() {
  for (i, s) in ADVERSARIAL_EXTRA.iter().enumerate() {
    let _ = compile_default(s);
    if i % 10 == 0 {
      println!("extra #{i:03}");
    }
  }
}

/// Output of safe-mode compile must itself be valid HTML in a minimal
/// sense: balanced quotes inside tag bodies. A leaking quote could let
/// downstream embedding break out of an attribute.
/// Random ASCII printable bytes mixed with markdown delimiters. The
/// alphabet skews toward chars known to confuse markdown parsers. A
/// failure (panic, hang) here means a state machine forgot a recovery
/// path.
fn lcg(seed: &mut u64) -> u32 {
  *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
  (*seed >> 33) as u32
}

#[test]
fn random_byte_fuzz_does_not_panic() {
  let alphabet: &[u8] = b" \n\t\r#*_`~![](){}<>/\\|+-=&;:'\"@.%abcXYZ012_";
  let mut seed: u64 = 0xD0CD_EADB_EEF0u64;
  for trial in 0..5000 {
    let len = (lcg(&mut seed) as usize % 256) + 1;
    let mut s = String::with_capacity(len);
    for _ in 0..len {
      let idx = (lcg(&mut seed) as usize) % alphabet.len();
      s.push(alphabet[idx] as char);
    }
    let _ = compile_default(&s);
    if trial % 500 == 0 {
      println!("random #{trial:04}");
    }
  }
}

/// Print + escape all single ASCII characters as paragraph-only input.
/// Should never crash and the output should not have unmatched raw `<`.
#[test]
fn every_ascii_byte_paragraph_compiles() {
  for b in 0u8..=127 {
    let s = format!("plain {} text", b as char);
    let html = compile_default(&s);
    let opens = html.matches('<').count();
    let closes = html.matches('>').count();
    assert_eq!(opens, closes, "unmatched < / > in html for byte {b:#x}: {html}");
  }
}

/// Random unicode codepoints (excluding surrogates) sprinkled in. Picks
/// from BMP + common supplementary planes via mod arithmetic; rejects
/// invalid char ranges by retrying.
#[test]
fn random_unicode_fuzz_does_not_panic() {
  let mut seed: u64 = 0xCAFEF00Du64;
  let scaffold = ["[%]( )", "**%**", "`%`", "# %", "> %", "- %", "<a>%</a>", "{%}", "|%|\n|-|\n"];
  for trial in 0..2000 {
    let mut s = String::new();
    let frame = scaffold[(lcg(&mut seed) as usize) % scaffold.len()];
    let mut chars = String::new();
    for _ in 0..((lcg(&mut seed) as usize % 16) + 1) {
      // Random codepoint up to 0x2FFFF, retry on invalid.
      let mut tries = 0;
      let c = loop {
        let cp = lcg(&mut seed) % 0x2FFFF;
        if let Some(c) = char::from_u32(cp) {
          break c;
        }
        tries += 1;
        if tries > 8 {
          break ' ';
        }
      };
      chars.push(c);
    }
    s.push_str(&frame.replace('%', &chars));
    let _ = compile_default(&s);
    if trial % 250 == 0 {
      println!("unicode #{trial:04}");
    }
  }
}

/// Deep nesting probes - recursion limits. The parser has a label-
/// nesting cap, but other constructs (blockquotes, JSX, expressions)
/// should also terminate without stack overflow.
/// More obscure adversarial inputs collected from real markdown-parser
/// CVEs and known bug reports across cmark / pulldown-cmark / mdx.
const CVE_CORPUS: &[&str] = &[
  // GFM emphasis flanking with adjacent punctuation (CVE-2017-9445 cmark fmt)
  "*emphasis*.text",
  "*emph(asis)*",
  "**bold[link](url)**",
  // CVE-2020-15240 - pulldown-cmark hang on nested footnotes
  "[^a]: see [^a]\n[^a]",
  "[^x]: refers to [^x]",
  // mdjs / mdx unterminated expression on EOF
  "{",
  "{`",
  "{`${",
  "{((",
  // HTML element name with unusual case
  "<MyComponent/>",
  "<MY-WEBCOMPONENT/>",
  "<a-1/>",
  // Trailing whitespace inside attribute name
  "<a href  =  \"x\">y</a>",
  "<a\thref=\"x\">y</a>",
  "<a\nhref=\"x\">y</a>",
  // Markdown-it issue #1131: list item with code block
  "1. para\n\n   ```js\n   code\n   ```\n",
  // pulldown-cmark: empty title
  "[a](url \"\")",
  "[a](url '')",
  // cmark-gfm: tasklist with empty body
  "- [ ]",
  "- [x]",
  // remark-mdx: ESM at column != 0
  " import x from 'y';",
  "\timport x from 'y';",
  // Footnote ref pointing at non-existent
  "see [^nope]",
  // Math near MDX expression
  "$x = {y}$",
  "{`$x$`}",
  // Backtick + tilde in info-string
  "```~~~\ncode\n```\n",
  "~~~```\ncode\n~~~\n",
  // Lazy continuation after blockquote
  "> a\nb",
  "> a\n>b",
  // Setext underline of multi-line para
  "a\nb\nc\n===\n",
  // Reference def directly in blockquote
  "> [a]: /url\n\n[a]\n",
  // Linked image with title
  "[![alt](img.png 'imgtitle')](url 'linktitle')",
  // Emphasis crossing inline code boundary
  "*open `code closes*` here",
  // HTML comment inside markdown
  "para <!--inline comment--> end",
  "<!--\nmulti\nline\ncomment\n-->",
  // CDATA section
  "<![CDATA[\nliteral & < > <\n]]>",
  // Processing instruction
  "<?xml version=\"1.0\"?>",
  // HTML declaration
  "<!DOCTYPE html>",
];

#[test]
fn cve_corpus_does_not_panic() {
  for (i, s) in CVE_CORPUS.iter().enumerate() {
    let _ = compile_default(s);
    if i % 10 == 0 {
      println!("cve #{i:03}");
    }
  }
}

/// Specifically attack integer counters by sending huge runs of a
/// single delimiter. Debug builds panic on overflow, so a passing test
/// here proves there are no `u8` / `u16` add / sub / mul overflows on
/// the hot path.
/// Combine large delimiter runs with other constructs so the runs cross
/// container boundaries (blockquote prefix, list, table cell). Mixed
/// containers historically trip overflow + state-machine bugs that a
/// single-construct probe misses.
#[test]
fn mixed_long_runs_in_containers_do_not_panic() {
  let widths = [50, 200, 500];
  for w in widths {
    let body = "*".repeat(w);
    // In paragraph after blockquote.
    let s = format!("> {body} text {body}\n");
    let _ = compile_default(&s);
    // In list item.
    let s = format!("- {body} text {body}\n");
    let _ = compile_default(&s);
    // In table cell.
    let s = format!("| {body} | {body} |\n|---|---|\n| {body} | {body} |\n");
    let _ = compile_default(&s);
    // In heading.
    let s = format!("# {body} title {body}\n");
    let _ = compile_default(&s);
    // In link label.
    let s = format!("[{body}](url)\n");
    let _ = compile_default(&s);
    // In code span.
    let s = format!("`{body}`\n");
    let _ = compile_default(&s);
  }
}

#[test]
fn long_delimiter_runs_do_not_overflow() {
  let widths = [10, 100, 255, 256, 500, 1000, 4096];
  let delims = ['*', '_', '~', '`', '#', '>', '-', '+', '=', '<'];
  for &w in &widths {
    for &c in &delims {
      let s = c.to_string().repeat(w);
      let _ = compile_default(&s);
    }
    // Open + content + close runs of equal length.
    for &c in &delims {
      let mut s = c.to_string().repeat(w);
      s.push_str("text");
      s.push_str(&c.to_string().repeat(w));
      let _ = compile_default(&s);
    }
  }
}

#[test]
fn deep_nesting_does_not_stack_overflow() {
  let mut s = String::new();
  // 500 nested blockquote prefixes (worst case for block dispatch).
  for _ in 0..500 {
    s.push_str("> ");
  }
  s.push_str("x\n");
  let _ = compile_default(&s);

  // 500 nested unordered list items.
  let mut s = String::new();
  for i in 0..500 {
    for _ in 0..i {
      s.push(' ');
    }
    s.push_str("- a\n");
  }
  let _ = compile_default(&s);

  // 200 nested JSX self-closing tags (parsed once but written nested).
  let mut s = String::new();
  for _ in 0..200 {
    s.push_str("<X>");
  }
  for _ in 0..200 {
    s.push_str("</X>");
  }
  let _ = compile_default(&s);

  // 200 nested mdx expression braces.
  let mut s = String::new();
  for _ in 0..200 {
    s.push('{');
  }
  s.push('x');
  for _ in 0..200 {
    s.push('}');
  }
  let _ = compile_default(&s);

  // Deep emphasis.
  let mut s = String::new();
  for _ in 0..200 {
    s.push('*');
  }
  s.push_str("text");
  for _ in 0..200 {
    s.push('*');
  }
  let _ = compile_default(&s);

  // Deep link nest (limited by MAX_LINK_LABEL_DEPTH but must not crash).
  let mut s = String::new();
  for _ in 0..100 {
    s.push('[');
  }
  s.push('x');
  for _ in 0..100 {
    s.push_str("](u)");
  }
  let _ = compile_default(&s);
}

/// Take a real MDX file, mutate bytes randomly (insert / delete /
/// replace), compile each mutant. Mutation fuzz traditionally catches
/// state machines that survive on valid input but trip on near-valid.
#[test]
fn mutation_fuzz_on_real_file_does_not_panic() {
  let candidates = [
    "../@duck-ui/apps/duck/content/docs/duck-auth/introduction.mdx",
    "../../@duck-ui/apps/duck/content/docs/duck-auth/introduction.mdx",
  ];
  let path = candidates.iter().find(|p| std::path::Path::new(p).exists());
  let Some(path) = path else {
    println!("seed file not found (skipped)");
    return;
  };
  let seed = std::fs::read_to_string(path).expect("read seed");
  let seed_bytes = seed.as_bytes();
  let mut rng: u64 = 0xBEEFCAFE_DEADBEEFu64;
  let alphabet: &[u8] = b" \n#*_`~[](){}<>|+-=&;:'\"\\@abcXYZ012";
  for mutant_i in 0..200 {
    // Build mutant: copy seed, then apply 1-5 random ops.
    let mut buf: Vec<u8> = seed_bytes.to_vec();
    let ops = (lcg(&mut rng) % 5) + 1;
    for _ in 0..ops {
      let op = lcg(&mut rng) % 3;
      let pos = if buf.is_empty() { 0 } else { (lcg(&mut rng) as usize) % buf.len() };
      match op {
        0 if !buf.is_empty() => {
          buf.remove(pos);
        },
        1 => {
          let c = alphabet[(lcg(&mut rng) as usize) % alphabet.len()];
          buf.insert(pos, c);
        },
        _ if !buf.is_empty() => {
          buf[pos] = alphabet[(lcg(&mut rng) as usize) % alphabet.len()];
        },
        _ => {},
      }
    }
    let Ok(s) = std::str::from_utf8(&buf) else {
      continue;
    };
    let _ = compile_default(s);
    if mutant_i % 25 == 0 {
      println!("mutant #{mutant_i:04}");
    }
  }
}

/// Many threads compile different inputs concurrently. Catches global
/// mutable state without locking (e.g. lazy-init races).
#[test]
fn parallel_compile_does_not_panic() {
  use std::thread;
  let inputs: Vec<&str> = NO_PANIC.iter().chain(XSS_INPUTS.iter()).copied().collect();
  let chunks: Vec<Vec<&str>> = inputs.chunks(20).map(|c| c.to_vec()).collect();
  let handles: Vec<_> = chunks
    .into_iter()
    .map(|chunk| {
      thread::spawn(move || {
        for s in chunk {
          let _ = compile_default(s);
        }
      })
    })
    .collect();
  for h in handles {
    h.join().expect("thread panicked");
  }
}

/// Walks every `.mdx` and `.md` file in the sibling docs corpus (if
/// it exists) and compiles each. A missing-handling bug in any real
/// document fails here. Set `DMC_FUZZ_DOCS=0` to skip in CI.
#[test]
fn real_docs_corpus_compiles_without_panic() {
  if std::env::var("DMC_FUZZ_DOCS").as_deref() == Ok("0") {
    return;
  }
  let candidates = ["../@duck-ui/apps/duck/content/docs", "../../@duck-ui/apps/duck/content/docs"];
  let root = candidates.iter().map(std::path::Path::new).find(|p| p.exists());
  let Some(root) = root else {
    println!("docs corpus not found (skipped)");
    return;
  };
  let mut count = 0;
  let mut diag_total = 0usize;
  let mut stack = vec![root.to_path_buf()];
  while let Some(dir) = stack.pop() {
    let Ok(rd) = std::fs::read_dir(&dir) else {
      continue;
    };
    for entry in rd.flatten() {
      let p = entry.path();
      if p.is_dir() {
        stack.push(p);
        continue;
      }
      let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
      if ext != "mdx" && ext != "md" {
        continue;
      }
      let Ok(src) = std::fs::read_to_string(&p) else {
        continue;
      };
      let mut diag: DiagnosticEngine<Code> = DiagnosticEngine::new();
      let _ = Compiler::compile(&src, &mut diag);
      let n = diag.iter().count();
      if n > 0 {
        println!("DIAG: {} ({n} diags)", p.display());
        for d in diag.get_diagnostics() {
          println!("  - {}: {}", d.code.code(), d.message);
        }
      }
      diag_total += n;
      count += 1;
    }
  }
  println!("compiled {count} real files with {diag_total} diagnostics total");
  assert!(count > 0, "expected to find some .mdx files");
}

#[test]
fn safe_html_tags_have_balanced_attribute_quotes() {
  for src in NO_PANIC.iter().chain(XSS_INPUTS.iter()).chain(ADVERSARIAL_EXTRA.iter()) {
    let html = compile_default(src);
    for tag in raw_tag_bodies(&html) {
      let dq = tag.matches('"').count();
      let sq = tag.matches('\'').count();
      assert!(dq.is_multiple_of(2), "unbalanced \" in tag <{tag}> for src={src:?}\n  html={html}");
      assert!(sq.is_multiple_of(2), "unbalanced ' in tag <{tag}> for src={src:?}\n  html={html}");
    }
  }
}
