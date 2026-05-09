use crate::config::PipelineConfig;
use dmc_diagnostic::{Code, metadata::SourceMeta};
use dmc_parser::ast::Document;
use duck_diagnostic::DiagnosticEngine;

/// One AST-to-AST pass. `transform` takes `&self` so a transformer is cheap
/// to share across threads or reuse across files.
pub trait Transformer {
  /// Stable identifier for logging / error reporting.
  fn name(&self) -> &str {
    "anonymous"
  }
  /// Mutate `doc` in place. May be a no-op when preconditions (config,
  /// environment, feature flags) aren't met.
  fn transform(&self, doc: &mut Document, meta: &SourceMeta, diag_engine: &mut DiagnosticEngine<Code>);
}

/// Ordered list of transformers run in registration order. Boxed + `Send +
/// Sync` so a `Pipeline` can be shared across worker threads.
#[derive(Default)]
pub struct Pipeline {
  transformers: Vec<Box<dyn Transformer + Send + Sync>>,
}

impl Pipeline {
  pub fn new() -> Self {
    Self { transformers: Vec::new() }
  }

  /// Append `t` to the run order. Returns `self` for builder chaining.
  #[allow(clippy::should_implement_trait)]
  pub fn add<T: Transformer + Send + Sync + 'static>(mut self, t: T) -> Self {
    self.transformers.push(Box::new(t));
    self
  }

  /// Default pipeline. Equivalent to `with_defaults_for(&PipelineConfig::default())`.
  pub fn with_defaults() -> Self {
    Self::with_defaults_for(&PipelineConfig::default())
  }

  /// Build the default pipeline tuned by `cfg`. Single uniform place where
  /// every config-dependent and feature-gated transformer is wired up:
  /// callers don't sprinkle `cfg!(feature = ...)` of their own.
  pub fn with_defaults_for(cfg: &PipelineConfig) -> Self {
    #[allow(unused_mut)]
    let mut p = Self::new()
      // Heading ids first: dedupe-aware github-slugger pass populates
      // `Heading.id` so every downstream consumer (autolink, MDX/HTML
      // emitters, TOC) sees the same value.
      .add(crate::AssignHeadingIds::new())
      .add(crate::CodeImport::new())
      .add(crate::BareUrlAutolink);
    if cfg.autolink_headings != Some(false) {
      p = p.add(crate::AutolinkHeadings::new());
    }

    if cfg.markdown_gfm == Some(false) {
      p = p.add(crate::DisableGfm);
    }

    #[cfg(feature = "npm-command")]
    {
      p = p.add(crate::NpmCommand::new());
    }

    #[cfg(feature = "mermaid")]
    {
      if cfg.mermaid_enabled != Some(false) {
        let m = cfg.mermaid.clone().map(crate::Mermaid::from_options).unwrap_or_default();
        p = p.add(m);
      }
    }

    #[cfg(feature = "emoji")]
    {
      if cfg.emoji != Some(false) {
        p = p.add(crate::Emoji);
      }
    }

    #[cfg(feature = "math")]
    {
      if let Some(engine) = cfg.math_engine {
        crate::Math::set_engine(engine);
      }
      if cfg.math != Some(false) {
        p = p.add(crate::Math);
      }
    }

    #[cfg(feature = "pretty-code")]
    {
      if cfg.pretty_code_enabled != Some(false) {
        let pc = cfg.pretty_code.as_ref().map(crate::PrettyCode::from_options).unwrap_or_default();
        p = p.add(pc);
      }
    }

    #[cfg(feature = "assets")]
    if let Some(opts) = &cfg.copy_linked_files {
      p =
        p.add(crate::CopyLinkedFiles::new(opts.source_dir.clone(), opts.assets_dir.clone(), opts.public_base.clone()));
    }

    p
  }

  /// Apply every registered transformer to `doc` in registration order.
  pub fn run(&self, doc: &mut Document, meta: &SourceMeta, engine: &'_ mut DiagnosticEngine<Code>) {
    for t in &self.transformers {
      t.transform(doc, meta, engine);
    }
  }

  /// Run with a synthesised `Origin::Inline` meta and a throwaway engine,
  /// discarding diagnostics. For tests + tooling without a `SourceMeta`.
  pub fn run_silent(&self, doc: &mut Document) {
    use dmc_diagnostic::metadata::Origin;
    use std::sync::Arc;
    let meta = SourceMeta { path: Arc::from("<test>"), origin: Origin::Inline("<test>") };
    let mut engine = DiagnosticEngine::new();
    self.run(doc, &meta, &mut engine);
  }
}
