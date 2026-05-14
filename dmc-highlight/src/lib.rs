//! Bundled `syntect` syntax + theme registry, loaded once per process.
//! Sources live in `assets/themes-bat/` and `assets/grammars-sublime/`;
//! `build.rs` emits the `Theme` + `Grammar` enums and the `THEMES` /
//! `GRAMMARS` slices included below.
//!
//! User-facing walkthrough: ../../dmc-docs/dmc-highlight/

use std::collections::BTreeMap;
use std::io::Cursor;
use std::sync::OnceLock;

use include_dir::{Dir, include_dir};
use syntect::easy::HighlightLines;
use syntect::highlighting::{HighlightState, Highlighter, RangedHighlightIterator, Style, ThemeSet};
use syntect::parsing::{ParseState, ScopeStack, SyntaxDefinition, SyntaxSet, SyntaxSetBuilder};
use syntect::util::LinesWithEndings;

// Embedded so the compiled `.node` is self-contained — the build-time
// CARGO_MANIFEST_DIR path would not resolve on any other machine.
static GRAMMARS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/grammars-sublime");
static THEMES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/themes-bat");

/// Re-exported `syntect` types so callers can consume highlight output
/// without depending on `syntect` directly.
pub use syntect::highlighting::{Color, FontStyle as HlFontStyle, Style as HlStyle};

include!(concat!(env!("OUT_DIR"), "/assets_gen.rs"));

/// Bundled themes + grammars. Lazily built by [`SyntaxBundle::get`] and
/// cached for the process lifetime.
pub struct SyntaxBundle {
  pub syntaxes: SyntaxSet,
  pub themes: ThemeSet,
}

impl SyntaxBundle {
  /// Global bundle. ~25-100 ms one-time parse cost on first call.
  pub fn get() -> &'static SyntaxBundle {
    static B: OnceLock<SyntaxBundle> = OnceLock::new();
    B.get_or_init(|| {
      // `add_plain_text_syntax` registers the fallback used by
      // `find_syntax_plain_text` for unknown languages.
      let mut builder = SyntaxSetBuilder::new();
      for f in GRAMMARS_DIR.files() {
        if f.path().extension().and_then(|s| s.to_str()) != Some("sublime-syntax") {
          continue;
        }
        let yaml = std::str::from_utf8(f.contents()).expect("sublime-syntax is utf8");
        let def = SyntaxDefinition::load_from_str(yaml, true, None).expect("parse sublime-syntax");
        builder.add(def);
      }
      builder.add_plain_text_syntax();
      let syntaxes: SyntaxSet = builder.build();

      // Key by file stem so the generated enum's `name()` lookups match.
      let mut themes_map: BTreeMap<String, syntect::highlighting::Theme> = BTreeMap::new();
      for f in THEMES_DIR.files() {
        if f.path().extension().and_then(|s| s.to_str()) != Some("tmTheme") {
          continue;
        }
        let stem = f.path().file_stem().and_then(|s| s.to_str()).expect("theme stem").to_string();
        let theme = ThemeSet::load_from_reader(&mut Cursor::new(f.contents())).expect("parse tmTheme");
        themes_map.insert(stem, theme);
      }
      let themes = ThemeSet { themes: themes_map };

      SyntaxBundle { syntaxes, themes }
    })
  }

  /// Sorted list of every bundled theme name (stable order via `BTreeMap`).
  /// Used to validate user-configured theme names and produce "did you mean"
  /// hints.
  pub fn bundled_theme_names(&self) -> Vec<&str> {
    self.themes.themes.keys().map(String::as_str).collect()
  }
}

/// Free-function alias of `SyntaxBundle::get().bundled_theme_names()`.
pub fn list_bundled_themes() -> Vec<&'static str> {
  SyntaxBundle::get().bundled_theme_names()
}

impl SyntaxBundle {
  /// Highlight `code`. Returns one `Vec<(Style, &str)>` per source line;
  /// unknown `lang` falls back to plain-text grammar.
  pub fn highlight<'a>(&'a self, code: &'a str, lang: Grammar, theme: Theme) -> Vec<Vec<(Style, &'a str)>> {
    let syntax =
      self.syntaxes.find_syntax_by_token(lang.name()).unwrap_or_else(|| self.syntaxes.find_syntax_plain_text());
    let theme = self.themes.themes.get(theme.name()).expect("theme present in bundle");
    let mut h = HighlightLines::new(syntax, theme);
    LinesWithEndings::from(code).map(|line| h.highlight_line(line, &self.syntaxes).unwrap_or_default()).collect()
  }

  /// As [`Self::highlight`] but with a free-form grammar name (e.g. `"rs"`,
  /// `"Rust"`), for callers without a `Grammar` enum value.
  pub fn highlight_by_name<'a>(&'a self, code: &'a str, lang: &str, theme: Theme) -> Vec<Vec<(Style, &'a str)>> {
    let syntax = self
      .syntaxes
      .find_syntax_by_extension(lang)
      .or_else(|| self.syntaxes.find_syntax_by_token(lang))
      .or_else(|| self.syntaxes.find_syntax_by_name(lang))
      .unwrap_or_else(|| self.syntaxes.find_syntax_plain_text());
    let theme = self.themes.themes.get(theme.name()).expect("theme present in bundle");
    let mut h = HighlightLines::new(syntax, theme);
    LinesWithEndings::from(code).map(|line| h.highlight_line(line, &self.syntaxes).unwrap_or_default()).collect()
  }
}

/// Highlight `code` by extension/token/name + bundled theme name. Both
/// lookups are forgiving: unknown `lang` falls back to plain text, unknown
/// `theme_name` falls back to the first bundled theme. Returns one
/// `Vec<(Style, &str)>` per source line.
pub fn highlight_code<'a>(code: &'a str, lang: Option<&str>, theme_name: &str) -> Vec<Vec<(Style, &'a str)>> {
  let bundle = SyntaxBundle::get();
  let syntax = lang
    .and_then(|l| {
      bundle
        .syntaxes
        .find_syntax_by_extension(l)
        .or_else(|| bundle.syntaxes.find_syntax_by_token(l))
        .or_else(|| bundle.syntaxes.find_syntax_by_name(l))
    })
    .unwrap_or_else(|| bundle.syntaxes.find_syntax_plain_text());
  let theme = bundle
    .themes
    .themes
    .get(theme_name)
    .or_else(|| bundle.themes.themes.values().next())
    .expect("at least one theme bundled");
  let mut h = HighlightLines::new(syntax, theme);
  LinesWithEndings::from(code).map(|line| h.highlight_line(line, &bundle.syntaxes).unwrap_or_default()).collect()
}

/// One highlighted token: source slice plus per-theme styles. `styles`
/// is indexed parallel to the `theme_names` passed to [`highlight_code_multi`].
#[derive(Debug, Clone)]
pub struct MultiToken<'a> {
  pub text: &'a str,
  pub styles: Vec<Style>,
}

fn styles_match(a: &[Style], b: &[Style]) -> bool {
  a.len() == b.len()
    && a
      .iter()
      .zip(b.iter())
      .all(|(x, y)| x.foreground == y.foreground && x.background == y.background && x.font_style == y.font_style)
}

/// Concatenate `a` and `b` if they are adjacent regions of the same `&str`.
fn join_adjacent<'a>(a: &'a str, b: &'a str) -> Option<&'a str> {
  let a_end = a.as_ptr() as usize + a.len();
  let b_start = b.as_ptr() as usize;
  if a_end != b_start {
    return None;
  }
  // SAFETY: adjacent regions of the same `&str` form a valid UTF-8 slice.
  let bytes = unsafe { std::slice::from_raw_parts(a.as_ptr(), a.len() + b.len()) };
  std::str::from_utf8(bytes).ok()
}

/// Highlight `code` once against multiple themes. Parse + scope walk run
/// once; each theme contributes only color resolution. Cost is
/// `O(parse) + O(themes * scope_walk)` instead of
/// `O(themes * (parse + scope_walk))`.
///
/// Token boundaries come from grammar scope changes (theme-independent),
/// so every theme's styles align on the same source slices.
pub fn highlight_code_multi<'a>(code: &'a str, lang: Option<&str>, theme_names: &[&str]) -> Vec<Vec<MultiToken<'a>>> {
  let bundle = SyntaxBundle::get();
  let syntax = lang
    .and_then(|l| {
      bundle
        .syntaxes
        .find_syntax_by_extension(l)
        .or_else(|| bundle.syntaxes.find_syntax_by_token(l))
        .or_else(|| bundle.syntaxes.find_syntax_by_name(l))
    })
    .unwrap_or_else(|| bundle.syntaxes.find_syntax_plain_text());
  let fallback = bundle.themes.themes.values().next().expect("at least one bundled theme");
  let themes: Vec<&_> = theme_names.iter().map(|n| bundle.themes.themes.get(*n).unwrap_or(fallback)).collect();
  let highlighters: Vec<Highlighter> = themes.iter().map(|t| Highlighter::new(t)).collect();
  let mut highlight_states: Vec<HighlightState> =
    highlighters.iter().map(|h| HighlightState::new(h, ScopeStack::new())).collect();
  let mut parse_state = ParseState::new(syntax);

  let mut out = Vec::new();
  for line in LinesWithEndings::from(code) {
    let ops = parse_state.parse_line(line, &bundle.syntaxes).unwrap_or_default();

    // Drive each theme's iterator over the SAME `ops` slice: boundaries
    // align because every iter walks identical scope-change positions.
    let mut per_theme: Vec<Vec<(Style, &str)>> = Vec::with_capacity(theme_names.len());
    for (i, st) in highlight_states.iter_mut().enumerate() {
      let toks: Vec<(Style, &str)> =
        RangedHighlightIterator::new(st, &ops, line, &highlighters[i]).map(|(style, text, _)| (style, text)).collect();
      per_theme.push(toks);
    }

    let token_count = per_theme.iter().map(Vec::len).min().unwrap_or(0);
    let mut tokens: Vec<MultiToken> = Vec::with_capacity(token_count);
    for tok_i in 0..token_count {
      let text = per_theme[0][tok_i].1;
      let styles: Vec<Style> = per_theme.iter().map(|v| v[tok_i].0).collect();
      // Coalesce with the previous token when every theme produces an
      // identical style — matches shiki / rehype-pretty-code span counts.
      if let Some(prev) = tokens.last_mut()
        && styles_match(&prev.styles, &styles)
        && let Some(joined) = join_adjacent(prev.text, text)
      {
        prev.text = joined;
        continue;
      }
      tokens.push(MultiToken { text, styles });
    }
    out.push(tokens);
  }
  out
}
