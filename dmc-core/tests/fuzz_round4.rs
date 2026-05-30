//! Round 4: AST invariants, idempotence-style properties, exhaustive
//! short-input enumeration, byte-pair probes.

use dmc::engine::compile::Compiler;
use dmc_diagnostic::Code;
use dmc_parser::ast::{Document, Node};
use duck_diagnostic::DiagnosticEngine;

fn compile_html(src: &str) -> String {
  let mut diag: DiagnosticEngine<Code> = DiagnosticEngine::new();
  Compiler::compile(src, &mut diag).html
}

fn parse_doc(src: &str) -> Document {
  dmc_parser::parse(src)
}

fn walk_nodes<'a>(nodes: &'a [Node], f: &mut dyn FnMut(&'a Node)) {
  for n in nodes {
    f(n);
    match n {
      Node::Paragraph(p) => walk_nodes(&p.children, f),
      Node::Heading(h) => walk_nodes(&h.children, f),
      Node::Blockquote(bq) => walk_nodes(&bq.children, f),
      Node::List(l) => walk_nodes(&l.children, f),
      Node::ListItem(li) => walk_nodes(&li.children, f),
      Node::Bold(i) | Node::Italic(i) | Node::Strikethrough(i) => walk_nodes(&i.children, f),
      Node::Link(l) => walk_nodes(&l.children, f),
      Node::Image(_) => {},
      _ => {},
    }
  }
}

/// CommonMark forbids nesting a Link inside another Link's children. The
/// parser already aborts the outer link in that case, so the AST should
/// never expose a Link node beneath another Link node's subtree.
#[test]
fn ast_has_no_nested_links() {
  let cases = [
    "[outer [inner](u1) tail](u2)",
    "[a [b [c](u3)](u2)](u1)",
    "[![image](i)](u)", // images may nest under links
    "[a **bold [link](u)**](u)",
  ];
  for src in cases {
    let d = parse_doc(src);
    let mut in_link: Vec<&Node> = Vec::new();
    walk_nodes(&d.children, &mut |n| {
      if matches!(n, Node::Link(_)) {
        in_link.push(n);
      }
    });
    // Now check each Link has no Link in its subtree.
    for l in &in_link {
      if let Node::Link(link) = l {
        let mut nested = false;
        walk_nodes(&link.children, &mut |n| {
          if matches!(n, Node::Link(_)) {
            nested = true;
          }
        });
        assert!(!nested, "Link contains nested Link for src={src:?}");
      }
    }
  }
}

/// All ASCII pairs (0..128) × (0..128). Catches single-byte dispatch
/// bugs that only fire on a specific 2-byte sequence (e.g. `<!`, `</`,
/// `*_`, `\\\\`). 16384 probes.
#[test]
fn ascii_pair_exhaustive() {
  for a in 0u8..=127 {
    for b in 0u8..=127 {
      let s = format!("{}{}", a as char, b as char);
      let _ = compile_html(&s);
    }
  }
}

/// HTML output structural: every `<p>` should have a matching `</p>`,
/// every `<h1>..<h6>` should match. Catches half-open tag emission.
#[test]
fn output_html_tag_balance_for_corpus() {
  let cases = [
    "para\n",
    "# h\n",
    "## h2\n",
    "### h3\n",
    "- a\n- b\n",
    "1. a\n2. b\n",
    "> q\n",
    "```\nc\n```\n",
    "[a](u)\n",
    "![](u.png)\n",
    "**bold** *italic*\n",
    "| a |\n|-|\n| 1 |\n",
    "<Comp/>\n",
    "<X>y</X>\n",
    "{1}\n",
    "footnote[^1]\n\n[^1]: text\n",
    "$x$\n",
    "$$y$$\n",
  ];
  for src in cases {
    let html = compile_html(src);
    for tag in [
      "p",
      "h1",
      "h2",
      "h3",
      "h4",
      "h5",
      "h6",
      "blockquote",
      "ul",
      "ol",
      "li",
      "pre",
      "code",
      "table",
      "thead",
      "tbody",
      "tr",
      "th",
      "td",
      "strong",
      "em",
    ] {
      let opens = html.matches(&format!("<{tag}>")).count() + html.matches(&format!("<{tag} ")).count();
      let closes = html.matches(&format!("</{tag}>")).count();
      assert_eq!(opens, closes, "unbalanced <{tag}> in output for src={src:?}\n  html={html}");
    }
  }
}

/// Compile, run again. Result for same input must be identical even
/// after thousands of iterations (no stateful drift through globals).
#[test]
fn repeated_compile_is_stable() {
  let src = "# heading\n\n**bold** *italic* `code` [link](url) and a paragraph with footnote[^x].\n\n[^x]: text\n";
  let first = compile_html(src);
  for _ in 0..1000 {
    let next = compile_html(src);
    assert_eq!(first, next, "drift detected");
  }
}

/// HTML-escaping is idempotent in safe mode: feeding compiled output
/// back as Markdown should not produce double-encoded entities.
#[test]
fn safe_mode_html_does_not_double_encode_on_round_trip() {
  let src = "ampersand & here, less <than, greater >than\n";
  let once = compile_html(src);
  let twice = compile_html(&once);
  // After two passes, `&amp;amp;` would indicate double encoding.
  assert!(!twice.contains("&amp;amp;"), "double encoding detected:\n  once={once}\n  twice={twice}");
  assert!(!twice.contains("&amp;lt;"), "double encoding detected:\n  once={once}\n  twice={twice}");
  assert!(!twice.contains("&amp;gt;"), "double encoding detected:\n  once={once}\n  twice={twice}");
}

/// Every 1-byte input (0..256) compiles. Some bytes only appear via
/// non-ASCII codepoints in source.
#[test]
fn every_byte_single_char_compiles() {
  for cp in 0u32..=0xFFFF {
    if let Some(c) = char::from_u32(cp) {
      let s = c.to_string();
      let _ = compile_html(&s);
    }
  }
}

/// 400 small synthetic inputs (1-5 chars each) composed only of markdown
/// delimiters - exhaustive coverage of dispatch combinations.
#[test]
fn small_delimiter_only_inputs() {
  let delims = ['*', '_', '`', '~', '[', ']', '(', ')', '<', '>', '|', '#', '+', '-', '!', '\\', '&', '{', '}'];
  let mut count = 0;
  for &a in &delims {
    for &b in &delims {
      for &c in &delims {
        let s = format!("{a}{b}{c}");
        let _ = compile_html(&s);
        count += 1;
      }
    }
  }
  println!("small-delim probes: {count}");
}

/// Tab expansion: CM 4-space tabstop. Inputs at every tab position
/// boundary should compile.
#[test]
fn tab_expansion_corner_cases() {
  let cases = [
    "\tcode",
    " \tcode",
    "  \tcode",
    "   \tcode",
    "    \tcode",
    "1.\titem",
    "1.  \titem",
    "-\titem",
    "- \titem",
    ">\tquote",
    "> \tquote",
    "  \t  hybrid",
    "\t\t\tdeep tabs",
  ];
  for s in cases {
    let _ = compile_html(s);
  }
}

/// Setext underline lengths and characters. CM allows underlines of
/// any length, including just one `=` or `-`.
#[test]
fn setext_variants() {
  let cases = [
    "x\n=\n",
    "x\n===\n",
    "x\n=========\n",
    "x\n-\n",
    "x\n-----\n",
    "x\n=-=\n", // not valid setext
    "x\n= =\n",
    "x\n  ===\n",
    "x\n   ===\n",
    "x\n    ===\n", // 4 spaces - not setext
    "x\n===\nmore",
    "**x**\n===\n",
    "[x](u)\n===\n",
    "\nx\n===\n",
    "x\n\n===\n", // blank between - not setext
  ];
  for s in cases {
    let _ = compile_html(s);
  }
}

/// Container blocks where the closing marker is missing — every open
/// construct must eventually close at EOF without infinite loops.
#[test]
fn unclosed_constructs_at_eof() {
  let cases = [
    "```\n",
    "```rust\nfn x() {\n",
    "~~~\n",
    "    indented\n        deep\n",
    "<a>\n",
    "<Comp\n",
    "{\n",
    "{`tpl${\n",
    "[a](\n",
    "[a](url 'unterminated\n",
    "<!--\n",
    "$\n",
    "$$\n",
    "---\nfm:\n",
    "[^a]: footnote",
    "| a |\n|--",
  ];
  for s in cases {
    let _ = compile_html(s);
  }
}
