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
//!
//! The memory size related types have been moved to [`crate::GetMemSize`] module
//! to consolidate memory size calculation and caching logic.

use std::fmt::Display;

// Re-export memory size types from get_mem_size module
pub use crate::{MemoizedMemorySize, MemorySize};

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

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use super::*;

    /// A test type that implements Display and tracks calculation calls
    #[derive(Debug, Clone, PartialEq)]
    struct TestValue {
        value: String,
        calculation_count: Rc<RefCell<usize>>,
    }

    impl TestValue {
        fn with_shared_counter(value: &str, counter: Rc<RefCell<usize>>) -> Self {
            Self {
                value: value.to_string(),
                calculation_count: counter,
            }
        }
    }

    impl Display for TestValue {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            *self.calculation_count.borrow_mut() += 1;
            write!(f, "{}", self.value)
        }
    }

    #[test]
    fn test_new_creates_empty_dirty_cache() {
        let cache: MemoizedValue<TestValue> = MemoizedValue::new();
        assert!(cache.is_dirty());
        assert!(cache.get_cached().is_none());
    }

    #[test]
    fn test_default_creates_empty_dirty_cache() {
        let cache: MemoizedValue<TestValue> = MemoizedValue::default();
        assert!(cache.is_dirty());
        assert!(cache.get_cached().is_none());
    }

    #[test]
    fn test_upsert_calculates_when_dirty() {
        let mut cache = MemoizedValue::new();
        let counter = Rc::new(RefCell::new(0));
        let counter_clone = counter.clone();

        cache.upsert(|| TestValue::with_shared_counter("test", counter_clone));

        assert!(!cache.is_dirty());
        assert_eq!(*counter.borrow(), 0); // Display not called yet

        // Verify the value is cached
        let cached = cache.get_cached().unwrap();
        assert_eq!(cached.value, "test");
    }

    #[test]
    fn test_upsert_does_not_recalculate_when_clean() {
        let mut cache = MemoizedValue::new();
        let counter = Rc::new(RefCell::new(0));
        let counter_clone = counter.clone();

        // First upsert
        cache.upsert(|| TestValue::with_shared_counter("test1", counter_clone.clone()));

        // Second upsert should not recalculate
        cache.upsert(|| TestValue::with_shared_counter("test2", counter_clone));

        let cached = cache.get_cached().unwrap();
        assert_eq!(cached.value, "test1"); // Should still be the first value
    }

    #[test]
    fn test_invalidate_marks_cache_dirty() {
        let mut cache = MemoizedValue::new();
        let counter = Rc::new(RefCell::new(0));

        cache.upsert(|| TestValue::with_shared_counter("test", counter));
        assert!(!cache.is_dirty());

        cache.invalidate();
        assert!(cache.is_dirty());
    }

    #[test]
    fn test_get_cached_returns_none_when_dirty() {
        let mut cache = MemoizedValue::new();
        let counter = Rc::new(RefCell::new(0));

        cache.upsert(|| TestValue::with_shared_counter("test", counter));
        assert!(cache.get_cached().is_some());

        cache.invalidate();
        assert!(cache.get_cached().is_none());
    }

    #[test]
    fn test_get_or_insert_with_calculates_when_needed() {
        let mut cache = MemoizedValue::new();
        let counter = Rc::new(RefCell::new(0));
        let counter_clone = counter.clone();

        let result = cache
            .get_or_insert_with(|| TestValue::with_shared_counter("test", counter_clone));

        assert_eq!(result.value, "test");
        assert!(!cache.is_dirty());
    }

    #[test]
    fn test_get_or_insert_with_returns_cached_when_clean() {
        let mut cache = MemoizedValue::new();
        let counter = Rc::new(RefCell::new(0));
        let counter_clone = counter.clone();

        // First call
        let result1_value = cache
            .get_or_insert_with(|| {
                TestValue::with_shared_counter("test1", counter_clone.clone())
            })
            .value
            .clone();

        // Second call should return the same cached value
        let result2_value = cache
            .get_or_insert_with(|| TestValue::with_shared_counter("test2", counter_clone))
            .value
            .clone();

        assert_eq!(result1_value, "test1");
        assert_eq!(result2_value, "test1"); // Should be the same
    }

    #[test]
    fn test_is_dirty_tracks_state_correctly() {
        let mut cache = MemoizedValue::new();
        let counter = Rc::new(RefCell::new(0));

        // Initially dirty
        assert!(cache.is_dirty());

        // After upsert, should be clean
        cache.upsert(|| TestValue::with_shared_counter("test", counter));
        assert!(!cache.is_dirty());

        // After invalidate, should be dirty again
        cache.invalidate();
        assert!(cache.is_dirty());
    }

    #[test]
    fn test_clone_preserves_state() {
        let mut cache = MemoizedValue::new();
        let counter = Rc::new(RefCell::new(0));

        cache.upsert(|| TestValue::with_shared_counter("test", counter));
        let cloned = cache.clone();

        assert_eq!(cache.is_dirty(), cloned.is_dirty());
        assert_eq!(
            cache.get_cached().map(|v| &v.value),
            cloned.get_cached().map(|v| &v.value)
        );
    }

    #[test]
    fn test_partial_eq_works_correctly() {
        let mut cache1 = MemoizedValue::new();
        let mut cache2 = MemoizedValue::new();
        let counter = Rc::new(RefCell::new(0));

        // Both empty and dirty - should be equal
        assert_eq!(cache1, cache2);

        // Both have same value - should be equal
        cache1.upsert(|| TestValue::with_shared_counter("test", counter.clone()));
        cache2.upsert(|| TestValue::with_shared_counter("test", counter));
        assert_eq!(cache1, cache2);

        // One is dirty, other is not - should not be equal
        cache1.invalidate();
        assert_ne!(cache1, cache2);
    }

    #[test]
    fn test_debug_format_shows_internal_state() {
        let mut cache = MemoizedValue::new();
        let counter = Rc::new(RefCell::new(0));

        let debug_empty = format!("{:?}", cache);
        assert!(debug_empty.contains("maybe_value: None"));
        assert!(debug_empty.contains("is_dirty: true"));

        cache.upsert(|| TestValue::with_shared_counter("test", counter));
        let debug_filled = format!("{:?}", cache);
        assert!(debug_filled.contains("maybe_value: Some"));
        assert!(debug_filled.contains("is_dirty: false"));
    }

    #[test]
    fn test_with_string_type() {
        let mut cache: MemoizedValue<String> = MemoizedValue::new();
        let call_count = Rc::new(RefCell::new(0));
        let call_count_clone = call_count.clone();

        let result = cache.get_or_insert_with(|| {
            *call_count_clone.borrow_mut() += 1;
            "Hello, World!".to_string()
        });

        assert_eq!(result, "Hello, World!");
        assert_eq!(*call_count.borrow(), 1);

        // Second call should not increment the counter
        let result2 = cache.get_or_insert_with(|| {
            *call_count_clone.borrow_mut() += 1;
            "Should not be called".to_string()
        });

        assert_eq!(result2, "Hello, World!");
        assert_eq!(*call_count.borrow(), 1); // Should still be 1
    }

    #[test]
    fn test_with_numeric_type() {
        let mut cache: MemoizedValue<i32> = MemoizedValue::new();
        let calculation_calls = Rc::new(RefCell::new(0));
        let calculation_calls_clone = calculation_calls.clone();

        cache.upsert(|| {
            *calculation_calls_clone.borrow_mut() += 1;
            42
        });

        assert_eq!(cache.get_cached(), Some(&42));
        assert_eq!(*calculation_calls.borrow(), 1);

        // Upsert again - should not recalculate
        cache.upsert(|| {
            *calculation_calls_clone.borrow_mut() += 1;
            99
        });

        assert_eq!(cache.get_cached(), Some(&42)); // Should still be 42
        assert_eq!(*calculation_calls.borrow(), 1); // Should still be 1
    }
}
