//! User-facing walkthrough: ../../dmc-docs/dmc-highlight/
//! Run `cargo doc --open -p dmc-highlight` for the inline rustdoc.

//! Bundled `syntect` syntax + theme registry. Loaded once per process,
//! shared across every code-block render. Sources live in
//! `assets/themes-bat/` (themes) and `assets/grammars-sublime/` (grammars).
//! `build.rs` scans both dirs and emits the `Theme` + `Grammar` enums
//! plus the `THEMES` / `GRAMMARS` slices included below.

use std::collections::BTreeMap;
use std::io::Cursor;
use std::sync::OnceLock;

use include_dir::{Dir, include_dir};
use syntect::easy::HighlightLines;
use syntect::highlighting::{HighlightState, Highlighter, RangedHighlightIterator, Style, ThemeSet};
use syntect::parsing::{ParseState, ScopeStack, SyntaxDefinition, SyntaxSet, SyntaxSetBuilder};
use syntect::util::LinesWithEndings;

// Embed the grammar + theme assets directly into the compiled binary so
// the resulting `.node` is self-contained and never reaches for the
// CARGO_MANIFEST_DIR build-time path at runtime (which would panic on
// any machine other than the one that compiled it).
static GRAMMARS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/grammars-sublime");
static THEMES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/themes-bat");

/// Re-exports of the `syntect` types that callers (e.g. the `pretty-code`
/// transformer) need to consume highlight output without depending on
/// `syntect` themselves.
pub use syntect::highlighting::{Color, FontStyle as HlFontStyle, Style as HlStyle};

include!(concat!(env!("OUT_DIR"), "/assets_gen.rs"));

/// Themes + grammars loaded from the bundled assets, ready for highlight
/// calls. Constructed lazily by [`SyntaxBundle::get`] and cached for the
/// process lifetime.
pub struct SyntaxBundle {
  pub syntaxes: SyntaxSet,
  pub themes: ThemeSet,
}

impl SyntaxBundle {
  /// Global bundle. ~25-100 ms one-time parse cost on first call (themes
  /// + grammars), free thereafter.
  pub fn get() -> &'static SyntaxBundle {
    static B: OnceLock<SyntaxBundle> = OnceLock::new();
    B.get_or_init(|| {
      // Build the SyntaxSet from the in-binary `assets/grammars-sublime`
      // bundle. `add_plain_text_syntax` registers the fallback grammar
      // that `find_syntax_plain_text` returns for unknown languages.
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

      // Themes ship as `.tmTheme` plist files; load each with
      // `ThemeSet::load_from_reader` and key by file stem so the existing
      // `name()` lookups in the generated enum keep working.
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

  /// Sorted list of every bundled theme name. Stable across calls because
  /// the bundle's `BTreeMap` iterates in sorted order. Used by upstream
  /// consumers (e.g. PrettyCode) to validate user-configured theme names
  /// at startup and surface a "did you mean" hint when one is missing.
  pub fn bundled_theme_names(&self) -> Vec<&str> {
    self.themes.themes.keys().map(String::as_str).collect()
  }
}

/// Free-function alias around `SyntaxBundle::get().bundled_theme_names()`.
/// Kept separate so callers can probe theme availability without holding
/// onto the bundle reference.
pub fn list_bundled_themes() -> Vec<&'static str> {
  SyntaxBundle::get().bundled_theme_names()
}

impl SyntaxBundle {
  // (continued below — split to keep the helper next to its method)

  /// Highlight `code` with the given grammar + theme. Returns one
  /// `Vec<(Style, &str)>` per source line. Falls back to plain-text
  /// grammar when `lang` is unknown.
  pub fn highlight<'a>(&'a self, code: &'a str, lang: Grammar, theme: Theme) -> Vec<Vec<(Style, &'a str)>> {
    let syntax =
      self.syntaxes.find_syntax_by_token(lang.name()).unwrap_or_else(|| self.syntaxes.find_syntax_plain_text());
    let theme = self.themes.themes.get(theme.name()).expect("theme present in bundle");
    let mut h = HighlightLines::new(syntax, theme);
    LinesWithEndings::from(code).map(|line| h.highlight_line(line, &self.syntaxes).unwrap_or_default()).collect()
  }

  /// As [`Self::highlight`] but takes a free-form grammar name (e.g. `"rs"`,
  /// `"Rust"`). Useful when callers don't have a `Grammar` enum value
  /// (e.g., from user config).
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

/// Highlight `code` using a grammar identified by extension/token/name and
/// a theme identified by its bundled name. Both lookups are forgiving:
/// unknown `lang` falls back to plain text (so build never errors on niche
/// languages) and unknown `theme_name` falls back to the first bundled
/// theme. One `Vec<(Style, &str)>` per source line.
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

/// One highlighted token: source slice plus per-theme styles, indexed in
/// the same order as the `theme_names` slice passed to
/// [`highlight_code_multi`].
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

/// Concatenate two source slices when they're adjacent in the original
/// string. Returns `None` when they are not (i.e. shouldn't be merged).
fn join_adjacent<'a>(a: &'a str, b: &'a str) -> Option<&'a str> {
  let a_end = a.as_ptr() as usize + a.len();
  let b_start = b.as_ptr() as usize;
  if a_end != b_start {
    return None;
  }
  // SAFETY: the two slices are adjacent regions of the same `&str`,
  // so concatenating them yields a valid UTF-8 slice of that string.
  let bytes = unsafe { std::slice::from_raw_parts(a.as_ptr(), a.len() + b.len()) };
  std::str::from_utf8(bytes).ok()
}

/// Highlight `code` once against multiple themes. The grammar parse and
/// scope-stack walk happen exactly once; each theme contributes only its
/// color resolution. Cost scales as `O(parse) + O(themes * scope_walk)`
/// rather than `O(themes * (parse + scope_walk))`, halving (or better)
/// per-file cost vs N independent calls to [`highlight_code`].
///
/// Token boundaries are theme-independent (they come from grammar scope
/// changes), so all themes contribute styles for the same source slices.
/// Returns one `Vec<MultiToken>` per source line.
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

    // Drive each theme's RangedHighlightIterator to completion against the
    // SAME `ops` slice. Boundaries align across themes because each iter
    // walks identical scope-change positions.
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
      // Merge with previous token when every theme produces the same
      // style. Matches shiki's adjacent-same-style coalescing so the
      // emitted span count tracks rehype-pretty-code output.
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
