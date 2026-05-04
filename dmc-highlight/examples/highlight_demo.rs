//! Quick syntect smoke test. Highlights a TSX snippet with a built-in
//! theme and writes the result as a self-contained HTML file.
//!
//! Run:
//!   cargo run --release -p dmc-highlight --example highlight_demo
//! Then open `dmc-highlight/tmp/highlight.html` in a browser.

use dmc_highlight::{Grammar, SyntaxBundle, Theme};
use std::fs;
use std::path::PathBuf;
use syntect::html::highlighted_html_for_string;

const SAMPLE: &str = r#"
function renderRadioGroup(props: Record<string, unknown> = {}) {
  return render(
    <RadioGroup {...props}>
      <RadioGroupItem value="a">
        <RadioGroupIndicator />
      </RadioGroupItem>
      <RadioGroupItem value="b">
        <RadioGroupIndicator />
      </RadioGroupItem>
      <RadioGroupItem value="c">
        <RadioGroupIndicator />
      </RadioGroupItem>
    </RadioGroup>,
  )
}
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let bundle = SyntaxBundle::get();
  let theme = Theme::CatppuccinMocha;
  let grammar = Grammar::TypsecriptReact;

  let theme_obj = bundle.themes.themes.get(theme.name()).ok_or("theme missing in bundle")?;
  let syntax = bundle
    .syntaxes
    .find_syntax_by_extension("tsx")
    .or_else(|| bundle.syntaxes.find_syntax_by_name("TypeScriptReact"))
    .or_else(|| bundle.syntaxes.find_syntax_by_token(grammar.name()))
    .ok_or("grammar missing in bundle")?;

  let body = highlighted_html_for_string(SAMPLE, &bundle.syntaxes, syntax, theme_obj)?;

  let bg = theme_obj.settings.background.unwrap_or(syntect::highlighting::Color { r: 13, g: 17, b: 23, a: 255 });
  let fg = theme_obj.settings.foreground.unwrap_or(syntect::highlighting::Color { r: 220, g: 220, b: 220, a: 255 });
  let bg_css = format!("#{:02x}{:02x}{:02x}", bg.r, bg.g, bg.b);
  let fg_css = format!("#{:02x}{:02x}{:02x}", fg.r, fg.g, fg.b);
  let theme_name = theme.name();
  let grammar_name = grammar.name();

  let html = format!(
    r#"<!doctype html>
<html><head><meta charset="utf-8"><title>highlight demo</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@700&display=swap" rel="stylesheet">
<style>
  body {{ background:{bg_css}; color:{fg_css}; font-family: "JetBrains Mono", ui-monospace, monospace; font-weight: 700; padding: 2rem; }}
  pre  {{ font-family: "JetBrains Mono", ui-monospace, monospace; font-weight: 700; font-size: 14px; line-height: 1.5; padding: 1rem; border-radius: 6px; overflow-x: auto; }}
  pre code, pre span {{ font-family: inherit; font-weight: inherit; }}
  h1   {{ font-family: ui-sans-serif, system-ui; font-weight: 500; font-size: 1.1rem; margin-bottom: 1rem; opacity: 0.7; }}
</style>
</head><body>
<h1>syntect highlight demo (theme: {theme_name}, lang: {grammar_name})</h1>
{body}
</body></html>
"#
  );

  let out_dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tmp"));
  fs::create_dir_all(&out_dir)?;
  let out = out_dir.join("highlight.html");
  fs::write(&out, html)?;
  println!("wrote {}", out.display());
  Ok(())
}
