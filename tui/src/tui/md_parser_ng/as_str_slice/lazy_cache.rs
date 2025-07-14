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

//! # Lazy Cache for [`super::AsStrSlice`]
//!
//! This module provides a lazy cache wrapper that only computes the cache
//! when it's actually needed, avoiding the overhead of cache creation for
//! simple operations that don't require it.
//!
//! ## Design Rationale
//!
//! The cache uses `RefCell<Option<AsStrSliceCache>>` for the following reasons:
//! - `Option`: Enables lazy initialization - starts as `None` and becomes `Some` on first
//!   access
//! - `RefCell`: Provides interior mutability, allowing cache creation from immutable
//!   `&self` methods
//! - `Rc`: Originally used for sharing cache between clones, but now each clone gets an
//!   independent cache to avoid bugs where different `AsStrSlice` instances at different
//!   positions would share the same cached data
//!
//! Note: The `Rc` wrapper could potentially be removed since we're no longer sharing
//! caches between clones, but it's kept for now to minimize changes.

use std::{cell::{Ref, RefCell},
          rc::Rc};

use crate::{AsStrSliceCache, GCString};

/// Lazy cache that computes the cache only when needed.
///
/// This cache is designed to avoid the overhead of computing character counts
/// and byte offsets for simple operations that don't need them. The cache is
/// only created when methods like `extract_to_line_end()` or `take_from()`
/// are called that require character-to-byte position mappings.
///
/// ## Important: Clone Behavior
///
/// When a `LazyCache` is cloned, the clone gets a completely independent cache.
/// This prevents bugs where multiple `AsStrSlice` instances at different positions
/// would incorrectly share cached position data.
#[derive(Debug)]
pub struct LazyCache<'a> {
    /// The lines reference for creating the cache
    lines: &'a [GCString],
    /// The actual cache, created on first access.
    /// Uses `Rc<RefCell<Option<...>>>` for:
    /// - [`Option`]: Lazy initialization
    /// - [`RefCell`]: Interior mutability (modify from &self)
    /// - [`Rc`]: Historical reasons (could be removed)
    cache: Rc<RefCell<Option<AsStrSliceCache>>>,
}

impl<'a> LazyCache<'a> {
    /// Create a new lazy cache.
    ///
    /// The cache starts uninitialized (`None`) and will be computed on first access.
    #[must_use]
    pub fn new(lines: &'a [GCString]) -> Self {
        Self {
            lines,
            cache: Rc::new(RefCell::new(None)),
        }
    }

    /// Create a new lazy cache with fresh state (used when cloning).
    ///
    /// This ensures each cloned `AsStrSlice` gets its own independent cache,
    /// preventing position data corruption between instances.
    #[must_use]
    fn new_independent(lines: &'a [GCString]) -> Self {
        Self {
            lines,
            cache: Rc::new(RefCell::new(None)),
        }
    }

    /// Get the cache, creating it if necessary.
    ///
    /// This method implements lazy initialization:
    /// - On first call: Computes character counts and byte offsets for all lines
    /// - On subsequent calls: Returns the cached data
    ///
    /// The computation involves iterating through all lines to build:
    /// - `LineMetadataCache`: Character counts and cumulative offsets
    /// - `LineByteOffsetCache`: Character-to-byte position mappings
    ///
    /// # Performance
    ///
    /// The initial cache creation is O(n*m) where n is the number of lines and
    /// m is the average line length. Subsequent accesses are O(1).
    ///
    /// # Panics
    ///
    /// This method will never panic. The `unwrap()` call is safe because the
    /// preceding logic guarantees that the cache is always `Some` when accessed.
    /// The method first checks if the cache is `None` and creates it if needed,
    /// ensuring the `Option` is always `Some` before the `unwrap()` is called.
    #[must_use]
    pub fn get(&self) -> Ref<'_, AsStrSliceCache> {
        // Check if cache exists
        if self.cache.borrow().is_none() {
            // Create the cache
            let new_cache = AsStrSliceCache::new(self.lines);
            *self.cache.borrow_mut() = Some(new_cache);
        }

        // Return a reference to the cache
        Ref::map(self.cache.borrow(), |opt| opt.as_ref().unwrap())
    }

    /// Check if the cache has been created
    #[must_use]
    pub fn is_created(&self) -> bool { self.cache.borrow().is_some() }
}

impl Clone for LazyCache<'_> {
    fn clone(&self) -> Self {
        // IMPORTANT: Create a new independent cache for the clone.
        // This is critical for correctness - if clones shared the same cache,
        // different AsStrSlice instances at different positions would corrupt
        // each other's cached position data.
        //
        // This was a bug that caused incorrect text extraction where "bold"
        // would be extracted as "his is " due to shared cache state.
        Self::new_independent(self.lines)
    }
}

impl PartialEq for LazyCache<'_> {
    fn eq(&self, other: &Self) -> bool {
        // Two lazy caches are equal if they reference the same lines.
        // We don't compare the cache contents because:
        // 1. The cache is lazily initialized and might not exist
        // 2. Two caches with the same lines will produce the same data
        // 3. We only care about structural equality, not cache state
        std::ptr::eq(self.lines, other.lines)
    }
}
