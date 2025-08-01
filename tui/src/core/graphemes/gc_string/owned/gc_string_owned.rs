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

//! [`GCStringOwned`] implementation for owned Unicode grapheme cluster strings.

use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

use crate::{ChUnit, ColIndex, ColWidth, InlineString, Seg, SegIndex, SegWidth, SegmentArray,
            graphemes::unicode_segment::{build_segments_for_str, calculate_display_width}};

/// Wide segments detection result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainsWideSegments {
    Yes,
    No,
}

/// Convenience constructor for `GCStringOwned`.
pub fn gc_string_owned(arg_from: impl Into<GCStringOwned>) -> GCStringOwned {
    arg_from.into()
}

/// Owned version of a Unicode grapheme cluster string with pre-computed segment metadata.
/// 
/// This type owns both the string data and the grapheme cluster metadata, making it suitable
/// for cases where the string needs to be stored or passed around independently.
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
    fn as_ref(&self) -> &str {
        self.string.as_str()
    }
}

impl From<&str> for GCStringOwned {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for GCStringOwned {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<InlineString> for GCStringOwned {
    fn from(value: InlineString) -> Self {
        Self::new(value.as_str())
    }
}

impl<'a> IntoIterator for &'a GCStringOwned {
    type Item = Seg;
    type IntoIter = std::iter::Copied<std::slice::Iter<'a, Seg>>;

    fn into_iter(self) -> Self::IntoIter {
        self.segments.iter().copied()
    }
}

impl From<&InlineString> for GCStringOwned {
    fn from(value: &InlineString) -> Self {
        Self::new(value.as_str())
    }
}

impl From<&'_ &str> for GCStringOwned {
    fn from(value: &'_ &str) -> Self {
        Self::new(*value)
    }
}

impl From<&String> for GCStringOwned {
    fn from(value: &String) -> Self {
        Self::new(value.as_str())
    }
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
    pub fn as_str(&self) -> &str {
        self.string.as_str()
    }

    /// Get the display width of the string.
    #[must_use]
    pub fn display_width(&self) -> ColWidth {
        self.display_width
    }

    /// Get the byte size of the string.
    #[must_use]
    pub fn bytes_size(&self) -> ChUnit {
        self.bytes_size
    }

    /// Get the number of grapheme clusters.
    #[must_use]
    pub fn len(&self) -> SegWidth {
        SegWidth::from(self.segments.len())
    }

    /// Check if the string is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

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
    pub fn iter(&self) -> impl Iterator<Item = Seg> + '_ {
        self.segments.iter().copied()
    }

    /// Get display width of a single character (utility method).
    #[must_use]
    pub fn width_char(ch: char) -> ColWidth {
        use unicode_width::UnicodeWidthChar;
        ColWidth::from(UnicodeWidthChar::width(ch).unwrap_or(0))
    }

    /// Check if this string contains wide segments (characters wider than 1 column).
    #[must_use]
    pub fn contains_wide_segments(&self) -> ContainsWideSegments {
        if self.segments.iter().any(|seg| seg.display_width > crate::width(1)) {
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
    pub fn width(&self) -> ColWidth {
        self.display_width
    }

    /// Get the last segment.
    #[must_use]
    pub fn last(&self) -> Option<Seg> {
        self.segments.last().copied()
    }
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

// Include existing submodules
pub use super::gc_string_owned_non_editor_impl::*;
pub use super::gc_string_owned_editor_impl::*;