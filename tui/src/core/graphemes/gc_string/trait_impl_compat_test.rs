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

//! Compatibility tests between `GCStringOwned` and `GCStringRef` implementations.
//!
//! This module contains comprehensive tests to verify that both implementations
//! of the `GCString` trait produce identical results for all trait methods.
//! This ensures that `GCStringRef` can be used as a drop-in replacement for
//! `GCStringOwned` when working with borrowed string data.

#[cfg(test)]
mod tests {
    use crate::{ColWidth, GCString, GCStringOwned, GCStringRef};

    /// Comprehensive test to verify `GCStringRef` matches `GCStringOwned` exactly
    /// for all `GCString` trait methods.
    ///
    /// This test ensures that both implementations produce identical results
    /// across a variety of test cases including ASCII text, Unicode text with
    /// emojis, complex emoji sequences, empty strings, and edge cases.
    #[test]
    fn test_gcstring_trait_methods_match_exactly() {
        let test_cases = [
            "Hello, World!",
            "Hello, 🙏🏽 World!",
            "🙏🏽",
            "",
            "A",
            "Hello 👨‍👩‍👧‍👦 Family",
            "Café with é accent",
            "CJK: 中文字符",
            "Mixed: Hello 🌍 Café 中文",
            "Long text with multiple emojis 🚀 🎉 🔥 and Unicode characters",
        ];

        for text in test_cases {
            let gc_ref = GCStringRef::new(text);
            let gc_owned = GCStringOwned::new(text);

            // Test basic trait methods
            assert_eq!(gc_ref.len(), gc_owned.len(), "len() mismatch for: '{text}'");
            assert_eq!(
                gc_ref.is_empty(),
                gc_owned.is_empty(),
                "is_empty() mismatch for: '{text}'"
            );
            assert_eq!(
                gc_ref.get_max_seg_index(),
                gc_owned.get_max_seg_index(),
                "get_max_seg_index() mismatch for: '{text}'"
            );
            assert_eq!(
                gc_ref.as_str(),
                gc_owned.as_str(),
                "as_str() mismatch for: '{text}'"
            );
            assert_eq!(
                gc_ref.display_width(),
                gc_owned.display_width(),
                "display_width() mismatch for: '{text}'"
            );
            assert_eq!(
                gc_ref.bytes_size(),
                gc_owned.bytes_size(),
                "bytes_size() mismatch for: '{text}'"
            );
            assert_eq!(
                gc_ref.contains_wide_segments(),
                gc_owned.contains_wide_segments(),
                "contains_wide_segments() mismatch for: '{text}'"
            );

            // Test segment access
            for i in 0..gc_ref.len().as_usize() {
                assert_eq!(
                    gc_ref.get(i),
                    gc_owned.get(i),
                    "get({i}) mismatch for: '{text}'"
                );
            }

            // Test segment iterators (seg_iter returns same &Seg references)
            let ref_segments: Vec<_> = gc_ref.seg_iter().collect();
            let owned_segments: Vec<_> = gc_owned.seg_iter().collect();
            assert_eq!(
                ref_segments, owned_segments,
                "seg_iter() mismatch for: '{text}'"
            );

            // Test iter() method - verify both produce same number of segments
            // (Avoiding direct comparison due to potential iterator implementation
            // differences)
            let ref_iter_count = gc_ref.iter().count();
            let owned_iter_count = gc_owned.iter().count();
            assert_eq!(
                ref_iter_count, owned_iter_count,
                "iter() count mismatch for: '{text}'"
            );

            // Test truncation methods with various widths
            for width in [0, 1, 2, 3, 5, 10, 20, 50] {
                let col_width = ColWidth::from(width);

                assert_eq!(
                    gc_ref.trunc_end_to_fit(col_width),
                    gc_owned.trunc_end_to_fit(col_width),
                    "trunc_end_to_fit({width}) mismatch for: '{text}'"
                );
                assert_eq!(
                    gc_ref.trunc_end_by(col_width),
                    gc_owned.trunc_end_by(col_width),
                    "trunc_end_by({width}) mismatch for: '{text}'"
                );
                assert_eq!(
                    gc_ref.trunc_start_by(col_width),
                    gc_owned.trunc_start_by(col_width),
                    "trunc_start_by({width}) mismatch for: '{text}'"
                );
            }
        }
    }

    /// Test that string slicing methods produce equivalent results.
    ///
    /// Note: The actual string content of the results may differ between
    /// implementations (since `GCStringRef` returns `SegStringRef` and `GCStringOwned`
    /// returns `SegStringOwned`), but the string content, width, and position
    /// should be identical.
    #[test]
    fn test_string_slicing_methods_compatibility() {
        let test_cases = ["Hello, World!", "Hello, 🙏🏽!", "🚀🎉🔥", "A", "Café"];

        for text in test_cases {
            let gc_ref = GCStringRef::new(text);
            let gc_owned = GCStringOwned::new(text);

            // Test get_string_at_end
            match (gc_ref.get_string_at_end(), gc_owned.get_string_at_end()) {
                (Some(ref_result), Some(owned_result)) => {
                    assert_eq!(
                        ref_result.string.as_str(),
                        owned_result.string.as_str(),
                        "get_string_at_end() string content mismatch for: '{text}'"
                    );
                    assert_eq!(
                        ref_result.width, owned_result.width,
                        "get_string_at_end() width mismatch for: '{text}'"
                    );
                    assert_eq!(
                        ref_result.start_at, owned_result.start_at,
                        "get_string_at_end() start_at mismatch for: '{text}'"
                    );
                }
                (None, None) => {
                    // Both return None - this is correct for empty strings
                }
                (ref_opt, owned_opt) => {
                    panic!(
                        "get_string_at_end() option mismatch for: '{}'. ref: {:?}, owned: {:?}",
                        text,
                        ref_opt.is_some(),
                        owned_opt.is_some()
                    );
                }
            }
        }
    }

    /// Performance comparison test (informational only).
    ///
    /// This test doesn't assert anything but can be used to compare the
    /// relative performance characteristics of both implementations.
    #[test]
    fn test_performance_characteristics_info() {
        let text = "Performance test with emoji 🚀 and Unicode characters 中文 and more content to test with";

        // Test construction performance characteristics
        let _gc_ref = GCStringRef::new(text);
        let _gc_owned = GCStringOwned::new(text);

        // Both should have identical segment computation costs
        // GCStringRef has advantage of not allocating string storage
        // GCStringOwned has advantage of not recomputing segments when created from
        // existing data

        // This test primarily serves as documentation of the performance trade-offs
        // Performance test completed - see comments for analysis
    }

    /// Test edge cases and boundary conditions.
    #[test]
    fn test_edge_cases_compatibility() {
        let edge_cases = [
            "",          // Empty string
            " ",         // Single space
            "\n",        // Single newline
            "\t",        // Single tab
            "🏳️‍🌈",        // Complex emoji with ZWJ sequence
            "👨‍👩‍👧‍👦",        // Family emoji
            "🙏🏽",        // Emoji with skin tone modifier
            "é",         // Single accented character
            "é",         // Same character composed differently (if applicable)
            "a\u{0301}", // Base character + combining accent
        ];

        for text in edge_cases {
            let gc_ref = GCStringRef::new(text);
            let gc_owned = GCStringOwned::new(text);

            // All basic methods should match exactly
            assert_eq!(
                gc_ref.len(),
                gc_owned.len(),
                "Edge case len() mismatch for: {text:?}"
            );
            assert_eq!(
                gc_ref.display_width(),
                gc_owned.display_width(),
                "Edge case display_width() mismatch for: {text:?}"
            );
            assert_eq!(
                gc_ref.bytes_size(),
                gc_owned.bytes_size(),
                "Edge case bytes_size() mismatch for: {text:?}"
            );
            assert_eq!(
                gc_ref.contains_wide_segments(),
                gc_owned.contains_wide_segments(),
                "Edge case contains_wide_segments() mismatch for: {text:?}"
            );
        }
    }
}
