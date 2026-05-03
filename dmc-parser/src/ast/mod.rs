pub mod jsx;
pub mod node;

pub use jsx::*;
pub use node::*;

/// Default `Span` for serde deserialization (`duck_diagnostic::Span` is not
/// `Serialize`/`Deserialize`).
pub fn default_span() -> duck_diagnostic::Span {
  duck_diagnostic::Span::new("", 0, 0, 0)
}
