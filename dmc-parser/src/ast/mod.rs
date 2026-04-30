pub mod jsx;
pub mod node;

pub use jsx::*;
pub use node::*;

/// Default `Span` used for serde deserialization since `duck_diagnostic::Span`
/// does not currently implement `Serialize`/`Deserialize`.
pub fn default_span() -> duck_diagnostic::Span {
  duck_diagnostic::Span::new("", 0, 0, 0)
}
