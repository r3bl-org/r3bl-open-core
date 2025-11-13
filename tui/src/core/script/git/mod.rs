// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Git operations module for repository status, branch management, and file tracking.
//!
//! This module provides async operations for working with git repositories, including:
//! - Repository status checking
//! - Branch listing, creation, checkout, and deletion
//! - Tracking changed files by extension
//!
//! The module follows the pattern from CLAUDE.md with private submodules and public
//! re-exports to provide a clean, flat API surface.

// Skip rustfmt for rest of file to preserve manual organization
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

// Private modules (hide internal structure).
mod types;
mod status_ops;
mod branch_ops;
mod file_ops;

// Public re-exports (expose stable flat API).
pub use types::*;
pub use status_ops::*;
pub use branch_ops::*;
pub use file_ops::*;

// Test fixtures module used by inner test modules in each of these files.
#[cfg(test)]
pub mod test_fixtures;
