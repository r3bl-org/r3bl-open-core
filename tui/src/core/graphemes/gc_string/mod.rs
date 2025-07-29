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

//! `GCString` implementations and related abstractions.
//!
//! This module contains all string types that work with grapheme clusters,
//! including owned and borrowed variants, along with their shared abstractions
//! and utilities.

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