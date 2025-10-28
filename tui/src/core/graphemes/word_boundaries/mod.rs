// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Word boundary detection for text editing operations.
//!
//! This module provides Unicode-safe word boundary detection for use in text editing
//! components like readline and editor. It handles:
//!
//! - **Whitespace boundaries**: spaces, tabs, newlines
//! - **Punctuation boundaries**: common punctuation characters
//! - **Unicode safety**: Works correctly with emoji, combining characters, and multi-byte
//!   sequences
//!
//! ## Usage
//!
//! ```rust
//! use r3bl_tui::core::graphemes::word_boundaries::*;
//!
//! let text = "hello-world foo";
//! let cursor = 11; // After "hello-world"
//!
//! // Find start of previous word
//! let prev_start = find_prev_word_start(text, cursor);
//! assert_eq!(prev_start, 6); // Points to 'w' in "world"
//!
//! // Find end of next word
//! let next_end = find_next_word_end(text, cursor);
//! assert_eq!(next_end, 15); // After "foo"
//! ```
//!
//! ## Word Boundary Rules
//!
//! A character is considered a word boundary if it is:
//! - Whitespace (`.is_whitespace()`)
//! - ASCII punctuation (`.is_ascii_punctuation()`)
//!
//! Everything else is considered a word character.
//!
//! ## Examples
//!
//! ```text
//! "hello world"  → words: ["hello", "world"]
//! "hello-world"  → words: ["hello", "world"] (hyphen is boundary)
//! "foo.bar()"    → words: ["foo", "bar"] (punctuation is boundary)
//! "hello  world" → words: ["hello", "world"] (multiple spaces treated as one boundary)
//! ```

mod word_boundary_detection;

pub use word_boundary_detection::*;
