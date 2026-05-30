//! Hardening probes for syntect bundle: arbitrary code snippets across
//! known + unknown languages must not panic and must reproduce input
//! lines verbatim after concatenation.

use dmc_highlight::highlight_code;

const SNIPPETS: &[(&str, &str)] = &[
  ("rust", "fn main() { println!(\"hi\"); }\n"),
  ("rs", "let x: u32 = 1;\n"),
  ("ts", "const x: number = 1;\n"),
  ("tsx", "const C = () => <div/>;\n"),
  ("js", "function f() { return 1; }\n"),
  ("jsx", "const X = <Y/>;\n"),
  ("py", "def f():\n  return 1\n"),
  ("go", "func main() { fmt.Println(\"hi\") }\n"),
  ("c", "int main(){return 0;}\n"),
  ("cpp", "int main(){return 0;}\n"),
  ("java", "class X { void f(){} }\n"),
  ("sh", "echo hi\n"),
  ("bash", "for i in 1 2 3; do echo $i; done\n"),
  ("toml", "[a]\nx = 1\n"),
  ("yaml", "a: 1\nb: 2\n"),
  ("json", "{\"a\": 1}\n"),
  ("css", ".x { color: red; }\n"),
  ("html", "<div>x</div>\n"),
  ("md", "# H\nbody\n"),
  ("sql", "SELECT 1;\n"),
  // unknown lang fallback
  ("unknown-lang", "raw text\n"),
  ("", "no lang\n"),
  ("klingon", "qapla'\n"),
  // edge inputs
  ("rust", ""),
  ("rust", "\n"),
  ("rust", "// only comment\n"),
  ("rust", "/* unterminated\n"),
  ("rust", "fn 🦆() {}\n"),
  ("rust", "let s = \"unicode 🦆 string\";\n"),
  ("ts", "type X = `template literal`;\n"),
];

#[test]
fn highlight_round_trip_preserves_text() {
  for (i, (lang, code)) in SNIPPETS.iter().enumerate() {
    let lines = highlight_code(code, Some(lang), "InspiredGitHub");
    let mut joined = String::new();
    for line in &lines {
      for (_style, slice) in line {
        joined.push_str(slice);
      }
    }
    assert_eq!(joined, *code, "highlight #{i} ({lang}) lost text");
    println!("highlight #{i:03} lang={lang} bytes={}", code.len());
  }
}

#[test]
fn highlight_unknown_lang_falls_back_to_plain() {
  let lines = highlight_code("raw text\n", Some("ya-but-no"), "InspiredGitHub");
  assert!(!lines.is_empty(), "fallback produced no lines");
}

#[test]
fn highlight_unknown_theme_does_not_panic() {
  let _ = highlight_code("fn x(){}\n", Some("rust"), "this-theme-does-not-exist");
}

#[test]
fn highlight_empty_code_returns_empty_lines() {
  let lines = highlight_code("", Some("rust"), "InspiredGitHub");
  assert!(lines.is_empty() || lines.iter().all(|l| l.is_empty()));
}
