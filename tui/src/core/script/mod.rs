/*
 *   Copyright (c) 2024-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

#![allow(clippy::literal_string_with_formatting_args)]

// Attach sources.
pub mod apt_install;
pub mod command_impl;
pub mod directory_change;
pub mod directory_create;
pub mod download;
pub mod environment;
pub mod fs_path;
pub mod github_api;
pub mod http_client;
pub mod permissions;

// Re-export.
pub use apt_install::*;
pub use command_impl::*;
pub use directory_change::*;
pub use directory_create::*;
pub use download::*;
pub use environment::*;
pub use fs_path::*;
pub use github_api::*;
pub use http_client::*;
pub use permissions::*;

pub const SCRIPT_MOD_DEBUG: bool = true;
