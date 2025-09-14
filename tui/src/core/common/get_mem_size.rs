// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::fmt::Display;

use crate::{MemoizedValue, format_as_kilobytes_with_commas};

pub trait GetMemSize {
    fn get_mem_size(&self) -> usize;
}

/// Calculates the total memory size of a slice of items that implement [`GetMemSize`].
/// This is useful when you need to calculate the total memory size of a collection of
/// items (eg [Vec] or [`smallvec::SmallVec`] of [`crate::GCStringOwned`]).
#[must_use]
pub fn slice_size<T: GetMemSize>(slice: &[T]) -> usize {
    slice.iter().map(GetMemSize::get_mem_size).sum::<usize>()
}

/// Calculates the total memory size of an iterator of items that implement
/// [`GetMemSize`]. This is useful when you need to calculate the total memory size of
/// an iterator of items (eg: from [`crate::RingBufferHeap`] or
/// [`crate::RingBufferStack`]) that contains items that are [Option] of [`GetMemSize`].
#[must_use]
pub fn iter_size<T: GetMemSize, I: Iterator<Item = Option<T>>>(iter: I) -> usize {
    iter.map(|item| item.as_ref().map_or(0, GetMemSize::get_mem_size))
        .sum::<usize>()
}

#[must_use]
pub fn ring_buffer_size<T: GetMemSize, const N: usize>(
    ring_buffer: &crate::RingBufferHeap<T, N>,
) -> usize {
    ring_buffer
        .iter()
        .map(GetMemSize::get_mem_size)
        .sum::<usize>()
}

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

/// Type alias for memory size memoization.
pub type MemoizedMemorySize = MemoizedValue<MemorySize>;

/// Trait extension that provides cached memory size calculations.
///
/// This trait is automatically implemented for any type that implements `GetMemSize`
/// and provides a way to cache expensive memory size calculations. This is particularly
/// useful for structs that are displayed frequently in telemetry logging.
///
/// # Example
/// ```
/// use r3bl_tui::{CachedMemorySize, GetMemSize, MemoizedMemorySize};
///
/// struct MyDataStructure {
///     data: Vec<String>,
///     // Add cache field for memory size
///     memory_size_cache: MemoizedMemorySize,
/// }
///
/// impl GetMemSize for MyDataStructure {
///     fn get_mem_size(&self) -> usize {
///         // Expensive calculation - sum of all string lengths
///         self.data.iter().map(|s| s.len()).sum::<usize>()
///             + std::mem::size_of::<Vec<String>>()
///     }
/// }
///
/// impl CachedMemorySize for MyDataStructure {
///     fn memory_size_cache(&self) -> &MemoizedMemorySize {
///         &self.memory_size_cache
///     }
///
///     fn memory_size_cache_mut(&mut self) -> &mut MemoizedMemorySize {
///         &mut self.memory_size_cache
///     }
/// }
///
/// impl MyDataStructure {
///     fn new() -> Self {
///         Self {
///             data: Vec::new(),
///             memory_size_cache: MemoizedMemorySize::default(),
///         }
///     }
///
///     fn add_data(&mut self, s: String) {
///         self.data.push(s);
///         // Invalidate cache when data changes
///         self.invalidate_memory_size_cache();
///     }
/// }
///
/// let mut my_struct = MyDataStructure::new();
///
/// // Add some data
/// my_struct.add_data("Hello".to_string());
/// my_struct.add_data("World".to_string());
///
/// // First call calculates and caches
/// let size1 = my_struct.get_cached_memory_size();
/// assert!(size1.size().unwrap() > 0);
///
/// // Add more data - this invalidates the cache
/// my_struct.add_data("More data with a longer string".to_string());
///
/// // This triggers recalculation
/// let size2 = my_struct.get_cached_memory_size();
/// assert!(size2.size().unwrap() > size1.size().unwrap());
/// ```
pub trait CachedMemorySize: GetMemSize {
    /// Returns an immutable reference to the memory size cache.
    fn memory_size_cache(&self) -> &MemoizedMemorySize;

    /// Returns a mutable reference to the memory size cache.
    fn memory_size_cache_mut(&mut self) -> &mut MemoizedMemorySize;

    /// Invalidates the memory size cache.
    /// Call this when the struct's content changes.
    fn invalidate_memory_size_cache(&mut self) {
        self.memory_size_cache_mut().invalidate();
    }

    /// Updates the cache with the current memory size if needed.
    /// The expensive calculation is only performed if the cache is dirty.
    fn update_memory_size_cache(&mut self) {
        if self.memory_size_cache().is_dirty() {
            let size = self.get_mem_size();
            self.memory_size_cache_mut()
                .upsert(|| MemorySize::new(size));
        }
    }

    /// Gets the cached memory size, updating if necessary.
    /// Returns a `MemorySize` that displays "?" if the cache is not available.
    fn get_cached_memory_size(&mut self) -> MemorySize {
        self.update_memory_size_cache();
        self.memory_size_cache()
            .get_cached()
            .cloned()
            .unwrap_or_else(MemorySize::unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_size_display() {
        // Test with known value.
        let size = MemorySize::new(1024 * 1024); // 1MB
        assert_eq!(format!("{size}"), "1,024 KB");

        // Test with unknown value.
        let unknown = MemorySize::unknown();
        assert_eq!(format!("{unknown}"), "?");

        // Test with default.
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
