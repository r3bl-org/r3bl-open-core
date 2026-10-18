// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

// Attach. Crate-wide (Intra-crate).
pub(crate) mod pty_engine_types; // Cross-folder rustdoc module link (generate_pty_test).

// Attach. Folder-local (Intra-module).
mod controlled_child;
mod pty_pair;
mod pty_size;
#[cfg(windows)]
mod windows_terminate_process;

// Re-export.
pub use controlled_child::*;
pub use pty_engine_types::*;
pub use pty_pair::*;
pub use pty_size::*;
