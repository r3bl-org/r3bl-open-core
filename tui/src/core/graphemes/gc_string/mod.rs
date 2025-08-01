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

//! [`GCStringOwned`] implementation for Unicode grapheme cluster handling.
//!
//! This module contains the [`GCStringOwned`] type that provides Unicode-safe string
//! operations with grapheme cluster support. It's used throughout the TUI system
//! for text formatting, clipping, and rendering operations.
//!
//! See the [module docs](crate::graphemes) for comprehensive information about Unicode
//! handling, grapheme clusters, and the three types of indices used in this system.

// Submodules
pub mod owned;

// Re-exports
pub use owned::*;
