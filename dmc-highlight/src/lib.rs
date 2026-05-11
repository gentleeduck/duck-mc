//! User-facing walkthrough: ../../dmc-docs/dmc-highlight/
//! Run `cargo doc --open -p dmc-highlight` for the inline rustdoc.

//! Bundled `syntect` syntax + theme registry. Loaded once per process,
//! shared across every code-block render. Sources live in
//! `assets/themes-bat/` (themes) and `assets/grammars-sublime/` (grammars).
//! `build.rs` scans both dirs and emits the `Theme` + `Grammar` enums
//! plus the `THEMES` / `GRAMMARS` slices included below.

use std::collections::BTreeMap;
use std::io::Cursor;
use std::sync::{Mutex, OnceLock};

use include_dir::{Dir, include_dir};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, HighlightState, Highlighter, RangedHighlightIterator, Style, ThemeSet};
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
  // (continued below - split to keep the helper next to its method)

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

// --- Class-based ("classed") output: color-tuple token classes ---------
//
// Instead of an inline `style="color:#hex"` per token, the `pretty-code`
// transformer can emit `<span class="dmc-89b4fa-8839ef">` where each
// hyphen-joined segment is the token's foreground hex in one configured
// theme (canonical theme order), plus an optional font-style suffix
// (`-b` / `-i` / `-u` / combos) derived from the default-mode token.
//
// The class string -> per-theme (foreground, font_style) mapping is
// recorded in a process-global registry as renders happen; the engine
// then walks the registry once at build end to emit one
// `dmc.<mode>.css` per configured theme via [`token_css`].

#[allow(clippy::type_complexity)]
static TOKEN_CLASSES: OnceLock<Mutex<BTreeMap<String, Vec<(Color, FontStyle)>>>> = OnceLock::new();

fn token_classes() -> &'static Mutex<BTreeMap<String, Vec<(Color, FontStyle)>>> {
  TOKEN_CLASSES.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// Lowercase `#rrggbb` hex body (no `#`, alpha dropped).
fn hex6(c: Color) -> String {
  format!("{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

/// Font-style suffix for one token style: `b` bold, `i` italic, `u`
/// underline, in that order, concatenated (e.g. `bi`). Empty when the
/// style has no flags.
fn font_style_suffix(fs: FontStyle) -> String {
  let mut s = String::new();
  if fs.contains(FontStyle::BOLD) {
    s.push('b');
  }
  if fs.contains(FontStyle::ITALIC) {
    s.push('i');
  }
  if fs.contains(FontStyle::UNDERLINE) {
    s.push('u');
  }
  s
}

/// Returns the CSS class for a token given its per-theme `styles` and the
/// per-theme DEFAULT foreground colors (`default_fgs`, one per theme, same
/// order). Returns `None` when the token should get NO class - i.e. for
/// every theme the token's foreground equals that theme's default
/// foreground AND it carries no bold/italic/underline - so it inherits
/// from `.dmc-pre`. Otherwise records the
/// `class -> per-theme (foreground, font_style)` mapping in the process
/// registry and returns `Some(class)`, e.g. `"dmc-89b4fa-8839ef"` (two
/// themes) or `"dmc-89b4fa"` (one theme), with a font-style suffix when
/// the default-mode (index 0) token has any: `-b` bold, `-i` italic,
/// `-u` underline, combos `-bi` etc (order b, i, u).
pub fn token_class_name(styles: &[Style], default_fgs: &[Option<Color>]) -> Option<String> {
  let n = styles.len().min(default_fgs.len());
  if n == 0 {
    return None;
  }
  let skip = (0..n).all(|i| {
    let fg = styles[i].foreground;
    let def = default_fgs[i].unwrap_or(fg);
    def.r == fg.r && def.g == fg.g && def.b == fg.b && styles[i].font_style.is_empty()
  });
  if skip {
    return None;
  }

  let mut name = String::from("dmc-");
  for (i, st) in styles.iter().take(n).enumerate() {
    if i > 0 {
      name.push('-');
    }
    name.push_str(&hex6(st.foreground));
  }
  let suffix = font_style_suffix(styles[0].font_style);
  if !suffix.is_empty() {
    name.push('-');
    name.push_str(&suffix);
  }

  let tuple: Vec<(Color, FontStyle)> = styles.iter().take(n).map(|s| (s.foreground, s.font_style)).collect();
  token_classes().lock().expect("token-class registry mutex poisoned").entry(name.clone()).or_insert(tuple);
  Some(name)
}

/// Clear the token-class registry. Call at the start of a build so
/// watch-mode rebuilds do not accumulate stale classes.
pub fn reset_token_classes() {
  token_classes().lock().expect("token-class registry mutex poisoned").clear();
}

/// CSS for one theme/mode. For every recorded class (BTreeMap order =
/// deterministic), emit
/// `[data-theme="<mode>"] .<class> { color:#<hex>; [font-weight:bold;] [font-style:italic;] [text-decoration:underline;] }`
/// using the registry tuple entry at index `mode_idx`. Plus a root rule
/// `[data-theme="<mode>"] .dmc-pre { color:#<default_fg>; [background-color:#<bg>;] }`
/// (the `background-color` is omitted when `include_bg` is false). When
/// `mode` is the empty string (single unnamed theme) the `[data-theme]`
/// prefix is dropped and selectors are bare. `theme_name` is used only to
/// look up the theme's default foreground/background from the bundle
/// (falls back to the first bundled theme when not found, matching
/// [`highlight_code_multi`]'s fallback).
pub fn token_css(mode_idx: usize, mode: &str, theme_name: &str, include_bg: bool) -> String {
  let bundle = SyntaxBundle::get();
  let fallback = bundle.themes.themes.values().next().expect("at least one bundled theme");
  let theme = bundle.themes.themes.get(theme_name).unwrap_or(fallback);

  let prefix =
    |sel: &str| -> String { if mode.is_empty() { sel.to_string() } else { format!("[data-theme=\"{mode}\"] {sel}") } };

  let mut out = String::new();

  // Root rule on `.dmc-pre`.
  out.push_str(&prefix(".dmc-pre"));
  out.push_str(" {");
  if let Some(fg) = theme.settings.foreground {
    out.push_str(&format!(" color:#{};", hex6(fg)));
  }
  if include_bg && let Some(bg) = theme.settings.background {
    out.push_str(&format!(" background-color:#{};", hex6(bg)));
  }
  out.push_str(" }\n");

  let reg = token_classes().lock().expect("token-class registry mutex poisoned");
  for (class, tuple) in reg.iter() {
    let Some((fg, fs)) = tuple.get(mode_idx).copied() else { continue };
    out.push_str(&prefix(&format!(".{class}")));
    out.push_str(" {");
    out.push_str(&format!(" color:#{};", hex6(fg)));
    if fs.contains(FontStyle::BOLD) {
      out.push_str(" font-weight:bold;");
    }
    if fs.contains(FontStyle::ITALIC) {
      out.push_str(" font-style:italic;");
    }
    if fs.contains(FontStyle::UNDERLINE) {
      out.push_str(" text-decoration:underline;");
    }
    out.push_str(" }\n");
  }
  out
}

#[cfg(test)]
mod classed_tests {
  use super::*;

  fn style(r: u8, g: u8, b: u8, fs: FontStyle) -> Style {
    Style { foreground: Color { r, g, b, a: 0xff }, background: Color { r: 0, g: 0, b: 0, a: 0xff }, font_style: fs }
  }

  #[test]
  fn token_class_name_hex_segments_for_non_default_token() {
    let s = [style(0x89, 0xb4, 0xfa, FontStyle::empty()), style(0x88, 0x39, 0xef, FontStyle::empty())];
    let defs = [Some(Color { r: 0xcd, g: 0xd6, b: 0xf4, a: 0xff }), Some(Color { r: 0x4c, g: 0x4f, b: 0x69, a: 0xff })];
    assert_eq!(token_class_name(&s, &defs).as_deref(), Some("dmc-89b4fa-8839ef"));
  }

  #[test]
  fn token_class_name_none_when_all_default_no_style() {
    let def = Color { r: 0xcd, g: 0xd6, b: 0xf4, a: 0xff };
    let s = [style(0xcd, 0xd6, 0xf4, FontStyle::empty())];
    assert_eq!(token_class_name(&s, &[Some(def)]), None);
  }

  #[test]
  fn token_class_name_font_style_suffix() {
    let s = [style(0x89, 0xb4, 0xfa, FontStyle::BOLD | FontStyle::ITALIC)];
    let defs = [Some(Color { r: 0xcd, g: 0xd6, b: 0xf4, a: 0xff })];
    assert_eq!(token_class_name(&s, &defs).as_deref(), Some("dmc-89b4fa-bi"));
  }

  #[test]
  fn registry_populated_and_css_rendered_then_reset() {
    reset_token_classes();
    let s = [style(0x89, 0xb4, 0xfa, FontStyle::empty()), style(0x88, 0x39, 0xef, FontStyle::empty())];
    let defs = [Some(Color { r: 0xcd, g: 0xd6, b: 0xf4, a: 0xff }), Some(Color { r: 0x4c, g: 0x4f, b: 0x69, a: 0xff })];
    let cls = token_class_name(&s, &defs).expect("non-default token gets a class");

    let css = token_css(0, "dark", "Catppuccin Mocha", false);
    assert!(css.contains("[data-theme=\"dark\"]"), "missing data-theme scope:\n{css}");
    assert!(css.contains(".dmc-pre"), "missing .dmc-pre root:\n{css}");
    assert!(css.contains(&format!(".{cls}")), "missing recorded class {cls}:\n{css}");
    assert!(!css.contains("background-color"), "include_bg=false should drop background-color:\n{css}");

    let css_bare = token_css(0, "", "Catppuccin Mocha", true);
    assert!(!css_bare.contains("[data-theme="), "bare mode should have no data-theme prefix:\n{css_bare}");
    assert!(css_bare.contains("background-color"), "include_bg=true should include background-color:\n{css_bare}");

    reset_token_classes();
    assert!(token_classes().lock().unwrap().is_empty(), "registry should be empty after reset");
  }
}
