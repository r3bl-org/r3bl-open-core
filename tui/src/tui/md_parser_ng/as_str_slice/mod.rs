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

//! # AsStrSlice Module - Unicode-Safe Virtual String Array for nom Parsers
//!
//! This module provides `AsStrSlice`, a wrapper type that enables efficient `nom` parsing
//! over multi-line text without materializing the entire content into a single string.
//! It was designed to solve performance bottlenecks in the R3BL TUI editor's render loop
//! when parsing markdown and other structured text formats.
//!
//! ## Why This Module Exists
//!
//! The R3BL TUI editor stores document content as `&[GCString]` (array of lines) in
//! `EditorContent`. To use `nom` parsers with this data structure, the original
//! implementation had to materialize the entire document into a contiguous string
//! during each render cycle, causing severe performance issues with large documents.
//!
//! `AsStrSlice` solves this by creating a "virtual array" that:
//! - Acts like a contiguous string for `nom` parsers
//! - Never copies or materializes the underlying line data
//! - Handles synthetic newlines between lines automatically
//! - Supports cheap cloning and slicing operations
//!
//! ## Key Features
//!
//! ### Unicode/UTF-8 Safety
//! **Critical**: This module uses CHARACTER-BASED indexing throughout, never byte-based
//! indexing. This ensures proper handling of multi-byte UTF-8 sequences like emojis
//! (`😀`). Mixing byte and character operations will cause panics or incorrect results.
//!
//! ### Virtual String Array
//! - Wraps `&[T]` where `T: AsRef<str>` (typically `&[GCString]`)
//! - Provides contiguous string-like access without copying data
//! - Automatically inserts synthetic newlines between lines
//! - Cheap clone operations (just copies references and indices)
//!
//! ### nom Parser Integration
//! - Implements `nom::Input` trait for seamless parser integration
//! - Compatible with all standard `nom` combinators
//! - Maintains position tracking for error reporting
//! - Supports both streaming and complete parsing modes
//!
//! ## Core Components
//!
//! - **`core`**: Contains the main `AsStrSlice` struct and implementation
//! - **`traits`**: Conversion traits and `nom::Input` implementation
//! - **`position`**: Position tracking and advancement logic
//!
//! ## Example Usage
//!
//! ```rust
//! use r3bl_tui::{AsStrSlice, GCString};
//! use nom::{bytes::complete::tag, IResult};
//!
//! // Create from line array (typical TUI editor usage)
//! let lines = vec![GCString::from("# Header"), GCString::from("Content")];
//! let slice = AsStrSlice::from(&lines[..]);
//!
//! // Use with nom parsers
//! fn parse_header(input: AsStrSlice) -> IResult<AsStrSlice, AsStrSlice> {
//!     tag("# ")(input)
//! }
//!
//! let (remaining, header_prefix) = parse_header(slice).unwrap();
//! ```
//!
//! ## ⚠️ CRITICAL: Unicode/UTF-8 Safety
//!
//! **This module uses CHARACTER-BASED indexing for Unicode/UTF-8 safety.**
//!
//! Never mix byte-based operations with character-based operations as this will cause
//! panics or incorrect results when processing multi-byte UTF-8 characters like emojis.
//!
//! **📖 For detailed safety guidelines, patterns, and examples, see the comprehensive
//! documentation on the [`AsStrSlice`] struct.**

// Attach.
#[rustfmt::skip]
pub mod compatibility;
#[rustfmt::skip]
pub mod as_str_slice_core;
#[rustfmt::skip]
pub mod iterators;
#[rustfmt::skip]
pub mod operations;
#[rustfmt::skip]
pub mod position;
#[rustfmt::skip]
pub mod traits;

// Re-export.
#[rustfmt::skip] 
pub use as_str_slice_core::*;
#[rustfmt::skip] 
pub use compatibility::*;
#[rustfmt::skip] 
pub use iterators::*;
#[rustfmt::skip] 
pub use operations::*;
#[rustfmt::skip] 
pub use position::*;
#[rustfmt::skip] 
pub use traits::*;
