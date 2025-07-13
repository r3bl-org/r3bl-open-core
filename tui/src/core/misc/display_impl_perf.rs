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

//! Performance optimization utilities for [`std::fmt::Display`] trait implementations.
//!
//! This module provides memoization structures to optimize expensive calculations
//! in [`std::fmt::Display`] trait implementations, particularly for telemetry logging in
//! the [`crate::TerminalWindow::main_event_loop`].

use std::fmt::Display;

use crate::format_as_kilobytes_with_commas;

/// Memory size wrapper for telemetry display.
///
/// This struct wraps a memory size value and provides a `Display` implementation
/// that shows the size in kilobytes with commas for readability, or "?" if the
/// size is not available.
#[derive(Debug, Clone, PartialEq)]
pub struct MemorySize {
    inner: Option<usize>,
}

impl MemorySize {
    /// Creates a new `MemorySize` with a value.
    #[must_use]
    pub fn new(size: usize) -> Self { Self { inner: Some(size) } }

    /// Creates a new `MemorySize` with no value.
    #[must_use]
    pub fn unknown() -> Self { Self { inner: None } }

    /// Gets the inner size value if available.
    #[must_use]
    pub fn size(&self) -> Option<usize> { self.inner }
}

impl Display for MemorySize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Get memory size from cache if available, otherwise show "?".
        let memory_str = if let Some(size) = self.inner {
            format_as_kilobytes_with_commas(size)
        } else {
            "?".into()
        };
        write!(f, "{memory_str}")
    }
}

impl Default for MemorySize {
    fn default() -> Self { Self::unknown() }
}

/// Memoized value calculation for performance optimization.
///
/// Generic version of the memoization cache that can hold any type `T` that
/// implements `Display`. This allows for flexible caching of different types
/// while maintaining the same invalidation and recalculation semantics.
#[derive(Debug, Clone, PartialEq)]
pub struct MemoizedValue<T: Display> {
    /// Cached value to avoid expensive recalculation.
    maybe_value: Option<T>,
    /// Flag indicating if the cached value needs to be recalculated.
    is_dirty: bool,
}

impl<T: Display> Default for MemoizedValue<T> {
    fn default() -> Self {
        Self {
            maybe_value: None,
            is_dirty: true,
        }
    }
}

impl<T: Display> MemoizedValue<T> {
    /// Creates a new `MemoizedValue` with no cached value.
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Marks the cache as invalid, requiring recalculation on next access.
    pub fn invalidate(&mut self) { self.is_dirty = true; }

    /// Updates the cache with a new value if dirty or not present.
    /// The provided closure is only called if recalculation is needed.
    pub fn upsert<F>(&mut self, calculate_fn: F)
    where
        F: FnOnce() -> T,
    {
        if self.is_dirty || self.maybe_value.is_none() {
            self.maybe_value = Some(calculate_fn());
            self.is_dirty = false;
        }
    }

    /// Gets the cached value if available and not dirty.
    #[must_use]
    pub fn get_cached(&self) -> Option<&T> {
        if self.is_dirty {
            None
        } else {
            self.maybe_value.as_ref()
        }
    }

    /// Gets the cached value or calculates and caches a new one.
    /// Useful for immutable contexts where [`Self::upsert`] can't be used.
    ///
    /// # Panics
    /// This method should not panic as the value is guaranteed to be set after `upsert`.
    pub fn get_or_insert_with<F>(&mut self, calculate_fn: F) -> &T
    where
        F: FnOnce() -> T,
    {
        self.upsert(calculate_fn);
        debug_assert!(
            self.maybe_value.is_some(),
            "Cached value should be set after upsert"
        );
        self.maybe_value
            .as_ref()
            .expect("Value should be cached after upsert")
    }

    /// Checks if the cache needs to be updated.
    #[must_use]
    pub fn is_dirty(&self) -> bool { self.is_dirty || self.maybe_value.is_none() }
}

/// Type alias for memory size memoization.
pub type MemoizedMemorySize = MemoizedValue<MemorySize>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_size_display() {
        // Test with known value
        let size = MemorySize::new(1024 * 1024); // 1MB
        assert_eq!(format!("{size}"), "1,024 KB");

        // Test with unknown value
        let unknown = MemorySize::unknown();
        assert_eq!(format!("{unknown}"), "?");

        // Test with default
        let default = MemorySize::default();
        assert_eq!(format!("{default}"), "?");
    }

    #[test]
    fn test_memoized_value() {
        let mut cache = MemoizedValue::new();

        // Initially empty and dirty.
        assert!(cache.is_dirty());
        assert_eq!(cache.get_cached(), None);

        // Calculate and cache.
        let mut calc_count = 0;
        cache.upsert(|| {
            calc_count += 1;
            MemorySize::new(100 * 1024)
        });

        assert!(!cache.is_dirty());
        assert!(cache.get_cached().is_some());
        assert_eq!(calc_count, 1);

        // Subsequent calls don't recalculate.
        cache.upsert(|| {
            calc_count += 1;
            MemorySize::new(200 * 1024)
        });

        assert_eq!(calc_count, 1); // Still 1.

        // Invalidate and recalculate.
        cache.invalidate();
        assert!(cache.is_dirty());
        assert_eq!(cache.get_cached(), None);

        cache.upsert(|| {
            calc_count += 1;
            MemorySize::new(200 * 1024)
        });

        assert_eq!(calc_count, 2);
    }
}
