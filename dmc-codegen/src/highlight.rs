//! Bundled `syntect` syntax + theme registry. Loaded once per process,
//! shared across every code-block render. Theme + grammar conversion
//! lives in `dmc-codegen/scripts/convert-shiki-assets.mjs`; the
//! converted plist files live in `assets/{themes,grammars}/`.

use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// Themes + grammars ready for `syntect` highlighting calls.
pub struct SyntaxBundle {
  pub syntaxes: SyntaxSet,
  pub themes: ThemeSet,
}

impl<'a> SyntaxBundle {
  /// Lazy-init global bundle. Loads from `assets/` on first use; ~25-75 ms
  /// cold (theme + grammar parse), then free for every subsequent render.
  pub fn bundle() -> &'static SyntaxBundle {
    static B: OnceLock<SyntaxBundle> = OnceLock::new();
    B.get_or_init(|| {
      let syntaxes = SyntaxSet::load_from_folder(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/grammars"))
        .expect("load syntect grammars");
      let themes =
        ThemeSet::load_from_folder(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/themes")).expect("load syntect themes");
      SyntaxBundle { syntaxes, themes }
    })
  }

  pub fn highlight_code(code: &'a str, lang: Option<&'a str>, theme_name: &str) -> Vec<Vec<(Style, &'a str)>> {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = lang
      .and_then(|l| ss.find_syntax_by_extension(l).or_else(|| ss.find_syntax_by_name(l)))
      .unwrap_or_else(|| ss.find_syntax_plain_text());

    let theme = ts.themes.get(theme_name).unwrap_or_else(|| &ts.themes["base16-ocean.dark"]);
    let mut h = HighlightLines::new(syntax, theme);

    LinesWithEndings::from(code).map(|line| h.highlight_line(line, &ss).unwrap_or_default()).collect()
  }
}
