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

//! Core components of the zero-copy gap buffer implementation.
//!
//! This module contains the fundamental data structures used by the gap buffer:
//! - [`ZeroCopyGapBuffer`] - The main buffer structure
//! - [`GapBufferLine`] - A line view combining content and metadata
//! - [`LineMetadata`] - Metadata about each line including segments and display info

// Attach.
pub mod zero_copy_gap_buffer;
pub mod gap_buffer_line;
pub mod line_metadata;

// Re-export.
pub use zero_copy_gap_buffer::*;
pub use gap_buffer_line::*;
pub use line_metadata::*;

/// Initial size of each line in bytes
pub const INITIAL_LINE_SIZE: usize = 256;

/// Page size for extending lines (bytes added when line overflows)
pub const LINE_PAGE_SIZE: usize = 256;