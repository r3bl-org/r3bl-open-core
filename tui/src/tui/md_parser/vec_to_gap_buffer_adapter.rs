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

//! This module provides an adapter to convert `VecEditorContentLines` to `ZeroCopyGapBuffer`
//! for compatibility with the new parser that requires `ZeroCopyGapBuffer` input.
//!
//! This is an interim solution until the editor is fully migrated to use `ZeroCopyGapBuffer`.

use crate::{ZeroCopyGapBuffer, GCString, SegIndex};

/// Convert a slice of `GCString` lines into a `ZeroCopyGapBuffer`.
///
/// This function takes editor content lines (Vec<GCString>) and converts them into
/// a `ZeroCopyGapBuffer` that can be passed to the new `parse_markdown` function.
///
/// # Arguments
/// * `lines` - A slice of `GCString` lines from the editor
///
/// # Returns
/// A `ZeroCopyGapBuffer` containing the converted content with proper null padding
#[must_use]
pub fn convert_vec_lines_to_gap_buffer(lines: &[GCString]) -> ZeroCopyGapBuffer {
    let mut buffer = ZeroCopyGapBuffer::new();

    for line in lines {
        // Add a new line to the buffer
        let line_index = buffer.add_line();

        // Get the text content from GCString
        let text = line.as_ref();

        // Insert the text at the beginning of the line
        if !text.is_empty() {
            // Use insert_at_grapheme which is the public API
            let _unused = buffer.insert_at_grapheme(
                line_index.into(),
                SegIndex::from(0),
                text
            );
        }
    }

    buffer
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assert_eq2, RowIndex};

    #[test]
    fn test_convert_empty_lines() {
        let lines: Vec<GCString> = vec![];
        let buffer = convert_vec_lines_to_gap_buffer(&lines);

        assert_eq2!(buffer.line_count(), 0);
        assert_eq2!(buffer.as_str(), "");
    }

    #[test]
    fn test_convert_single_line() {
        let lines = vec![
            GCString::from("Hello, world!")
        ];
        let buffer = convert_vec_lines_to_gap_buffer(&lines);

        assert_eq2!(buffer.line_count(), 1);
        assert_eq2!(buffer.get_line_content(RowIndex::from(0)), Some("Hello, world!"));

        // Check that the buffer has proper null padding
        let full_str = buffer.as_str();
        assert!(full_str.starts_with("Hello, world!\n"));
        assert!(full_str.contains('\0'));
    }

    #[test]
    fn test_convert_multiple_lines() {
        let lines = vec![
            GCString::from("# Title"),
            GCString::from(""),
            GCString::from("Some content"),
            GCString::from("- List item"),
        ];
        let buffer = convert_vec_lines_to_gap_buffer(&lines);

        assert_eq2!(buffer.line_count(), 4);
        assert_eq2!(buffer.get_line_content(RowIndex::from(0)), Some("# Title"));
        assert_eq2!(buffer.get_line_content(RowIndex::from(1)), Some(""));
        assert_eq2!(buffer.get_line_content(RowIndex::from(2)), Some("Some content"));
        assert_eq2!(buffer.get_line_content(RowIndex::from(3)), Some("- List item"));
    }

    #[test]
    fn test_convert_with_unicode() {
        let lines = vec![
            GCString::from("Hello 👋 世界"),
            GCString::from("Émojis: 🦀💻🎉"),
            GCString::from("Café ☕"),
        ];
        let buffer = convert_vec_lines_to_gap_buffer(&lines);

        assert_eq2!(buffer.line_count(), 3);
        assert_eq2!(buffer.get_line_content(RowIndex::from(0)), Some("Hello 👋 世界"));
        assert_eq2!(buffer.get_line_content(RowIndex::from(1)), Some("Émojis: 🦀💻🎉"));
        assert_eq2!(buffer.get_line_content(RowIndex::from(2)), Some("Café ☕"));
    }

    #[test]
    fn test_convert_code_block() {
        let lines = vec![
            GCString::from("```rust"),
            GCString::from("fn main() {"),
            GCString::from("    println!(\"Hello\");"),
            GCString::from("}"),
            GCString::from("```"),
        ];
        let buffer = convert_vec_lines_to_gap_buffer(&lines);
        
        assert_eq2!(buffer.line_count(), 5);
        assert_eq2!(buffer.get_line_content(RowIndex::from(0)), Some("```rust"));
        assert_eq2!(buffer.get_line_content(RowIndex::from(1)), Some("fn main() {"));
        assert_eq2!(buffer.get_line_content(RowIndex::from(2)), Some("    println!(\"Hello\");"));
        assert_eq2!(buffer.get_line_content(RowIndex::from(3)), Some("}"));
        assert_eq2!(buffer.get_line_content(RowIndex::from(4)), Some("```"));
    }
}