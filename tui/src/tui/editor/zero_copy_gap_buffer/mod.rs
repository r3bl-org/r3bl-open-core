// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Zero-copy gap buffer implementation for efficient text editing.
//!
//! This module provides a buffer data structure where each line is stored
//! as a dynamically-sized byte array padded with null characters. This enables zero-copy
//! access as `&str` for the markdown parser while maintaining efficient Unicode support.
//! Unlike a traditional gap buffer, this implementation maintains separate buffers for
//! each line rather than a single gap in a monolithic buffer.
//!
//! # Module Architecture
//!
//! This module provides a concrete `ZeroCopyGapBuffer` implementation with specialized
//! methods organized into focused modules:
//!
//! ## Core Implementation
//! - `core` - Core gap buffer implementation and fundamental operations
//!
//! ## Implementation Extensions (`implementations` module)
//! Specialized method implementations that extend `ZeroCopyGapBuffer`:
//! - `implementations::basic` - Fundamental line operations (insert, delete, access)
//! - `implementations::access` - Zero-copy buffer access utilities
//! - `implementations::insert` - Text insertion algorithms
//! - `implementations::delete` - Text deletion algorithms
//! - `implementations::segment_builder` - Grapheme segment reconstruction
//!
//! ## Simple Direct Usage
//!
//! `ZeroCopyGapBuffer` is used directly as a concrete type:
//!
//! ```rust
//! use r3bl_tui::{ZeroCopyGapBuffer, row};
//! let mut buffer = ZeroCopyGapBuffer::new();
//! buffer.push_line("Hello World");
//! let content = buffer.get_line_content(row(0));
//! ```
//!
//! All operations are available as inherent methods on `ZeroCopyGapBuffer` - no trait
//! indirection is needed. This provides better performance and simpler APIs compared to
//! generic trait-based approaches.
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
//! This module implements a **"validate once, trust thereafter"** approach to UTF-8
//! safety that maximizes both correctness and performance:
//!
//! ## Input Validation (Write Path)
//!
//! UTF-8 validation occurs **at the API boundaries** where content enters the system:
//!
//! - **[`insert_text_at_grapheme(text:
//!   &str)`][ZeroCopyGapBuffer::insert_text_at_grapheme]** - Rust's `&str` type
//!   guarantees valid UTF-8
//! - **File loading** - Use `std::fs::read_to_string()` or `String::from_utf8()` which
//!   validate
//! - **User input** - Terminal/UI frameworks provide pre-validated strings
//! - **Paste operations** - System clipboard provides UTF-8 validated content
//!
//! This ensures that **only valid UTF-8 ever enters the buffer**.
//!
//! ## Zero-Copy Access (Read Path)
//!
//! Once content is in the buffer, all read operations use `unsafe { from_utf8_unchecked()
//! }` for **maximum performance**:
//!
//! - **[`as_str()`][ZeroCopyGapBuffer::as_str]** - Zero-copy access to entire buffer
//! - **[`get_line_content()`][ZeroCopyGapBuffer::get_line_content]** - Zero-copy access
//!   to individual lines
//! - **[`rebuild_line_segments()`][ZeroCopyGapBuffer::rebuild_line_segments]** - Fast
//!   string creation during metadata updates
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
//!
//! # Performance Benchmarks
//!
//! The following benchmarks were measured on a release build using Rust's built-in
//! benchmarking framework. All measurements are in nanoseconds per iteration (ns/iter).
//!
//! ## Buffer Storage Operations
//!
//! Basic buffer management operations show excellent performance:
//!
//! - **`add_line`**: 3.73 ns - Adding a new line to the buffer
//! - **`add_100_lines`**: 1,615.61 ns - Batch creation of 100 lines (~16 ns/line)
//! - **`remove_line_middle`**: 72.32 ns - Removing a line from the middle of buffer
//! - **`extend_line_capacity`**: 12.24 ns - Growing a line by 256 bytes
//! - **`can_insert_check`**: 0.32 ns - Checking if text fits in current capacity
//!
//! ## Text Insertion Operations
//!
//! Text insertion maintains good performance across various scenarios:
//!
//! - **`insert_small_text`**: 88.45 ns - Inserting "Hello" (5 chars)
//! - **`insert_unicode`**: 217.93 ns - Inserting "Hello 😀 世界" with emoji/CJK
//! - **`insert_at_end`**: 157.67 ns - Appending text to existing content
//! - **`insert_middle_of_text`**: 229.68 ns - Inserting in the middle of text
//! - **`insert_causes_extension`**: 408.11 ns - Inserting 300 chars (triggers growth)
//! - **`insert_empty_line`**: 4.27 ns - Adding a new empty line
//! - **`insert_at_end_with_optimization`**: 152.34 ns - Optimized end-of-line append
//!
//! ## Text Deletion Operations
//!
//! Deletion operations are efficient due to in-place buffer manipulation:
//!
//! - **`delete_single_grapheme`**: 168.05 ns - Delete one character
//! - **`delete_unicode_grapheme`**: 334.69 ns - Delete emoji character
//! - **`delete_range_small`**: 235.93 ns - Delete range of 10 characters
//! - **`delete_from_beginning`**: 165.61 ns - Delete first 5 characters
//! - **`delete_entire_line_content`**: 128.95 ns - Clear all line content
//! - **`delete_complex_grapheme_cluster`**: 559.87 ns - Delete family emoji cluster
//!
//! ## Zero-Copy Access Operations
//!
//! The zero-copy design delivers exceptional read performance:
//!
//! - **`as_str_small_buffer`**: 0.19 ns - Access entire buffer as &str (10 lines)
//! - **`as_str_large_buffer`**: 0.19 ns - Access entire buffer as &str (100 lines)
//! - **`get_line_content`**: 0.37 ns - Access single line content
//! - **`get_line_slice_10_lines`**: 0.88 ns - Access slice of 10 lines
//! - **`get_line_with_newline`**: 0.57 ns - Access line including newline
//! - **`as_bytes`**: 0.19 ns - Raw byte access to buffer
//! - **`find_line_containing_byte`**: 1.72 ns - Locate line by byte offset
//! - **`is_valid_utf8`**: 426.08 ns - Full UTF-8 validation (50 lines)
//!
//! ## Segment Reconstruction
//!
//! Grapheme cluster analysis performance varies by content complexity:
//!
//! - **`rebuild_single_line_ascii`**: 79.07 ns - ASCII text (36 chars)
//! - **`rebuild_single_line_unicode`**: 365.45 ns - Unicode with emojis
//! - **`rebuild_batch_10_lines`**: 753.85 ns - Batch rebuild (~75 ns/line)
//! - **`append_optimization_single_char`**: 1.48 ns - Optimized single char append
//! - **`full_rebuild_after_append_single_char`**: 100.48 ns - Full rebuild comparison
//! - **`append_optimization_word`**: 2.91 ns - Optimized word append
//! - **`full_rebuild_after_append_word`**: 273.74 ns - Full rebuild comparison
//!
//! ## Performance Analysis
//!
//! The benchmarks demonstrate several key performance characteristics:
//!
//! 1. **True Zero-Copy Access**: Read operations (0.19-0.88 ns) are essentially free,
//!    showing that we're returning direct pointers without any processing overhead.
//!
//! 2. **Efficient Unicode Handling**: Unicode operations are 2-3x slower than ASCII but
//!    still sub-microsecond, making them suitable for real-time text editing.
//!
//! 3. **Scalable Line Management**: Adding 100 lines takes only ~16 ns per line,
//!    demonstrating good scalability for large documents.
//!
//! 4. **Fast Capacity Checks**: The 0.32 ns `can_insert` check enables efficient capacity
//!    planning without performance impact.
//!
//! 5. **Predictable Growth Cost**: Line extension (12.24 ns) and content that triggers
//!    growth (408.11 ns) show well-bounded performance even during reallocation.
//!
//! 6. **Massive Append Optimization Gains**: The end-of-line append optimization shows
//!    50-90x performance improvement over full segment rebuilding:
//!    - Single character append: 1.48 ns (optimized) vs 100.48 ns (full rebuild) - **68x
//!      faster**
//!    - Word append: 2.91 ns (optimized) vs 273.74 ns (full rebuild) - **94x faster**
//!
//! These benchmarks confirm that the zero-copy gap buffer design achieves its goal of
//! providing high-performance text editing operations while maintaining Unicode safety
//! and memory efficiency.
//!
//! # Optimization Strategies
//!
//! ## Segment Rebuilding
//!
//! After any text modification (insertion or deletion), the buffer rebuilds the
//! grapheme cluster segments for the affected line. This ensures that:
//!
//! - Display width calculations remain accurate
//! - Unicode grapheme boundaries are properly tracked
//! - Cursor movement respects grapheme clusters
//!
//! The rebuild operation parses the line content using the Unicode segmentation
//! library to identify grapheme cluster boundaries and calculate display widths.
//! While this adds some overhead to each edit operation, it ensures correctness
//! for all Unicode text, including emojis, combining characters, and complex scripts.

// Core implementation modules.
mod core;

// Specialized algorithms and optimizations.
mod implementations;

// Adapters for converting to ZeroCopyGapBuffer.
mod adapters;

// Re-export core types and constants.
pub use core::*;

// Note: implementation modules extend [`ZeroCopyGapBuffer`] through inherent method
// implementations. They are not re-exported as they provide specialized capabilities
// accessible directly on ZeroCopyGapBuffer instances.
