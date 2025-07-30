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

use super::super::owned::gc_string_owned::wide_segments::ContainsWideSegments;
use crate::{ChUnit, ColIndex, ColWidth, Seg, SegIndex, SegWidth, ch, seg_width, width};

/// Trait for accessing the underlying data needed for [`crate::GCString`] operations.
///
/// This abstraction allows the same logic to work with both [`crate::GCStringOwned`] and
/// [`crate::GCStringRef`] without duplicating the implementation details; it does not
/// care about ownership model. This trait
/// has no connection with [`crate::GCString`] trait, which very much cares about the
/// ownership model, and is the public API.
///
/// This module provides shared functionality that can be used by both
/// [`crate::GCStringOwned`] and [`crate::GCStringRef`] implementations, reducing code
/// duplication and ensuring consistent behavior across different grapheme string types.
/// See [`crate::gc_string`] for more details on this two trait design.
///
/// The functions in this module operate on any type that provides access to the
/// necessary string data and segment information through the [`crate::GCStringData`]
/// trait.
///
/// ## Key benefits
///
/// - Implementation-agnostic - doesn't care about ownership model
/// - Low-level data access - just getters for raw data
/// - No business logic - pure data retrieval
/// - Enables code reuse - allows shared algorithms in this module
///   [`crate::gc_string::common`]
pub trait GCStringData {
    /// Returns a reference to the underlying string.
    fn string_data(&self) -> &str;

    /// Returns an iterator over the segments.
    fn segments_iter(&self) -> impl DoubleEndedIterator<Item = &Seg>;

    /// Returns the display width of the string.
    fn display_width(&self) -> ColWidth;

    /// Returns the byte size of the string.
    fn bytes_size(&self) -> ChUnit;

    /// Returns the number of segments.
    fn segments_len(&self) -> usize;

    /// Returns a segment at the given index.
    fn get_segment(&self, index: usize) -> Option<&Seg>;
}

// ╭─────────────────────────────────────────────────────────────────────────────╮
// │                           Basic Trait Methods                               │
// ╰─────────────────────────────────────────────────────────────────────────────╯

/// Returns the number of grapheme clusters (segments) in the string.
pub fn gc_len<T: GCStringData>(data: &T) -> SegWidth { data.segments_len().into() }

/// Returns true if the string contains no grapheme clusters.
pub fn gc_is_empty<T: GCStringData>(data: &T) -> bool { gc_len(data) == seg_width(0) }

/// Returns the maximum segment index of the string.
pub fn gc_get_max_seg_index<T: GCStringData>(data: &T) -> SegIndex {
    gc_len(data).convert_to_seg_index()
}

/// Gets a segment at the given index.
pub fn gc_get<T: GCStringData>(data: &T, seg_index: impl Into<SegIndex>) -> Option<Seg> {
    let index = seg_index.into().as_usize();
    data.get_segment(index).copied()
}

/// Returns the underlying string as a string slice.
pub fn gc_as_str<T: GCStringData>(data: &T) -> &str { data.string_data() }

/// Returns the display width of the string.
pub fn gc_display_width<T: GCStringData>(data: &T) -> ColWidth { data.display_width() }

/// Returns the byte size of the underlying string.
pub fn gc_bytes_size<T: GCStringData>(data: &T) -> ChUnit { data.bytes_size() }

// ╭─────────────────────────────────────────────────────────────────────────────╮
// │                         Wide Segment Detection                              │
// ╰─────────────────────────────────────────────────────────────────────────────╯

/// Checks if the string contains any wide segments (characters wider than 1 column).
pub fn gc_contains_wide_segments<T: GCStringData>(data: &T) -> ContainsWideSegments {
    if data.segments_iter().any(|seg| seg.display_width > width(1)) {
        ContainsWideSegments::Yes
    } else {
        ContainsWideSegments::No
    }
}

// ╭─────────────────────────────────────────────────────────────────────────────╮
// │                           Truncation Methods                                │
// ╰─────────────────────────────────────────────────────────────────────────────╯

/// Truncate at the end to fit within the given column width.
pub fn gc_trunc_end_to_fit<T: GCStringData>(
    data: &T,
    arg_col_width: impl Into<ColWidth>,
) -> &str {
    let mut avail_cols: ColWidth = arg_col_width.into();
    let mut string_end_byte_index = 0;

    for seg in data.segments_iter() {
        let seg_display_width = seg.display_width;
        if avail_cols < seg_display_width {
            break;
        }
        string_end_byte_index += seg.bytes_size;
        avail_cols -= seg_display_width;
    }

    &data.string_data()[..string_end_byte_index]
}

/// Truncate at the end by the given column width.
pub fn gc_trunc_end_by<T: GCStringData>(
    data: &T,
    arg_col_width: impl Into<ColWidth>,
) -> &str {
    let mut countdown_col_count: ColWidth = arg_col_width.into();
    let mut string_end_byte_index = ch(0);

    let rev_iter = data.segments_iter().rev();

    for seg in rev_iter {
        let seg_display_width = seg.display_width;
        string_end_byte_index = seg.start_byte_index;
        countdown_col_count -= seg_display_width;
        if *countdown_col_count == ch(0) {
            // We are done skipping.
            break;
        }
    }

    &data.string_data()[..string_end_byte_index.as_usize()]
}

/// Truncate at the start by the given column width.
pub fn gc_trunc_start_by<T: GCStringData>(
    data: &T,
    arg_col_width: impl Into<ColWidth>,
) -> &str {
    let mut skip_col_count: ColWidth = arg_col_width.into();
    let mut string_start_byte_index = 0;

    for segment in data.segments_iter() {
        let seg_display_width = segment.display_width;
        if *skip_col_count == ch(0) {
            // We are done skipping.
            break;
        }

        // Skip segment.unicode_width.
        skip_col_count -= seg_display_width;
        string_start_byte_index += segment.bytes_size;
    }

    &data.string_data()[string_start_byte_index..]
}

// ╭─────────────────────────────────────────────────────────────────────────────╮
// │                         String Slicing Helpers                              │
// ╰─────────────────────────────────────────────────────────────────────────────╯

/// Find the segment that contains or starts at the given column index.
pub fn find_segment_at_col<T: GCStringData>(
    data: &T,
    target_col: ColIndex,
) -> Option<&Seg> {
    for seg in data.segments_iter() {
        let seg_start = seg.start_display_col_index;
        let seg_end = seg_start + seg.display_width;

        if target_col >= seg_start && target_col < seg_end {
            return Some(seg);
        }
    }
    None
}

/// Find the first segment that starts after the given column index.
pub fn find_segment_right_of_col<T: GCStringData>(
    data: &T,
    target_col: ColIndex,
) -> Option<&Seg> {
    for seg in data.segments_iter() {
        let seg_start = seg.start_display_col_index;
        if seg_start > target_col {
            return Some(seg);
        }
    }
    None
}

/// Find the byte index that represents the end of content left of the given column.
pub fn find_end_byte_left_of_col<T: GCStringData>(
    data: &T,
    target_col: ColIndex,
) -> Option<usize> {
    let mut end_byte = 0;

    for seg in data.segments_iter() {
        let seg_start = seg.start_display_col_index;
        if seg_start >= target_col {
            break;
        }
        end_byte = seg.end_byte_index.as_usize();
    }

    if end_byte > 0 { Some(end_byte) } else { None }
}

/// Get the last segment in the string.
pub fn get_last_segment<T: GCStringData>(data: &T) -> Option<&Seg> {
    data.segments_iter().last()
}

/// Calculate the display width from a starting column to the end.
pub fn calculate_width_from_col<T: GCStringData>(
    data: &T,
    start_col: ColIndex,
) -> ColWidth {
    data.display_width() - ColWidth::from(start_col.as_u16())
}

/// Calculate the display width up to a given column position.
pub fn calculate_width_up_to_col<T: GCStringData>(
    data: &T,
    target_col: ColIndex,
) -> ColWidth {
    let mut width = ColWidth::from(0);

    for seg in data.segments_iter() {
        let seg_start = seg.start_display_col_index;
        if seg_start >= target_col {
            break;
        }
        width = ColWidth::from(seg_start.as_u16()) + seg.display_width;
    }

    width
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{col, graphemes::gc_string::owned::GCStringOwned};

    // The GCStringData implementation for GCStringOwned is in gc_string_owned.rs

    #[test]
    fn test_basic_functions() {
        let gc = GCStringOwned::new("Hello 🙏🏽 World!");

        assert_eq!(gc_len(&gc).as_usize(), 14);
        assert!(!gc_is_empty(&gc));
        assert_eq!(gc_as_str(&gc), "Hello 🙏🏽 World!");
        assert_eq!(gc_display_width(&gc), width(15));
    }

    #[test]
    fn test_truncation() {
        let gc = GCStringOwned::new("Hello 🙏🏽 World!");

        let truncated = gc_trunc_end_to_fit(&gc, width(7));
        assert_eq!(truncated, "Hello ");

        let truncated_by = gc_trunc_end_by(&gc, width(8));
        assert_eq!(truncated_by, "Hello ");
    }

    #[test]
    fn test_wide_segments() {
        let gc_with_emoji = GCStringOwned::new("Hello 🙏🏽");
        assert_eq!(
            gc_contains_wide_segments(&gc_with_emoji),
            ContainsWideSegments::Yes
        );

        let gc_ascii = GCStringOwned::new("Hello World");
        assert_eq!(
            gc_contains_wide_segments(&gc_ascii),
            ContainsWideSegments::No
        );
    }

    #[test]
    fn test_segment_finding() {
        let gc = GCStringOwned::new("Hi📦");

        // Test finding segment at specific column
        let seg_at_0 = find_segment_at_col(&gc, col(0));
        assert!(seg_at_0.is_some());

        // Test finding segment to the right
        let seg_right = find_segment_right_of_col(&gc, col(0));
        assert!(seg_right.is_some());

        // Test last segment
        let last_seg = get_last_segment(&gc);
        assert!(last_seg.is_some());
    }
}
