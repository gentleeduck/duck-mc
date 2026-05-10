//! User-facing walkthrough: ../../dmc-docs/dmc-parser/
//! Run `cargo doc --open -p dmc-parser` for the inline rustdoc.

pub mod ast;
mod block;
mod inline;
mod jsx;
pub mod parser;
pub mod refs;
pub mod slugger;
mod table;

pub use parser::{ParseOptions, Parser, parse, parse_inline_str, parse_with};
pub use slugger::{Slugger, github_slugify};
