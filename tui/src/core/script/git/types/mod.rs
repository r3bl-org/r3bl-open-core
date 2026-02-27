// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Type definitions and constants for git operations.

#![rustfmt::skip]

// Private modules (hide internal structure)
mod core;
mod local_branch_info;
mod constants;

// Public re-exports (expose stable flat API)
pub use core::*;
pub use local_branch_info::*;
pub use constants::*;
