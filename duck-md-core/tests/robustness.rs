use duck_md::compile;

const SAMPLES: &[&str] = &[
  "",
  " ",
  "\n\n\n",
  "---\nbroken: \n  - [ unclosed\n",        // bad yaml
  "import { from",                          // truncated import
  "<Foo bar=",                              // bad jsx attr
  "<Foo>",                                  // unclosed jsx
  "**bold without close",
  "`code without close",
  "```rust\nno closing fence",
  "[link without close",
  "[text](href without close",
  "{expression without close",
  "{/* comment without close",
  "<!-- html comment without close",
  "<>frag without close",
  "## heading then unclosed jsx <Foo\n",
  "💀💩🚀\n# emoji heading\n",
  "\u{0000}\u{0001}\u{0002}",               // control chars
  "a < b < c\n",                            // ambiguous lt
  "{{{{{{{{",                               // many opens
  "}}}}}}}}}",                              // many closes
  "<><><><></><><><><>",                    // chained frags
];

#[test]
fn does_not_panic_on_malformed_input() {
  for s in SAMPLES {
    let result = std::panic::catch_unwind(|| compile(s));
    assert!(result.is_ok(), "panic on input: {:?}", s);
  }
}

#[test]
fn malformed_input_still_compiles() {
  for s in SAMPLES {
    let out = compile(s);
    // body should always be a function string, even if empty
    assert!(out.body.contains("_createMdxContent"), "no body for {:?}", s);
  }
}
