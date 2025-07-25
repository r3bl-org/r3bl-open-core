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

//! Zero-copy gap buffer implementation for efficient text editing.
//!
//! This module provides a buffer data structure where each line is stored
//! as a dynamically-sized byte array padded with null characters. This enables zero-copy
//! access as `&str` for the markdown parser while maintaining efficient Unicode support.
//! Unlike a traditional gap buffer, this implementation maintains separate buffers for
//! each line rather than a single gap in a monolithic buffer.
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
//!
//! # UTF-8 Safety Architecture
//!
//! This module implements a **"validate once, trust thereafter"** approach to UTF-8 safety
//! that maximizes both correctness and performance:
//!
//! ## Input Validation (Write Path)
//!
//! UTF-8 validation occurs **at the API boundaries** where content enters the system:
//!
//! - **[`insert_at_grapheme(text: &str)`][ZeroCopyGapBuffer::insert_at_grapheme]** - Rust's `&str` type guarantees valid UTF-8
//! - **File loading** - Use `std::fs::read_to_string()` or `String::from_utf8()` which validate
//! - **User input** - Terminal/UI frameworks provide pre-validated strings
//! - **Paste operations** - System clipboard provides UTF-8 validated content
//!
//! This ensures that **only valid UTF-8 ever enters the buffer**.
//!
//! ## Zero-Copy Access (Read Path)
//!
//! Once content is in the buffer, all read operations use `unsafe { from_utf8_unchecked() }`
//! for **maximum performance**:
//!
//! - **[`as_str()`][ZeroCopyGapBuffer::as_str]** - Zero-copy access to entire buffer
//! - **[`get_line_content()`][ZeroCopyGapBuffer::get_line_content]** - Zero-copy access to individual lines
//! - **[`rebuild_line_segments()`][ZeroCopyGapBuffer::rebuild_line_segments]** - Fast string creation during metadata updates
//!
//! This avoids redundant UTF-8 validation in performance-critical paths like:
//! - Markdown parsing (needs zero-copy `&str` access)
//! - Text rendering (frequent line content access)
//! - Segment rebuilding (called after every edit operation)
//!
//! ## Safety Guarantees
//!
//! The unsafe usage is **architecturally sound** because:
//!
//! 1. **Type System Validation**: `&str` parameters ensure UTF-8 validity at input
//! 2. **Controlled Mutations**: All buffer modifications maintain UTF-8 boundaries
//! 3. **Null-Padding Safety**: Unused capacity filled with `\0` (valid UTF-8)
//! 4. **Debug Assertions**: Development builds validate UTF-8 to catch violations
//! 5. **Comprehensive Testing**: Tests verify UTF-8 handling including edge cases
//!
//! ## Performance Benefits
//!
//! This architecture provides significant performance advantages:
//!
//! - **Zero allocation** in read paths (no `Cow<str>` from [`String::from_utf8_lossy`])
//! - **Zero validation overhead** in hot loops (segment rebuilding, parsing)
//! - **Direct slice operations** without UTF-8 scanning
//! - **Optimal benchmark performance** (production builds skip all validation)
//!
//! ## Error Handling Strategy
//!
//! - **Input validation**: Return `Result<T, E>` for invalid UTF-8 at boundaries
//! - **Internal operations**: Use debug assertions to catch invariant violations
//! - **Production safety**: Trust the input validation and skip redundant checks
//!
//! This approach follows Rust's philosophy of "zero-cost abstractions" while maintaining
//! memory safety through careful API design rather than runtime validation.

// Attach.
mod buffer_storage;
mod segment_construction;
mod text_deletion;
mod text_insertion;
mod zero_copy_access;

// Re-export.
pub use buffer_storage::*;
// Note: zero_copy_access, text_insertion, text_deletion and segment_construction
// extend [`ZeroCopyGapBuffer`] impl, no separate exports needed
