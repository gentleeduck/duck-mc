mod compile;

pub use compile::{compile, CompileOutput, Metadata, TocItem};
pub use duck_md_ast as ast;
pub use duck_md_parser::parse;
