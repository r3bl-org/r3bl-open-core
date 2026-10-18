// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Type definitions and constants for git operations.

#![rustfmt::skip]

// Attach.
mod core;
mod local_branch_info;
mod constants;

// Re-export.
pub use core::*;
pub use local_branch_info::*;
pub(crate) use constants::*;
