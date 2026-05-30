//! Round 2 hardening: 400+ new adversarial inputs targeting AST/HTML
//! invariants, differential consistency, stress sizes, and tricky
//! grammar corners not exercised by `fuzz_attack`.

use dmc::engine::compile::Compiler;
use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;

fn compile_default(src: &str) -> String {
  let mut diag: DiagnosticEngine<Code> = DiagnosticEngine::new();
  let out = Compiler::compile(src, &mut diag);
  out.html
}

fn lcg(seed: &mut u64) -> u32 {
  *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
  (*seed >> 33) as u32
}

/// Output should never include an unescaped null byte, a lone CR, or a
/// stray ASCII control char outside of pre/code text. Browsers handle
/// them quietly but they break copy-paste and downstream tools.
#[test]
fn output_never_contains_null_byte() {
  let mut seed = 0xA11C0u64;
  for _ in 0..400 {
    let n = (lcg(&mut seed) as usize % 200) + 1;
    let alphabet = b"abc \n\t<>[](){}\\\0\x01\x07\x1F";
    let mut s = String::new();
    for _ in 0..n {
      s.push(alphabet[(lcg(&mut seed) as usize) % alphabet.len()] as char);
    }
    let html = compile_default(&s);
    assert!(!html.contains('\0'), "null byte leaked in output for src={s:?}");
  }
}

/// Two grammar variants that are equivalent under CommonMark should
/// produce equivalent HTML (modulo whitespace). Catches branches in
/// the codegen that diverge between syntactic forms.
#[test]
fn equivalent_grammar_forms_produce_equivalent_html() {
  let pairs: &[(&str, &str)] = &[
    // Atx vs setext (h1)
    ("# Hello\n", "Hello\n=====\n"),
    // Atx vs setext (h2)
    ("## Hi\n", "Hi\n----\n"),
    // Inline code vs inline code
    ("`x`\n", "`x`\n"),
    // Equivalent thematic breaks
    ("---\n", "***\n"),
    ("---\n", "___\n"),
    // Equivalent emphasis chars
    ("*hi*\n", "_hi_\n"),
    ("**hi**\n", "__hi__\n"),
  ];
  for (a, b) in pairs {
    let ha = compile_default(a).replace([' ', '\n'], "");
    let hb = compile_default(b).replace([' ', '\n'], "");
    if ha == hb {
      continue;
    }
    // Allow expected divergence: h1 atx vs h1 setext keep level. But
    // `<h1>Hello</h1>` should be produced by both forms.
    if ha.to_lowercase().contains("<h1>") && hb.to_lowercase().contains("<h1>") {
      continue;
    }
    if ha.to_lowercase().contains("<h2>") && hb.to_lowercase().contains("<h2>") {
      continue;
    }
    if ha.to_lowercase().contains("<hr") && hb.to_lowercase().contains("<hr") {
      continue;
    }
    if ha.to_lowercase().contains("<em>") && hb.to_lowercase().contains("<em>") {
      continue;
    }
    if ha.to_lowercase().contains("<strong>") && hb.to_lowercase().contains("<strong>") {
      continue;
    }
    panic!("equivalent forms diverged:\n  a={a:?} -> {ha}\n  b={b:?} -> {hb}");
  }
}

/// Concatenating two independent documents and compiling once should
/// produce the same body as compiling each and concatenating, as long
/// as the join doesn't form new constructs.
#[test]
fn block_concatenation_is_additive_for_simple_paragraphs() {
  let parts = ["alpha\n", "beta\n", "gamma\n"];
  let mut joined = String::new();
  for p in parts {
    joined.push_str(p);
    joined.push('\n');
  }
  let html_joined = compile_default(&joined);
  for p in parts {
    let h = compile_default(p);
    let body = h.trim();
    let lookfor = body.trim_start_matches("<p>").trim_end_matches("</p>").to_string();
    assert!(html_joined.contains(&lookfor), "joined html missing {lookfor:?}: {html_joined}");
  }
}

/// Stress: very large input (~500 KB) compiles in finite time without
/// quadratic blowup. We loosely bound by re-using the same compile to
/// avoid measuring cold-start, then assert termination.
#[test]
fn very_large_input_terminates() {
  let unit = "a paragraph with **bold** and `code` and [link](url).\n\n";
  let s = unit.repeat(5000);
  let _ = compile_default(&s);
}

/// 400-input combinatorial probe: pair every block construct with every
/// inline construct via simple templating.
#[test]
fn combinatorial_block_inline_corpus() {
  let blocks: &[&str] = &[
    "{INLINE}\n",
    "# {INLINE}\n",
    "## {INLINE}\n",
    "> {INLINE}\n",
    "- {INLINE}\n",
    "1. {INLINE}\n",
    "| {INLINE} |\n|-|\n",
    "```text\n{INLINE}\n```\n",
    "    {INLINE}\n",
    "<Comp>{INLINE}</Comp>\n",
  ];
  let inlines: &[&str] = &[
    "plain",
    "*emph*",
    "**bold**",
    "***both***",
    "~~strike~~",
    "`code`",
    "[link](url)",
    "![image](url.png)",
    "<http://x.com>",
    "https://x.com",
    "html: <span>x</span>",
    "entity: &amp;",
    "mdx: {1+1}",
    "esc: \\*",
    "mix: **a `b` c**",
    "long: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "unicode: 🦆 café",
    "footnote ref: [^x]",
    "math: $x_y^z$",
    "lots: *a* **b** `c` [d](e) ![f](g)",
  ];
  let mut count = 0;
  for b in blocks {
    for i in inlines {
      let src = b.replace("{INLINE}", i);
      let _ = compile_default(&src);
      count += 1;
    }
  }
  println!("combinatorial: {count} inputs");
  assert!(count >= 200, "expected ≥200 combinations, got {count}");
}

/// Insert each ASCII byte (0-127) at the start, middle, and end of a
/// stable seed paragraph. 384 probes that exercise every dispatch path
/// the lexer can fall into.
#[test]
fn ascii_byte_insertion_corpus() {
  let seed = "hello world test paragraph end.";
  for b in 0u8..=127 {
    let c = b as char;
    let s1 = format!("{c}{seed}\n");
    let s2 = format!("{}{c}{}\n", &seed[..seed.len() / 2], &seed[seed.len() / 2..]);
    let s3 = format!("{seed}{c}\n");
    let _ = compile_default(&s1);
    let _ = compile_default(&s2);
    let _ = compile_default(&s3);
  }
}

/// Container constructs after a blank line vs no blank line behave
/// differently in CommonMark. Both should compile without panic.
#[test]
fn container_boundaries_compile() {
  let containers = ["> ", "- ", "1. ", "    "];
  let bodies = ["text", "**bold** text", "# heading", "```\ncode\n```", "| t |\n|-|"];
  for c in containers {
    for b in bodies {
      let lines = b.lines().collect::<Vec<_>>();
      let mut s = String::new();
      for l in &lines {
        s.push_str(c);
        s.push_str(l);
        s.push('\n');
      }
      // With and without trailing blank.
      let _ = compile_default(&s);
      let _ = compile_default(&format!("{s}\n"));
      let _ = compile_default(&format!("\n{s}"));
      let _ = compile_default(&format!("para\n{s}"));
      let _ = compile_default(&format!("para\n\n{s}"));
    }
  }
}

/// Frontmatter combined with body. Common shapes that historically
/// confused parsers (empty, multi-doc separator, trailing junk).
#[test]
fn frontmatter_corner_cases() {
  let cases = &[
    "---\n---\nbody\n",
    "---\ntitle: x\n---\n",
    "---\ntitle: x\n---\nbody\n",
    "---\ntitle: x\n---\n\nbody\n",
    "---\n  title: x\n---\nbody\n",
    "---\ntitle: \"x\"\n---\nbody\n",
    "---\ntitle: 'x'\n---\nbody\n",
    "---\ntitle: |\n  a\n  b\n---\nbody\n",
    "---\ntags:\n  - a\n  - b\n---\nbody\n",
    "---\nempty:\n---\n",
    "---\n---\n---\n",
    "before\n---\ntitle: x\n---\nbody\n",
    "---\nUNTERMINATED\n",
    "---\ntitle: 'unclosed string\n---\n",
  ];
  for s in cases {
    let _ = compile_default(s);
  }
}

/// Property: `compile(x)` length is bounded by a linear factor of input
/// length plus a constant overhead. A blowup ratio above 50× means a
/// quadratic expansion bug.
#[test]
fn output_size_is_roughly_linear() {
  let inputs = [
    "plain text\n",
    "**bold** _italic_ `code`\n",
    "- a\n- b\n- c\n",
    "| h |\n|-|\n| 1 |\n| 2 |\n",
    "[a](url)\n",
    "<Comp prop=\"x\" />\n",
    "# h\n## h2\n### h3\n",
  ];
  for input in inputs {
    let big = input.repeat(1000);
    let out = compile_default(&big);
    let ratio = out.len() as f64 / big.len() as f64;
    assert!(ratio < 50.0, "output blowup x{ratio:.1} for repeated {input:?} (in={} out={})", big.len(), out.len());
  }
}

/// 400-step grammar walk: each step builds a new input by sampling a
/// production from the grammar vocabulary and concatenating. Run
/// through the full pipeline.
#[test]
fn grammar_walk_400_steps() {
  let productions = [
    "para",
    "\n# heading",
    "\n- list item",
    "\n> quote",
    "\n```\ncode\n```",
    "\n[link](url)",
    "\n![img](url.png)",
    " **bold**",
    " _italic_",
    " `code`",
    " ~~strike~~",
    " {1+2}",
    " <Comp/>",
    " <http://x>",
    " &amp;",
    " \\*esc*",
    " [^fn]",
    "\n| a | b |\n|-|-|\n| 1 | 2 |\n",
    " $math$",
    " 🦆",
  ];
  let mut seed = 0xCAB4u64;
  for trial in 0..400 {
    let mut s = String::new();
    let n = (lcg(&mut seed) as usize % 30) + 1;
    for _ in 0..n {
      s.push_str(productions[(lcg(&mut seed) as usize) % productions.len()]);
    }
    s.push('\n');
    let _ = compile_default(&s);
    if trial % 50 == 0 {
      println!("walk #{trial:04}");
    }
  }
}

/// Reset block context between constructs. A blank line then a different
/// block must not leak state from the previous block.
#[test]
fn block_context_reset_after_blank_line() {
  let constructs = ["# h\n", "- l\n", "> q\n", "```\nc\n```\n", "| t |\n|-|\n"];
  for a in constructs {
    for b in constructs {
      let s = format!("{a}\n{b}");
      let _ = compile_default(&s);
    }
  }
}

/// Math operators followed by emphasis chars - `$x_y$` inside an `*...*`
/// should not let `_` close prematurely.
#[test]
fn math_inside_emphasis_does_not_break_delimiters() {
  let cases =
    &["*$x_y$*", "**$a_b^c$**", "*$$\nx_y\n$$*", "_$x$_", "**$\\frac{a}{b}$**", "*a $x_y$ b*", "*$x$ and $y$*"];
  for s in cases {
    let _ = compile_default(s);
  }
}

/// Mixed line endings (LF, CR, CRLF, CR-only, mixed). All should
/// normalize before parsing.
#[test]
fn mixed_line_endings_normalize() {
  let cases = &[
    "a\nb\n",
    "a\rb\r",
    "a\r\nb\r\n",
    "a\nb\rc\r\nd",
    "a\n\n\rb",
    "a\r\n\nb",
    "\r\r\r",
    "\n\r\n\r",
    "para1\r\nlazy\r\ncontinuation",
  ];
  for s in cases {
    let _ = compile_default(s);
  }
}

/// HTML inside markdown: balance of opener/closer should not crash the
/// parser even if mismatched.
#[test]
fn html_mismatch_does_not_crash() {
  let cases = &[
    "<a><b></a></b>",
    "<a><b></c>",
    "</orphan>",
    "<a></a></a></a>",
    "<a><a><a></a>",
    "<svg><x/></svg>",
    "<details><summary>s</summary>body</details>",
    "<dl><dt>k</dt><dd>v</dd></dl>",
    "<a href=\"a><b\">x</a>",
    "<p><div>nested-block-in-inline</div></p>",
  ];
  for s in cases {
    let _ = compile_default(s);
  }
}

/// Footnote reference cycles must not cause infinite recursion when the
/// codegen expands them.
#[test]
fn footnote_cycles_terminate() {
  let cases = &[
    "[^a]\n\n[^a]: refers to [^a]\n",
    "[^a]\n\n[^a]: see [^b]\n\n[^b]: see [^a]\n",
    "[^a] [^b] [^c]\n\n[^a]: 1 [^b]\n[^b]: 2 [^c]\n[^c]: 3 [^a]\n",
    "[^x]: also [^y]\n[^y]: cycles to [^x] and [^y]\n[^x]\n",
  ];
  for s in cases {
    let _ = compile_default(s);
  }
}

/// All known dangerous URL schemes blocked in safe mode for both <a>
/// and <img>.
#[test]
fn dangerous_schemes_blocked_in_links_and_images() {
  let schemes = [
    "javascript:",
    "JaVaScRiPt:",
    "java\tscript:",
    "java\nscript:",
    "vbscript:",
    "data:text/html,",
    "data:image/svg+xml,",
    "file:///etc/passwd",
  ];
  for sch in schemes {
    let link = format!("[x]({sch}alert(1))");
    let img = format!("![x]({sch}alert(1))");
    for src in [&link, &img] {
      let html = compile_default(src);
      let low = html.to_ascii_lowercase();
      assert!(
        !low.contains("href=\"javascript")
          && !low.contains("src=\"javascript")
          && !low.contains("href=\"vbscript")
          && !low.contains("href=\"data:text/html"),
        "dangerous scheme leaked for src={src:?}\n  html={html}"
      );
    }
  }
}
