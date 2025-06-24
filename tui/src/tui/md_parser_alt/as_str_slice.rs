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

#[cfg(test)]
mod tests_as_str_slice_test_case {
    use crate::{as_str_slice_test_case, assert_eq2};

    #[test]
    fn test_as_str_slice_creation() {
        // Single string.
        as_str_slice_test_case!(input, "@title: Something");
        assert_eq2!(input.lines.len(), 1);
        assert_eq2!(input.lines[0].as_ref(), "@title: Something");

        // Multiple strings.
        as_str_slice_test_case!(input, "@title: Something", "more content", "even more");
        assert_eq2!(input.lines.len(), 3);
        assert_eq2!(input.lines[0].as_ref(), "@title: Something");
        assert_eq2!(input.lines[1].as_ref(), "more content");
        assert_eq2!(input.lines[2].as_ref(), "even more");

        // With trailing comma (optional).
        as_str_slice_test_case!(input, "@title: Something",);
        assert_eq2!(input.lines.len(), 1);
        assert_eq2!(input.lines[0].as_ref(), "@title: Something");
    }
}

/// Unit tests for the [AsStrSlice] struct and its methods.
#[cfg(test)]
mod tests_as_str_slice_basic_functionality {
    use nom::Input;

    use crate::{as_str_slice_test_case, assert_eq2, idx, len, AsStrSlice};

    #[test]
    fn test_gc_string_slice_basic_functionality() {
        as_str_slice_test_case!(slice, "Hello world", "This is a test", "Third line");

        // Test that we can iterate through characters.
        let mut chars: Vec<char> = vec![];
        let mut current = slice;
        while let Some(ch) = current.current_char() {
            chars.push(ch);
            current.advance();
        }

        let expected = "Hello world\nThis is a test\nThird line\n"; // Trailing newline for multiple lines
        let result: String = chars.into_iter().collect();
        std::assert_eq!(result, expected);
    }

    #[test]
    fn test_nom_input_position() {
        as_str_slice_test_case!(slice, "hello", "world");

        // Test position finding
        let pos = slice.position(|c| c == 'w');
        std::assert_eq!(pos, Some(6)); // "hello\n" = 6 chars, then 'w'

        let pos = slice.position(|c| c == 'z');
        std::assert_eq!(pos, None); // 'z' not found
    }

    pub mod fixtures {
        use crate::GCString;

        pub fn create_test_lines() -> Vec<GCString> {
            vec![
                GCString::new("Hello world"),
                GCString::new("Second line"),
                GCString::new("Third line"),
                GCString::new(""),
                GCString::new("Fifth line"),
            ]
        }

        pub fn create_simple_lines() -> Vec<GCString> {
            vec![GCString::new("abc"), GCString::new("def")]
        }
    }

    // Test From trait implementations
    #[test]
    fn test_from_slice() {
        let lines = fixtures::create_test_lines();
        let slice = AsStrSlice::from(lines.as_slice());

        assert_eq!(slice.line_index, idx(0));
        assert_eq!(slice.char_index, idx(0));
        assert_eq!(slice.max_len, None);
        assert_eq!(slice.lines.len(), 5);
    }

    #[test]
    fn test_from_vec() {
        let lines = fixtures::create_test_lines();
        let slice = AsStrSlice::from(&lines);

        assert_eq!(slice.line_index, idx(0));
        assert_eq!(slice.char_index, idx(0));
        assert_eq!(slice.max_len, None);
        assert_eq!(slice.lines.len(), 5);
    }

    // Test Clone and PartialEq traits
    #[test]
    fn test_clone_and_partial_eq() {
        let lines = fixtures::create_test_lines();
        let slice1 = AsStrSlice::from(lines.as_slice());
        let slice2 = slice1.clone();

        assert_eq!(slice1, slice2);

        let slice3 = slice1.take_from(1);
        assert_ne!(slice1, slice3);
    }

    // Test with_limit constructor and behavior.
    #[test]
    fn test_with_limit() {
        let lines = fixtures::create_test_lines();

        // Basic constructor test
        let slice = AsStrSlice::with_limit(&lines, idx(1), idx(3), Some(len(5)));
        assert_eq!(slice.line_index, idx(1));
        assert_eq!(slice.char_index, idx(3));
        assert_eq!(slice.max_len, Some(len(5)));

        // Test behavior with limit
        let content = slice.extract_to_line_end();
        assert_eq!(content, "ond l"); // "Second line" starting at index 3 with max 5 chars

        // Test with limit spanning multiple lines
        let multi_line_slice =
            AsStrSlice::with_limit(&lines, idx(0), idx(6), Some(len(15)));
        let result = multi_line_slice.to_inline_string();
        assert_eq!(result, "world\nSecond li"); // 15 chars total

        // Test with no limit
        let no_limit_slice = AsStrSlice::with_limit(&lines, idx(0), idx(6), None);
        let result = no_limit_slice.to_inline_string();
        assert_eq!(result, "world\nSecond line\nThird line\n\nFifth line\n");

        // Test with zero limit
        let zero_limit_slice =
            AsStrSlice::with_limit(&lines, idx(0), idx(0), Some(len(0)));
        assert_eq!(zero_limit_slice.current_char(), None);
        assert_eq!(zero_limit_slice.input_len(), 0);
        assert_eq!(zero_limit_slice.to_inline_string(), "");

        // Test with out-of-bounds line index
        let oob_slice = AsStrSlice::with_limit(&lines, idx(10), idx(0), None);
        assert_eq!(oob_slice.current_char(), None);
        assert_eq!(oob_slice.input_len(), 0);
        assert_eq!(oob_slice.to_inline_string(), "");

        // Test with out-of-bounds char index
        let oob_char_slice = AsStrSlice::with_limit(&lines, idx(0), idx(100), None);
        assert_eq!(oob_char_slice.current_char(), None);
        assert_eq!(oob_char_slice.to_inline_string(), "");
    }

    // Test extract_remaining_text_content_in_line
    #[test]
    fn test_extract_remaining_text_content_in_line() {
        let lines = fixtures::create_test_lines();
        let slice = AsStrSlice::from(lines.as_slice());

        // From beginning of first line.
        assert_eq!(slice.extract_to_line_end(), "Hello world");

        // From middle of first line.
        let slice_offset = slice.take_from(6);
        assert_eq!(slice_offset.extract_to_line_end(), "world");

        // From empty line
        let slice_empty = AsStrSlice::with_limit(&lines, idx(3), idx(0), None);
        assert_eq!(slice_empty.extract_to_line_end(), "");

        // With max_len limit
        let slice_limited = AsStrSlice::with_limit(&lines, idx(0), idx(0), Some(len(5)));
        assert_eq!(slice_limited.extract_to_line_end(), "Hello");

        // Out of bounds
        let slice_oob = AsStrSlice::with_limit(&lines, idx(10), idx(0), None);
        assert_eq!(slice_oob.extract_to_line_end(), "");
    }

    // Test current_char and advance
    #[test]
    fn test_current_char_and_advance() {
        let lines = fixtures::create_simple_lines(); // Creates ["abc", "def"]
        let mut slice = AsStrSlice::from(lines.as_slice());
        // Input appears as: "abc\ndef\n" (synthetic \n added between lines + trailing \n)
        // Positions: a(0), b(1), c(2), \n(3), d(4), e(5), f(6), \n(7)

        // Test normal characters
        assert_eq!(slice.current_char(), Some('a'));
        slice.advance();
        assert_eq!(slice.current_char(), Some('b'));
        slice.advance();
        assert_eq!(slice.current_char(), Some('c'));
        slice.advance();

        // Test synthetic newline between lines
        assert_eq!(slice.current_char(), Some('\n'));
        slice.advance();

        // Test second line
        assert_eq!(slice.current_char(), Some('d'));
        slice.advance();
        assert_eq!(slice.current_char(), Some('e'));
        slice.advance();
        assert_eq!(slice.current_char(), Some('f'));
        slice.advance();

        // Test trailing newline for multiple lines.
        assert_eq!(slice.current_char(), Some('\n'));
        slice.advance();

        // Test end of input.
        assert_eq!(slice.current_char(), None);
    }

    #[test]
    fn test_advance_with_max_len_zero_at_end_of_line() {
        // This test specifically covers the scenario that was causing the parser to hang:
        // When max_len=0 and we're at the end of a line, advance() should still move to
        // the next line.

        as_str_slice_test_case!(slice, "short line", "next line");
        let mut slice = slice;

        // Advance to end of first line
        for _ in 0..10 {
            // "short line" has 10 characters
            slice.advance();
        }

        // At this point we should be at the end of the first line
        assert_eq2!(slice.line_index, idx(0));
        assert_eq2!(slice.char_index, idx(10));

        // Now set max_len to 0 to simulate the problematic condition
        slice.max_len = Some(len(0));

        // The advance() should still work and move us to the next line
        slice.advance();

        // We should now be at the beginning of the second line
        assert_eq2!(slice.line_index, idx(1));
        assert_eq2!(slice.char_index, idx(0));
    }

    #[test]
    fn test_advance_through_multiline_content() {
        // Test advancing through multiple lines to ensure proper line transitions
        as_str_slice_test_case!(slice, "ab", "cd", "ef");
        let mut slice = slice;

        let expected_positions = vec![
            // First line: "ab"
            (0, 0, Some('a')),
            (0, 1, Some('b')),
            (0, 2, Some('\n')), // Synthetic newline
            // Second line: "cd"
            (1, 0, Some('c')),
            (1, 1, Some('d')),
            (1, 2, Some('\n')), // Synthetic newline
            // Third line: "ef"
            (2, 0, Some('e')),
            (2, 1, Some('f')),
            (2, 2, Some('\n')), // Trailing synthetic newline
            (2, 3, None),       // Past end
        ];

        for (expected_line, expected_char, expected_current_char) in expected_positions {
            assert_eq2!(slice.line_index.as_usize(), expected_line);
            assert_eq2!(slice.char_index.as_usize(), expected_char);
            assert_eq2!(slice.current_char(), expected_current_char);

            if slice.current_char().is_some() {
                slice.advance();
            }
        }
    }

    #[test]
    fn test_advance_with_max_len_constraint() {
        // Test that advance() respects max_len constraints
        as_str_slice_test_case!(slice, "hello world", "second line");
        let mut limited_slice = slice.take(5); // Only "hello"

        // Should be able to advance 5 times to consume "hello"
        for i in 0..5 {
            assert_eq2!(limited_slice.char_index.as_usize(), i);
            limited_slice.advance();
        }

        // At this point, max_len should be 0 and we should be at position 5
        assert_eq2!(limited_slice.char_index.as_usize(), 5);
        assert_eq2!(limited_slice.max_len, Some(len(0)));

        // Further advances should not move the position when max_len is exhausted
        // (unless we're at end of line transitioning to next line)
        let original_position = (limited_slice.line_index, limited_slice.char_index);
        limited_slice.advance();
        // Position should remain the same since we're in the middle of a line with
        // max_len=0
        assert_eq2!(
            (limited_slice.line_index, limited_slice.char_index),
            original_position
        );
    }

    #[test]
    fn test_advance_single_line_behavior() {
        // Test advance behavior with single line (no trailing newline)
        as_str_slice_test_case!(slice, "hello");
        let mut slice = slice;

        // Advance through all characters
        for i in 0..5 {
            assert_eq2!(slice.char_index.as_usize(), i);
            slice.advance();
        }

        // After consuming all characters, we should be at the end
        assert_eq2!(slice.char_index.as_usize(), 5);
        assert_eq2!(slice.current_char(), None); // No trailing newline for single line

        // Further advances should be no-ops
        let final_position = (slice.line_index, slice.char_index);
        slice.advance();
        assert_eq2!((slice.line_index, slice.char_index), final_position);
    }

    #[test]
    fn test_advance_empty_lines() {
        // Test advance behavior with empty lines
        as_str_slice_test_case!(slice, "", "content", "");
        let mut slice = slice;

        let expected_sequence = vec![
            (0, 0, Some('\n')), // Empty first line -> synthetic newline
            (1, 0, Some('c')),  // Start of "content"
            (1, 1, Some('o')),
            (1, 2, Some('n')),
            (1, 3, Some('t')),
            (1, 4, Some('e')),
            (1, 5, Some('n')),
            (1, 6, Some('t')),
            (1, 7, Some('\n')), // End of "content" -> synthetic newline
            (2, 0, Some('\n')), // Empty last line -> trailing newline
            (2, 1, None),       // Past end
        ];

        for (expected_line, expected_char, expected_current_char) in expected_sequence {
            assert_eq2!(slice.line_index.as_usize(), expected_line);
            assert_eq2!(slice.char_index.as_usize(), expected_char);
            assert_eq2!(slice.current_char(), expected_current_char);

            if slice.current_char().is_some() {
                slice.advance();
            }
        }
    }
}

#[cfg(test)]
mod tests_is_empty_character_exhaustion {
    use crate::{as_str_slice_test_case, assert_eq2, idx, len, AsStrSlice, GCString};

    #[test]
    fn test_is_empty_basic_behavior() {
        // Empty slice should be empty
        {
            let empty_lines: &[GCString] = &[];
            let slice = AsStrSlice::from(empty_lines);
            assert_eq2!(slice.is_empty(), true);
        }

        // Non-empty slice at start should not be empty
        {
            as_str_slice_test_case!(slice, "hello");
            assert_eq2!(slice.is_empty(), false);
        }
    }

    #[test]
    fn test_is_empty_when_current_taken_equals_total_size() {
        // Test the new behavior: is_empty() returns true when current_taken >= total_size
        {
            as_str_slice_test_case!(slice, "hello", "world");
            let mut slice = slice;

            // Initially not empty
            assert_eq2!(slice.is_empty(), false);
            assert_eq2!(slice.current_taken < slice.total_size, true);

            // Advance through all characters
            while slice.current_char().is_some() {
                slice.advance();
            }

            // Now should be empty because current_taken >= total_size
            assert_eq2!(slice.is_empty(), true);
            assert_eq2!(slice.current_taken >= slice.total_size, true);
        }
    }

    #[test]
    fn test_is_empty_with_max_len_zero() {
        // max_len = 0 should make slice empty regardless of content
        {
            as_str_slice_test_case!(slice, limit: 0, "hello world");
            assert_eq2!(slice.is_empty(), true);
            assert_eq2!(slice.max_len, Some(len(0)));
        }
    }

    #[test]
    fn test_is_empty_past_available_lines() {
        // When line_index >= lines.len(), should be empty
        {
            as_str_slice_test_case!(slice, "hello");
            let past_end = AsStrSlice::with_limit(
                slice.lines,
                idx(1), // Past the only line (index 0)
                idx(0),
                None,
            );
            assert_eq2!(past_end.is_empty(), true);
        }
    }

    #[test]
    fn test_is_empty_single_line_exhausted() {
        // Single line: when all characters consumed, should be empty
        {
            as_str_slice_test_case!(slice, "hi");
            let mut slice = slice;

            // Advance to end of line
            slice.advance(); // 'h'
            slice.advance(); // 'i'

            // Now at end of single line, should be empty
            assert_eq2!(slice.is_empty(), true);
            assert_eq2!(slice.current_char(), None);
        }
    }

    #[test]
    fn test_is_empty_multiline_exhausted() {
        // Multiple lines: when all characters consumed, should be empty
        {
            as_str_slice_test_case!(slice, "a", "b");
            let mut slice = slice;

            // Total: "a" (1) + "\n" (1) + "b" (1) + "\n" (1) = 4 chars
            let expected_total = 4;
            assert_eq2!(slice.total_size.as_usize(), expected_total);

            // Advance through all characters
            for _ in 0..expected_total {
                assert_eq2!(slice.is_empty(), false);
                slice.advance();
            }

            // Now should be empty
            assert_eq2!(slice.is_empty(), true);
            assert_eq2!(slice.current_taken.as_usize(), expected_total);
        }
    }

    #[test]
    fn test_is_empty_with_unicode() {
        // Test with Unicode characters
        {
            as_str_slice_test_case!(slice, "😀hello");
            let mut slice = slice;

            // Should not be empty initially
            assert_eq2!(slice.is_empty(), false);

            // Advance through all characters: 😀(1) + hello(5) = 6 chars
            for _ in 0..6 {
                assert_eq2!(slice.is_empty(), false);
                slice.advance();
            }

            // Now should be empty
            assert_eq2!(slice.is_empty(), true);
        }
    }

    #[test]
    fn test_is_empty_empty_lines_in_multiline() {
        // Test with empty lines in multiline content
        {
            as_str_slice_test_case!(slice, "", "content", "");
            let mut slice = slice;

            // Initially not empty
            assert_eq2!(slice.is_empty(), false);

            // Advance through all: "" + "\n" + "content" + "\n" + "" + "\n" = 10 chars
            let expected_chars = ['\n', 'c', 'o', 'n', 't', 'e', 'n', 't', '\n', '\n'];

            for expected_char in expected_chars {
                assert_eq2!(slice.is_empty(), false);
                assert_eq2!(slice.current_char(), Some(expected_char));
                slice.advance();
            }

            // Now should be empty
            assert_eq2!(slice.is_empty(), true);
        }
    }

    #[test]
    fn test_is_empty_respects_max_len() {
        // When max_len limits available characters, is_empty should respect that
        {
            as_str_slice_test_case!(slice, limit: 3, "hello world");
            let mut slice = slice;

            // Initially not empty
            assert_eq2!(slice.is_empty(), false);

            // Advance 3 characters (the limit)
            for _ in 0..3 {
                assert_eq2!(slice.is_empty(), false);
                slice.advance();
            }

            // Now should be empty due to max_len limit, even though there's more content
            assert_eq2!(slice.is_empty(), true);
            assert_eq2!(slice.max_len, Some(len(0))); // Should be exhausted
        }
    }

    #[test]
    fn test_is_empty_consistency_with_current_char() {
        // is_empty() should be consistent with current_char() returning None
        {
            as_str_slice_test_case!(slice, "test");
            let mut slice = slice;

            while !slice.is_empty() {
                assert_eq2!(slice.current_char().is_some(), true);
                slice.advance();
            }

            // When empty, current_char should return None
            assert_eq2!(slice.is_empty(), true);
            assert_eq2!(slice.current_char(), None);
        }
    }

    #[test]
    fn test_is_empty_edge_case_character_exhaustion_vs_line_exhaustion() {
        // Test the edge case where we're at an empty line but have consumed all chars
        {
            as_str_slice_test_case!(slice, "text", "");
            let mut slice = slice;

            // Advance through "text" + synthetic newline = 5 chars
            for _ in 0..5 {
                slice.advance();
            }

            // Now we're at the start of the empty line, line_index=1, char_index=0
            assert_eq2!(slice.line_index.as_usize(), 1);
            assert_eq2!(slice.char_index.as_usize(), 0);

            // But we haven't consumed all characters yet (empty line has a trailing
            // newline)
            assert_eq2!(slice.is_empty(), false);
            assert_eq2!(slice.current_char(), Some('\n')); // Synthetic newline from empty line

            // Advance one more to consume the trailing newline
            slice.advance();

            // Now should be empty (all characters consumed)
            assert_eq2!(slice.is_empty(), true);
            assert_eq2!(slice.current_taken >= slice.total_size, true);
        }
    }
}
