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
//! as a dynamically-sized byte array padded with null characters. This enables zero-copy
//! access as `&str` for the markdown parser while maintaining efficient Unicode support.
//!
//! # Key Features
//!
//! - Dynamic line buffers starting at 256 bytes, growing as needed
//! - Null-padded storage for efficient in-place editing
//! - Zero-copy access for parsing operations
//! - Unicode-safe text manipulation using grapheme clusters
//! - Metadata caching for performance
//!
//! # Line Storage Architecture
//!
//! Each line in the buffer follows a specific storage model:
//!
//! ## Initial Allocation
//! - **Starting size**: 256 bytes per line (`INITIAL_LINE_SIZE`)
//! - **Initialization**: All bytes set to `\0` (null bytes)
//! - **Content layout**: `[content][newline][null padding...]`
//!
//! ## Dynamic Growth
//! When text insertion exceeds the current line capacity:
//! - **Growth increment**: 256 bytes (`LINE_PAGE_SIZE`)
//! - **Growth strategy**: Extends in fixed-size pages to minimize allocations
//! - **Buffer shifting**: Subsequent lines are shifted to accommodate growth
//! - **Null padding**: New capacity is immediately null-initialized
//!
//! ## Example Storage Layout
//! ```text
//! Line 0: [H][e][l][l][o][\n][\0][\0]...[\0]  // 256 bytes total
//! Line 1: [W][o][r][l][d][\n][\0][\0]...[\0]  // 256 bytes total
//!
//! After inserting 300 characters into Line 0:
//! Line 0: [300 chars...][\n][\0][\0]...[\0]   // 512 bytes (grew by 256)
//! Line 1: [W][o][r][l][d][\n][\0][\0]...[\0]  // 256 bytes (shifted in buffer)
//! ```
//!
//! This approach provides:
//! - **Efficient small edits**: Most text fits in initial 256-byte allocation
//! - **Scalable large content**: Lines can grow to accommodate any text size
//! - **Predictable performance**: Growth occurs in fixed increments
//! - **Memory efficiency**: Only allocates what's needed, when needed
//!
//! # Null-Padding Invariant
//!
//! **CRITICAL**: This module maintains a strict invariant that all unused capacity
//! in each line buffer MUST be filled with null bytes (`\0`). This invariant is
//! essential for:
//!
//! - **Security**: Prevents information leakage from uninitialized memory
//! - **Correctness**: Ensures predictable buffer state for zero-copy operations
//! - **Performance**: Enables safe slice operations without bounds checking
//!
//! All operations (insert, delete, extend) MUST maintain this invariant by:
//! 1. Initializing new memory with `\0`
//! 2. Clearing gaps left by content shifts
//! 3. Padding unused capacity after modifications
//!
//! Violation of this invariant may lead to buffer corruption, security vulnerabilities,
//! or undefined behavior in zero-copy access operations.

// Attach.
mod buffer_storage;
mod text_deletion;
mod text_insertion;
mod zero_copy_access;

// Re-export.
pub use buffer_storage::*;
// Note: zero_copy_access, text_insertion and text_deletion extend LineBuffer impl, no
// separate exports needed
