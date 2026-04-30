//! Codegen layer: turn a parsed `Document` into renderable output.
//!
//! Two emitters live here:
//! - [`HtmlEmitter`] — static HTML (SSR / SSG output).
//! - [`MdxBodyEmitter`] — JS body for MDX runtime React rendering.

mod escape;
pub mod html;
pub mod mdx;

pub use html::{HtmlEmitter, render_html};
pub use mdx::{MdxBodyEmitter, render_mdx_body};
