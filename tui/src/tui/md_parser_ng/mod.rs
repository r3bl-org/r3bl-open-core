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

//! # Next Generation Markdown Parser (`md_parser_ng`)
//!
//! This module provides a **high-performance, zero-allocation markdown parser** designed
//! as a modernized replacement for the original [`crate::md_parser`] module. It leverages
//! advanced virtual array techniques and Unicode-compliant processing to deliver
//! **O(1) operations** with **no memory copying** for superior performance in
//! real-time text editing scenarios.
//!
//! ## Key Performance Characteristics
//!
//! - **🚀 Zero-allocation parsing**: No string copying or memory allocation during
//!   parsing
//! - **⚡ O(1) virtual array access**: Virtual array abstraction over line-based data
//! - **🦄 Unicode-safe**: Full UTF-8 support with proper grapheme cluster handling
//! - **🔧 Real-time optimized**: Designed for interactive text editor performance
//! - **📦 Memory efficient**: Minimal memory footprint with cheap cloning via
//!   [`crate::GCString`]
//!
//! ## Architecture Overview
//!
//! The parser is built around the [`AsStrSlice`] abstraction, which provides a crucial
//! bridge between how text editors store content (as arrays of lines) and how parsers
//! need to access it (as continuous character streams). This virtual array approach
//! eliminates the need for expensive string concatenation or copying operations.
//!
//! ### Core Components
//!
//! - **[`parse_markdown_ng()`]**: Main entry point for parsing complete markdown
//!   documents
//! - **[`AsStrSlice`]**: Virtual array abstraction with [`nom::Input`] compatibility
//! - **Parser Modules**: Specialized parsers for different markdown elements
//!
//! ## Parser Categories
//!
//! ### Line Parsers
//! Most parsers in this module are **line parsers** that process a single line of text
//! and return the remainder. These include:
//!
//! - **[mod@standard_ng]**: Common markdown elements (headings, text)
//! - **[mod@extended_ng]**: Specialized formats (metadata, key-value pairs)
//! - **[mod@fragment_ng]**: Inline elements (bold, italic, links, code spans)
//!
//! ### Block Parsers
//! **Block parsers** handle multi-line structures that span across line boundaries:
//!
//! - **[mod@block_ng]**: Multi-line elements (code blocks, smart lists with nesting)
//!
//! ## Virtual Array Technology
//!
//! The [`AsStrSlice`] provides **polymorphic behavior** as a nom-compatible struct:
//!
//! Since [`AsStrSlice`] implements [`nom::Input`], any function expecting a
//! [`nom::Input`] can seamlessly accept [`AsStrSlice`]. This flexibility allows
//! treating the same data structure as either a virtual array or nom input,
//! enabling sophisticated parsing patterns without performance penalties.
//!
//! ## Unicode & Performance
//!
//! - **Grapheme cluster aware**: Proper handling of complex Unicode characters
//! - **Emoji support**: Robust processing of multi-byte emoji sequences
//! - **No string slicing**: Virtual indexing eliminates expensive substring operations
//! - **Synthetic newlines**: Maintains line boundaries without copying data
//!
//! ## Usage in Text Editors
//!
//! This parser is specifically optimized for **interactive text editing** where:
//! - Documents can be very large (thousands of lines)
//! - Parsing must happen in real-time as users type
//! - Memory usage must remain minimal and predictable
//! - Unicode content must be handled correctly without crashes

// Attach sources.
pub mod as_str_slice;
pub mod block_ng;
pub mod extended_ng;
pub mod fragment_ng;
pub mod parse_markdown_ng;
pub mod standard_ng;

// Re-export.
pub use as_str_slice::*;
pub use block_ng::*;
pub use extended_ng::*;
pub use fragment_ng::*;
pub use parse_markdown_ng::*;
pub use standard_ng::*;
