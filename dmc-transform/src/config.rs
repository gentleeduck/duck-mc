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

/// Top-level config consumed by [`crate::Pipeline::with_defaults_for`].
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
  /// Mermaid render config. `None` keeps the bundled defaults
  /// (light + dark theme pair, `htmlLabels:false`, responsive SVG,
  /// centered labels).
  pub mermaid: Option<MermaidOptions>,
  /// Same shape for the `Mermaid` transformer. `Some(false)` drops the
  /// transformer (mermaid blocks left as code fences).
  pub mermaid_enabled: Option<bool>,
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

/// How multi-theme pretty-code output is laid out in the DOM.
///
/// - `CssVars` (default, fast): single `<pre><code>` tree. Each styled
///   token carries `style="--dmc-light:#XXX;--dmc-dark:#YYY"`, the
///   `<pre>` carries default `color`/`background-color` from the
///   primary mode. Consumer CSS swaps themes by overriding `color` to
///   the matching `--dmc-*` variable inside whichever class /
///   media-query controls the theme. ~25% faster than `Split`.
/// - `Split` (velite parity): one full
///   `<pre data-theme="<mode>"><code>...</code></pre>` subtree per
///   theme, each with solid `color:#XXX` per token. Matches velite +
///   rehype-pretty-code byte-for-byte; consumer CSS shows/hides whole
///   panes by `[data-theme]`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MultiThemeStrategy {
  /// One `<pre data-theme="<mode>">...</pre>` subtree per theme. Each
  /// subtree carries solid `color:#XXX` styles, no CSS custom
  /// properties. Default because the per-token style strings stay
  /// shorter (no `--dmc-{mode}` pairs), the consumer flips themes by
  /// toggling a single `[data-theme]` CSS rule, and it matches the
  /// velite + rehype-pretty-code byte shape that consumers were
  /// already styled for. The phase-6 flamegraph confirms PrettyCode
  /// dominates compile time, so cutting the per-token work - even
  /// at the cost of duplicated pre subtrees - is the right default.
  #[default]
  Split,
  /// One `<pre>` subtree carrying `--dmc-{mode}` / `--dmc-{mode}-bg`
  /// custom properties per token, plus solid fallbacks for the
  /// `default_mode`. Consumer CSS toggles via `var(--dmc-active)`.
  /// Slightly larger per-token style strings, but only one DOM
  /// subtree regardless of theme count - pick this when you have
  /// `>2` themes or need media-query / class-toggle theme switching
  /// without re-rendering the code surface.
  #[serde(alias = "cssVars")]
  CssVars,
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
///
/// Every field below has a sensible default; callers only set the knobs
/// they care about. Field names match the TS-side `PrettyCodeOptions`
/// shape exported from `@gentleduck/md`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PrettyCodeOptions {
  /// Theme spec. String for single-theme, object for multi-mode.
  pub theme: PrettyCodeTheme,
  /// Mode key whose colors fill the unprefixed `color` /
  /// `background-color` attrs. Only meaningful for [`PrettyCodeTheme::Multi`].
  /// When unset, resolves to `"dark"` if present, else the first key.
  pub default_mode: Option<String>,
  /// Multi-theme DOM strategy. Default `CssVars` (single tree, faster).
  /// Set to `Split` for one `<pre data-theme="...">` subtree per theme
  /// (velite parity, ~2x the AST nodes). Single-theme mode ignores this
  /// - there's only one tree either way.
  pub multi_theme_strategy: Option<MultiThemeStrategy>,
  /// Class-based output. When `true`, tokens are emitted once as
  /// `<span class="dmc-...">` (theme-agnostic scope classes, no inline
  /// colors, no per-theme `<pre>` duplication) and the build writes one
  /// `dmc.<mode>.css` (or `dmc.css` for a single unnamed theme) per
  /// configured theme to the output data dir, scoped under
  /// `[data-theme="<mode>"]`. Default `false` (inline-style modes).
  pub classed: Option<bool>,
  /// Keep the `__dmcRaw__` attribute on each `<pre>` so consumer
  /// `<PreBlock>` can offer a Copy button without re-parsing the tree.
  /// Default `true`.
  pub keep_raw_string: Option<bool>,
  /// Wrap the per-theme `<pre>` blocks in a
  /// `<div data-dmc-fragment="">` envelope. Default
  /// `true`. Set `false` to emit just the `<pre>` siblings (compatible
  /// with consumers that wrap themselves).
  pub fragment_wrapper: Option<bool>,
  /// Class on each line `<span>`. Default `"line"`.
  pub line_class: Option<String>,
  /// Attribute name set on highlighted lines (from `{1,3-5}` meta).
  /// Default `"data-dmc-line-highlighted"`.
  pub highlighted_line_attr: Option<String>,
  /// Language used when a fence has no `lang` and for unknown langs
  /// when [`Self::fallback_to_plaintext`] is on. Default `"plaintext"`.
  pub default_language: Option<String>,
  /// When `true` (default), unknown fence languages render as plain
  /// text. When `false`, the code block is left as a `CodeBlock` node
  /// for downstream tooling.
  pub fallback_to_plaintext: Option<bool>,
  /// Render a `<figcaption data-dmc-title>` from the
  /// fence's `title="..."` meta. Default `true`.
  pub render_title: Option<bool>,
  /// Include `data-language` on every emitted `<pre>` and `<code>`.
  /// Default `true`.
  pub include_data_language: Option<bool>,
  /// Emit a solid `background-color` on `<pre>` from the primary theme.
  /// Default `true`. Set to `false` to skip the inline bg so the outer
  /// `[data-dmc-fragment]` wrapper (or consumer chrome) owns the
  /// surface color. Per-mode `--dmc-{mode}-bg` custom properties are
  /// always emitted regardless of this flag, so consumer CSS can still
  /// opt back in via `var(--dmc-{mode}-bg)` if it wants.
  pub include_pre_background: Option<bool>,
  /// Languages to skip - these blocks are passed through unchanged.
  /// `mermaid` is always skipped (owned by the mermaid transformer);
  /// add other langs to keep them as raw `CodeBlock` nodes.
  pub skip_languages: Vec<String>,
  /// Expand tab characters to N spaces before highlighting. `None`
  /// preserves original tabs.
  pub tab_size: Option<u32>,
}

impl PrettyCodeOptions {
  /// Canonical ordered `(mode_key, theme_name)` list - the order token
  /// styles are produced in and the order CSS files are written in. A
  /// single theme yields one pair with an empty mode key. A multi-theme
  /// map is sorted: `"light"` first, `"dark"` next, then any others
  /// alphabetically. This is the single source of truth for theme
  /// ordering shared by `PrettyCode` and the engine's CSS writer.
  pub fn resolved_themes(&self) -> Vec<(String, String)> {
    match &self.theme {
      PrettyCodeTheme::Single(name) => vec![(String::new(), name.clone())],
      PrettyCodeTheme::Multi(map) => {
        let mut themes: Vec<(String, String)> = map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        themes.sort_by(|a, b| {
          fn rank(k: &str) -> u8 {
            match k {
              "light" => 0,
              "dark" => 1,
              _ => 2,
            }
          }
          rank(&a.0).cmp(&rank(&b.0)).then_with(|| a.0.cmp(&b.0))
        });
        themes
      },
    }
  }
}

/// Mermaid theme spec. Either a single theme name (renders once,
/// attaches one `chartSvg` attr) or a map of `mode -> theme name` that
/// renders per-mode and attaches one `${mode}Svg` attr per entry.
///
/// Recognised theme names (passed through to `mmdc --theme`):
/// `default`, `dark`, `forest`, `neutral`, `base`. Anything else is
/// forwarded verbatim - mermaid-cli will validate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MermaidThemeMode {
  /// One render. Output JSX gets a single `chartSvg` attr.
  Single(String),
  /// Per-mode render. Output JSX gets `${key}Svg` per entry. The default
  /// `{ "light": "default", "dark": "dark" }` reproduces the historical
  /// `lightSvg` + `darkSvg` shape.
  Multi(BTreeMap<String, String>),
}

impl Default for MermaidThemeMode {
  fn default() -> Self {
    Self::Multi(
      [("light".to_string(), "default".to_string()), ("dark".to_string(), "dark".to_string())].into_iter().collect(),
    )
  }
}

// Mermaid initialize sub-config types
// Each diagram block + the top-level enums get a typed Rust struct so
// the user-facing config is fully type-checked end-to-end. Mirror the
// TS-side MermaidInitializeConfig shape; new mermaid releases extend
// these structs (serde tolerates unknown keys so missing fields just
// pass through unchecked).

/// CSS dimension that mermaid accepts as either a numeric pixel value
/// (e.g. `14`) or a string with units (e.g. `"14px"`, `"1.2em"`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CssDimension {
  Number(f64),
  String(String),
}

/// CSS font-weight: numeric (`400`, `700`) or named string
/// (`"normal"`, `"bold"`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CssFontWeight {
  Number(u32),
  String(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MermaidSecurityLevel {
  Strict,
  Loose,
  Antiscript,
  Sandbox,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MermaidLogLevel {
  Debug,
  Info,
  Warn,
  Error,
  Fatal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MermaidLook {
  Classic,
  Neo,
  HandDrawn,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MermaidLayout {
  Dagre,
  Elk,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MermaidFlowchartRenderer {
  #[serde(rename = "dagre-d3")]
  DagreD3,
  #[serde(rename = "dagre-wrapper")]
  DagreWrapper,
  Elk,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MermaidFlowchartCurve {
  Basis,
  Linear,
  Cardinal,
  StepBefore,
  StepAfter,
  Natural,
  MonotoneX,
  MonotoneY,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MermaidAlign {
  Left,
  Center,
  Right,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MermaidErLayoutDirection {
  Tb,
  Bt,
  Lr,
  Rl,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MermaidGanttDisplayMode {
  #[serde(rename = "")]
  Default,
  Compact,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MermaidGanttWeekday {
  Monday,
  Tuesday,
  Wednesday,
  Thursday,
  Friday,
  Saturday,
  Sunday,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MermaidSankeyAlignment {
  Left,
  Right,
  Center,
  Justify,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MermaidThemeVariables {
  pub background: Option<String>,
  pub font_family: Option<String>,
  pub font_size: Option<String>,
  pub primary_color: Option<String>,
  pub primary_text_color: Option<String>,
  pub primary_border_color: Option<String>,
  pub secondary_color: Option<String>,
  pub secondary_text_color: Option<String>,
  pub secondary_border_color: Option<String>,
  pub tertiary_color: Option<String>,
  pub tertiary_text_color: Option<String>,
  pub tertiary_border_color: Option<String>,
  pub note_bkg_color: Option<String>,
  pub note_text_color: Option<String>,
  pub note_border_color: Option<String>,
  pub line_color: Option<String>,
  pub text_color: Option<String>,
  pub main_bkg: Option<String>,
  pub error_bkg_color: Option<String>,
  pub error_text_color: Option<String>,
  pub node_bkg: Option<String>,
  pub node_border: Option<String>,
  pub cluster_bkg: Option<String>,
  pub cluster_border: Option<String>,
  pub default_link_color: Option<String>,
  pub title_color: Option<String>,
  pub edge_label_background: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MermaidFlowchartConfig {
  pub html_labels: Option<bool>,
  pub use_max_width: Option<bool>,
  pub default_renderer: Option<MermaidFlowchartRenderer>,
  pub curve: Option<MermaidFlowchartCurve>,
  pub diagram_padding: Option<u32>,
  pub node_spacing: Option<u32>,
  pub rank_spacing: Option<u32>,
  pub padding: Option<u32>,
  pub title_top_margin: Option<u32>,
  pub wrapping_width: Option<u32>,
  pub arrow_marker_absolute: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MermaidSequenceConfig {
  pub use_max_width: Option<bool>,
  pub hide_unused_participants: Option<bool>,
  pub activation_width: Option<u32>,
  pub diagram_margin_x: Option<u32>,
  pub diagram_margin_y: Option<u32>,
  pub actor_margin: Option<u32>,
  pub width: Option<u32>,
  pub height: Option<u32>,
  pub box_margin: Option<u32>,
  pub box_text_margin: Option<u32>,
  pub note_margin: Option<u32>,
  pub message_margin: Option<u32>,
  pub message_align: Option<MermaidAlign>,
  pub mirror_actors: Option<bool>,
  pub force_menus: Option<bool>,
  pub bottom_margin_adj: Option<i32>,
  pub right_angles: Option<bool>,
  pub show_sequence_numbers: Option<bool>,
  pub actor_font_size: Option<CssDimension>,
  pub actor_font_family: Option<String>,
  pub actor_font_weight: Option<CssFontWeight>,
  pub note_font_size: Option<CssDimension>,
  pub note_font_family: Option<String>,
  pub note_font_weight: Option<CssFontWeight>,
  pub note_align: Option<MermaidAlign>,
  pub message_font_size: Option<CssDimension>,
  pub message_font_family: Option<String>,
  pub message_font_weight: Option<CssFontWeight>,
  pub wrap: Option<bool>,
  pub wrap_padding: Option<u32>,
  pub label_box_width: Option<u32>,
  pub label_box_height: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MermaidGanttConfig {
  pub use_max_width: Option<bool>,
  pub title_top_margin: Option<u32>,
  pub bar_height: Option<u32>,
  pub bar_gap: Option<u32>,
  pub top_padding: Option<u32>,
  pub right_padding: Option<u32>,
  pub left_padding: Option<u32>,
  pub grid_line_start_padding: Option<u32>,
  pub font_size: Option<u32>,
  pub section_font_size: Option<CssDimension>,
  pub number_section_styles: Option<u32>,
  pub axis_format: Option<String>,
  pub tick_interval: Option<String>,
  pub top_axis: Option<bool>,
  pub display_mode: Option<MermaidGanttDisplayMode>,
  pub weekday: Option<MermaidGanttWeekday>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MermaidErConfig {
  pub use_max_width: Option<bool>,
  pub title_top_margin: Option<u32>,
  pub diagram_padding: Option<u32>,
  pub layout_direction: Option<MermaidErLayoutDirection>,
  pub min_entity_width: Option<u32>,
  pub min_entity_height: Option<u32>,
  pub entity_padding: Option<u32>,
  pub stroke: Option<String>,
  pub fill: Option<String>,
  pub font_size: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MermaidPieConfig {
  pub use_max_width: Option<bool>,
  pub text_position: Option<f32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MermaidNodeRendererConfig {
  pub use_max_width: Option<bool>,
  pub title_top_margin: Option<u32>,
  pub default_renderer: Option<MermaidFlowchartRenderer>,
  pub arrow_marker_absolute: Option<bool>,
  pub divider_margin: Option<u32>,
  pub padding: Option<u32>,
  pub text_height: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MermaidGitGraphConfig {
  pub use_max_width: Option<bool>,
  pub title_top_margin: Option<u32>,
  pub diagram_padding: Option<u32>,
  pub main_branch_name: Option<String>,
  pub main_branch_order: Option<u32>,
  pub show_commit_label: Option<bool>,
  pub show_branches: Option<bool>,
  pub rotate_commit_label: Option<bool>,
  pub parallel_commits: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MermaidJourneyConfig {
  pub use_max_width: Option<bool>,
  pub diagram_margin_x: Option<u32>,
  pub diagram_margin_y: Option<u32>,
  pub left_margin: Option<u32>,
  pub width: Option<u32>,
  pub height: Option<u32>,
  pub box_margin: Option<u32>,
  pub box_text_margin: Option<u32>,
  pub note_margin: Option<u32>,
  pub message_margin: Option<u32>,
  pub message_align: Option<MermaidAlign>,
  pub bottom_margin_adj: Option<i32>,
  pub right_angles: Option<bool>,
  pub task_font_size: Option<CssDimension>,
  pub task_font_family: Option<String>,
  pub task_margin: Option<u32>,
  pub activation_width: Option<u32>,
  pub text_placement: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MermaidMindmapConfig {
  pub use_max_width: Option<bool>,
  pub padding: Option<u32>,
  pub max_node_width: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MermaidTimelineConfig {
  pub use_max_width: Option<bool>,
  pub disable_multicolor: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MermaidSankeyConfig {
  pub use_max_width: Option<bool>,
  pub node_alignment: Option<MermaidSankeyAlignment>,
  pub show_values: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MermaidXyChartConfig {
  pub use_max_width: Option<bool>,
  pub width: Option<u32>,
  pub height: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MermaidBlockConfig {
  pub use_max_width: Option<bool>,
  pub padding: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MermaidRequirementConfig {
  pub use_max_width: Option<bool>,
  pub rect_min_width: Option<u32>,
  pub rect_min_height: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MermaidC4Config {
  pub use_max_width: Option<bool>,
  pub diagram_margin_x: Option<u32>,
  pub diagram_margin_y: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MermaidArchitectureConfig {
  pub use_max_width: Option<bool>,
  pub padding: Option<u32>,
  pub icon_size: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MermaidRadarConfig {
  pub use_max_width: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MermaidTreemapConfig {
  pub use_max_width: Option<bool>,
  pub padding: Option<u32>,
}

/// Top-level mermaid configuration. **Single flat object** - every
/// `mermaid.initialize()` knob (themeVariables, flowchart, sequence,
/// gantt, look, layout, ...) lives at the same level as the dmc-side
/// rendering knobs (responsiveSvg, centerLabels, outputDir, ...). All
/// fields are typed end-to-end; no `serde_json::Value` catch-all.
///
/// `None` on `CompileConfig.mermaid` means "use built-in defaults"
/// (light + dark themes, htmlLabels:false, flowchart spacing).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MermaidOptions {
  // dmc render knobs
  /// Theme spec. Single string (one render -> `chartSvg`) or
  /// `mode -> theme` map (per-mode render -> `${mode}Svg` each).
  /// Always stripped from the mermaid configFile in
  /// `build_mermaid_config` - it's a dmc-side knob, not a mermaid one.
  pub theme: MermaidThemeMode,
  /// `mmdc --backgroundColor`. Default `"transparent"`.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub background_color: Option<String>,
  /// Apply the responsive-width post-process. Default `true`.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub responsive_svg: Option<bool>,
  /// Inject `text-anchor="middle"` on label `<text>` / `<tspan>` so
  /// flowchart node labels center inside their `<rect>` when
  /// `htmlLabels:false` is in effect. Default `true`.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub center_labels: Option<bool>,
  /// Disk cache directory.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub output_dir: Option<PathBuf>,
  /// Forwarded to `mmdc --puppeteerConfigFile`.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub puppeteer_config_file: Option<PathBuf>,

  // mermaid.initialize()
  /// Override the bundled `htmlLabels: false` default. `true` switches
  /// flowchart node labels back to HTML-in-`<foreignObject>`.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub html_labels: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub theme_variables: Option<MermaidThemeVariables>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub font_family: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub font_size: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub start_on_load: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub arrow_marker_absolute: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub deterministic_ids: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub deterministic_id_seed: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub max_text_size: Option<u64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub max_edges: Option<u64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub security_level: Option<MermaidSecurityLevel>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub log_level: Option<MermaidLogLevel>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub look: Option<MermaidLook>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub layout: Option<MermaidLayout>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub hand_drawn_seed: Option<i64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub wrap: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub flowchart: Option<MermaidFlowchartConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub sequence: Option<MermaidSequenceConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub gantt: Option<MermaidGanttConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub er: Option<MermaidErConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub pie: Option<MermaidPieConfig>,
  /// `class` is a Rust keyword - exposed under `class` in JSON via
  /// `serde(rename)`.
  #[serde(rename = "class", alias = "classDiagram", skip_serializing_if = "Option::is_none")]
  pub class_diagram: Option<MermaidNodeRendererConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub state: Option<MermaidNodeRendererConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub git_graph: Option<MermaidGitGraphConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub journey: Option<MermaidJourneyConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub mindmap: Option<MermaidMindmapConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub timeline: Option<MermaidTimelineConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub sankey: Option<MermaidSankeyConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub xy_chart: Option<MermaidXyChartConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub block: Option<MermaidBlockConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub requirement: Option<MermaidRequirementConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub c4: Option<MermaidC4Config>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub architecture: Option<MermaidArchitectureConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub radar: Option<MermaidRadarConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub treemap: Option<MermaidTreemapConfig>,
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
