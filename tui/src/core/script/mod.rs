// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#![allow(clippy::literal_string_with_formatting_args)]

// Attach sources.
pub mod apt_install;
pub mod command_impl;
pub mod crates_api;
pub mod directory_change;
pub mod directory_create;
pub mod download;
pub mod environment;
pub mod fs_path;
pub mod github_api;
pub mod http_client;
pub mod permissions;
pub mod temp_dir;

// Re-export.
pub use apt_install::*;
pub use command_impl::*;
pub use crates_api::*;
pub use directory_change::*;
pub use directory_create::*;
pub use download::*;
pub use environment::*;
pub use fs_path::*;
pub use github_api::*;
pub use http_client::*;
pub use permissions::*;
pub use temp_dir::*;

pub const SCRIPT_MOD_DEBUG: bool = true;
