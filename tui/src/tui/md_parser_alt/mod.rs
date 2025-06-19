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

//! # Polymorphic behavior of nom-compatible struct
//!
//! Since [AsStrSlice] implements [nom::Input], any function that can receive a
//! [nom::Input] can accept [AsStrSlice] type.
//!
//! Depending on your needs, you can interchangeably treat a [AsStrSlice] as a
//! [nom::Input], or [AsStrSlice] as needed, so there is a lot of flexibility in how to
//! access the `input`, in a "nom compatible" way. Also [crate::GCString] is
//! [Clone] and it is very cheap. These features are used in many of the
//! functions in this module.
//!
//! # Line vs block parsers
//!
//! Almost all the parsers in this module are line parsers, meaning they parse a single
//! line of text, and return the remainder of the input after parsing that line.
//!
//! The parsers that are not line parsers are block parsers, meaning they parse across
//! line breaks and peek ahead to find the end of the block (which may span multiple
//! lines). Block parser are in the [mod@block_alt] module.

// Attach sources.
pub mod as_str_slice;
pub mod block_alt;
pub mod extended_alt;
pub mod fragment_alt;
pub mod standard_alt;

// Re-export.
pub use as_str_slice::*;
pub use block_alt::*;
pub use extended_alt::*;
pub use fragment_alt::*;
pub use standard_alt::*;
