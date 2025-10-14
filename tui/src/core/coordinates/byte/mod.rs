// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Byte-level coordinates for UTF-8 text processing.
//!
//! This module provides coordinate types for working with byte positions in UTF-8 strings
//! and buffers. All types use `usize` internally for compatibility with Rust string
//! slicing.
//!
//! # Core Types
//!
//! - [`ByteIndex`]: Absolute byte position (0-based)
//! - [`ByteLength`]: Byte count/size (1-based)
//! - [`ByteOffset`]: Relative byte displacement
//!
//! # Usage
//!
//! These types are primarily used in:
//! - `InlineString` within `GCStringOwned`
//! - UTF-8 text processing where character boundaries don't align with byte boundaries
//! - String slicing operations
//!
//! # Example
//!
//! ```rust
//! use r3bl_tui::{ByteIndex, byte_index, ByteIndexRangeExt};
//!
//! let text = "Hello 世界";
//! let start = byte_index(0);
//! let end = byte_index(5);
//!
//! // Convert ByteIndex range to usize range for slicing
//! let slice = &text[(start..end).to_usize_range()];
//! assert_eq!(slice, "Hello");
//! ```

// Attach source files.
pub mod byte_index;
pub mod byte_length;
pub mod byte_offset;

// Re-export.
pub use byte_index::*;
pub use byte_length::*;
pub use byte_offset::*;
