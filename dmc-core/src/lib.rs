//! User-facing walkthrough: ../../dmc-docs/dmc-core/
//! Run `cargo doc --open -p dmc-core` for the inline rustdoc.
#![allow(clippy::result_large_err)]

//! Orchestrator: lex -> parse -> transform -> codegen pipeline plus a
//! multi-file engine that processes whole collections per a velite-style
//! config. See [`crate::engine::compile::Compiler::compile`] for
//! single-source use; [`crate::Engine::run`]
//! for batch builds.

pub mod cli;
pub mod engine;
pub mod loaders;

pub use dmc_parser::{ast, parse};
pub use dmc_transform::{MermaidOptions, MermaidThemeMode, PrettyCodeOptions, PrettyCodeTheme};
pub use engine::Engine;
