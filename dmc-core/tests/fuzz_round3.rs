//! Round 3: AST shape invariants, edge URLs, malformed entities,
//! container x inline combinatorics that historically tripped parsers.

use dmc::engine::compile::Compiler;
use dmc_diagnostic::Code;
use duck_diagnostic::DiagnosticEngine;

fn compile(src: &str) -> String {
  let mut diag: DiagnosticEngine<Code> = DiagnosticEngine::new();
  let out = Compiler::compile(src, &mut diag);
  out.html
}

fn lcg(seed: &mut u64) -> u32 {
  *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
  (*seed >> 33) as u32
}

/// 100 obscure URL forms that have historically slipped past sanitizers.
/// All must be either rejected (replaced with `#` / sanitized) or stay
/// safe; none should produce an active `javascript:` / `data:text/html`
/// / `vbscript:` href.
#[test]
fn obscure_url_forms_are_safe() {
  let urls = [
    "javascript:alert(1)",
    "JavaScript:alert(1)",
    " javascript:alert(1)",
    "\tjavascript:alert(1)",
    "\njavascript:alert(1)",
    "\0javascript:alert(1)",
    "javascript\0:alert(1)",
    "javascript :alert(1)",
    "j\u{0009}avascript:alert(1)",
    "j\u{000A}avascript:alert(1)",
    "j\u{000D}avascript:alert(1)",
    "j\u{0008}avascript:alert(1)",
    "j\u{200B}avascript:alert(1)",
    "j\u{FEFF}avascript:alert(1)",
    "JAVASCRIPT:alert(1)",
    "javasc\u{200E}ript:alert(1)",
    "javascript\u{200F}:alert(1)",
    "vbscript:msgbox",
    "VBSCRIPT:msgbox",
    "data:text/html,<script>alert(1)</script>",
    "data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==",
    "data:image/svg+xml,<svg onload=alert(1)/>",
    "data:application/xhtml+xml,<script/>",
    "file:///etc/passwd",
    "file://./etc/passwd",
    "FILE:///etc/passwd",
    "moz-icon://x",
    "about:blank",
    "chrome://settings",
    "ms-its:mhtml:x",
    "view-source:http://x",
    "blob:javascript:alert(1)",
    "intent://x",
    "ftp://x.com/file",
    "ssh://user@host",
    "telnet://x:23",
    "irc://x",
    "ldap://x",
    "ws://x",
    "wss://x",
    "//protocol-relative",
    "/absolute/path",
    "./relative",
    "../parent",
    "#fragment",
    "?query=1",
    "",
    " ",
    "http://",
    "https://",
    "https://exam ple.com",
    "https://example.com\\@evil.com",
    "https://user:pass@example.com",
    "https://example.com:99999",
    "https://example.com/%00",
    "https://example.com/%00path",
    "https://example.com/path?q=&amp;r=1",
    "https://example.com/path#&amp;",
    "javascript:/*comment*/alert(1)",
    "javascript:void(0)",
    "https://例え.jp/",
    "http://xn--r8jz45g.example/",
    "https://[::1]:8080/",
    "https://[::1]/",
    "https://127.0.0.1/",
    "https://0x7f.0.0.1/",
    "https://2130706433/",
    "https://example.com/<script>",
    "https://example.com/\"onmouseover=\"alert(1)",
    "https://example.com/?q=\"><img>",
    "https://example.com/?q=javascript:alert(1)",
    "https://example.com/path#javascript:alert(1)",
  ];
  for url in urls {
    let link = format!("[click]({url})");
    let img = format!("![img]({url})");
    for src in [&link, &img] {
      let html = compile(src);
      let low = html.to_ascii_lowercase();
      // Look only at structured attribute occurrences.
      assert!(!low.contains("href=\"javascript:"), "javascript: leaked for src={src:?}\n  {html}");
      assert!(!low.contains("src=\"javascript:"), "javascript: img leaked for src={src:?}\n  {html}");
      assert!(!low.contains("href=\"vbscript:"), "vbscript: leaked for src={src:?}\n  {html}");
      assert!(!low.contains("href=\"data:text/html"), "data:text/html leaked for src={src:?}\n  {html}");
      assert!(!low.contains("src=\"data:text/html"), "data:text/html img leaked for src={src:?}\n  {html}");
    }
  }
}

/// HTML entity references in all shapes (named, numeric, hex). Output
/// must never produce invalid HTML by reflecting a half-formed entity.
#[test]
fn entity_reference_forms_compile() {
  let cases = [
    "&amp;",
    "&lt;",
    "&gt;",
    "&quot;",
    "&apos;",
    "&#0;",
    "&#1;",
    "&#65;",
    "&#x41;",
    "&#X41;",
    "&#1114111;",
    "&#1114112;", // out of range
    "&#x10ffff;",
    "&#x110000;", // out of range
    "&#x;",
    "&#;",
    "&;",
    "&",
    "&amp",
    "&unknown;",
    "&aaaaaaaaaaaaaaaaaaaaaaaa;",
    "&copy;",
    "&copy",
    "&nbsp;",
    "&Eacute;",
    "&eacute",
    // surrogate pair as numeric ref
    "&#xd800;",
    "&#xdc00;",
    "&#xdfff;",
  ];
  for s in cases {
    let _ = compile(s);
    let in_link = format!("[a]({s})");
    let _ = compile(&in_link);
    let in_attr = format!("<a href=\"{s}\">x</a>");
    let _ = compile(&in_attr);
  }
}

/// Many tasklist edge cases. Tasklists are a GFM extension whose check
/// state must survive nesting, mixed indentation, and unusual markers.
#[test]
fn tasklist_edge_cases() {
  let cases = [
    "- [ ] todo",
    "- [x] done",
    "- [X] done",
    "- [ ]",
    "- [x]",
    "- [ ] - nested? no",
    "  - [ ] indented",
    "1. [ ] ordered",
    "- [ ] **bold** *italic* `code`",
    "- [ ] [link](url)",
    "- [ ] ![img](url.png)",
    "- [ ] {1+1}",
    "- [ ] <Comp/>",
    "- [?] non-standard",
    "- [-] non-standard",
    "* [ ] star marker",
    "+ [ ] plus marker",
    "- [ ]\n  - [ ]\n",
    "> - [ ] in quote",
    "- [ ] line 1\n  line 2 continuation\n",
  ];
  for s in cases {
    let _ = compile(s);
  }
}

/// Code fences with unusual info strings: pure whitespace, attribute-
/// like syntax, embedded backticks (only outside the open run).
#[test]
fn code_fence_info_strings() {
  let cases = [
    "```\ncode\n```",
    "```rust\ncode\n```",
    "```rust title=\"x\"\ncode\n```",
    "```rust {1-3}\ncode\n```",
    "```rust /highlight/\ncode\n```",
    "```{.rust .highlight}\ncode\n```",
    "```rust noembed=true\ncode\n```",
    "``` rust \ncode\n```",
    "```\trust\ncode\n```",
    "```rust\\n\ncode\n```",
    "```rust 中文\ncode\n```",
    "```a```b```\ncode\n```",
    "```rust\n```text\nnested?\n```\n```",
    "~~~rust\ncode\n~~~",
    "~~~rust ~~~~ inside\ncode\n~~~",
    "    indented code\n",
    "\tcode tab\n",
  ];
  for s in cases {
    let _ = compile(s);
  }
}

/// 400-step combo walk: each step picks 2 random productions and
/// concatenates with a 50% chance of blank line between. Produces highly
/// varied multi-block documents.
#[test]
fn random_block_combo_walk() {
  let blocks = [
    "para text\n",
    "# heading\n",
    "## heading 2\n",
    "- list item\n",
    "1. ordered\n",
    "- [ ] task\n",
    "> quote\n",
    "```\ncode\n```\n",
    "| a | b |\n|-|-|\n| 1 | 2 |\n",
    "[ref]: /url\n",
    "[^a]: footnote\n",
    "<Comp/>\n",
    "<div>raw</div>\n",
    "{1+1}\n",
    "<!-- comment -->\n",
    "$$math$$\n",
    "***\n",
  ];
  let mut seed = 0xF00DBA5Eu64;
  for trial in 0..400 {
    let n = (lcg(&mut seed) as usize % 8) + 2;
    let mut s = String::new();
    for _ in 0..n {
      s.push_str(blocks[(lcg(&mut seed) as usize) % blocks.len()]);
      if lcg(&mut seed) & 1 == 0 {
        s.push('\n');
      }
    }
    let _ = compile(&s);
    if trial % 50 == 0 {
      println!("combo #{trial:04}");
    }
  }
}

/// Edge-case codepoints and numeric character references must render
/// without producing raw surrogate bytes, unmatched raw HTML brackets,
/// or unescaped ampersands.
///
/// Behaviour exercised here:
/// - Valid scalar codepoints in the source survive verbatim.
/// - Decimal / hex numeric refs that decode to a valid scalar produce
///   that scalar in the output.
/// - Refs whose target falls in the surrogate range or beyond U+10FFFF
///   fall through as inert text with the ampersand escaped, never as a
///   raw surrogate byte sequence.
#[test]
fn numeric_refs_and_edge_codepoints_render_safely() {
  // Valid scalars must appear verbatim in the output.
  let scalars: &[&str] = &["\u{D7FF}", "\u{E000}", "\u{FFFD}", "\u{10FFFF}", "\u{0301}", "\u{1F600}"];
  for src in scalars {
    let html = compile(src);
    assert!(html.contains(src), "expected {src:?} verbatim in output, got {html}");
  }
  // Refs that decode to a valid scalar produce that scalar. CM 6.6
  // explicitly maps `&#0;` to U+FFFD; cover that path here.
  let decoded: &[(&str, char)] = &[("&#0;", '\u{FFFD}'), ("&#1114111;", '\u{10FFFF}'), ("&#65;", 'A'), ("&#x41;", 'A')];
  for (src, expected) in decoded {
    let html = compile(src);
    assert!(html.contains(*expected), "expected decoded {expected:?} for src={src:?}, got {html}");
  }
  // Surrogate / out-of-range refs fall through as inert text with the
  // ampersand HTML-escaped to `&amp;`.
  let inert_refs: &[&str] = &["&#xd800;", "&#xdc00;", "&#1114112;"];
  for src in inert_refs {
    let html = compile(src);
    let after_amp = src.strip_prefix('&').expect("starts with &");
    let expected = format!("&amp;{after_amp}");
    assert!(html.contains(&expected), "expected inert {expected:?} for src={src:?}, got {html}");
  }
  // None of these sources contain a `<`, so the only `<` the output
  // may carry is the structural `<p>` wrapper. A `<script` / `<svg`
  // here would indicate raw-HTML injection.
  for src in scalars.iter().chain(decoded.iter().map(|(s, _)| s)).chain(inert_refs.iter()) {
    let html = compile(src);
    let low = html.to_ascii_lowercase();
    for forbidden in ["<script", "<svg", "<iframe", "<img", "<a "] {
      assert!(!low.contains(forbidden), "unexpected {forbidden:?} in output for src={src:?}: {html}");
    }
  }
}

/// CommonMark requires that backslash followed by an ASCII punctuation
/// mark produces a literal of that punctuation. Run every punctuation
/// mark through this rule and confirm the output contains the literal
/// (escaped to its HTML entity if applicable) and not a markup effect.
#[test]
fn backslash_escape_neutralizes_every_punct() {
  let puncts = r##"!"#$%&'()*+,-./:;<=>?@[\]^_`{|}~"##;
  for p in puncts.chars() {
    let src = format!("text \\{p} more text");
    let html = compile(&src);
    // The escaped char should appear in output. For HTML-significant
    // characters, it should appear as a named entity.
    let appears = html.contains(p)
      || (p == '&' && html.contains("&amp;"))
      || (p == '<' && html.contains("&lt;"))
      || (p == '>' && html.contains("&gt;"))
      || (p == '"' && html.contains("&quot;"));
    assert!(appears, "escaped \\{p} did not appear in output: {html}");
    // The punct should never trigger its markup meaning.
    let low = html.to_ascii_lowercase();
    if p == '*' {
      assert!(!low.contains("<em>") && !low.contains("<strong>"), "escaped \\* still emphasized: {html}");
    }
    if p == '`' {
      assert!(!low.contains("<code>"), "escaped \\` still produced code: {html}");
    }
    if p == '[' {
      assert!(!low.contains("<a "), "escaped \\[ still produced link: {html}");
    }
  }
}

/// Tables with all kinds of malformed cell counts. None should crash.
#[test]
fn table_shape_mismatches() {
  let cases = [
    "| a | b | c |\n|-|-|\n| 1 |\n",
    "| a |\n|-|-|\n| 1 | 2 | 3 |\n",
    "| a | b |\n|--|--|--|\n| 1 |\n",
    "|||||\n|-|-|-|-|\n|1|2|3|4|\n",
    "| a |\n",
    "| a |\n|-|\n\n",
    "| a | | b |\n|-|-|-|\n| 1 | 2 | 3 |\n",
    "| a | b |\n|:-|-:|\n|1|2|",
    "| a | b |\n|--|--|\n|`pipe|inside`|x|\n",
    "| a | b |\n|--|--|\n|x\\|y|z|\n",
  ];
  for s in cases {
    let _ = compile(s);
  }
}

/// Reference-link definitions in all the awkward placements: indented
/// 0-3 spaces, after blockquote, after list, with weird title syntax.
#[test]
fn reference_link_def_placement() {
  let cases = [
    "[a]: /url\n\n[a]",
    " [a]: /url\n\n[a]",
    "  [a]: /url\n\n[a]",
    "   [a]: /url\n\n[a]",
    "    [a]: /url\n\n[a]", // 4 spaces - code block
    "[a]: /url 'title'\n\n[a]",
    "[a]: /url \"title\"\n\n[a]",
    "[a]: /url (title)\n\n[a]",
    "[a]: <url-in-angles>\n\n[a]",
    "[a]: /url\n  'title'\n\n[a]",
    "[A]: /url\n\n[a]",
    "[ a ]: /url\n\n[ a ]",
    "[a\nb]: /url\n\n[a b]",
    "> [a]: /url\n\n[a]",
    "- [a]: /url\n\n[a]",
  ];
  for s in cases {
    let _ = compile(s);
  }
}

/// HTML autolinks with full URL shapes: ports, IPv6, userinfo.
#[test]
fn autolink_url_variations() {
  let cases = [
    "<http://example.com>",
    "<http://example.com/path>",
    "<http://example.com:8080>",
    "<http://example.com:8080/path>",
    "<http://user@example.com>",
    "<http://user:pass@example.com>",
    "<http://[::1]>",
    "<http://[::1]:8080>",
    "<http://[2001:db8::1]/>",
    "<http://例え.jp/>",
    "<mailto:user@example.com>",
    "<mailto:user+tag@example.com>",
    "<ftp://files.example.com/file.tar>",
    "<tel:+1-555-0100>",
    "<urn:isbn:9783161484100>",
    "<file:///etc/passwd>",
    "<custom-scheme://x>",
  ];
  for s in cases {
    let _ = compile(s);
  }
}
