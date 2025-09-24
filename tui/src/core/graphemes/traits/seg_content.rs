// Copyright (c) 2024 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::ops::Range;

use crate::{ByteIndex, ColIndex, ColWidth, Seg};

/// Core segment content reference for zero-copy access to grapheme cluster segments.
///
/// This struct provides a unified way to access segment content and metadata
/// without copying the underlying string data. The lifetime parameter `'a` represents
/// the lifetime of the borrowed string content, ensuring that the `SegContent`
/// cannot outlive the string it references.
#[derive(Debug, Clone, Copy)]
pub struct SegContent<'a> {
    /// The actual string content of the segment.
    pub content: &'a str,
    /// The segment metadata.
    pub seg: Seg,
}

impl SegContent<'_> {
    /// Get the string content of this segment.
    #[must_use]
    pub fn as_str(&self) -> &str { self.content }

    /// Get the display width of this segment.
    #[must_use]
    pub fn width(&self) -> ColWidth { self.seg.display_width }

    /// Get the starting column index of this segment.
    #[must_use]
    pub fn start_col(&self) -> ColIndex { self.seg.start_display_col_index }

    /// Get a reference to the underlying segment metadata.
    #[must_use]
    pub fn seg(&self) -> &Seg { &self.seg }

    /// Get the byte range of this segment within the original string.
    #[must_use]
    pub fn byte_range(&self) -> Range<ByteIndex> {
        self.seg.start_byte_index..self.seg.end_byte_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{byte_index, col, seg_index, width};

    // Helper function to create a test segment
    fn create_test_seg(
        start_byte: usize,
        end_byte: usize,
        display_width: usize,
        segment_index: usize,
        start_col: usize,
    ) -> Seg {
        Seg {
            start_byte_index: byte_index(start_byte),
            end_byte_index: byte_index(end_byte),
            display_width: width(display_width),
            seg_index: seg_index(segment_index),
            bytes_size: crate::len(end_byte - start_byte),
            start_display_col_index: col(start_col),
        }
    }

    #[test]
    fn test_seg_content_creation() {
        let content = "a";
        let seg = create_test_seg(0, 1, 1, 0, 0);

        let seg_content = SegContent { content, seg };

        assert_eq!(seg_content.content, "a");
        assert_eq!(seg_content.seg.seg_index, seg_index(0));
    }

    #[test]
    fn test_as_str() {
        let content = "hello";
        let seg = create_test_seg(0, 5, 5, 0, 0);
        let seg_content = SegContent { content, seg };

        assert_eq!(seg_content.as_str(), "hello");
        assert_eq!(seg_content.as_str(), content);
    }

    #[test]
    fn test_width() {
        let content = "😀";
        let seg = create_test_seg(0, 4, 2, 0, 0); // Emoji is 4 bytes but 2 columns wide
        let seg_content = SegContent { content, seg };

        assert_eq!(seg_content.width(), width(2));
    }

    #[test]
    fn test_start_col() {
        let content = "b";
        let seg = create_test_seg(1, 2, 1, 1, 5); // Character at column 5
        let seg_content = SegContent { content, seg };

        assert_eq!(seg_content.start_col(), col(5));
    }

    #[test]
    fn test_seg_reference() {
        let content = "test";
        let seg = create_test_seg(0, 4, 4, 0, 0);
        let seg_content = SegContent { content, seg };

        let seg_ref = seg_content.seg();
        assert_eq!(seg_ref.start_byte_index, byte_index(0));
        assert_eq!(seg_ref.end_byte_index, byte_index(4));
        assert_eq!(seg_ref.display_width, width(4));
    }

    #[test]
    fn test_byte_range() {
        let content = "hello";
        let seg = create_test_seg(5, 10, 5, 1, 3);
        let seg_content = SegContent { content, seg };

        let range = seg_content.byte_range();
        assert_eq!(range.start, byte_index(5));
        assert_eq!(range.end, byte_index(10));
        assert_eq!(range, byte_index(5)..byte_index(10));
    }

    #[test]
    fn test_emoji_segment() {
        let content = "😀"; // 4-byte emoji, 2 columns wide
        let seg = create_test_seg(0, 4, 2, 0, 0);
        let seg_content = SegContent { content, seg };

        assert_eq!(seg_content.as_str(), "😀");
        assert_eq!(seg_content.width(), width(2));
        assert_eq!(seg_content.start_col(), col(0));

        let range = seg_content.byte_range();
        assert_eq!(range.start, byte_index(0));
        assert_eq!(range.end, byte_index(4));
    }

    #[test]
    fn test_complex_unicode_segment() {
        let content = "👨‍👩‍👧‍👦"; // Complex family emoji
        let byte_len = content.len();
        let seg = create_test_seg(0, byte_len, 2, 0, 0); // Complex emoji typically 2 columns
        let seg_content = SegContent { content, seg };

        assert_eq!(seg_content.as_str(), content);
        assert_eq!(seg_content.width(), width(2));

        let range = seg_content.byte_range();
        assert_eq!(range.start, byte_index(0));
        assert_eq!(range.end, byte_index(byte_len));
    }

    #[test]
    fn test_zero_width_segment() {
        let content = "\u{200D}"; // Zero-width joiner
        let seg = create_test_seg(5, 8, 0, 2, 10); // Zero width but has bytes
        let seg_content = SegContent { content, seg };

        assert_eq!(seg_content.as_str(), content);
        assert_eq!(seg_content.width(), width(0));
        assert_eq!(seg_content.start_col(), col(10));

        let range = seg_content.byte_range();
        assert_eq!(range.start, byte_index(5));
        assert_eq!(range.end, byte_index(8));
    }

    #[test]
    fn test_segment_copy_semantics() {
        let content = "test";
        let seg = create_test_seg(0, 4, 4, 0, 0);
        let seg_content1 = SegContent { content, seg };
        let seg_content2 = seg_content1; // Should copy

        assert_eq!(seg_content1.as_str(), seg_content2.as_str());
        assert_eq!(seg_content1.width(), seg_content2.width());
        assert_eq!(seg_content1.byte_range(), seg_content2.byte_range());
    }

    #[test]
    fn test_segment_clone() {
        let content = "test";
        let seg = create_test_seg(0, 4, 4, 0, 0);
        let seg_content1 = SegContent { content, seg };
        let seg_content2 = seg_content1.clone();

        assert_eq!(seg_content1.as_str(), seg_content2.as_str());
        assert_eq!(seg_content1.width(), seg_content2.width());
        assert_eq!(seg_content1.byte_range(), seg_content2.byte_range());
    }

    #[test]
    fn test_debug_format() {
        let content = "debug";
        let seg = create_test_seg(0, 5, 5, 0, 0);
        let seg_content = SegContent { content, seg };

        let debug_str = format!("{:?}", seg_content);
        assert!(debug_str.contains("SegContent"));
        // The exact format may vary, but it should contain the struct name
    }

    #[test]
    fn test_lifetime_correctness() {
        // Test that SegContent correctly borrows from the string
        let content = String::from("lifetime_test");
        let seg = create_test_seg(0, 13, 13, 0, 0);

        // This should compile and work correctly
        let seg_content = SegContent {
            content: &content,
            seg
        };

        assert_eq!(seg_content.as_str(), "lifetime_test");
        assert_eq!(seg_content.byte_range(), byte_index(0)..byte_index(13));
    }

    #[test]
    fn test_byte_range_consistency() {
        let content = "range_test";
        let start = 2;
        let end = 8;
        let seg = create_test_seg(start, end, end - start, 0, 0);
        let seg_content = SegContent { content, seg };

        let range = seg_content.byte_range();
        assert_eq!(range.start.as_usize(), start);
        assert_eq!(range.end.as_usize(), end);

        // Range should be consistent with segment data
        assert_eq!(range.start, seg_content.seg.start_byte_index);
        assert_eq!(range.end, seg_content.seg.end_byte_index);
    }

    #[test]
    fn test_different_content_lengths() {
        let test_cases = [
            ("", 0, 0),         // Empty string
            ("a", 0, 1),        // Single ASCII
            ("ab", 0, 2),       // Two ASCII
            ("😀", 0, 4),       // Single emoji (4 bytes)
            ("a😀b", 1, 5),     // Mixed content
        ];

        for (content, start, end) in test_cases {
            let seg = create_test_seg(start, end, 1, 0, 0);
            let seg_content = SegContent { content, seg };

            assert_eq!(seg_content.as_str(), content);
            let range = seg_content.byte_range();
            assert_eq!(range.start.as_usize(), start);
            assert_eq!(range.end.as_usize(), end);
        }
    }
}
