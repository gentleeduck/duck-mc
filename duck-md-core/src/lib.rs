mod compile;
pub mod engine;

pub use compile::{compile, CompileOutput, Metadata, TocItem};
pub use duck_md_ast as ast;
pub use duck_md_parser::parse;
pub use engine::{run, CollectionConfig, EngineConfig, EngineReport, CollectionReport, DocRecord};
