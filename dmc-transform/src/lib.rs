//! User-facing walkthrough: ../../dmc-docs/dmc-transform/
//! Run `cargo doc --open -p dmc-transform` for the inline rustdoc.

pub mod builtin;
pub mod config;
pub mod pipeline;
pub mod visit;

#[cfg(feature = "assets")]
pub use builtin::CopyLinkedFiles;
#[cfg(feature = "emoji")]
pub use builtin::Emoji;
#[cfg(feature = "math")]
pub use builtin::Math;
#[cfg(feature = "mermaid")]
pub use builtin::Mermaid;
#[cfg(feature = "npm-command")]
pub use builtin::NpmCommand;
#[cfg(feature = "pretty-code")]
pub use builtin::PrettyCode;
pub use builtin::{
  AssignHeadingIds, AutolinkHeadings, BareUrlAutolink, CodeImport, ComponentPreview, ComponentSource, DisableGfm,
};
pub use config::{
  CopyLinkedFilesOptions, MathEngine, MermaidAlign, MermaidArchitectureConfig, MermaidBlockConfig, MermaidC4Config,
  MermaidErConfig, MermaidErLayoutDirection, MermaidFlowchartConfig, MermaidFlowchartCurve, MermaidFlowchartRenderer,
  MermaidGanttConfig, MermaidGanttDisplayMode, MermaidGanttWeekday, MermaidGitGraphConfig, MermaidJourneyConfig,
  MermaidLayout, MermaidLogLevel, MermaidLook, MermaidMindmapConfig, MermaidNodeRendererConfig, MermaidOptions,
  MermaidPieConfig, MermaidRadarConfig, MermaidRequirementConfig, MermaidSankeyAlignment, MermaidSankeyConfig,
  MermaidSecurityLevel, MermaidSequenceConfig, MermaidThemeMode, MermaidThemeVariables, MermaidTimelineConfig,
  MermaidTreemapConfig, MermaidXyChartConfig, MultiThemeStrategy, PipelineConfig, PrettyCodeOptions, PrettyCodeTheme,
};
pub use pipeline::{Pipeline, Transformer};
pub use visit::{NodeAction, Visitor, walk_root};
