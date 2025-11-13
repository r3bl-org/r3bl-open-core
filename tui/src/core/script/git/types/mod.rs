// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Type definitions and constants for git operations.

// Skip rustfmt for rest of file to preserve manual organization
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

// Private modules (hide internal structure)
mod core;
mod local_branch_info;
mod constants;

// Public re-exports (expose stable flat API)
pub use core::*;
pub use local_branch_info::*;
pub use constants::*;
