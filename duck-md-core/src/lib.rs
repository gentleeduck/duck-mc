mod compile;
pub mod engine;

pub use compile::{compile, compile_with_pipeline, CompileOutput, Metadata, TocItem};
pub use duck_md_parser::{ast, parse};
pub use engine::{run, CollectionConfig, EngineConfig, EngineReport, CollectionReport};
