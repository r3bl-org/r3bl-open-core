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

//! This module provides adapters to convert various input formats to `ZeroCopyGapBuffer`
//! for compatibility with the new parser that requires `ZeroCopyGapBuffer` input.
//!
//! This is an interim solution until the editor is fully migrated to use `ZeroCopyGapBuffer`.
//!
//! The module provides the following adapters:
//! - `convert_vec_lines_to_gap_buffer()` - Converts `&[GCString]` to `ZeroCopyGapBuffer`
//! - `convert_str_to_gap_buffer()` - Converts `&str` to `ZeroCopyGapBuffer`

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

/// Convert a string slice into a `ZeroCopyGapBuffer`.
///
/// This function takes a string (typically from include_str! or test data) and converts it
/// into a `ZeroCopyGapBuffer` that can be passed to the new `parse_markdown` function.
///
/// The string is split by newlines and each line is added to the buffer with proper null padding.
///
/// # Arguments
/// * `text` - A string slice containing the text to convert
///
/// # Returns
/// A `ZeroCopyGapBuffer` containing the converted content with proper null padding
#[must_use]
pub fn convert_str_to_gap_buffer(text: &str) -> ZeroCopyGapBuffer {
    let mut buffer = ZeroCopyGapBuffer::new();
    
    // Handle empty string case
    if text.is_empty() {
        return buffer;
    }
    
    // Split by newlines, preserving empty lines
    let lines: Vec<&str> = text.split('\n').collect();
    
    // If the text ends with a newline, split will create an empty string at the end
    // We should process all lines in that case
    let num_lines_to_process = if text.ends_with('\n') {
        lines.len() - 1  // Skip the last empty element from split
    } else {
        lines.len()      // Process all lines
    };
    
    for i in 0..num_lines_to_process {
        // Add a new line to the buffer
        let line_index = buffer.add_line();
        
        // Get the line text
        let line_text = lines[i];
        
        // Insert the text content if not empty
        if !line_text.is_empty() {
            let _unused = buffer.insert_at_grapheme(
                line_index.into(),
                SegIndex::from(0),
                line_text
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
    
    #[test]
    fn test_convert_str_empty() {
        let text = "";
        let buffer = convert_str_to_gap_buffer(text);
        
        assert_eq2!(buffer.line_count(), 0);
        assert_eq2!(buffer.as_str(), "");
    }
    
    #[test]
    fn test_convert_str_single_line_no_newline() {
        let text = "Hello, world!";
        let buffer = convert_str_to_gap_buffer(text);
        
        assert_eq2!(buffer.line_count(), 1);
        assert_eq2!(buffer.get_line_content(RowIndex::from(0)), Some("Hello, world!"));
    }
    
    #[test]
    fn test_convert_str_single_line_with_newline() {
        let text = "Hello, world!\n";
        let buffer = convert_str_to_gap_buffer(text);
        
        assert_eq2!(buffer.line_count(), 1);
        assert_eq2!(buffer.get_line_content(RowIndex::from(0)), Some("Hello, world!"));
    }
    
    #[test]
    fn test_convert_str_multiple_lines() {
        let text = "# Heading\n\nParagraph text\nAnother line";
        let buffer = convert_str_to_gap_buffer(text);
        
        assert_eq2!(buffer.line_count(), 4);
        assert_eq2!(buffer.get_line_content(RowIndex::from(0)), Some("# Heading"));
        assert_eq2!(buffer.get_line_content(RowIndex::from(1)), Some(""));
        assert_eq2!(buffer.get_line_content(RowIndex::from(2)), Some("Paragraph text"));
        assert_eq2!(buffer.get_line_content(RowIndex::from(3)), Some("Another line"));
    }
    
    #[test]
    fn test_convert_str_markdown_document() {
        let text = "# Title\n\n## Section 1\n\nSome content here.\n\n- Item 1\n- Item 2\n\n```rust\nfn main() {}\n```";
        let buffer = convert_str_to_gap_buffer(text);
        
        assert_eq2!(buffer.line_count(), 12);
        assert_eq2!(buffer.get_line_content(RowIndex::from(0)), Some("# Title"));
        assert_eq2!(buffer.get_line_content(RowIndex::from(2)), Some("## Section 1"));
        assert_eq2!(buffer.get_line_content(RowIndex::from(4)), Some("Some content here."));
        assert_eq2!(buffer.get_line_content(RowIndex::from(6)), Some("- Item 1"));
        assert_eq2!(buffer.get_line_content(RowIndex::from(10)), Some("fn main() {}"));
        
        // Verify null padding is present
        let full_str = buffer.as_str();
        assert!(full_str.contains('\0'));
    }
    
    #[test]
    fn test_convert_str_empty_lines_at_end() {
        let text = "Line 1\nLine 2\n\n";
        let buffer = convert_str_to_gap_buffer(text);
        
        assert_eq2!(buffer.line_count(), 3);
        assert_eq2!(buffer.get_line_content(RowIndex::from(0)), Some("Line 1"));
        assert_eq2!(buffer.get_line_content(RowIndex::from(1)), Some("Line 2"));
        assert_eq2!(buffer.get_line_content(RowIndex::from(2)), Some(""));
    }
}