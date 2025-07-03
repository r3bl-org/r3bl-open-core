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

/// Implement [`FindSubstring`] trait for [`AsStrSlice`]. This is required by the
/// [`nom::bytes::complete::take_until`] parser function.
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
/// - Since [`AsStrSlice`] itself works with [char], see [`crate::StringChars`]'s
///   [`Iterator::Item`] impl, all the `usize` in the interface related to index and offset
///   are actually character based.
/// - The [`CharacterIndexNomCompat`] type alias marks that this `usize` represents a
///   character offset, working around nom's constraint that we cannot modify the
///   `FindSubstring` trait signature.
impl FindSubstring<&str> for AsStrSlice<'_> {
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
            return self
                .find_substring_across_multiple_lines_full_materialization(sub_str);
        }

        // Optimized path: search line by line without full materialization.
        self.find_substring_optimized_line_by_line_no_copy(sub_str)
    }
}

impl AsStrSlice<'_> {
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
    fn find_substring_across_multiple_lines_full_materialization(
        &self,
        sub_str: &str,
    ) -> Option<usize> {
        let full_text = self.extract_to_slice_end();
        full_text.as_ref().find(sub_str)
    }

    /// Optimized implementation that searches line by line without materialization.
    /// This leverages the pre-parsed line structure for better performance.
    ///
    /// ## Performance Characteristics
    ///
    /// - **O(1) line access**: Uses pre-parsed line indices for direct line access
    /// - **Zero-copy design**: Avoids materializing the full remaining text AND uses
    ///   string slicing for mid-line searches
    /// - **No allocations**: Neither line-start nor mid-line searches require memory
    ///   allocation
    /// - **Early termination**: Returns immediately upon finding the first match
    ///
    /// ## Unicode Safety and Byte-to-Character Conversion
    ///
    /// A critical Unicode handling bug can be introduced when using
    /// `String::find()` since it returns byte positions, but the parser infrastructure
    /// expects character positions. For Unicode text containing multi-byte characters
    /// (emojis, accented characters, CJK text, etc.), byte positions ≠ character
    /// positions.
    ///
    /// We explicitly convert byte positions to character positions:
    /// ```no_run
    /// # let line_str = "";
    /// # let sub_str = "";
    /// line_str.find(sub_str).map(|byte_pos| {
    ///     // Convert byte position to character position
    ///     line_str[..byte_pos].chars().count()
    /// });
    /// ```
    /// ## Character-to-Byte Index Conversion Algorithm
    ///
    /// **Core Challenge**: We have character positions but need byte positions for string
    /// slicing. In Unicode strings, character positions ≠ byte positions due to
    /// multi-byte characters.
    ///
    /// **Example**: `"prefix🎯middle"` where 🎯 is 4 bytes but 1 character
    /// ```text
    /// Char pos: p(0) r(1) e(2) f(3) i(4) x(5) 🎯(6) m(7) i(8) d(9) d(10)
    /// Byte pos: p(0) r(1) e(2) f(3) i(4) x(5) 🎯(6-9) m(10) i(11) d(12)
    /// ```
    ///
    /// If `line_start_pos = 7` (character 'm'), we need byte position 10 to slice
    /// correctly.
    ///
    /// **Algorithm**: Using [`str::char_indices()`] for safe conversion
    /// 1. `char_indices()` yields `(byte_pos, char)` pairs for each character.
    /// 2. `.nth(line_start_pos)` gets the line_start_pos-th character and its byte
    ///    position.
    /// 3. This gives us the exact byte boundary where our character starts.
    /// 4. `&line_str[start_byte_pos..]` creates a valid UTF-8 slice without copying.
    ///
    /// This approach ensures both **Unicode correctness** and **zero-copy performance**.
    ///
    /// Character positions are calculated using Unicode-aware methods to ensure
    /// correct behavior with multi-byte UTF-8 sequences and grapheme clusters.
    fn find_substring_optimized_line_by_line_no_copy(
        &self,
        sub_str: &str,
    ) -> Option<usize> {
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

            // Check if we're past the end of this line (at synthetic newline).
            let line_char_count = line_str.len_chars().as_usize();
            if line_start_pos >= line_char_count {
                // We're at the synthetic newline position - move to next line.
                chars_searched += 1; // Count the newline.
                continue;
            }

            // Search for the substring within this line portion.
            let match_pos = if line_start_pos == 0 {
                // Search the entire line and convert byte position to character position.
                line_str.find(sub_str).map(|byte_pos| {
                    // Convert byte position to character position
                    line_str[..byte_pos].chars().count()
                })
            } else {
                // Search from mid-line using string slicing (see function docs for
                // algorithm details).
                let mut char_indices = line_str.char_indices();
                let start_byte_pos = char_indices
                    .nth(line_start_pos) // Skip to the line_start_pos-th character.
                    .map_or(line_str.len(), |(byte_pos, _)| byte_pos); // If past end, use string length.

                if start_byte_pos < line_str.len() {
                    // Create string slice starting from correct byte boundary.
                    let remaining_str = &line_str[start_byte_pos..];

                    // Search within slice and convert result back to character positions.
                    remaining_str.find(sub_str).map(|byte_pos| {
                        // Convert byte position within slice to character position within
                        // slice.
                        remaining_str[..byte_pos].chars().count()
                    })
                } else {
                    None
                }
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
                chars_searched += 1; // for the newline.
            }
        }

        // Not found.
        None
    }
}

#[cfg(test)]
mod tests_find_substring_optimized {
    use super::*;
    use crate::as_str_slice_test_case;

    #[test]
    fn test_byte_to_char_conversion_bug_fix() {
        // Test the specific Unicode inline code bug that was fixed
        // This tests the byte-to-character position conversion

        as_str_slice_test_case!(slice, "`code 🎯`");

        // Create a slice that starts after the opening backtick
        let mut advanced = slice.clone();
        advanced.advance(); // Skip opening backtick

        // Find the closing backtick - this should work now with the fix
        let result = advanced.find_substring("`");

        // Should find the closing backtick at character position 6
        // (c,o,d,e, ,🎯 = 6 characters from start of "code 🎯`")
        assert_eq!(result, Some(6), "Should find closing backtick after emoji");
    }

    #[test]
    fn test_unicode_character_positions() {
        // Test that byte positions are correctly converted to character positions
        // for various Unicode characters

        // Emoji (4 bytes, 1 character)
        as_str_slice_test_case!(slice, "a🎯b");
        assert_eq!(
            slice.find_substring("🎯"),
            Some(1),
            "Emoji at character position 1"
        );
        assert_eq!(
            slice.find_substring("b"),
            Some(2),
            "Letter 'b' at character position 2"
        );

        // Accented character (2 bytes, 1 character)
        as_str_slice_test_case!(slice2, "café");
        assert_eq!(
            slice2.find_substring("é"),
            Some(3),
            "Accented char at position 3"
        );

        // CJK character (3 bytes, 1 character)
        as_str_slice_test_case!(slice3, "a中b");
        assert_eq!(
            slice3.find_substring("中"),
            Some(1),
            "CJK char at position 1"
        );
        assert_eq!(
            slice3.find_substring("b"),
            Some(2),
            "Letter 'b' at position 2"
        );
    }

    #[test]
    fn test_mid_line_search_with_string_slicing() {
        // Test the string slicing path for mid-line searches
        // This happens when searching from within a line (not at line start)

        as_str_slice_test_case!(slice, "prefix🎯target");

        // Advance to middle of line
        let mut advanced = slice.clone();
        for _ in 0..7 {
            // Skip "prefix🎯" (7 characters)
            advanced.advance();
        }

        // This should use the string slicing path in find_substring_optimized
        let result = advanced.find_substring("target");
        assert_eq!(
            result,
            Some(0),
            "Should find 'target' at relative position 0"
        );
    }

    #[test]
    fn test_line_start_search_without_slicing() {
        // Test the direct line_str.find() path that doesn't need slicing
        // This happens when searching from the start of a line

        as_str_slice_test_case!(slice, "🎯target");

        // This should use the direct line_str.find() path
        let result = slice.find_substring("target");
        assert_eq!(
            result,
            Some(1),
            "Should find 'target' at character position 1"
        );
    }

    #[test]
    fn test_optimized_vs_fallback_consistency() {
        // Ensure the optimized path gives the same results as the fallback path
        // for various Unicode content

        let test_cases = vec!["simple", "🎯emoji", "café", "中文", "∀x∈ℝ"];

        for test_str in test_cases {
            let input = format!("prefix{test_str}suffix");
            as_str_slice_test_case!(slice, &input);

            // Get result from optimized path (used when search doesn't contain newlines)
            let optimized_result = slice.find_substring(test_str);

            // Get result from fallback path
            let fallback_result =
                slice.find_substring_across_multiple_lines_full_materialization(test_str);

            assert_eq!(
                optimized_result, fallback_result,
                "Optimized and fallback should match for '{test_str}'",
            );
        }
    }

    #[test]
    fn test_complex_unicode_scenarios() {
        // Test complex combinations that could trigger byte/char conversion bugs

        // Combining characters and diacritics
        as_str_slice_test_case!(slice1, "Poke\u{0301}mon"); // é as combining character
        assert_eq!(slice1.find_substring("é"), None); // Should not find decomposed é
        assert_eq!(slice1.find_substring("mon"), Some(5)); // Should find at correct position

        // Multiple emoji sequence
        as_str_slice_test_case!(slice2, "🎯🦀🚀test");
        assert_eq!(slice2.find_substring("🦀"), Some(1));
        assert_eq!(slice2.find_substring("test"), Some(3));

        // Mixed scripts
        as_str_slice_test_case!(slice3, "Rust🦀中文тест");
        assert_eq!(slice3.find_substring("🦀"), Some(4));
        assert_eq!(slice3.find_substring("中文"), Some(5));
        assert_eq!(slice3.find_substring("тест"), Some(7));
    }

    #[test]
    fn test_multiline_search_paths() {
        // Test that single-line searches use optimized path
        // and multi-line searches use fallback path

        as_str_slice_test_case!(slice_multi, "line1", "line2🎯test", "line3");

        // Single line search should use optimized path (no newlines in search term)
        let result_single = slice_multi.find_substring("🎯");
        assert_eq!(result_single, Some(11)); // After "line1\nline2" = 5+1+5 = 11

        // Multi-line search should use fallback path (contains newline in search term)
        let result_multi = slice_multi.find_substring("🎯test\nline3");
        assert_eq!(result_multi, Some(11)); // 🎯 starts at position 11
    }

    #[test]
    fn test_edge_case_empty_and_single_char() {
        // Test edge cases with empty strings and single characters

        as_str_slice_test_case!(slice1, "");
        assert_eq!(slice1.find_substring("x"), None);
        assert_eq!(slice1.find_substring(""), Some(0)); // Empty substring always matches at 0

        as_str_slice_test_case!(slice2, "🎯");
        assert_eq!(slice2.find_substring("🎯"), Some(0));
        assert_eq!(slice2.find_substring("x"), None);

        as_str_slice_test_case!(slice3, "a");
        assert_eq!(slice3.find_substring("a"), Some(0));
        assert_eq!(slice3.find_substring(""), Some(0));
    }

    #[test]
    fn test_performance_characteristics() {
        // Test that demonstrates the performance benefit of the optimized path
        // by avoiding materialization of large content

        // Create a large slice to test performance characteristics
        let large_lines: Vec<String> = (0..1000)
            .map(|i| format!("Line {i} with emoji 🎯 and content"))
            .collect();
        let gc_lines: Vec<crate::GCString> = large_lines
            .iter()
            .map(|s| crate::GCString::from(s.as_str()))
            .collect();
        let slice = AsStrSlice::from(gc_lines.as_slice());

        // Search for something in the first few lines - should be fast
        let result = slice.find_substring("Line 5");
        assert!(result.is_some());

        // Search for emoji should also work correctly
        let emoji_result = slice.find_substring("🎯");
        assert!(emoji_result.is_some());
    }

    #[test]
    fn test_string_slicing_necessity_analysis() {
        // This test analyzes the string slicing approach for mid-line searches
        // and demonstrates the scenarios where it's used

        as_str_slice_test_case!(slice, "prefix🎯middle🦀suffix");

        // Test from line start - uses direct find() on full line
        let result_start = slice.find_substring("prefix");
        assert_eq!(result_start, Some(0));

        // Test from middle - uses string slicing (no allocation!)
        let mut advanced = slice.clone();
        for _ in 0..7 {
            // Skip "prefix🎯" (7 characters)
            advanced.advance();
        }
        let result_middle = advanced.find_substring("middle");
        assert_eq!(result_middle, Some(0)); // Relative to current position

        // Verify consistency
        let full_result = slice.find_substring("middle");
        assert_eq!(full_result, Some(7)); // Absolute position
    }

    #[test]
    fn test_byte_char_boundary_safety() {
        // Test that we never create invalid string slices when dealing with
        // multi-byte characters at boundaries

        // Character that spans multiple bytes at different positions
        let test_cases = vec![
            ("🎯", "🎯"),   // Start of string
            ("a🎯", "🎯"),  // After ASCII
            ("🎯a", "a"),   // ASCII after emoji
            ("🎯🦀", "🦀"), // Emoji after emoji
        ];

        for (input, search) in test_cases {
            as_str_slice_test_case!(slice, input);
            let result = slice.find_substring(search);
            assert!(result.is_some(), "Should find '{search}' in '{input}'");

            // Test advancing through the string doesn't break on character boundaries
            let mut advanced = slice.clone();
            while !advanced.is_empty() {
                let _ = advanced.find_substring(search);
                advanced.advance();
            }
        }
    }

    #[test]
    fn test_unicode_content_and_handle_byte_and_char_index_correctly() {
        // Test the exact scenario that triggered the original bug discovery
        // This helps document the real-world context where the bug occurred

        // The markdown: `code 🎯`
        as_str_slice_test_case!(slice, "`code 🎯`");

        // Simulate what the inline code parser does:
        // 1. Find opening backtick (position 0)
        let opening = slice.find_substring("`");
        assert_eq!(opening, Some(0));

        // 2. Advance past opening backtick
        let mut after_opening = slice.clone();
        after_opening.advance();

        // 3. Find closing backtick (this is where the bug was)
        let closing = after_opening.find_substring("`");
        assert_eq!(closing, Some(6)); // After "code 🎯" (6 characters)

        // 4. The content between should be "code 🎯"
        // This test verifies that the character position calculation is correct
        let content_length = closing.unwrap();
        assert_eq!(content_length, 6); // "code 🎯" is 6 characters
    }
}
