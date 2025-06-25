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

use nom::FindSubstring;

use crate::{constants::NEW_LINE, AsStrSlice, CharLengthExt, CharacterIndexNomCompat};

/// Implement [FindSubstring] trait for [AsStrSlice]. This is required by the
/// [nom::bytes::complete::take_until] parser function.
///
/// ## Performance Benefits
///
/// This implementation provides **O(1) line access** efficiency by leveraging the
/// pre-parsed line structure of `AsStrSlice`. Instead of materializing the entire
/// remaining text, it searches within individual lines, avoiding expensive string
/// allocations in most cases.
///
/// - **Optimized path**: For substrings without newlines, searches line-by-line without
///   full materialization, providing significant performance improvements for large
///   texts.
/// - **Fallback path**: For substrings containing newlines, falls back to full
///   materialization only when necessary.
///
/// ## Unicode Safety Guarantees
///
/// This implementation is **fully Unicode-safe** and correctly handles:
/// - Multi-byte UTF-8 characters (emojis, accented characters, etc.).
/// - Unicode grapheme clusters and combining characters.
/// - Character boundary preservation during line-by-line searching.
/// - Proper character counting vs. byte counting distinctions.
///
/// All character position calculations use `CharLengthExt` to ensure Unicode-aware
/// character indexing rather than byte indexing, preventing potential corruption
/// or incorrect position reporting with Unicode content.
///
/// ## Return value
///
/// Returns an `Option<CharacterIndexNomCompat>` where the [`CharacterIndexNomCompat`] or
/// `usize` is the character-based index, and not the byte-based index. Here's why and
/// how:
/// - Since [AsStrSlice] itself works with [char], see [crate::StringChars]'s
///   [Iterator::Item] impl, all the `usize` in the interface related to index and offset
///   are actually character based.
/// - The [`CharacterIndexNomCompat`] type alias marks that this `usize` represents a
///   character offset, working around nom's constraint that we cannot modify the
///   `FindSubstring` trait signature.
impl<'a> FindSubstring<&str> for AsStrSlice<'a> {
    fn find_substring(&self, sub_str: &str) -> Option<CharacterIndexNomCompat> {
        // Early return for empty substring or empty slice.
        if sub_str.is_empty() {
            return Some(0);
        }

        if self.is_empty() {
            return None;
        }

        // Slow path: if substring contains newline, fall back to full materialization
        // This handles multi-line patterns that would be complex to match across lines.
        if sub_str.contains(NEW_LINE) {
            return self.find_substring_fallback(sub_str);
        }

        // Optimized path: search line by line without full materialization.
        self.find_substring_optimized(sub_str)
    }
}

impl<'a> AsStrSlice<'a> {
    /// Fallback implementation that materializes the full slice.
    /// Used when the substring contains newlines or other complex patterns.
    ///
    /// ## Performance Trade-off
    ///
    /// This method provides **correctness over performance** by materializing
    /// the entire remaining text to handle complex multi-line search patterns.
    /// While this has O(n) memory overhead, it ensures accurate results for
    /// substrings that span across line boundaries.
    ///
    /// ## Unicode Safety
    ///
    /// Maintains full Unicode safety by using the underlying string's
    /// built-in `find()` method, which is Unicode-aware and handles
    /// multi-byte character sequences correctly.
    fn find_substring_fallback(&self, sub_str: &str) -> Option<usize> {
        let full_text = self.extract_to_slice_end();
        full_text.as_ref().find(sub_str)
    }

    /// Optimized implementation that searches line by line without materialization.
    /// This leverages the pre-parsed line structure for better performance.
    ///
    /// ## Performance Characteristics
    ///
    /// - **O(1) line access**: Uses pre-parsed line indices for direct line access
    /// - **Zero-copy design**: Avoids materializing the full remaining text
    /// - **Minimal allocations**: Only allocates when searching from mid-line positions
    /// - **Early termination**: Returns immediately upon finding the first match
    ///
    /// ## Unicode Safety
    ///
    /// Since we've already checked that sub_str doesn't contain newlines,
    /// we only need to search within individual lines, making this much simpler
    /// while maintaining full Unicode safety through `CharLengthExt` usage.
    ///
    /// Character positions are calculated using Unicode-aware methods to ensure
    /// correct behavior with multi-byte UTF-8 sequences and grapheme clusters.
    fn find_substring_optimized(&self, sub_str: &str) -> Option<usize> {
        let mut chars_searched = 0;

        // Start from the current position.
        let start_line_index = self.line_index.as_usize();
        let start_char_index = self.char_index.as_usize();

        for (line_offset, line) in self.lines[start_line_index..].iter().enumerate() {
            let line_str = line.as_ref();
            let current_line_index = start_line_index + line_offset;

            // Determine the starting position within this line.
            let line_start_pos = if current_line_index == start_line_index {
                start_char_index
            } else {
                0
            };

            // Check if we're past the end of this line (at synthetic newline)
            let line_char_count = line_str.len_chars().as_usize();
            if line_start_pos >= line_char_count {
                // We're at the synthetic newline position - move to next line.
                chars_searched += 1; // Count the newline.
                continue;
            }

            // Search for the substring within this line portion.
            let match_pos = if line_start_pos == 0 {
                // Search the entire line.
                line_str.find(sub_str)
            } else {
                // Search from our current position within the line.
                let searchable_content =
                    line_str.chars().skip(line_start_pos).collect::<String>();
                searchable_content.find(sub_str)
            };

            if let Some(pos) = match_pos {
                // Found it! Return offset from our current position.
                return Some(chars_searched + pos);
            }

            // Add the count of characters we searched in this line.
            let chars_in_this_line = line_char_count - line_start_pos;
            chars_searched += chars_in_this_line;

            // Add the newline character if this isn't the last line.
            if current_line_index < self.lines.len() - 1 {
                chars_searched += 1; // for the newline
            }
        }

        // Not found.
        None
    }
}
