//! Pipeline-level configuration. Compiled unconditionally so callers can
//! describe the pipeline (themes, copy-linked-files paths, gfm switch, ...)
//! whether or not the matching feature flags are enabled.
//!
//! When a config field describes a transformer whose feature is off,
//! [`Pipeline::with_defaults_for`](crate::Pipeline::with_defaults_for)
//! silently skips it -- the field is still allowed in the config so user
//! settings round-trip cleanly across builds with different feature sets.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Top-level config consumed by [`Pipeline::with_defaults_for`].
/// All fields are optional; the empty config (`PipelineConfig::default()`)
/// reproduces the historical `Pipeline::with_defaults()` behavior.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PipelineConfig {
  /// When `false`, append the `disable-gfm` transformer that strips GFM
  /// extensions (tables, strikethrough, autolinks, task lists).
  pub markdown_gfm: Option<bool>,
  /// Pretty-code theme + multi-mode settings. `None` keeps the bundled
  /// defaults (Catppuccin Latte/Mocha pair, dark primary).
  pub pretty_code: Option<PrettyCodeOptions>,
  /// LaTeX rendering engine. `None` -> [`MathEngine::Katex`].
  pub math_engine: Option<MathEngine>,
  /// When `Some`, append the `copy-linked-files` transformer with the
  /// supplied paths.
  pub copy_linked_files: Option<CopyLinkedFilesOptions>,
  /// When `Some(false)`, do not push the `Emoji` transformer. `None`
  /// or `Some(true)` keeps the default behaviour (transformer added
  /// when the `emoji` feature is on). Used by the plugin gate to drop
  /// the native transformer when the user prefers `remark-emoji`.
  pub emoji: Option<bool>,
  /// Same shape for the `AutolinkHeadings` transformer. Set to
  /// `Some(false)` when the user prefers `rehype-slug` /
  /// `rehype-autolink-headings`.
  pub autolink_headings: Option<bool>,
  /// Same shape for the `Math` transformer. Set to `Some(false)` when
  /// the user prefers `remark-math` / `rehype-katex` / `rehype-mathjax`.
  pub math: Option<bool>,
  /// Same shape for the `PrettyCode` transformer. Set to `Some(false)`
  /// when the user prefers `rehype-pretty-code` / `shiki`.
  pub pretty_code_enabled: Option<bool>,
}

/// Which engine renders `$...$` / `$$...$$` math.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MathEngine {
  /// Embedded KaTeX via `quick-js`. Output matches `rehype-katex` byte
  /// for byte. Slow per-expression (1-5 ms each).
  #[default]
  Katex,
  /// `pulldown-latex` -> MathML. Fast (microseconds). Browser MathML
  /// rendering is functional but visually plainer than KaTeX HTML.
  Mathml,
}

/// Pretty-code theme spec.
///
/// Deserialised untagged: pass a JSON string for single-theme, or an
/// object `{ light: "...", dark: "...", ... }` for multi-mode output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PrettyCodeTheme {
  /// Single bundled theme name. Emits per-token `style="color:#xxx"`.
  Single(String),
  /// Map of `mode -> bundled theme name`. Mode keys are arbitrary
  /// (`light`, `dark`, `dim`, ...); they appear in the emitted CSS as
  /// `--dmc-{mode}` / `--dmc-{mode}-bg` CSS custom properties.
  Multi(BTreeMap<String, String>),
}

impl Default for PrettyCodeTheme {
  fn default() -> Self {
    Self::Multi(
      [("light".to_string(), "Catppuccin Latte".to_string()), ("dark".to_string(), "Catppuccin Mocha".to_string())]
        .into_iter()
        .collect(),
    )
  }
}

/// Top-level pretty-code configuration. Stored on `CompileConfig` as
/// `Option<PrettyCodeOptions>`; `None` means "use built-in defaults".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PrettyCodeOptions {
  /// Theme spec. String for single-theme, object for multi-mode.
  pub theme: PrettyCodeTheme,
  /// Mode key whose colors fill the unprefixed `color` /
  /// `background-color` attrs. Only meaningful for [`PrettyCodeTheme::Multi`].
  /// When unset, resolves to `"dark"` if present, else the first key.
  pub default_mode: Option<String>,
}

/// Paths consumed by the `copy-linked-files` transformer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyLinkedFilesOptions {
  /// Directory the source `.mdx` lives in -- used to resolve relative
  /// `src` / `href` attrs.
  pub source_dir: PathBuf,
  /// Output asset directory (where copies are written).
  pub assets_dir: PathBuf,
  /// Public URL prefix prepended to rewritten asset paths.
  pub public_base: String,
}
