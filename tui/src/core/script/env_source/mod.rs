// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Fast cross-platform environment loader. See [`try_env_source`] for more info.
//!
//! [`try_env_source`]: crate::try_env_source

#![rustfmt::skip]

// Attach.
mod args;
mod core;
mod diff;
mod filter;
mod formatters;
mod parser;
mod run_shell;

// Re-export.
pub use args::*;
pub use core::*;
pub use diff::{EnvDiff, EnvDiffChunk};
pub use filter::*;
pub use formatters::*;
pub use parser::*;
pub use run_shell::*;

// Conformance test suite.
#[cfg(test)]
mod conformance_tests;
