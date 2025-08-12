/*
 *   Copyright (c) 2025 R3BL LLC
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

// Attach.
pub mod common_impl;
pub mod osc_seq;
pub mod pty_config;
pub mod pty_core;
pub mod pty_read_only;
pub mod pty_read_write;

// Re-export.
pub use osc_seq::*;
pub use pty_config::*;
pub use pty_core::*;
// Internal implementations - not exported.
pub(crate) use pty_read_only::spawn_pty_read_only_impl;
pub(crate) use pty_read_write::spawn_pty_read_write_impl;
