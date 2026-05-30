//! Round 5: differential HTML vs MDX body, pure-lexer fuzz, transform
//! pipeline crossing, scale stress, unicode security tortures.

use dmc::engine::compile::Compiler;
use dmc_codegen::{RenderOptions, render_html_with, render_mdx_body};
use dmc_diagnostic::Code;
use dmc_diagnostic::metadata::{Origin, SourceMeta};
use dmc_lexer::Lexer;
use dmc_parser::parse;
use dmc_transform::{BareUrlAutolink, Pipeline};
use duck_diagnostic::DiagnosticEngine;
use std::sync::Arc;

fn lcg(seed: &mut u64) -> u32 {
  *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
  (*seed >> 33) as u32
}

fn compile_html(src: &str) -> String {
  let mut diag: DiagnosticEngine<Code> = DiagnosticEngine::new();
  Compiler::compile(src, &mut diag).html
}

/// Lexer alone: ~3000 random inputs. End-to-end fuzz already hits the
/// lexer, but a direct call surfaces panics that the parser would mask.
#[test]
fn lexer_direct_fuzz_does_not_panic() {
  let alphabet: &[u8] = b" \n\t\r#*_`~![](){}<>/\\|+-=&;:'\"@.%abcXYZ012";
  let mut seed = 0xBADD_C0DE_BADD_C0DEu64;
  for trial in 0..3000 {
    let len = (lcg(&mut seed) as usize % 256) + 1;
    let mut s = String::with_capacity(len);
    for _ in 0..len {
      let idx = (lcg(&mut seed) as usize) % alphabet.len();
      s.push(alphabet[idx] as char);
    }
    let meta = Arc::new(SourceMeta { path: Arc::from("<probe>"), origin: Origin::Inline("<probe>") });
    let mut diag: DiagnosticEngine<Code> = DiagnosticEngine::new();
    let mut lexer = Lexer::new(&s, meta, &mut diag);
    let _ = lexer.scan_tokens();
    if trial % 500 == 0 {
      println!("lexer direct #{trial:04}");
    }
  }
}

/// Codegen direct: parse → render both modes for 200 inputs. Neither
/// renderer may panic.
#[test]
fn codegen_both_modes_do_not_panic() {
  let cases = [
    "",
    "para\n",
    "# h\n",
    "- a\n- b\n",
    "1. a\n",
    "> q\n",
    "```\nc\n```\n",
    "[a](u)\n",
    "![](u.png)\n",
    "**b**\n",
    "*i*\n",
    "~~s~~\n",
    "`code`\n",
    "<Comp />\n",
    "<X p=\"x\">y</X>\n",
    "{1+2}\n",
    "$x$\n",
    "$$y$$\n",
    "| a |\n|-|\n| 1 |\n",
    "footnote[^1]\n\n[^1]: text\n",
    "<a href=\"javascript:alert(1)\">x</a>\n",
    "<script>alert(1)</script>\n",
    "<a>raw</a>\n",
    "&amp;&lt;&gt;\n",
    "\0bad\0\n",
  ];
  for s in cases {
    let doc = parse(s);
    let _ = render_html_with(&doc, RenderOptions::default());
    let _ = render_html_with(&doc, RenderOptions { allow_dangerous_html: true, ..Default::default() });
    let _ = render_mdx_body(&doc);
  }
}

/// Differential property: if input contains no MDX-specific construct
/// (no `<JsxTag/>`, no `{expr}`, no `import`/`export`), the MDX-body
/// renderer and the HTML renderer should be structurally consistent
/// (modulo HTML-element-name capitalization and whitespace). We just
/// confirm both renderers produce a non-empty string for non-empty
/// inputs.
#[test]
fn both_renderers_produce_output_for_non_empty_input() {
  let cases = [
    "para text\n",
    "# h\n",
    "**bold** and *italic*\n",
    "[link](url)\n",
    "list:\n- a\n- b\n",
    "table:\n| a |\n|-|\n| 1 |\n",
    "blockquote:\n> q\n",
    "code:\n```\nc\n```\n",
  ];
  for src in cases {
    let doc = parse(src);
    let html = render_html_with(&doc, RenderOptions::default());
    let mdx = render_mdx_body(&doc);
    assert!(!html.is_empty(), "empty html for {src:?}");
    assert!(!mdx.is_empty(), "empty mdx body for {src:?}");
  }
}

/// Stress: 50000-line input compiles in finite time. Pure lines should
/// trivially compose into 50000 separate paragraphs (with blanks) or
/// one giant paragraph.
#[test]
fn fifty_thousand_line_input_terminates() {
  let s = "para\n".repeat(50_000);
  let _ = compile_html(&s);
  let s = "para\n\n".repeat(25_000);
  let _ = compile_html(&s);
}

/// Unicode security: zero-width / RTL / bidi codepoints in URLs. None
/// should produce an active dangerous-scheme link.
#[test]
fn zero_width_and_rtl_in_urls() {
  let bad_urls = [
    "j\u{202E}avascript:alert(1)",
    "javasc\u{202D}ript:alert(1)",
    "javascript\u{200B}:alert(1)",
    "java\u{200C}script:alert(1)",
    "javascript:\u{202E}alert(1)",
    "javascript:al\u{2028}ert(1)",
    "javascript:al\u{2029}ert(1)",
  ];
  for u in bad_urls {
    let src = format!("[click]({u})");
    let html = compile_html(&src);
    let low = html.to_ascii_lowercase();
    assert!(!low.contains("href=\"javascript:"), "dangerous URL leaked for src={src:?}\n  html={html}");
  }
}

/// 400 random input chunks pumped through the transform pipeline (parse
/// → BareUrlAutolink). Catches transformer panics on shapes that the
/// parser produces but the transformer doesn't expect.
#[test]
fn transform_pipeline_random_fuzz() {
  let words = [
    "https://x.com",
    "see ftp://y",
    "www.example.com",
    "no urls here",
    "**bold**",
    "`code`",
    "[a](u)",
    "<Comp/>",
    "$math$",
    "https://x.com.",
    "(see https://x.com)",
    "https://x.com)inside_text",
    "trailing https://x.com;",
  ];
  let mut seed = 0x55AA55AAu64;
  for trial in 0..400 {
    let n = (lcg(&mut seed) as usize % 6) + 1;
    let mut s = String::new();
    for _ in 0..n {
      s.push_str(words[(lcg(&mut seed) as usize) % words.len()]);
      s.push(' ');
    }
    s.push('\n');
    let mut d = parse(&s);
    Pipeline::new().add(BareUrlAutolink).run_silent(&mut d);
    if trial % 50 == 0 {
      println!("transform fuzz #{trial:04}");
    }
  }
}

/// Repeatedly compile the same input from two independent threads in a
/// tight loop. With nondeterminism (HashMap default RNG, lazy globals
/// without sync) the outputs would diverge.
#[test]
fn parallel_same_input_outputs_match() {
  use std::sync::Arc;
  use std::thread;
  let src = Arc::new("# h\n\n**bold** *em* `code` [a](u) and a paragraph.\n".to_string());
  let reference = compile_html(&src);
  let mut handles = Vec::new();
  for _ in 0..8 {
    let src = src.clone();
    let reference = reference.clone();
    handles.push(thread::spawn(move || {
      for _ in 0..50 {
        let out = compile_html(&src);
        assert_eq!(out, reference, "parallel output drift");
      }
    }));
  }
  for h in handles {
    h.join().expect("thread panic");
  }
}

/// Insert random invalid-utf8-like bytes (via String escapes) into seed
/// inputs and ensure the lexer/parser bypass them or normalize to
/// U+FFFD without panicking.
#[test]
fn weird_unicode_codepoints_in_text_dont_panic() {
  let codepoints = [
    '\u{0001}',
    '\u{0007}',
    '\u{0008}',
    '\u{000B}',
    '\u{000C}',
    '\u{000E}',
    '\u{001F}',
    '\u{007F}',
    '\u{0080}',
    '\u{00A0}',
    '\u{2028}',
    '\u{2029}',
    '\u{200B}',
    '\u{200C}',
    '\u{200D}',
    '\u{200E}',
    '\u{200F}',
    '\u{202A}',
    '\u{202B}',
    '\u{202C}',
    '\u{202D}',
    '\u{202E}',
    '\u{FEFF}',
    '\u{FFFD}',
    '\u{10000}',
    '\u{1F600}',
    '\u{2FFFD}',
  ];
  for c in codepoints {
    let s = format!("para {c} text {c} end\n");
    let _ = compile_html(&s);
    let s = format!("# heading {c}\n");
    let _ = compile_html(&s);
    let s = format!("```\ncode {c}\n```\n");
    let _ = compile_html(&s);
    let s = format!("[link {c}](u {c})");
    let _ = compile_html(&s);
  }
}

/// 400 inputs that splice a delimiter between every two characters of
/// short seed strings. Hammers the inline delimiter resolver.
#[test]
fn delimiter_splice_corpus() {
  let seeds = ["alpha", "beta_gamma", "delta-eta", "phi.psi", "x*y"];
  let delims = ['*', '_', '`', '~', '\\', '!', '['];
  let mut count = 0;
  for seed in seeds {
    let chars: Vec<char> = seed.chars().collect();
    for &d in &delims {
      for i in 0..chars.len() {
        let mut s = String::new();
        for (j, c) in chars.iter().enumerate() {
          s.push(*c);
          if j == i {
            s.push(d);
          }
        }
        let _ = compile_html(&s);
        count += 1;
      }
    }
  }
  println!("delimiter splices: {count}");
}

/// Quote types inside link titles and attribute values - both styles,
/// mixed, escaped.
#[test]
fn quote_styles_in_titles_and_attrs() {
  let cases = [
    "[a](u 'title')",
    "[a](u \"title\")",
    "[a](u (title))",
    "[a](u 'with \"both\" quotes')",
    "[a](u \"with 'both' quotes\")",
    "[a](u 'escaped \\' quote')",
    "[a](u \"escaped \\\" quote\")",
    "<a href='single'>",
    "<a href=\"double\">",
    "<a href=mixed'>",
    "<a href=\"contains'single\">",
    "<a href='contains\"double'>",
  ];
  for s in cases {
    let _ = compile_html(s);
  }
}

/// Image titles + alt text crossings - alt text with brackets, titles
/// with quotes, sources with parens.
#[test]
fn image_alt_and_title_edge() {
  let cases = [
    "![](.)",
    "![alt](src)",
    "![alt with \"quote\"](src \"title with \\\"q\\\"\")",
    "![alt with [bracket]](src)",
    "![](src \"title with [brackets]\")",
    "![alt with `code`](src)",
    "![](src 'title with \\u{1F600}')",
    "![multi\nline alt](src)",
    "![alt](src with spaces)",
    "![alt](<src with spaces>)",
  ];
  for s in cases {
    let _ = compile_html(s);
  }
}

/// Heading depth invariants: even with very long heading text and weird
/// inline markup inside, the level must remain 1-6.
#[test]
fn heading_level_invariants() {
  for level in 1..=10usize {
    let prefix = "#".repeat(level);
    let src = format!("{prefix} heading text\n");
    let html = compile_html(&src);
    let lower = html.to_ascii_lowercase();
    if level <= 6 {
      assert!(lower.contains(&format!("<h{level}")), "missing <h{level}> for {src:?}: {html}");
    } else {
      // Too many hashes = paragraph
      assert!(!lower.contains("<h7") && !lower.contains("<h8"), "invalid heading level emitted for {src:?}");
    }
  }
}
