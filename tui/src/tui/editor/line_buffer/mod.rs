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

//! Line buffer implementation for efficient text editing.
//!
//! This module provides a gap buffer-like data structure where each line is stored
//! as a fixed-size byte array padded with null characters. This enables zero-copy
//! access as `&str` for the markdown parser while maintaining efficient Unicode support.
//!
//! # Key Features
//!
//! - Fixed-size line buffers (256 bytes by default)
//! - Null-padded storage for efficient in-place editing
//! - Zero-copy access for parsing operations
//! - Unicode-safe text manipulation using grapheme clusters
//! - Metadata caching for performance

// Attach.
mod line_buffer_impl;

// Re-export.
pub use line_buffer_impl::*;
