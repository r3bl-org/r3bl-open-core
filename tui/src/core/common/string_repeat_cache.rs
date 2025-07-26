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

use std::{collections::HashMap, sync::LazyLock};

/// Static cache for space strings to avoid repeated allocations.
/// Pre-computes space strings for common lengths (0 to 64 spaces).
/// This cache is used across the entire TUI library to optimize
/// performance in render loops and parsers.
static SPACE_CACHE: LazyLock<HashMap<usize, String>> = LazyLock::new(|| {
    let mut cache = HashMap::new();
    // Pre-populate cache for common space lengths (0 to 64 spaces)
    // This covers most practical use cases for indentation, padding, and formatting
    for i in 0..=64 {
        cache.insert(i, " ".repeat(i));
    }
    cache
});

/// Static cache for horizontal line strings to avoid repeated allocations.
/// Pre-computes horizontal line strings for common lengths (0 to 64 chars).
/// This is commonly used for TUI borders, separators, and decorations.
static HLINE_CACHE: LazyLock<HashMap<usize, String>> = LazyLock::new(|| {
    let mut cache = HashMap::new();
    // Pre-populate cache for common horizontal line lengths
    for i in 0..=64 {
        cache.insert(i, "─".repeat(i));
    }
    cache
});

/// Static cache for heading hash strings to avoid repeated allocations.
/// Pre-computes heading hash strings for common lengths (0 to 10 chars).
/// This is used for markdown heading formatting (e.g., "###" for H3).
static HASH_CACHE: LazyLock<HashMap<usize, String>> = LazyLock::new(|| {
    let mut cache = HashMap::new();
    // Pre-populate cache for common heading levels (0 to 10)
    // Markdown typically supports up to 6 levels, but we cache a bit more
    for i in 0..=10 {
        cache.insert(i, "#".repeat(i));
    }
    cache
});

/// Get a cached space string for the given length.
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
pub fn get_spaces(count: usize) -> &'static str {
    if let Some(spaces) = SPACE_CACHE.get(&count) {
        spaces.as_str()
    } else {
        // Fallback for unusually large space counts
        // This should be rare, but we handle it gracefully
        if count == 0 {
            ""
        } else {
            // For very large counts, we still allocate, but this should be extremely rare
            // In practice, space counts rarely exceed 64 characters
            Box::leak(" ".repeat(count).into_boxed_str())
        }
    }
}

/// Get a cached horizontal line string for the given length.
/// Falls back to allocation for very large counts (>64).
///
/// # Performance
/// - Cache hit: O(1) lookup with no allocation
/// - Cache miss: Falls back to allocation for counts > 64
///
/// # Examples
/// ```
/// use r3bl_tui::get_hlines;
///
/// let line5 = get_hlines(5);   // "─────" (cached)
/// let line20 = get_hlines(20); // "────────────────────" (cached)
/// let line100 = get_hlines(100); // 100 horizontal line chars (allocated)
/// ```
pub fn get_hlines(count: usize) -> &'static str {
    if let Some(hlines) = HLINE_CACHE.get(&count) {
        hlines.as_str()
    } else {
        // Fallback for unusually large counts
        if count == 0 {
            ""
        } else {
            Box::leak("─".repeat(count).into_boxed_str())
        }
    }
}

/// Get a cached hash string for the given length.
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
pub fn get_hashes(count: usize) -> &'static str {
    if let Some(hashes) = HASH_CACHE.get(&count) {
        hashes.as_str()
    } else {
        // Fallback for unusually large counts
        if count == 0 {
            ""
        } else {
            Box::leak("#".repeat(count).into_boxed_str())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cached_spaces() {
        // Test various cached sizes
        assert_eq!(get_spaces(0), "");
        assert_eq!(get_spaces(1), " ");
        assert_eq!(get_spaces(2), "  ");
        assert_eq!(get_spaces(4), "    ");
        assert_eq!(get_spaces(8), "        ");
        assert_eq!(get_spaces(16), "                ");
        assert_eq!(get_spaces(32), "                                ");
        assert_eq!(get_spaces(64), " ".repeat(64));
    }

    #[test]
    fn test_large_space_count() {
        // Test fallback for large counts
        let spaces100 = get_spaces(100);
        assert_eq!(spaces100.len(), 100);
        assert!(spaces100.chars().all(|c| c == ' '));
    }

    #[test]
    fn test_cache_consistency() {
        // Verify that multiple calls return the same reference
        let spaces1 = get_spaces(4);
        let spaces2 = get_spaces(4);
        assert_eq!(spaces1.as_ptr(), spaces2.as_ptr()); // Same memory address
    }

    #[test]
    fn test_edge_cases() {
        assert_eq!(get_spaces(0), "");
        assert_eq!(get_spaces(1), " ");
        assert_eq!(get_spaces(64), " ".repeat(64)); // Boundary case
        assert_eq!(get_spaces(65), " ".repeat(65)); // Just over boundary
    }

    #[test]
    fn test_cached_hlines() {
        // Test various cached sizes
        assert_eq!(get_hlines(0), "");
        assert_eq!(get_hlines(1), "─");
        assert_eq!(get_hlines(2), "──");
        assert_eq!(get_hlines(4), "────");
        assert_eq!(get_hlines(8), "────────");
        assert_eq!(get_hlines(16), "────────────────");
        assert_eq!(get_hlines(32), "────────────────────────────────");
        assert_eq!(get_hlines(64), "─".repeat(64));
    }

    #[test]
    fn test_large_hline_count() {
        // Test fallback for large counts
        let hlines100 = get_hlines(100);
        assert_eq!(hlines100.chars().count(), 100);
        assert!(hlines100.chars().all(|c| c == '─'));
    }

    #[test]
    fn test_hline_cache_consistency() {
        // Verify that multiple calls return the same reference
        let hlines1 = get_hlines(4);
        let hlines2 = get_hlines(4);
        assert_eq!(hlines1.as_ptr(), hlines2.as_ptr()); // Same memory address
    }

    #[test]
    fn test_cached_hashes() {
        // Test various cached sizes
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
        // Test fallback for large counts
        let hashes15 = get_hashes(15);
        assert_eq!(hashes15.chars().count(), 15);
        assert!(hashes15.chars().all(|c| c == '#'));
    }

    #[test]
    fn test_hash_cache_consistency() {
        // Verify that multiple calls return the same reference
        let hashes1 = get_hashes(3);
        let hashes2 = get_hashes(3);
        assert_eq!(hashes1.as_ptr(), hashes2.as_ptr()); // Same memory address
    }
}
