//! Orchestrator: the lex → parse → transform → codegen pipeline glued into
//! one entry point ([`compile`]) plus a multi-file engine ([`engine::run`])
//! that processes whole collections according to a velite-flavoured config.
//!
//! - [`compile`] / [`compile_with_pipeline`]: single-source helpers used by
//!   tests, REPLs, and quick CLI invocations.
//! - [`engine::run`]: glob a tree, compile each `.md` / `.mdx`, validate
//!   frontmatter against a [`dmc-schema`] schema, write the velite-style
//!   index next to the build artifacts.

mod compile;
pub mod engine;
pub mod loaders;

pub use compile::{CompileOutput, Metadata, TocItem, compile, compile_with_pipeline};
pub use dmc_parser::{ast, parse};
pub use engine::Engine;
