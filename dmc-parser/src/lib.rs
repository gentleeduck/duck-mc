//! User-facing walkthrough: ../../dmc-docs/dmc-parser/
//! Run `cargo doc --open -p dmc-parser` for the inline rustdoc.

pub mod ast;
mod block;
mod inline;
mod jsx;
pub mod parser;
pub mod slugger;
mod table;

pub use parser::{Parser, parse, parse_inline_str};
pub use slugger::{Slugger, github_slugify};
