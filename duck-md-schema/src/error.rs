use thiserror::Error;

#[derive(Debug, Clone, Error)]
#[error("{path}: {message}")]
pub struct ValidationError {
  pub path: String,
  pub message: String,
}

impl ValidationError {
  pub fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
    Self { path: path.into(), message: message.into() }
  }

  pub fn root(message: impl Into<String>) -> Self {
    Self { path: String::new(), message: message.into() }
  }

  pub fn at(mut self, field: &str) -> Self {
    if self.path.is_empty() {
      self.path = field.to_string();
    } else if self.path.starts_with('[') {
      self.path = format!("{field}{}", self.path);
    } else {
      self.path = format!("{field}.{}", self.path);
    }
    self
  }

  pub fn at_index(mut self, idx: usize) -> Self {
    self.path = format!("[{idx}]{}", if self.path.is_empty() {
      String::new()
    } else if self.path.starts_with('[') {
      self.path.clone()
    } else {
      format!(".{}", self.path)
    });
    self
  }
}
