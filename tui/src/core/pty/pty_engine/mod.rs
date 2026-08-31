// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

// Attach.
pub mod controlled_child;
pub mod pty_engine_types;
pub mod pty_pair;
pub mod pty_size;
#[cfg(windows)]
mod windows_terminate_process;

// Re-export.
pub use controlled_child::*;
pub use pty_engine_types::*;
pub use pty_pair::*;
pub use pty_size::*;
