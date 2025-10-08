// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`GCStringOwned`] implementation for owned Unicode grapheme cluster strings.

use crate::{ChUnit, ColIndex, ColWidth, CowInlineString, GraphemeString,
            GraphemeStringMut, InlineString, Seg, SegContent, SegIndex, SegLength,
            SegmentArray,
            graphemes::unicode_segment::{build_segments_for_str,
                                         calculate_display_width}};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

/// Wide segments detection result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainsWideSegments {
    Yes,
    No,
}

/// Owned version of a Unicode grapheme cluster string with pre-computed segment metadata.
///
/// This type owns both the string data and the grapheme cluster metadata, making it
/// suitable for cases where the string needs to be stored or passed around independently.
#[derive(Clone, PartialEq, Eq)]
pub struct GCStringOwned {
    /// The underlying string data (owned).
    pub string: InlineString,
    /// Pre-computed grapheme cluster segments.
    pub segments: SegmentArray,
    /// Display width of the entire string.
    pub display_width: ColWidth,
    /// Byte size of the string.
    pub bytes_size: ChUnit,
}

impl Debug for GCStringOwned {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "GCStringOwned({:?})", self.string.as_str())
    }
}

impl Display for GCStringOwned {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.string.as_str())
    }
}

impl AsRef<str> for GCStringOwned {
    fn as_ref(&self) -> &str { self.string.as_str() }
}

impl From<&str> for GCStringOwned {
    fn from(value: &str) -> Self { Self::new(value) }
}

impl From<String> for GCStringOwned {
    fn from(value: String) -> Self { Self::new(value) }
}

impl From<InlineString> for GCStringOwned {
    fn from(value: InlineString) -> Self { Self::new(value.as_str()) }
}

impl<'a> IntoIterator for &'a GCStringOwned {
    type Item = Seg;
    type IntoIter = std::iter::Copied<std::slice::Iter<'a, Seg>>;

    fn into_iter(self) -> Self::IntoIter { self.segments.iter().copied() }
}

impl From<&InlineString> for GCStringOwned {
    fn from(value: &InlineString) -> Self { Self::new(value.as_str()) }
}

impl From<&'_ &str> for GCStringOwned {
    fn from(value: &'_ &str) -> Self { Self::new(*value) }
}

impl From<&String> for GCStringOwned {
    fn from(value: &String) -> Self { Self::new(value.as_str()) }
}

impl GCStringOwned {
    /// Create a new `GCStringOwned` from a string, computing grapheme cluster segments.
    pub fn new(input: impl AsRef<str>) -> Self {
        let string: InlineString = input.as_ref().into();
        let segments = build_segments_for_str(string.as_str());
        let display_width = calculate_display_width(&segments);
        let bytes_size = ChUnit::from(string.len());

        Self {
            string,
            segments,
            display_width,
            bytes_size,
        }
    }

    /// Get the string as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str { self.string.as_str() }

    /// Get the display width of the string.
    #[must_use]
    pub fn display_width(&self) -> ColWidth { self.display_width }

    /// Get the byte size of the string.
    #[must_use]
    pub fn bytes_size(&self) -> ChUnit { self.bytes_size }

    /// Get the number of grapheme clusters.
    #[must_use]
    pub fn len(&self) -> SegLength { self.segments.len().into() }

    /// Get the number of grapheme cluster segments.
    /// This is the preferred method for semantic clarity.
    #[must_use]
    pub fn segment_count(&self) -> SegLength { self.segments.len().into() }

    /// Check if the string is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.segments.is_empty() }

    /// Get a segment by index.
    pub fn get(&self, seg_index: impl Into<SegIndex>) -> Option<Seg> {
        let index = seg_index.into().as_usize();
        self.segments.get(index).copied()
    }

    /// Get the maximum segment index.
    #[must_use]
    pub fn get_max_seg_index(&self) -> SegIndex {
        if self.segments.is_empty() {
            SegIndex::from(0)
        } else {
            SegIndex::from(self.segments.len() - 1)
        }
    }

    /// Iterate over segments.
    pub fn iter(&self) -> impl Iterator<Item = Seg> + '_ { self.segments.iter().copied() }

    /// Get display width of a single character (utility method).
    #[must_use]
    pub fn width_char(ch: char) -> ColWidth {
        use unicode_width::UnicodeWidthChar;
        ColWidth::from(UnicodeWidthChar::width(ch).unwrap_or(0))
    }

    /// Check if this string contains wide segments (characters wider than 1 column).
    #[must_use]
    pub fn contains_wide_segments(&self) -> ContainsWideSegments {
        if self
            .segments
            .iter()
            .any(|seg| seg.display_width > crate::width(1))
        {
            ContainsWideSegments::Yes
        } else {
            ContainsWideSegments::No
        }
    }

    /// Iterate over the segments.
    pub fn seg_iter(&self) -> impl Iterator<Item = Seg> + '_ {
        self.segments.iter().copied()
    }

    /// Get the display width of the string (alias for `display_width()`).
    #[must_use]
    pub fn width(&self) -> ColWidth { self.display_width }

    /// Get the last segment.
    #[must_use]
    pub fn last(&self) -> Option<Seg> { self.segments.last().copied() }
}

/// Result type for string operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegStringOwned {
    /// The grapheme cluster slice as `GCStringOwned` (owns both string and segments).
    pub string: GCStringOwned,
    /// The display width of the slice.
    pub width: ColWidth,
    /// The display col index at which this grapheme cluster starts.
    pub start_at: ColIndex,
}

// GraphemeString trait implementation for GCStringOwned.
impl GraphemeString for GCStringOwned {
    type SegmentIterator<'a> = std::iter::Copied<std::slice::Iter<'a, Seg>>;
    type StringSlice<'a> = CowInlineString<'a>;

    fn as_str(&self) -> &str { self.as_str() }

    fn segments(&self) -> &[Seg] { &self.segments }

    fn display_width(&self) -> ColWidth { self.display_width }

    fn segment_count(&self) -> SegLength { self.segment_count() }

    fn byte_size(&self) -> ChUnit { self.bytes_size }

    fn get_seg(&self, index: SegIndex) -> Option<Seg> { self.get(index) }

    fn check_is_in_middle_of_grapheme(&self, col: ColIndex) -> Option<Seg> {
        self.check_is_in_middle_of_grapheme(col)
    }

    fn get_seg_at(&self, col: ColIndex) -> Option<SegContent<'_>> {
        self.get_string_at(col).and_then(|seg_string| {
            self.segments
                .iter()
                .find(|seg| seg.start_display_col_index == seg_string.start_at)
                .map(|seg| SegContent {
                    content: seg.get_str(self),
                    seg: *seg,
                })
        })
    }

    fn get_seg_right_of(&self, col: ColIndex) -> Option<SegContent<'_>> {
        self.get_string_at_right_of(col).and_then(|seg_string| {
            self.segments
                .iter()
                .find(|seg| seg.start_display_col_index == seg_string.start_at)
                .map(|seg| SegContent {
                    content: seg.get_str(self),
                    seg: *seg,
                })
        })
    }

    fn get_seg_left_of(&self, col: ColIndex) -> Option<SegContent<'_>> {
        self.get_string_at_left_of(col).and_then(|seg_string| {
            self.segments
                .iter()
                .find(|seg| seg.start_display_col_index == seg_string.start_at)
                .map(|seg| SegContent {
                    content: seg.get_str(self),
                    seg: *seg,
                })
        })
    }

    fn get_seg_at_end(&self) -> Option<SegContent<'_>> {
        self.last().map(|seg| SegContent {
            content: seg.get_str(self),
            seg,
        })
    }

    fn clip(&self, start_col: ColIndex, width: ColWidth) -> Self::StringSlice<'_> {
        CowInlineString::Borrowed(self.clip(start_col, width))
    }

    fn trunc_end_to_fit(&self, width: ColWidth) -> Self::StringSlice<'_> {
        CowInlineString::Borrowed(self.trunc_end_to_fit(width))
    }

    fn trunc_end_by(&self, width: ColWidth) -> Self::StringSlice<'_> {
        CowInlineString::Borrowed(self.trunc_end_by(width))
    }

    fn trunc_start_by(&self, width: ColWidth) -> Self::StringSlice<'_> {
        CowInlineString::Borrowed(self.trunc_start_by(width))
    }

    fn segments_iter(&self) -> Self::SegmentIterator<'_> { self.segments.iter().copied() }

    fn is_empty(&self) -> bool { self.is_empty() }

    fn last(&self) -> Option<Seg> { self.last() }

    fn contains_wide_segments(&self) -> ContainsWideSegments {
        self.contains_wide_segments()
    }
}

// GraphemeStringMut trait implementation for GCStringOwned.
impl GraphemeStringMut for GCStringOwned {
    type MutResult = GCStringOwned; // Returns new instances (immutable paradigm)

    fn insert_text(&mut self, col: ColIndex, text: &str) -> Option<Self::MutResult> {
        // Create a new string with text inserted at the column.
        let (new_string, _width) = self.insert_chunk_at_col(col, text);
        Some(GCStringOwned::new(new_string))
    }

    fn delete_range(
        &mut self,
        start: ColIndex,
        end: ColIndex,
    ) -> Option<Self::MutResult> {
        // Split at start position.
        if let Some((left, _)) = self.split_at_display_col(start) {
            let left_string = GCStringOwned::new(left);

            // Split at end position to get the part after.
            if let Some((_, right)) = self.split_at_display_col(end) {
                // Combine left and right parts.
                let combined = format!("{}{}", left_string.as_str(), right);
                Some(GCStringOwned::new(combined))
            } else {
                // Nothing after end, just return the left part.
                Some(left_string)
            }
        } else {
            None
        }
    }

    fn replace_range(
        &mut self,
        start: ColIndex,
        end: ColIndex,
        text: &str,
    ) -> Option<Self::MutResult> {
        // First delete the range.
        self.delete_range(start, end).and_then(|deleted| {
            // Then insert the new text at the start position.
            let mut temp = deleted;
            temp.insert_text(start, text)
        })
    }

    fn truncate(&mut self, col: ColIndex) -> Option<Self::MutResult> {
        // Split at the column and return the left part.
        if let Some((left, _)) = self.split_at_display_col(col) {
            Some(GCStringOwned::new(left))
        } else {
            None
        }
    }
}

impl From<(Seg, &GCStringOwned)> for SegStringOwned {
    fn from((seg, gc_string): (Seg, &GCStringOwned)) -> Self {
        let seg_str = seg.get_str(gc_string);
        Self {
            string: GCStringOwned::new(seg_str),
            width: seg.display_width,
            start_at: seg.start_display_col_index,
        }
    }
}
