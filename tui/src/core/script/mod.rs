// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#![allow(clippy::literal_string_with_formatting_args)]

// Private modules (hide internal structure)
mod package_manager;
mod command_impl;
mod crates_api;
mod directory_change;
mod directory_create;
mod download;
#[cfg(any(test, doc))]
pub mod environment;
#[cfg(not(any(test, doc)))]
mod environment;
#[cfg(any(test, doc))]
pub mod fs_path;
#[cfg(not(any(test, doc)))]
mod fs_path;
#[cfg(any(test, doc))]
pub mod git;
#[cfg(not(any(test, doc)))]
mod git;
mod github_api;
mod http_client;
mod permissions;
mod temp_dir;

// Re-export.
pub use package_manager::*;
pub use command_impl::*;
pub use crates_api::*;
pub use directory_change::*;
pub use directory_create::*;
pub use download::*;
pub use environment::*;
pub use fs_path::*;
pub use git::*;
pub use github_api::*;
pub use http_client::*;
pub use permissions::*;
pub use temp_dir::*;

pub const SCRIPT_MOD_DEBUG: bool = true;
