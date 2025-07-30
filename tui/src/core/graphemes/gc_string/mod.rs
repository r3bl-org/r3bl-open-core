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

//! [`GCString`] trait implementations and related abstractions.
//!
//! This module contains all string types that work with grapheme clusters,
//! including owned and borrowed variants, along with their shared abstractions
//! and utilities.
//!
//! See the [module docs](crate::graphemes) for
//! comprehensive information about Unicode handling, grapheme clusters, and the three
//! types of indices used in this system.
//!
//! This module provides two main traits:
//! - [`GCString`] for high-level string operations. This cares about the ownership model
//!   of the string data, allowing implementations to define how they handle string data,
//!   whether owned or borrowed. The associated type [`GCString::StringResult`] allows
//!   each implementation to return its appropriate string result type.
//! - [`GCStringData`] for low-level data access. This trait does not care about ownership
//!   and is used for shared data access logic, for types that implement it. These are
//!   also the same types that implement [`GCString`] trait). It provides methods to
//!   access the underlying string data, segments, and their properties without exposing
//!   ownership details.
//!
//! ## Why can't we just have one trait?
//!
//! Why This Can't Be Just One Trait
//!
//! Problem 1: Associated Types vs Generic Parameters
//! - [`GCString`] needs associated types ([`GCString::StringResult`]) for type safety
//! - `[GCStringData]` needs to be generic over implementations for code reuse
//! - These are incompatible requirements
//!
//! Problem 2: Different Abstraction Levels
//! - [`GCStringData`]: "How do I access the raw data?"
//! - [`GCString`]: "What operations can I perform on grapheme strings?"
//! - Mixing these concerns would violate single responsibility principle
//!
//! Problem 3: Code Reuse Architecture
//! - The shared functions in [`crate::gc_string::common`] need a stable, simple interface
//!   (`[GCStringData]`)
//! - They shouldn't be coupled to the complex public API (`[GCString]`)
//! - This allows algorithms to be implemented once and shared
//!
//! ## Benefits of this two trait design
//!
//! This is a sophisticated separation of concerns that achieves:
//! - Code Reuse: Complex Unicode algorithms written once, used by both implementations
//! - Type Safety: Associated types ensure owned/borrowed types don't mix incorrectly
//! - Clean Architecture: Data access separated from business logic
//! - Maintainability: Changes to algorithms happen in one place
//! - Performance: No unnecessary allocations or copies
//!
//! The current design is an idiomatic example of how to handle owned vs borrowed types in
//! Rust while maintaining both performance and maintainability. It's not
//! over-engineered.

// Submodules
pub mod borrowed;
pub mod common;
pub mod gc_string_trait;
pub mod iterator;
pub mod owned;

// Re-exports
pub use borrowed::*;
pub use common::*;
pub use gc_string_trait::*;
pub use iterator::*;
pub use owned::*;

// Tests
pub mod trait_impl_compat_test;
