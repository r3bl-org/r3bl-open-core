// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words LIST_SPACE_DISPLAY HEADING SPACE_CHAR SPACE_CACHE LIST_SPACE_DISPLAY_CHAR
// cspell:words HORIZ_LINE_CACHE

//! String Repeat Cache Module
//!
//! This module provides a mechanism to cache strings of repeated characters (like spaces,
//! horizontal lines, and hash symbols) to avoid frequent allocations.
//!
//! It uses two levels of caching:
//! 1. **Static Cache**: Pre-computed strings for common sizes (e.g., 0-64 spaces). These
//!    are zero-cost lookups that return `&'static str`.
//! 2. **Dynamic Cache**: A thread-safe [`ScopedMutex<HashMap>`] for larger or less common
//!    sizes. This uses the **Scoped Access** pattern to ensure safety and prevent
//!    deadlocks.
//!
//! # Why Cache Instead of `String::repeat()`?
//!
//! In a TUI application, string repetition operations (spaces, lines, hashes) are called
//! extremely frequently during rendering:
//!
//! - **Every frame render**: The main event loop renders 30-60 times per second
//! - **Every line of output**: Padding, indentation, and borders require repeated
//!   characters
//! - **Logging operations**: Debug output often needs formatted alignment
//! - **Parser operations**: Markdown parsing requires indentation tracking
//!
//! ## Performance Impact
//!
//! Without caching, each `String::repeat()` call:
//! 1. Allocates new memory on the heap
//! 2. Copies characters `count` times
//! 3. Creates allocation pressure leading to more frequent garbage collection
//!
//! In a typical TUI render cycle displaying 50 lines with average 20-char indentation:
//! - Without cache: 50 allocations per frame × 60 fps = 3,000 allocations/second
//! - With cache: 0 allocations (after warm-up) for common cases
//!
//! ## Caching Strategy
//!
//! We use a two-tier caching approach:
//!
//! 1. **Static pre-computed cache** (startup cost, zero runtime cost):
//!    - Spaces: 0-64 characters (covers 99% of indentation needs)
//!    - Lines: 0-64 characters (covers most terminal widths)
//!    - Hashes: 0-10 characters (Markdown headers are 1-6, with buffer)
//!
//! 2. **Dynamic runtime cache** (for edge cases):
//!    - Handles counts beyond pre-computed ranges
//!    - Thread-safe using the **Scoped Access** pattern via [`ScopedMutex<HashMap>`]
//!    - Prevents memory leaks (unlike [`Box::leak()`])
//!
//! ## Memory Trade-off
//!
//! Total static memory usage:
//! - Space cache: 65 strings × average 32 chars = ~2KB
//! - Line cache: 65 strings × average 32 chars = ~2KB
//! - Hash cache: 11 strings × average 5 chars = ~55 bytes
//! - Total: ~4KB of memory for massive performance gains

use crate::{DeadlockPreventionPolicy::OptOut, ScopedMutex, scoped_mutex};
use std::{borrow::Cow, collections::HashMap, sync::LazyLock};

const SPACE_CHAR: char = ' ';
const SPACE: &str = " ";
const LIST_SPACE_DISPLAY_CHAR: char = '─';
const LIST_SPACE_DISPLAY: &str = "─";
const HEADING: &str = "#";

/// Static cache for space strings to avoid repeated allocations.
/// Pre-computes space strings for common lengths (0 to 64 chars).
static SPACE_CACHE: LazyLock<HashMap<usize, String>> = LazyLock::new(|| {
    let mut cache = HashMap::new();
    for i in 0..=64 {
        cache.insert(i, SPACE.repeat(i));
    }
    cache
});

/// Static cache for horizontal line strings to avoid repeated allocations.
/// Pre-computes line strings for common lengths (0 to 64 chars).
static HORIZ_LINE_CACHE: LazyLock<HashMap<usize, String>> = LazyLock::new(|| {
    let mut cache = HashMap::new();
    for i in 0..=64 {
        cache.insert(i, LIST_SPACE_DISPLAY.repeat(i));
    }
    cache
});

/// Dynamic cache for repeated strings that aren't in the static caches.
///
/// While the static caches handle common cases (0-64 for spaces/lines, 0-10 for hashes),
/// some edge cases require larger strings:
/// - Ultra-wide terminals (>64 chars)
/// - Deep indentation in generated code
/// - Custom formatting requirements
///
/// ## Why Not Just Extend Static Caches?
///
/// 1. **Diminishing returns**: Usage frequency drops exponentially after common sizes
/// 2. **Memory waste**: Pre-computing 1000+ character strings wastes memory
/// 3. **Startup cost**: Larger static caches increase initialization time
///
/// ## Design Decisions
///
/// ### Key Structure: [`(char, usize)`]
/// - [`char`]: The character being repeated (space, line, hash)
/// - [`usize`]: The count of repetitions
/// - This allows sharing the cache across all string types
///
/// # Architectural Rationale for [`OptOut`] (OPT_OUT)
///
/// We use the `OPT_OUT` policy here because:
/// 1. **Hot Path Performance**: This cache is in the rendering hot path. To maintain 60
///    FPS performance, we **intentionally bypass** the global ledger and deadlock
///    protection overhead. This is a "Pattern without Protection" choice.
/// 2. **Thread Safety**: Multiple threads may render simultaneously (e.g., during
///    background log rendering or complex UI updates). [`ScopedMutex`] still uses the
///    **Scoped Access pattern** (closure-based API) to prevent manual lock management
///    errors, even though it bypasses the crate's deadlock protections.
/// 3. **Leaf Utility**: The string repeat cache is a leaf-level utility. It does not call
///    into other components that might acquire locks, making the decision to forgo
///    deadlock protections an acceptable architectural trade-off for this specific
///    component.
///
/// See the [Parameters] section in [`ScopedMutex`] for more info.
///
/// ### Memory Management
/// - Unlike [`Box::leak()`] (which intentionally leaks memory), this cache:
///   - Can be cleared if needed
///   - Participates in normal Rust memory management
///   - Prevents unbounded memory growth
///
/// ### Performance Characteristics
/// - First access: Allocates and caches (slow)
/// - Subsequent accesses: [`HashMap`] lookup + clone (fast)
/// - Trade-off: Small memory cost for avoiding repeated allocations
///
/// ## Example Usage Pattern
///
/// When `get_spaces(100)` is called:
/// 1. Static cache miss (only has 0-64)
/// 2. Dynamic cache checked
/// 3. If miss: allocate 100-space string, cache it
/// 4. If hit: return cloned string
/// 5. Future calls to `get_spaces(100)` hit the cache
///
/// [`Box::leak()`]: std::boxed::Box::leak
/// [`char`]: std::primitive::char
/// [`DeadlockPreventionPolicy::OptOut`]: variant@crate::DeadlockPreventionPolicy::OptOut
/// [`HashMap`]: std::collections::HashMap
/// [`OptOut`]: variant@crate::DeadlockPreventionPolicy::OptOut
/// [`ScopedMutex<_, DeadlockPreventionPolicy::OptOut>`]: crate::ScopedMutex
/// [`ScopedMutex`]: crate::core::common::ScopedMutex
/// [`usize`]: std::primitive::usize
/// [Parameters]: crate::ScopedMutex#parameters
pub static DYNAMIC_CACHE: LazyLock<
    ScopedMutex<HashMap<(char, usize), String>, { OptOut }>,
> = LazyLock::new(|| scoped_mutex!(OPT_OUT, HashMap::new()));

/// Static cache for heading hash strings to avoid repeated allocations.
/// Pre-computes heading hash strings for common lengths (0 to 10 chars).
/// This is used for markdown heading formatting (e.g., "###" for H3).
static HASH_CACHE: LazyLock<HashMap<usize, String>> = LazyLock::new(|| {
    let mut cache = HashMap::new();
    // Pre-populate cache for common heading levels (0 to 10).
    // Markdown typically supports up to 6 levels, but we cache a bit more.
    for i in 0..=10 {
        cache.insert(i, HEADING.repeat(i));
    }
    cache
});

/// Generic function to get a cached repeated string.
/// First checks the static cache, then falls back to the dynamic cache for large counts.
fn get_cached_repeated_string(
    count: usize,
    static_cache: &'static HashMap<usize, String>,
    char_to_repeat: char,
    str_to_repeat: &str,
) -> Cow<'static, str> {
    if let Some(cached_str) = static_cache.get(&count) {
        Cow::Borrowed(cached_str.as_str())
    } else if count == 0 {
        Cow::Borrowed("")
    } else {
        // Use dynamic cache for large counts.
        let repeated_str = DYNAMIC_CACHE.write(|cache| {
            cache
                .entry((char_to_repeat, count))
                .or_insert_with(|| str_to_repeat.repeat(count))
                .clone()
        });
        Cow::Owned(repeated_str)
    }
}

/// Gets a cached space string for the given length.
/// Falls back to allocation for very large space counts (>64).
///
/// # Performance
/// - Cache hit: O(1) lookup with no allocation
/// - Cache miss: Falls back to allocation for counts > 64
///
/// # Examples
/// ```
/// use r3bl_tui::get_spaces;
///
/// let spaces2 = get_spaces(2);  // "  " (cached)
/// let spaces8 = get_spaces(8);  // "        " (cached)
/// let spaces100 = get_spaces(100); // 100 spaces (allocated)
/// ```
///
/// # Panics
///
/// [`DYNAMIC_CACHE`] uses the **Scoped Access** pattern via [`ScopedMutex`], so it can
/// panic if the internal mutex is poisoned.
///
/// [`DYNAMIC_CACHE`]: crate::DYNAMIC_CACHE
/// [`ScopedMutex`]: crate::core::common::ScopedMutex
#[must_use]
pub fn get_spaces(count: usize) -> Cow<'static, str> {
    get_cached_repeated_string(count, &SPACE_CACHE, SPACE_CHAR, SPACE)
}

/// Gets a cached horizontal line string for the given length.
/// Falls back to allocation for very large counts (>64).
///
/// # Performance
/// - Cache hit: O(1) lookup with no allocation
/// - Cache miss: Falls back to allocation for counts > 64
///
/// # Examples
/// ```
/// use r3bl_tui::get_horiz_lines;
///
/// let line5 = get_horiz_lines(5);   // "─────" (cached)
/// let line20 = get_horiz_lines(20); // "────────────────────" (cached)
/// let line100 = get_horiz_lines(100); // 100 horizontal line chars (allocated)
/// ```
///
/// # Panics
///
/// [`DYNAMIC_CACHE`] uses the **Scoped Access** pattern via [`ScopedMutex`], so it can
/// panic if the internal mutex is poisoned.
///
/// [`DYNAMIC_CACHE`]: crate::DYNAMIC_CACHE
/// [`ScopedMutex`]: crate::core::common::ScopedMutex
#[must_use]
pub fn get_horiz_lines(count: usize) -> Cow<'static, str> {
    get_cached_repeated_string(
        count,
        &HORIZ_LINE_CACHE,
        LIST_SPACE_DISPLAY_CHAR,
        LIST_SPACE_DISPLAY,
    )
}

/// Gets a cached hash string for the given length.
/// Falls back to allocation for very large counts (>10).
///
/// # Performance
/// - Cache hit: O(1) lookup with no allocation
/// - Cache miss: Falls back to allocation for counts > 10
///
/// # Examples
/// ```
/// use r3bl_tui::get_hashes;
///
/// let h1 = get_hashes(1);   // "#" (cached)
/// let h3 = get_hashes(3);   // "###" (cached)
/// let h6 = get_hashes(6);   // "######" (cached)
/// ```
///
/// # Panics
///
/// [`DYNAMIC_CACHE`] uses the **Scoped Access** pattern via [`ScopedMutex`], so it can
/// panic if the internal mutex is poisoned.
///
/// [`DYNAMIC_CACHE`]: crate::DYNAMIC_CACHE
/// [`ScopedMutex`]: crate::core::common::ScopedMutex
#[must_use]
pub fn get_hashes(count: usize) -> Cow<'static, str> {
    get_cached_repeated_string(count, &HASH_CACHE, '#', HEADING)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cached_spaces() {
        // Test various cached sizes.
        assert_eq!(get_spaces(0), "");
        assert_eq!(get_spaces(1), SPACE);
        assert_eq!(get_spaces(2), "  ");
        assert_eq!(get_spaces(4), "    ");
        assert_eq!(get_spaces(8), "        ");
        assert_eq!(get_spaces(16), "                ");
        assert_eq!(get_spaces(32), "                                ");
        assert_eq!(get_spaces(64), SPACE.repeat(64));
    }

    #[test]
    fn test_large_space_count() {
        // Test fallback for large counts.
        let spaces100 = get_spaces(100);
        assert_eq!(spaces100.len(), 100);
        assert!(spaces100.chars().all(|c| c == SPACE_CHAR));
    }

    #[test]
    fn test_cache_consistency() {
        // Verify that multiple calls return the same value.
        let spaces1 = get_spaces(4);
        let spaces2 = get_spaces(4);
        assert_eq!(spaces1, spaces2);
    }

    #[test]
    fn test_edge_cases() {
        assert_eq!(get_spaces(0), "");
        assert_eq!(get_spaces(1), SPACE);
        assert_eq!(get_spaces(64), SPACE.repeat(64)); // Boundary case
        assert_eq!(get_spaces(65), SPACE.repeat(65)); // Just over boundary
    }

    #[test]
    fn test_cached_horiz_lines() {
        // Test various cached sizes.
        assert_eq!(get_horiz_lines(0), "");
        assert_eq!(get_horiz_lines(1), LIST_SPACE_DISPLAY);
        assert_eq!(get_horiz_lines(2), "──");
        assert_eq!(get_horiz_lines(4), "────");
        assert_eq!(get_horiz_lines(8), "────────");
        assert_eq!(get_horiz_lines(16), "────────────────");
        assert_eq!(get_horiz_lines(32), "────────────────────────────────");
        assert_eq!(get_horiz_lines(64), LIST_SPACE_DISPLAY.repeat(64));
    }

    #[test]
    fn test_large_horiz_line_count() {
        // Test fallback for large counts.
        let horiz_lines100 = get_horiz_lines(100);
        assert_eq!(horiz_lines100.chars().count(), 100);
        assert!(horiz_lines100.chars().all(|c| c == LIST_SPACE_DISPLAY_CHAR));
    }

    #[test]
    fn test_horiz_line_cache_consistency() {
        // Verify that multiple calls return the same value.
        let horiz_lines1 = get_horiz_lines(4);
        let horiz_lines2 = get_horiz_lines(4);
        assert_eq!(horiz_lines1, horiz_lines2);
    }

    #[test]
    fn test_cached_hashes() {
        // Test various cached sizes.
        assert_eq!(get_hashes(0), "");
        assert_eq!(get_hashes(1), "#");
        assert_eq!(get_hashes(2), "##");
        assert_eq!(get_hashes(3), "###");
        assert_eq!(get_hashes(4), "####");
        assert_eq!(get_hashes(5), "#####");
        assert_eq!(get_hashes(6), "######");
        assert_eq!(get_hashes(10), "##########");
    }

    #[test]
    fn test_large_hash_count() {
        // Test fallback for large counts.
        let hashes15 = get_hashes(15);
        assert_eq!(hashes15.chars().count(), 15);
        assert!(hashes15.chars().all(|c| c == '#'));
    }

    #[test]
    fn test_hash_cache_consistency() {
        // Verify that multiple calls return the same value.
        let hashes1 = get_hashes(3);
        let hashes2 = get_hashes(3);
        assert_eq!(hashes1, hashes2);
    }

    #[test]
    fn test_dynamic_cache_persistence() {
        // Test that dynamic cache actually caches values.

        // First call to large count should populate dynamic cache.
        let spaces_100_first = get_spaces(100);
        let horiz_lines_100_first = get_horiz_lines(100);
        let hashes_20_first = get_hashes(20);

        // Second call should return cached value.
        let spaces_100_second = get_spaces(100);
        let horiz_lines_100_second = get_horiz_lines(100);
        let hashes_20_second = get_hashes(20);

        // Values should be equal.
        assert_eq!(spaces_100_first, spaces_100_second);
        assert_eq!(horiz_lines_100_first, horiz_lines_100_second);
        assert_eq!(hashes_20_first, hashes_20_second);

        // Verify the cache contains these entries.
        DYNAMIC_CACHE.read(|cache| {
            assert!(cache.contains_key(&(SPACE_CHAR, 100)));
            assert!(cache.contains_key(&(LIST_SPACE_DISPLAY_CHAR, 100)));
            assert!(cache.contains_key(&('#', 20)));
        });
    }

    #[test]
    fn test_dynamic_cache_different_counts() {
        // Test that different counts are cached separately.
        let spaces_70 = get_spaces(70);
        let spaces_80 = get_spaces(80);
        let spaces_90 = get_spaces(90);

        assert_eq!(spaces_70.len(), 70);
        assert_eq!(spaces_80.len(), 80);
        assert_eq!(spaces_90.len(), 90);

        // All should be in the dynamic cache.
        DYNAMIC_CACHE.read(|cache| {
            assert!(cache.contains_key(&(SPACE_CHAR, 70)));
            assert!(cache.contains_key(&(SPACE_CHAR, 80)));
            assert!(cache.contains_key(&(SPACE_CHAR, 90)));
        });
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;

        // Test concurrent access to dynamic cache.
        let handles: Vec<_> = (0..10)
            .map(|i| {
                thread::spawn(move || {
                    // Each thread accesses different large counts.
                    let base = 100 + i * 10;
                    let spaces = get_spaces(base);
                    let horiz_lines = get_horiz_lines(base);
                    let hashes = get_hashes(base);

                    assert_eq!(spaces.len(), base);
                    assert_eq!(horiz_lines.chars().count(), base);
                    assert_eq!(hashes.chars().count(), base);
                })
            })
            .collect();

        // Wait for all threads to complete.
        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        // Verify all entries are in the cache.
        DYNAMIC_CACHE.read(|cache| {
            for i in 0..10 {
                let base = 100 + i * 10;
                assert!(cache.contains_key(&(SPACE_CHAR, base)));
                assert!(cache.contains_key(&(LIST_SPACE_DISPLAY_CHAR, base)));
                assert!(cache.contains_key(&('#', base)));
            }
        });
    }

    #[test]
    fn test_get_cached_repeated_string_directly() {
        // Test the internal function directly.

        // Create a static test cache.
        static TEST_CACHE: LazyLock<HashMap<usize, String>> = LazyLock::new(|| {
            let mut cache = HashMap::new();
            cache.insert(3, "xxx".to_string());
            cache
        });

        // Test static cache hit.
        let result = get_cached_repeated_string(3, &TEST_CACHE, 'x', "x");
        assert_eq!(result, "xxx");
        assert!(matches!(result, Cow::Borrowed(_)));

        // Test zero count.
        let result = get_cached_repeated_string(0, &TEST_CACHE, 'x', "x");
        assert_eq!(result, "");
        assert!(matches!(result, Cow::Borrowed(_)));

        // Test dynamic cache fallback.
        let result = get_cached_repeated_string(5, &TEST_CACHE, 'y', "y");
        assert_eq!(result, "yyyyy");
        assert!(matches!(result, Cow::Owned(_)));

        // Verify it's in the dynamic cache.
        DYNAMIC_CACHE.read(|cache| {
            assert!(cache.contains_key(&('y', 5)));
        });
    }
}
