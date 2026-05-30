//! Hardening probes: cycle parse → transform → assert no panic for a
//! corpus of inputs. Feature-gated transformers are only invoked when
//! the matching feature is enabled.

use dmc_transform::{BareUrlAutolink, Pipeline};

const PROBES: &[&str] = &[
  "",
  "plain text",
  "see https://example.com for info\n",
  "many: https://a.com http://b.com https://c.com\n",
  "trail: see http://x.com.\n",
  "paren: (see http://x.com)\n",
  "semi: see https://example.com/p;\n",
  "in-link: [a](https://x.com)\n",
  "in-code: `https://x.com`\n",
  "in-code-block:\n```\nhttps://x.com\n```\n",
  "in-quote: > visit http://x.com\n",
  "www only: www.example.com\n",
  "deep: https://a.b.c.d.e/very/deep/path?q=1&r=2#frag\n",
  "unicode in URL: https://例え.jp/path\n",
  "trailing punct chain: see http://x.com),.;:!?\n",
  "tag: <http://x.com>\n",
  "ref: [a][b]\n\n[b]: https://example.com\n",
  "image: ![alt](https://i.example.com/x.png)\n",
  "table:\n| u |\n|---|\n| https://x.com |\n",
  "list:\n- https://a.com\n- https://b.com\n",
  // many lines
  "para 1\n\npara 2\n\npara 3 https://x.com\n",
  // jsx surrounding
  "<Comp>see https://x.com</Comp>\n",
  // mdx expression
  "{`https://x.com`}\n",
  // multibyte adjacency
  "🦆 https://x.com 🦆\n",
  // angle autolink in label
  "[`<http://x.com>`](https://example.com)\n",
  // multi-paren URLs
  "see https://en.wikipedia.org/wiki/Foo_(disambiguation) for more\n",
  // nested parens in URL with trailing parenthesis
  "(see https://en.wikipedia.org/wiki/Foo_(bar))\n",
  // mailto
  "<mailto:a@b.com>\n",
  // FTP
  "see ftp://files.example.com/x.tar\n",
];

#[test]
fn pipeline_does_not_panic_on_corpus() {
  for (i, src) in PROBES.iter().enumerate() {
    let mut d = dmc_parser::parse(src);
    Pipeline::new().add(BareUrlAutolink).run_silent(&mut d);
    println!("transform probe #{i:03} ok ({} bytes)", src.len());
  }
}

#[test]
fn parens_inside_urls_round_trip_with_autolink() {
  let src = "see https://en.wikipedia.org/wiki/Foo_(disambiguation) for more\n";
  let mut d = dmc_parser::parse(src);
  Pipeline::new().add(BareUrlAutolink).run_silent(&mut d);
  let para = d
    .children
    .iter()
    .find_map(|n| match n {
      dmc_parser::ast::Node::Paragraph(p) => Some(p),
      _ => None,
    })
    .expect("paragraph");
  let link = para
    .children
    .iter()
    .find_map(|n| match n {
      dmc_parser::ast::Node::Link(l) => Some(l),
      _ => None,
    })
    .expect("link");
  assert_eq!(link.href, "https://en.wikipedia.org/wiki/Foo_(disambiguation)");
}
