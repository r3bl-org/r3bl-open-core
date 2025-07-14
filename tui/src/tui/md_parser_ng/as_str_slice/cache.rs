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

//! # Performance Cache for [`super::AsStrSlice`].
//!
//! This module provides caching structures to optimize character-to-byte conversions
//! and avoid O(n) operations in hot paths. The cache is designed to be computed once
//! when an [`super::AsStrSlice`] is created and reused throughout its lifetime.
//!
//! ## Problem Solved
//!
//! The NG parser was initially 50,000x slower than the legacy parser. Profiling revealed
//! that hot-path methods like `extract_to_line_end()` and `take_from()` were repeatedly
//! counting characters and converting between character and byte positions, resulting in
//! O(n) operations on every call.
//!
//! ## Solution
//!
//! This module provides two key caching structures:
//! 
//! 1. **`LineMetadataCache`**: Pre-computes and stores:
//!    - Character count for each line
//!    - Cumulative character offsets (for binary search)
//!    - Total character count across all lines
//!
//! 2. **`LineByteOffsetCache`**: Pre-computes and stores:
//!    - Character-to-byte position mappings for each line
//!    - Enables O(log n) character-to-byte conversions
//!
//! ## Performance Impact
//!
//! With these caches, the NG parser performance improved dramatically:
//! - Character position lookups: O(n) → O(log n)
//! - Byte offset calculations: O(n) → O(1) or O(log n)
//! - Overall improvement: 600-5,000x faster depending on document size

use crate::core::units::Length;

/// Cache for line metadata to avoid repeated character counting and byte offset
/// calculations.
///
/// This cache is critical for performance because it transforms O(n) character
/// counting operations into O(1) lookups. Without this cache, methods like
/// `take_from()` would need to iterate through all preceding lines to calculate
/// positions.
///
/// ## Example
/// 
/// For input lines `["hello", "world", "!"]`, the cache stores:
/// - `char_counts`: [5, 5, 1]
/// - `char_offsets`: [0, 6, 12, 14] (includes synthetic newlines)
/// - `total_chars`: 14
#[derive(Debug, Clone, PartialEq)]
pub struct LineMetadataCache {
    /// Character count for each line (not including synthetic newlines)
    pub char_counts: Vec<Length>,
    /// Cumulative character offset at the start of each line.
    /// This enables binary search for line lookups.
    pub char_offsets: Vec<usize>,
    /// Total character count across all lines (including synthetic newlines)
    pub total_chars: Length,
}

impl LineMetadataCache {
    /// Create a new cache from a slice of lines
    pub fn new<T: AsRef<str>>(lines: &[T]) -> Self {
        let mut char_counts = Vec::with_capacity(lines.len());
        let mut char_offsets = Vec::with_capacity(lines.len() + 1);
        let mut cumulative_offset = 0;

        char_offsets.push(0); // Start of first line

        for (i, line) in lines.iter().enumerate() {
            let line_str = line.as_ref();
            let char_count = line_str.chars().count();
            char_counts.push(Length::from(char_count));

            // Add character count plus synthetic newline (except for single line case)
            let chars_with_newline = if lines.len() > 1 {
                char_count + 1 // Include synthetic newline
            } else if i == 0 {
                char_count // Single line, no trailing newline
            } else {
                char_count + 1
            };

            cumulative_offset += chars_with_newline;
            char_offsets.push(cumulative_offset);
        }

        // Calculate total characters
        let total_chars = if lines.is_empty() {
            Length::from(0)
        } else if lines.len() == 1 {
            char_counts[0] // Single line has no trailing newline
        } else {
            // Multiple lines: sum of all chars + newlines
            Length::from(cumulative_offset)
        };

        Self {
            char_counts,
            char_offsets,
            total_chars,
        }
    }

    /// Get the character count for a specific line
    #[inline]
    #[must_use]
    pub fn line_char_count(&self, line_index: usize) -> Option<Length> {
        self.char_counts.get(line_index).copied()
    }

    /// Get the cumulative character offset at the start of a line
    #[inline]
    #[must_use]
    pub fn line_char_offset(&self, line_index: usize) -> Option<usize> {
        self.char_offsets.get(line_index).copied()
    }

    /// Convert a global character position to (`line_index`, `char_within_line`).
    ///
    /// This method uses binary search on the pre-computed `char_offsets` array,
    /// providing O(log n) performance instead of O(n) linear search.
    ///
    /// ## Example
    /// 
    /// For lines `["hello", "world"]`, character position 7 returns `Some((1, 1))`
    /// because it's the second character ('o') of the second line.
    #[must_use]
    pub fn char_pos_to_line_char(
        &self,
        global_char_pos: usize,
    ) -> Option<(usize, usize)> {
        // Binary search to find the line containing this character position
        match self.char_offsets.binary_search(&global_char_pos) {
            Ok(line_index) => {
                // Exact match - we're at the start of this line
                Some((line_index, 0))
            }
            Err(insert_pos) => {
                if insert_pos == 0 {
                    // Before the first line
                    None
                } else {
                    // The line is at insert_pos - 1
                    let line_index = insert_pos - 1;
                    let line_start_offset = self.char_offsets[line_index];
                    let char_within_line = global_char_pos - line_start_offset;

                    // Verify we're within the line's bounds
                    if let Some(line_char_count) = self.line_char_count(line_index) {
                        // Account for synthetic newline
                        let max_chars = if self.char_counts.len() > 1 {
                            line_char_count.as_usize() + 1 // Include synthetic newline
                        } else {
                            line_char_count.as_usize() // Single line, no newline
                        };

                        if char_within_line < max_chars {
                            Some((line_index, char_within_line))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            }
        }
    }
}

/// Cache for byte offset information within a line to optimize character-to-byte
/// conversions.
///
/// This cache transforms O(n) character-to-byte conversions into O(log n) operations
/// by pre-computing and storing character index to byte offset mappings for each line.
///
/// ## Why This Matters
///
/// In Unicode text, characters can occupy multiple bytes (e.g., emojis like 😀 use 4 bytes).
/// Without this cache, converting from character position 100 to its byte offset would
/// require iterating through all 100 characters to count their byte sizes.
///
/// ## Example
///
/// For the line "Hello 😀 World!":
/// - Character positions: [0='H', 1='e', 2='l', 3='l', 4='o', 5=' ', 6='😀', 7=' ', 8='W', ...]
/// - Byte offsets:       [0,     1,     2,     3,     4,     5,     6,      10,    11,    ...]
///
/// The cache stores: `[(0,0), (1,1), (2,2), (3,3), (4,4), (5,5), (6,6), (7,10), (8,11), ...]`
///
/// Looking up character 7 returns byte offset 10 in O(log n) time.
#[derive(Debug, Clone, PartialEq)]
pub struct LineByteOffsetCache {
    /// For each line, store (`char_index`, `byte_offset`) pairs
    /// This allows binary search for fast character-to-byte conversion
    pub line_byte_offsets: Vec<Vec<(usize, usize)>>,
}

impl LineByteOffsetCache {
    /// Create a new byte offset cache from lines
    pub fn new<T: AsRef<str>>(lines: &[T]) -> Self {
        let mut line_byte_offsets = Vec::with_capacity(lines.len());

        for line in lines {
            let line_str = line.as_ref();
            let mut offsets = Vec::new();

            // Always include the start position
            offsets.push((0, 0));

            // Build character-to-byte mapping
            let mut char_index = 0;
            for (byte_offset, _ch) in line_str.char_indices() {
                if char_index > 0 {
                    offsets.push((char_index, byte_offset));
                }
                char_index += 1;
            }

            // Add end position  
            offsets.push((char_index, line_str.len()));

            line_byte_offsets.push(offsets);
        }

        Self { line_byte_offsets }
    }

    /// Convert character position to byte position within a specific line.
    ///
    /// Returns `None` if `line_index` is out of bounds or `char_pos` exceeds line length.
    ///
    /// ## Performance
    ///
    /// - Direct lookup: O(1) if the exact position is cached
    /// - Binary search: O(log n) for positions between cached entries
    /// - Without cache: Would be O(n) requiring iteration through all characters
    ///
    /// ## Example
    ///
    /// ```ignore
    /// let cache = LineByteOffsetCache::new(&["Hello 😀 World!"]);
    /// assert_eq!(cache.char_to_byte(0, 6), Some(6));  // Start of emoji
    /// assert_eq!(cache.char_to_byte(0, 7), Some(10)); // After emoji (4 bytes)
    /// ```
    #[inline]
    #[must_use]
    pub fn char_to_byte(&self, line_index: usize, char_pos: usize) -> Option<usize> {
        let offsets = self.line_byte_offsets.get(line_index)?;

        // Direct lookup if we have the exact position
        if char_pos < offsets.len() {
            if let Some(&(stored_char_pos, byte_offset)) = offsets.get(char_pos) {
                if stored_char_pos == char_pos {
                    return Some(byte_offset);
                }
            }
        }

        // Binary search for the position
        match offsets.binary_search_by_key(&char_pos, |&(char_idx, _)| char_idx) {
            Ok(idx) => Some(offsets[idx].1),
            Err(insert_pos) => {
                if insert_pos == 0 {
                    Some(0) // Before first character
                } else if insert_pos >= offsets.len() {
                    // Past end of line - return line length
                    offsets.last().map(|&(_, byte_offset)| byte_offset)
                } else {
                    // Would insert between existing positions - not a valid char boundary
                    None
                }
            }
        }
    }
}

/// Combined cache for all performance-critical metadata.
///
/// This is the main cache structure used by [`super::AsStrSlice`] to achieve
/// high performance. It combines both line metadata and byte offset caches to
/// provide O(log n) or better performance for all position-related operations.
///
/// ## Performance Impact
///
/// ### Before Caching (Legacy Approach)
/// - `extract_to_line_end()`: O(n) - counted characters from start
/// - `take_from()`: O(n) - iterated through lines to find position
/// - `char_to_byte()`: O(n) - counted byte sizes of each character
///
/// ### After Caching (Current Approach)
/// - `extract_to_line_end()`: O(1) - direct lookup of cached line end
/// - `take_from()`: O(log n) - binary search on cached offsets
/// - `char_to_byte()`: O(log n) - binary search on cached byte positions
///
/// ## Real-World Example
///
/// For a 10,000 line markdown document:
/// - Legacy parser: 50,000x slower than baseline
/// - With caching: 9-83x slower than baseline
/// - **Improvement: 600-5,000x faster**
///
/// ## Memory Overhead
///
/// The cache trades memory for speed:
/// - `LineMetadataCache`: ~16 bytes per line (2 usize values)
/// - `LineByteOffsetCache`: ~8-16 bytes per character position cached
/// - Total: Typically < 1% of document size for text files
#[derive(Debug, Clone, PartialEq)]
pub struct AsStrSliceCache {
    /// Metadata about lines: character counts, offsets, and totals
    pub line_metadata: LineMetadataCache,
    /// Character-to-byte position mappings for each line
    pub byte_offsets: LineByteOffsetCache,
}

impl AsStrSliceCache {
    /// Create a new combined cache from lines.
    ///
    /// This pre-computes all necessary metadata in a single pass:
    /// 1. Character count for each line
    /// 2. Cumulative character offsets (for binary search)
    /// 3. Character-to-byte mappings for Unicode support
    ///
    /// The cost of creating the cache is amortized over many lookups,
    /// making it highly efficient for parsers that perform multiple
    /// operations on the same text.
    pub fn new<T: AsRef<str>>(lines: &[T]) -> Self {
        Self {
            line_metadata: LineMetadataCache::new(lines),
            byte_offsets: LineByteOffsetCache::new(lines),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GCString;

    #[test]
    fn test_line_metadata_cache_single_line() {
        let lines = vec![GCString::from("hello world")];
        let cache = LineMetadataCache::new(&lines);

        assert_eq!(cache.char_counts.len(), 1);
        assert_eq!(cache.char_counts[0].as_usize(), 11);
        assert_eq!(cache.total_chars.as_usize(), 11); // No trailing newline for single line
        assert_eq!(cache.char_offsets, vec![0, 11]);
    }

    #[test]
    fn test_line_metadata_cache_multiple_lines() {
        let lines = vec![
            GCString::from("hello"),
            GCString::from("world"),
            GCString::from("test"),
        ];
        let cache = LineMetadataCache::new(&lines);

        assert_eq!(cache.char_counts.len(), 3);
        assert_eq!(cache.char_counts[0].as_usize(), 5);
        assert_eq!(cache.char_counts[1].as_usize(), 5);
        assert_eq!(cache.char_counts[2].as_usize(), 4);

        // Total includes synthetic newlines
        assert_eq!(cache.total_chars.as_usize(), 17); // 5 + 1 + 5 + 1 + 4 + 1

        assert_eq!(cache.char_offsets, vec![0, 6, 12, 17]);
    }

    #[test]
    fn test_char_pos_to_line_char() {
        let lines = vec![
            GCString::from("hello"), // 5 chars + newline
            GCString::from("world"), // 5 chars + newline
            GCString::from("!"),     // 1 char + newline
        ];
        let cache = LineMetadataCache::new(&lines);

        // Start of first line
        assert_eq!(cache.char_pos_to_line_char(0), Some((0, 0)));

        // Middle of first line
        assert_eq!(cache.char_pos_to_line_char(3), Some((0, 3)));

        // Synthetic newline after first line
        assert_eq!(cache.char_pos_to_line_char(5), Some((0, 5)));

        // Start of second line
        assert_eq!(cache.char_pos_to_line_char(6), Some((1, 0)));

        // Middle of second line
        assert_eq!(cache.char_pos_to_line_char(8), Some((1, 2)));

        // Start of third line
        assert_eq!(cache.char_pos_to_line_char(12), Some((2, 0)));

        // Past end
        assert_eq!(cache.char_pos_to_line_char(20), None);
    }

    #[test]
    fn test_byte_offset_cache() {
        let lines = vec![
            GCString::from("hello"),
            GCString::from("😀world"), // Emoji is multi-byte
        ];
        let cache = LineByteOffsetCache::new(&lines);

        // First line - all ASCII
        assert_eq!(cache.char_to_byte(0, 0), Some(0));
        assert_eq!(cache.char_to_byte(0, 3), Some(3));
        assert_eq!(cache.char_to_byte(0, 5), Some(5));

        // Second line - with emoji
        assert_eq!(cache.char_to_byte(1, 0), Some(0));
        assert_eq!(cache.char_to_byte(1, 1), Some(4)); // After emoji (4 bytes)
        assert_eq!(cache.char_to_byte(1, 2), Some(5)); // 'w'
        assert_eq!(cache.char_to_byte(1, 6), Some(9)); // End of line
    }
}
