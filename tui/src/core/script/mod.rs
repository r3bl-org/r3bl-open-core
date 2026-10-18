// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#![allow(clippy::literal_string_with_formatting_args)]

// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their macros).
#[macro_use]
mod command_impl;
#[macro_use]
mod directory_change;
#[macro_use]
mod temp_dir;

// Attach. Crate-wide (Intra-crate).
pub(crate) mod env_source; // Cross-folder rustdoc module link (tui/mod.rs).

// Attach. Folder-local (Intra-module).
mod fs_path;
mod crates_api;
mod directory_create;
mod download;
mod environment;
mod git;
mod github_api;
mod http_client;
mod package_manager;
mod permissions;

// Re-export.
pub use command_impl::*;
pub use crates_api::*;
pub use directory_change::*;
pub use directory_create::*;
pub use download::*;
pub use env_source::*;
pub use environment::*;
pub use fs_path::*;
pub use git::*;
pub use github_api::*;
pub use http_client::*;
pub use package_manager::*;
pub use permissions::*;
pub use temp_dir::*;

pub const DEBUG_SCRIPT: bool = true;
