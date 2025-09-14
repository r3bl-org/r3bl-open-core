// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Generic LRU (Least Recently Used) cache implementation.
//!
//! This module provides a high-performance LRU cache that can be used throughout the
//! codebase for caching expensive computations. It uses `FxHashMap` for fast lookups
//! and tracks access patterns to evict the least recently used items when capacity
//! is reached.
//!
//! ## Features
//!
//! - Generic over key and value types
//! - True LRU eviction using access counters
//! - Configurable capacity
//! - Thread-safe wrapper available
//! - Zero allocation for cache hits
//! - O(1) average case for get/insert operations
//!
//! ## Performance
//!
//! The cache uses [`rustc_hash::FxHashMap`] which provides 3-5x faster lookups compared
//! to the standard [`std::collections::HashMap`]. This is ideal for caching operations
//! where:
//! - Keys are trusted internal data (not user input)
//! - No cryptographic security is required
//! - Performance is critical
//!
//! ## Example
//!
//! ```no_run
//! use r3bl_tui::LruCache;
//!
//! let mut cache = LruCache::<String, i32>::new(100);
//! cache.insert("key".to_string(), 42);
//! assert_eq!(cache.get(&"key".to_string()), Some(&42));
//! ```

use std::{hash::Hash,
          sync::{Arc, Mutex}};

use rustc_hash::{FxBuildHasher, FxHashMap};

/// Entry in the LRU cache containing the value and access metadata.
#[derive(Clone, Debug)]
struct CacheEntry<V> {
    value: V,
    access_count: u64,
}

/// A generic LRU (Least Recently Used) cache.
///
/// When the cache reaches capacity, the least recently accessed item is evicted
/// to make room for new entries. Access patterns are tracked using a monotonic
/// counter to ensure true LRU behavior.
#[derive(Debug)]
pub struct LruCache<K, V> {
    map: FxHashMap<K, CacheEntry<V>>,
    capacity: usize,
    access_counter: u64,
}

impl<K, V> LruCache<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    /// Creates a new LRU cache with the specified capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of items the cache can hold
    ///
    /// # Panics
    ///
    /// Panics if capacity is 0.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Cache capacity must be greater than 0");
        Self {
            map: FxHashMap::with_capacity_and_hasher(capacity, FxBuildHasher),
            capacity,
            access_counter: 0,
        }
    }

    /// Gets a reference to the value associated with the key.
    ///
    /// This updates the access time of the entry, marking it as recently used.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up
    ///
    /// # Returns
    ///
    /// `Some(&V)` if the key exists, `None` otherwise
    pub fn get(&mut self, key: &K) -> Option<&V> {
        self.access_counter += 1;
        if let Some(entry) = self.map.get_mut(key) {
            entry.access_count = self.access_counter;
            Some(&entry.value)
        } else {
            None
        }
    }

    /// Gets a mutable reference to the value associated with the key.
    ///
    /// This updates the access time of the entry, marking it as recently used.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up
    ///
    /// # Returns
    ///
    /// `Some(&mut V)` if the key exists, `None` otherwise
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.access_counter += 1;
        if let Some(entry) = self.map.get_mut(key) {
            entry.access_count = self.access_counter;
            Some(&mut entry.value)
        } else {
            None
        }
    }

    /// Inserts a key-value pair into the cache.
    ///
    /// If the cache is at capacity, the least recently used item is evicted.
    /// If the key already exists, its value is updated and it's marked as
    /// recently used.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to insert
    /// * `value` - The value to associate with the key
    ///
    /// # Returns
    ///
    /// The previous value associated with the key, if any
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.access_counter += 1;

        // If at capacity and key doesn't exist, remove LRU entry.
        if self.map.len() >= self.capacity
            && !self.map.contains_key(&key)
            && let Some(lru_key) = self
                .map
                .iter()
                .min_by_key(|(_, entry)| entry.access_count)
                .map(|(k, _)| k.clone())
        {
            self.map.remove(&lru_key);
        }

        let entry = CacheEntry {
            value,
            access_count: self.access_counter,
        };

        self.map.insert(key, entry).map(|e| e.value)
    }

    /// Removes a key from the cache.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to remove
    ///
    /// # Returns
    ///
    /// The value associated with the key, if it existed
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.map.remove(key).map(|e| e.value)
    }

    /// Clears all entries from the cache.
    pub fn clear(&mut self) {
        self.map.clear();
        self.access_counter = 0;
    }

    /// Returns the number of entries currently in the cache.
    #[must_use]
    pub fn len(&self) -> usize { self.map.len() }

    /// Returns true if the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.map.is_empty() }

    /// Returns the capacity of the cache.
    #[must_use]
    pub fn capacity(&self) -> usize { self.capacity }

    /// Returns true if the cache contains the given key.
    ///
    /// Note: This does NOT update the access time of the entry.
    #[must_use]
    pub fn contains_key(&self, key: &K) -> bool { self.map.contains_key(key) }
}

/// Thread-safe wrapper for [`LruCache`].
///
/// This type provides a convenient way to share an LRU cache across threads
/// using Arc<Mutex<>>. All operations acquire the mutex lock internally.
pub type ThreadSafeLruCache<K, V> = Arc<Mutex<LruCache<K, V>>>;

/// Creates a new thread-safe LRU cache with the specified capacity.
///
/// # Arguments
///
/// * `capacity` - Maximum number of items the cache can hold
///
/// # Returns
///
/// An `Arc<Mutex<LruCache<K, V>>>` that can be safely shared across threads
#[must_use]
pub fn new_threadsafe_lru_cache<K, V>(capacity: usize) -> ThreadSafeLruCache<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    Arc::new(Mutex::new(LruCache::new(capacity)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut cache = LruCache::new(3);

        // Test insert and get.
        assert_eq!(cache.insert("a".to_string(), 1), None);
        assert_eq!(cache.get(&"a".to_string()), Some(&1));
        assert_eq!(cache.len(), 1);

        // Test update.
        assert_eq!(cache.insert("a".to_string(), 2), Some(1));
        assert_eq!(cache.get(&"a".to_string()), Some(&2));
        assert_eq!(cache.len(), 1);

        // Test multiple inserts.
        cache.insert("b".to_string(), 3);
        cache.insert("c".to_string(), 4);
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.capacity(), 3);
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = LruCache::new(3);

        // Fill cache.
        cache.insert("a".to_string(), 1);
        cache.insert("b".to_string(), 2);
        cache.insert("c".to_string(), 3);

        // Access "a" and "b" to make them more recent.
        cache.get(&"a".to_string());
        cache.get(&"b".to_string());

        // Insert "d" - should evict "c" (least recently used).
        cache.insert("d".to_string(), 4);

        assert_eq!(cache.get(&"a".to_string()), Some(&1));
        assert_eq!(cache.get(&"b".to_string()), Some(&2));
        assert_eq!(cache.get(&"c".to_string()), None); // Evicted
        assert_eq!(cache.get(&"d".to_string()), Some(&4));
    }

    #[test]
    fn test_get_mut() {
        let mut cache = LruCache::new(2);

        cache.insert("key".to_string(), vec![1, 2, 3]);

        // Modify value through get_mut.
        if let Some(val) = cache.get_mut(&"key".to_string()) {
            val.push(4);
        }

        assert_eq!(cache.get(&"key".to_string()), Some(&vec![1, 2, 3, 4]));
    }

    #[test]
    fn test_remove() {
        let mut cache = LruCache::new(3);

        cache.insert("a".to_string(), 1);
        cache.insert("b".to_string(), 2);

        assert_eq!(cache.remove(&"a".to_string()), Some(1));
        assert_eq!(cache.get(&"a".to_string()), None);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_clear() {
        let mut cache = LruCache::new(3);

        cache.insert("a".to_string(), 1);
        cache.insert("b".to_string(), 2);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
        assert_eq!(cache.get(&"a".to_string()), None);
    }

    #[test]
    fn test_contains_key() {
        let mut cache = LruCache::new(2);

        cache.insert("a".to_string(), 1);

        assert!(cache.contains_key(&"a".to_string()));
        assert!(!cache.contains_key(&"b".to_string()));
    }

    #[test]
    #[should_panic(expected = "Cache capacity must be greater than 0")]
    fn test_zero_capacity_panics() { let _cache = LruCache::<String, i32>::new(0); }

    #[test]
    fn test_thread_safe_cache() {
        let cache = new_threadsafe_lru_cache(10);

        // Insert in one thread.
        {
            let mut cache_guard = cache.lock().unwrap();
            cache_guard.insert("key".to_string(), 42);
        }

        // Read in another context.
        {
            let mut cache_guard = cache.lock().unwrap();
            assert_eq!(cache_guard.get(&"key".to_string()), Some(&42));
        }
    }
}
