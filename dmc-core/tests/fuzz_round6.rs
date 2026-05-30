//! Round 6: large-scale mutation fuzz, big inputs, MDX-body-specific
//! edge cases, and content that historically triggered bugs in
//! mdx-js / remark-mdx.

use dmc::engine::compile::Compiler;
use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;

fn compile(src: &str) -> String {
  let mut diag: DiagnosticEngine<Code> = DiagnosticEngine::new();
  Compiler::compile(src, &mut diag).html
}

fn lcg(seed: &mut u64) -> u32 {
  *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
  (*seed >> 33) as u32
}

/// 1 MiB pure-text input must compile without quadratic blowup.
#[test]
fn one_megabyte_paragraph_terminates() {
  let unit = "lorem ipsum dolor sit amet, consectetur adipiscing elit. ";
  let s = unit.repeat(20_000); // ~ 1.1 MB
  let _ = compile(&s);
}

/// 1 MiB with random markdown delimiters mixed in. Worst case for the
/// inline parser's delim-stack.
#[test]
fn one_megabyte_with_delimiters_terminates() {
  let mut s = String::with_capacity(1_000_000);
  let mut seed = 0xBA_DC_AF_E0u64;
  while s.len() < 1_000_000 {
    let cp = lcg(&mut seed) % 6;
    s.push_str(match cp {
      0 => "word ",
      1 => "**bold** ",
      2 => "*em* ",
      3 => "`code` ",
      4 => "[a](u) ",
      _ => "\n",
    });
  }
  let _ = compile(&s);
}

/// 400 mutation rounds, each on a different seed corpus. Mutation is
/// 1-3 random byte ops (insert/delete/replace) on a syntactically
/// valid input. Catches near-valid panic shapes.
#[test]
fn mass_mutation_fuzz() {
  let seeds: &[&str] = &[
    "# heading\n\nbody **bold** *em*.\n",
    "- a\n- b\n- c\n\n```\ncode\n```\n",
    "| a | b |\n|---|---|\n| 1 | 2 |\n",
    "[link](url)\n![image](url.png)\n",
    "> blockquote\n> > nested\n",
    "<Comp prop={1+1}>child</Comp>\n",
    "{() => x + y}\n",
    "footnote[^1]\n\n[^1]: text\n",
    "$inline$ and $$display$$\n",
    "---\ntitle: x\n---\nbody\n",
  ];
  let alphabet: &[u8] = b" \n\t*_`~[](){}<>|+-=&;:'\"\\@abcXYZ012";
  let mut rng = 0xAB_CD_EF_01u64;
  for mut_n in 0..400 {
    let seed = seeds[(lcg(&mut rng) as usize) % seeds.len()];
    let mut bytes: Vec<u8> = seed.bytes().collect();
    let ops = (lcg(&mut rng) % 3) + 1;
    for _ in 0..ops {
      let op = lcg(&mut rng) % 3;
      let pos = if bytes.is_empty() { 0 } else { (lcg(&mut rng) as usize) % bytes.len() };
      match op {
        0 if !bytes.is_empty() => {
          bytes.remove(pos);
        },
        1 => {
          let c = alphabet[(lcg(&mut rng) as usize) % alphabet.len()];
          bytes.insert(pos, c);
        },
        _ if !bytes.is_empty() => {
          bytes[pos] = alphabet[(lcg(&mut rng) as usize) % alphabet.len()];
        },
        _ => {},
      }
    }
    if let Ok(s) = std::str::from_utf8(&bytes) {
      let _ = compile(s);
    }
    if mut_n % 50 == 0 {
      println!("mutation #{mut_n:04}");
    }
  }
}

/// MDX-specific torture: combinations of JSX, MDX expressions, ESM
/// imports/exports, comments. Patterns drawn from real MDX bug reports.
#[test]
fn mdx_combination_torture() {
  let cases = [
    "import X from 'y'\n\n<X/>\n",
    "import X from 'y';\nimport Y from 'z';\n\n<X/><Y/>\n",
    "export const x = 1;\n\n{x}\n",
    "import { a, b } from 'c';\n\n<a/>\n",
    "import X from 'y'\n\n# h\n\nbody\n",
    "{1 + 2}\n",
    "{['a', 'b'].map(x => <li>{x}</li>)}\n",
    "<Comp>{`inline ${expr} tpl`}</Comp>\n",
    "<Comp prop={() => <Inner/>}/>\n",
    "<Comp prop={`tpl`}>{`tpl2`}</Comp>\n",
    "<Outer>\n  <Inner1/>\n  <Inner2/>\n</Outer>\n",
    "<Outer>\n  text\n  more text\n</Outer>\n",
    "<>fragment</>\n",
    "<>\n  fragment with children\n</>\n",
    "{/* mdx comment */}\n",
    "{/* multi\nline\ncomment */}\n",
    "<!-- HTML comment in MDX -->\n",
    "<a href={`#${id}`}>x</a>\n",
    "<style>{`.a { color: red; }`}</style>\n",
    "<script>{`alert(1)`}</script>\n",
    "{(<X/>)}\n",
    "{x && <Y/>}\n",
    "{x ? <Y/> : <Z/>}\n",
    "{[1,2,3]}\n",
    "{{a: 1, b: 2}}\n",
    "<a-b-c/>\n",
    "<A.B/>\n",
    "<A.B.C/>\n",
    "<a:b/>\n",
    "<X xmlns=\"http://ex\"/>\n",
  ];
  for s in cases {
    let _ = compile(s);
  }
}

/// Lazy continuation lines under containers. CM has subtle rules about
/// when a non-prefixed line continues a quote/list item vs starts a
/// new block.
#[test]
fn lazy_continuation_corpus() {
  let cases = [
    "> a\nlazy continues quote\n",
    "> a\n\nnot lazy - new para\n",
    "- a\n  lazy continues item\n",
    "- a\nlazy at col 0 might also continue\n",
    "- a\n\n  paragraph in item\n",
    "1. a\n   lazy in ordered item\n",
    "> > a\nlazy into deepest quote\n",
    "> a\n> b\nlazy onto last line\n",
    "- a\n  - b\n    lazy in deeper\n",
    "> # heading\nlazy under heading?\n",
  ];
  for s in cases {
    let _ = compile(s);
  }
}

/// Setext heading interactions with other constructs: paragraph spans,
/// indented underline, content blocks above.
#[test]
fn setext_interactions() {
  let cases = [
    "a\n=\nb",
    "a\nb\n=\n",
    "**a**\n=\n",
    "[link](u)\n=\n",
    "> a\n=\n",
    "- a\n=\n",
    "```\na\n```\n=\n",
    "a\n\n=\n",
    "a\n=\n=\n",
    "a\n===========\n",
    "a\n   ===\n",
    "a\n    ===\n",
  ];
  for s in cases {
    let _ = compile(s);
  }
}

/// Reference link with definition AFTER use, across blank lines, in
/// containers.
#[test]
fn forward_reference_link_def() {
  let cases = [
    "[a]\n\n[a]: /url\n",
    "before [a] after\n\n[a]: /url\n",
    "[a]\n\nmiddle\n\n[a]: /url\n",
    "[a]\n\n> [a]: /url\n",
    "[a]\n\n- [a]: /url\n",
    "[`code`]\n\n[`code`]: /url\n",
    "[**bold**]\n\n[**bold**]: /url\n",
    "[A]\n\n[a]: /url\n",       // case-fold
    "[ a b ]\n\n[a b]: /url\n", // ws collapse
  ];
  for s in cases {
    let _ = compile(s);
  }
}

/// Deeply nested list items with mixed marker types. Each item carries
/// inline content that itself has nested constructs.
#[test]
fn deep_nested_list_items() {
  let mut s = String::new();
  for i in 0..30 {
    for _ in 0..i {
      s.push(' ');
    }
    let marker = match i % 3 {
      0 => "- ",
      1 => "* ",
      _ => "+ ",
    };
    s.push_str(marker);
    s.push_str("**item** with `code` and [link](u)\n");
  }
  let _ = compile(&s);
}

/// Block-level HTML mixed with markdown around it. HTML blocks have 7
/// types per CM 4.6, each with subtle continuation rules.
#[test]
fn html_block_types() {
  let cases = [
    // Type 1: <script>, <pre>, <style>
    "<pre>raw\n  preformatted\n</pre>\n",
    "<style>\n.a { color: red; }\n</style>\n",
    "<script>\nvar x = 1;\n</script>\n",
    // Type 2: <!-- ... -->
    "<!-- comment\nmulti line\n-->\n",
    // Type 3: <?xml ?>
    "<?xml version=\"1.0\"?>\nbody\n",
    // Type 4: <!DOCTYPE
    "<!DOCTYPE html>\nbody\n",
    // Type 5: <![CDATA[
    "<![CDATA[\nraw content\n]]>\n",
    // Type 6: any block-level tag
    "<div>\nbody\n</div>\n",
    "<table>\n<tr><td>x</td></tr>\n</table>\n",
    // Type 7: open or self-closing tag on its own line
    "<custom-element>\nbody\n</custom-element>\n",
  ];
  for s in cases {
    let _ = compile(s);
  }
}

/// Pathological table input - 100 columns × 100 rows. Largeish.
#[test]
fn large_table_terminates() {
  let mut s = String::new();
  s.push('|');
  for i in 0..100 {
    s.push_str(&format!(" h{i} |"));
  }
  s.push('\n');
  s.push('|');
  for _ in 0..100 {
    s.push_str("---|");
  }
  s.push('\n');
  for r in 0..100 {
    s.push('|');
    for c in 0..100 {
      s.push_str(&format!(" {r}.{c} |"));
    }
    s.push('\n');
  }
  let _ = compile(&s);
}

/// Math: large block, weird latex commands, nested $.
#[test]
fn math_edge_cases() {
  let cases = [
    "$x$",
    "$$x$$",
    "$ x $",
    "$$ x $$",
    "$$\nmulti\nline\n$$",
    "$\\frac{1}{2}$",
    "$$\\begin{align}\nx &= 1\\\\\ny &= 2\n\\end{align}$$",
    "$a_b^c$",
    "$\\sum_{i=0}^{n} i^2$",
    "$\\{1, 2, 3\\}$",
    "$$\\int_{-\\infty}^{\\infty} e^{-x^2} dx$$",
    "$\\$$\n",
    "$$$$\n",
    "para with $inline$ and $$display$$ together\n",
    "list:\n- $x$\n- $y$\n",
    "table:\n| $x$ | $y$ |\n|-|-|\n",
  ];
  for s in cases {
    let _ = compile(s);
  }
}
