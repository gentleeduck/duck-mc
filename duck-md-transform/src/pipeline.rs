use duck_diagnostic::DiagnosticEngine;
use duck_md_diagnostic::{Code, metadata::SourceMeta};
use duck_md_parser::ast::Document;
use std::cell::RefMut;

/// One AST→AST pass. Implementations typically build a private `Visitor` and
/// drive it across the document. `transform` takes `&self` so a transformer
/// is cheap to share across threads or reuse across files. Diagnostics flow
/// into the shared `DiagnosticEngine<Code>` borrow.
pub trait Transformer {
  /// Stable identifier for logging / error reporting. Defaults to
  /// `"anonymous"`; override per implementation.
  fn name(&self) -> &str {
    "anonymous"
  }
  /// Mutate `doc` in place. `engine` collects any transform-level diagnostics
  /// (`crate::diagnostic::Code`). May be a no-op if the transformer's
  /// preconditions (config, environment, feature flags) aren't met.
  fn transform(&self, doc: &mut Document, meta: &SourceMeta, engine: &mut DiagnosticEngine<Code>);
}

/// Ordered list of transformers run in registration order against a single
/// `Document`. Transformers are boxed + `Send + Sync` so a `Pipeline` can be
/// shared across worker threads.
#[derive(Default)]
pub struct Pipeline {
  transformers: Vec<Box<dyn Transformer + Send + Sync>>,
}

impl Pipeline {
  /// Empty pipeline. Add transformers with [`Pipeline::add`].
  pub fn new() -> Self {
    Self { transformers: Vec::new() }
  }

  /// Append `t` to the run order. Returns `self` for builder-style chaining.
  #[allow(clippy::should_implement_trait)]
  pub fn add<T: Transformer + Send + Sync + 'static>(mut self, t: T) -> Self {
    self.transformers.push(Box::new(t));
    self
  }

  /// Default pipeline used by the `compile` flow: code-import resolution,
  /// npm/yarn/pnpm/bun command derivation, bare-URL autolinking, heading
  /// anchors, and (with `mermaid` feature) mermaid SVG rendering.
  pub fn with_defaults() -> Self {
    #[allow(unused_mut)]
    let mut p = Self::new()
      .add(crate::CodeImport::new())
      .add(crate::BareUrlAutolink)
      .add(crate::AutolinkHeadings::new());

    #[cfg(feature = "npm_command")]
    {
      p = p.add(crate::NpmCommand);
    }

    #[cfg(feature = "mermaid")]
    {
      p = p.add(crate::Mermaid::default());
    }
    p
  }

  /// Apply every registered transformer to `doc` in registration order.
  /// Diagnostics from all passes accumulate in the shared `engine` borrow.
  pub fn run(
    &self,
    doc: &mut Document,
    meta: &SourceMeta,
    mut engine: RefMut<'_, DiagnosticEngine<Code>>,
  ) {
    for t in &self.transformers {
      t.transform(doc, meta, &mut engine);
    }
  }

  /// Convenience for tests + tooling that don't care about diagnostics or
  /// source identity. Synthesises an `Origin::Inline` meta and a throwaway
  /// engine, discarding anything emitted. For real callers that have a
  /// SourceMeta in hand, use [`Pipeline::run`] directly.
  pub fn run_silent(&self, doc: &mut Document) {
    use duck_md_diagnostic::metadata::Origin;
    use std::sync::Arc;
    let meta =
      SourceMeta { path: Arc::from("<test>"), version: 0, origin: Origin::Inline("<test>") };
    let engine = std::cell::RefCell::new(DiagnosticEngine::new());
    self.run(doc, &meta, engine.borrow_mut());
  }
}
