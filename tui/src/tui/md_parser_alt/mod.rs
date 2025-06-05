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

//! # Naming convention and semantics
//! This naming convention is used throughout this file to make sure code is readable,
//! understandable, and maintainable.
//!
//! | Fn Name Pattern     | Purpose                                                                    |
//! | ------------------- | -------------------------------------------------------------------------- |
//! | `parse_*()`         | Boundary detection: Splits items from input into remainder and output      |
//! | `*_extract`         | Content extraction: Convert already-split-input using `parse_*()` to model |
//! | `*_parser()`        | A function that receives an input, and is called by `parse_*()`            |
//!
//! # Polymorphic behavior of nom-compatible struct
//! Since [AsStrSlice] implements [nom::Input], any function that can receive a
//! [nom::Input] can accept [AsStrSlice] type.
//!
//! Depending on your needs, you can interchangeably treat a [AsStrSlice] as a
//! [nom::Input], or [AsStrSlice] as needed, so there is a lot of flexibility in how to
//! access the `input`, in a "nom compatible" way. Also [CGStringSlice] is
//! [Clone] and it is very cheap. These features are used in many of the
//! functions in this module.

// Attach sources.
pub mod extended_alt;
pub mod fragment_alt;
pub mod parser_impl;
pub mod string_slice;

// Re-export.
pub use extended_alt::*;
pub use fragment_alt::*;
pub use parser_impl::*;
pub use string_slice::*;
