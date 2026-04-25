use duck_md_ast::Document;

pub trait Transformer {
  fn name(&self) -> &str {
    "anonymous"
  }
  fn transform(&self, doc: &mut Document);
}

#[derive(Default)]
pub struct Pipeline {
  transformers: Vec<Box<dyn Transformer + Send + Sync>>,
}

impl Pipeline {
  pub fn new() -> Self {
    Self {
      transformers: Vec::new(),
    }
  }

  #[allow(clippy::should_implement_trait)]
  pub fn add<T: Transformer + Send + Sync + 'static>(mut self, t: T) -> Self {
    self.transformers.push(Box::new(t));
    self
  }

  pub fn with_defaults() -> Self {
    Self::new().add(crate::AutolinkHeadings::default())
  }

  pub fn run(&self, doc: &mut Document) {
    for t in &self.transformers {
      t.transform(doc);
    }
  }
}
