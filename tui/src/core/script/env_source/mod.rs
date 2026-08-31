// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Fast cross-platform environment loader. See [`try_env_source`] for more info.
//!
//! [`try_env_source`]: crate::try_env_source

#![rustfmt::skip]

// Private modules (hide internal structure).
mod args;
mod core;
pub mod diff;
mod filter;
mod formatters;
mod parser;
mod run_shell;

// Public re-exports (expose stable API).
pub use args::*;
pub use core::*;
pub use diff::{EnvDiff, EnvDiffChunk};
pub use filter::*;
pub use formatters::*;
pub use parser::*;
pub use run_shell::*;

// Conformance test suite.
#[cfg(test)]
pub mod conformance_tests;
