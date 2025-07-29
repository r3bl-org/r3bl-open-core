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

use std::fmt::Debug;

use super::{gc_string_owned::wide_segments::ContainsWideSegments,
            segment_builder::{build_segments_for_str, calculate_display_width}};
use crate::{ChUnit, ColIndex, ColWidth, GCString, Seg, SegIndex, SegWidth, ch,
            gc_string_owned_sizing::SegmentArray, seg_width, width};

/// Borrowed version of `GCStringOwned` that doesn't own the string data
/// but owns its segment metadata. Used for efficient operations with
/// borrowed string content while maintaining grapheme cluster information.
///
/// This type is particularly useful when working with `ZeroCopyGapBuffer`
/// and `GapBufferLineInfo`, where the string data is borrowed from the
/// buffer but we need to maintain segment information for grapheme operations.
///
/// # Ownership Model
///
/// - **String data**: Borrowed (`&'a str`) - no allocation for string content
/// - **Segment metadata**: Owned (`SegmentArray`) - computed or reused from existing data
///
/// # Example Usage
///
/// ```rust
/// use r3bl_tui::GCStringRef;
///
/// // From arbitrary string (computes segments)
/// let gc_ref = GCStringRef::new("Hello 🙏🏽 World");
///
/// // From ZeroCopyGapBuffer (reuses pre-computed segments)
/// let (content, info) = buffer.get_line_with_info(row_index)?;
/// let gc_ref = GCStringRef::from_gap_buffer_line(content, info);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GCStringRef<'a> {
    /// Borrowed string content (does NOT own the string data)
    string: &'a str,
    /// Owned segment array - computed from the borrowed string
    segments: SegmentArray,
    /// Display width (computed from segments)
    display_width: ColWidth,
    /// Byte size (derived from string length)
    bytes_size: ChUnit,
}

/// Borrowed version of `SegStringOwned` for use with `GCStringRef`.
/// This avoids unnecessary allocations when working with borrowed string data.
#[derive(Debug, PartialEq, Eq)]
pub struct SegStringRef<'a> {
    /// The grapheme cluster slice as `GCStringRef` (owns segments, borrows string)
    pub string: GCStringRef<'a>,
    /// The display width of the slice
    pub width: ColWidth,
    /// The display col index at which this grapheme cluster starts
    pub start_at: ColIndex,
}

/// Convenience constructor for `GCStringRef` (similar to `gc_string_owned`)
pub fn gc_string_ref<'a>(arg_from: impl Into<GCStringRef<'a>>) -> GCStringRef<'a> {
    arg_from.into()
}

// ╭─────────────────────────────────────────────────────────────────────────────╮
// │                              Constructors                                   │
// ╰─────────────────────────────────────────────────────────────────────────────╯

impl<'a> GCStringRef<'a> {
    /// Create `GCStringRef` from borrowed string, computing segments.
    ///
    /// This constructor computes the grapheme cluster segments from scratch,
    /// which has some computational cost but provides maximum flexibility.
    ///
    /// # Performance Note
    ///
    /// If you're working with `ZeroCopyGapBuffer` data, prefer
    /// `from_gap_buffer_line()` which can reuse pre-computed segments.
    #[must_use]
    pub fn new(string: &'a str) -> Self {
        let segments = build_segments_for_str(string);
        let display_width = calculate_display_width(&segments);
        let bytes_size = ChUnit::from(string.len());

        Self {
            string,
            segments,
            display_width,
            bytes_size,
        }
    }

    /// Create `GCStringRef` from `GapBufferLineInfo`, reusing existing segments.
    ///
    /// This is the most efficient constructor when working with `ZeroCopyGapBuffer`
    /// as it reuses the pre-computed segment information from `GapBufferLineInfo`.
    ///
    /// # Arguments
    ///
    /// * `content` - Borrowed string content from the gap buffer
    /// * `info` - Line metadata containing pre-computed segments
    #[must_use]
    pub fn from_gap_buffer_line(
        content: &'a str,
        info: &crate::GapBufferLineInfo,
    ) -> Self {
        Self {
            string: content,
            segments: info.segments.clone(), // Efficient SmallVec clone
            display_width: info.display_width,
            bytes_size: ChUnit::from(content.len()),
        }
    }

    /// Create `GCStringRef` with pre-computed segments (for advanced use cases).
    ///
    /// This constructor is useful when you already have computed segments
    /// and want to avoid recomputation.
    #[must_use]
    pub fn with_segments(
        string: &'a str,
        segments: SegmentArray,
        display_width: ColWidth,
    ) -> Self {
        let bytes_size = ChUnit::from(string.len());

        Self {
            string,
            segments,
            display_width,
            bytes_size,
        }
    }
}

// ╭─────────────────────────────────────────────────────────────────────────────╮
// │                            GCString Trait Impl                              │
// ╰─────────────────────────────────────────────────────────────────────────────╯

impl<'a> GCString for GCStringRef<'a> {
    type StringResult = SegStringRef<'a>;

    fn len(&self) -> SegWidth { self.segments.len().into() }

    fn is_empty(&self) -> bool { self.len() == seg_width(0) }

    fn get_max_seg_index(&self) -> SegIndex { self.len().convert_to_seg_index() }

    fn get(&self, seg_index: impl Into<SegIndex>) -> Option<Seg> {
        let index = seg_index.into().as_usize();
        self.segments.get(index).copied()
    }

    fn seg_iter(&self) -> Box<dyn Iterator<Item = &Seg> + '_> {
        Box::new(self.segments.iter())
    }

    fn iter(&self) -> Box<dyn Iterator<Item = Seg> + '_> {
        Box::new(self.segments.iter().copied())
    }

    fn as_str(&self) -> &str { self.string }

    fn display_width(&self) -> ColWidth { self.display_width }

    fn bytes_size(&self) -> ChUnit { self.bytes_size }

    fn contains_wide_segments(&self) -> ContainsWideSegments {
        if self.segments.iter().any(|seg| seg.display_width > width(1)) {
            ContainsWideSegments::Yes
        } else {
            ContainsWideSegments::No
        }
    }

    fn trunc_end_to_fit(&self, arg_col_width: impl Into<ColWidth>) -> &str {
        let mut avail_cols: ColWidth = arg_col_width.into();
        let mut string_end_byte_index = 0;

        for seg in self.seg_iter() {
            let seg_display_width = seg.display_width;
            if avail_cols < seg_display_width {
                break;
            }
            string_end_byte_index += seg.bytes_size;
            avail_cols -= seg_display_width;
        }

        &self.string[..string_end_byte_index]
    }

    fn trunc_end_by(&self, arg_col_width: impl Into<ColWidth>) -> &str {
        let mut countdown_col_count: ColWidth = arg_col_width.into();
        let mut string_end_byte_index = ch(0);

        let rev_iter = self.segments.iter().rev();

        for seg in rev_iter {
            let seg_display_width = seg.display_width;
            string_end_byte_index = seg.start_byte_index;
            countdown_col_count -= seg_display_width;
            if *countdown_col_count == ch(0) {
                // We are done skipping.
                break;
            }
        }

        &self.string[..string_end_byte_index.as_usize()]
    }

    fn trunc_start_by(&self, arg_col_width: impl Into<ColWidth>) -> &str {
        let mut skip_col_count: ColWidth = arg_col_width.into();
        let mut string_start_byte_index = 0;

        for segment in self.seg_iter() {
            let seg_display_width = segment.display_width;
            if *skip_col_count == ch(0) {
                // We are done skipping.
                break;
            }

            // Skip segment.unicode_width.
            skip_col_count -= seg_display_width;
            string_start_byte_index += segment.bytes_size;
        }

        &self.string[string_start_byte_index..]
    }

    fn get_string_at(
        &self,
        arg_col_index: impl Into<ColIndex>,
    ) -> Option<Self::StringResult> {
        let target_col = arg_col_index.into();

        for seg in &self.segments {
            let seg_start = seg.start_display_col_index;
            let seg_end = seg_start + seg.display_width;

            if target_col >= seg_start && target_col < seg_end {
                let start_byte = seg.start_byte_index.as_usize();
                let end_byte = seg.end_byte_index.as_usize();
                let slice = &self.string[start_byte..end_byte];

                let gc_ref = GCStringRef::new(slice);
                return Some(SegStringRef {
                    string: gc_ref,
                    width: seg.display_width,
                    start_at: seg.start_display_col_index,
                });
            }
        }

        None
    }

    fn get_string_at_right_of(
        &self,
        arg_col_index: impl Into<ColIndex>,
    ) -> Option<Self::StringResult> {
        let target_col = arg_col_index.into();

        for seg in &self.segments {
            let seg_start = seg.start_display_col_index;
            if seg_start > target_col {
                let start_byte = seg.start_byte_index.as_usize();
                let slice = &self.string[start_byte..];

                let gc_ref = GCStringRef::new(slice);
                let width = self.display_width - ColWidth::from(seg_start.as_u16());

                return Some(SegStringRef {
                    string: gc_ref,
                    width,
                    start_at: seg_start,
                });
            }
        }

        None
    }

    fn get_string_at_left_of(
        &self,
        arg_col_index: impl Into<ColIndex>,
    ) -> Option<Self::StringResult> {
        let target_col = arg_col_index.into();
        let mut end_byte = 0;
        let mut width = ColWidth::from(0);

        for seg in &self.segments {
            let seg_start = seg.start_display_col_index;
            if seg_start >= target_col {
                break;
            }
            end_byte = seg.end_byte_index.as_usize();
            width = ColWidth::from(seg_start.as_u16()) + seg.display_width;
        }

        if end_byte > 0 {
            let slice = &self.string[..end_byte];
            let gc_ref = GCStringRef::new(slice);

            Some(SegStringRef {
                string: gc_ref,
                width,
                start_at: ColIndex::from(0),
            })
        } else {
            None
        }
    }

    fn get_string_at_end(&self) -> Option<Self::StringResult> {
        if let Some(last_seg) = self.segments.last() {
            let start_byte = last_seg.start_byte_index.as_usize();
            let end_byte = last_seg.end_byte_index.as_usize();
            let slice = &self.string[start_byte..end_byte];

            let gc_ref = GCStringRef::new(slice);
            Some(SegStringRef {
                string: gc_ref,
                width: last_seg.display_width,
                start_at: last_seg.start_display_col_index,
            })
        } else {
            None
        }
    }
}

// ╭─────────────────────────────────────────────────────────────────────────────╮
// │                              Conversion Traits                              │
// ╰─────────────────────────────────────────────────────────────────────────────╯

impl<'a> From<&'a str> for GCStringRef<'a> {
    fn from(string: &'a str) -> Self { Self::new(string) }
}

impl<'a> From<(&'a str, &crate::GapBufferLineInfo)> for GCStringRef<'a> {
    fn from((content, info): (&'a str, &crate::GapBufferLineInfo)) -> Self {
        Self::from_gap_buffer_line(content, info)
    }
}

impl<'a> From<(Seg, &'a GCStringRef<'a>)> for SegStringRef<'a> {
    fn from((seg, gc_ref): (Seg, &'a GCStringRef<'a>)) -> Self {
        let start_byte = seg.start_byte_index.as_usize();
        let end_byte = seg.end_byte_index.as_usize();
        let slice = &gc_ref.string[start_byte..end_byte];

        let string = GCStringRef::new(slice);

        Self {
            string,
            width: seg.display_width,
            start_at: seg.start_display_col_index,
        }
    }
}

// ╭─────────────────────────────────────────────────────────────────────────────╮
// │                                   Tests                                     │
// ╰─────────────────────────────────────────────────────────────────────────────╯

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GCStringOwned, col};

    #[test]
    fn test_basic_construction() {
        let text = "Hello, 🙏🏽 World!";
        let gc_ref = GCStringRef::new(text);

        assert_eq!(gc_ref.as_str(), text);
        assert!(!gc_ref.is_empty());
        assert_eq!(gc_ref.bytes_size(), ChUnit::from(text.len()));
    }

    #[test]
    fn test_segments_match_owned_version() {
        let text = "Hello, 🙏🏽 World!";
        let gc_ref = GCStringRef::new(text);
        let gc_owned = GCStringOwned::new(text);

        // Should have same number of segments
        assert_eq!(gc_ref.len(), gc_owned.len());
        assert_eq!(gc_ref.display_width(), gc_owned.display_width());

        // Each segment should match
        for i in 0..gc_ref.len().as_usize() {
            let ref_seg = gc_ref.get(i).unwrap();
            let owned_seg = gc_owned.get(i).unwrap();
            assert_eq!(ref_seg, owned_seg);
        }
    }

    #[test]
    fn test_string_slicing() {
        let text = "Hello, 🙏🏽 World!";
        let gc_ref = GCStringRef::new(text);

        // Test get_string_at
        if let Some(seg_str) = gc_ref.get_string_at(col(0)) {
            assert_eq!(seg_str.string.as_str(), "H");
        }

        // Test get_string_at_end
        if let Some(seg_str) = gc_ref.get_string_at_end() {
            assert_eq!(seg_str.string.as_str(), "!");
        }
    }

    #[test]
    fn test_truncation() {
        let text = "Hello, World!";
        let gc_ref = GCStringRef::new(text);

        let truncated = gc_ref.trunc_end_to_fit(ColWidth::from(5));
        assert_eq!(truncated, "Hello");

        let truncated_by = gc_ref.trunc_end_by(ColWidth::from(8));
        assert_eq!(truncated_by, "Hello");
    }

    #[test]
    fn test_wide_segments() {
        let text_with_emoji = "Hello 🙏🏽";
        let gc_ref = GCStringRef::new(text_with_emoji);

        assert_eq!(gc_ref.contains_wide_segments(), ContainsWideSegments::Yes);

        let text_ascii = "Hello World";
        let gc_ref_ascii = GCStringRef::new(text_ascii);

        assert_eq!(
            gc_ref_ascii.contains_wide_segments(),
            ContainsWideSegments::No
        );
    }

    #[test]
    fn test_empty_string() {
        let gc_ref = GCStringRef::new("");

        assert!(gc_ref.is_empty());
        assert_eq!(gc_ref.len(), seg_width(0));
        assert_eq!(gc_ref.display_width(), ColWidth::from(0));
        assert_eq!(gc_ref.get_string_at_end(), None);
    }

    #[test]
    fn test_conversion_traits() {
        let text = "Hello, World!";

        // Test From<&str>
        let gc_ref: GCStringRef = text.into();
        assert_eq!(gc_ref.as_str(), text);

        // Test gc_string_ref convenience function
        let gc_ref2 = gc_string_ref(text);
        assert_eq!(gc_ref2.as_str(), text);
    }
}
