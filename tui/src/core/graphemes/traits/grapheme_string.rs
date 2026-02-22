// Copyright (c) 2024 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ChUnit, ColIndex, ColWidth, ContainsWideSegments, Seg, SegContent, SegIndex,
            SegLength};
use std::fmt::Display;

/// Single-line grapheme-aware string trait providing core operations
/// for working with strings that are aware of grapheme cluster boundaries.
///
/// This trait is implemented by types that represent a single line of text
/// with grapheme cluster awareness, such as `GCStringOwned` and `GapBufferLine`.
pub trait GraphemeString {
    /// Associated type for iterator over segments.
    ///
    /// The lifetime parameter `'a` represents the lifetime of the iterator and its
    /// yielded items. The constraint `Self: 'a` ensures that the iterator cannot
    /// outlive the string it borrows from.
    type SegmentIterator<'a>: Iterator<Item = Seg> + 'a
    where
        Self: 'a;

    /// Associated type for string slice operations.
    ///
    /// This allows different implementations to return appropriate string types:
    /// - `GCStringOwned` returns `CowInlineString` for flexible ownership
    /// - `GapBufferLine` returns `&str` for zero-copy operations
    ///
    /// The lifetime parameter `'a` represents the lifetime of the string slice.
    /// The constraint `Self: 'a` ensures that the slice cannot outlive the
    /// string it was derived from.
    type StringSlice<'a>: AsRef<str> + Display
    where
        Self: 'a;

    // Core properties

    /// Gets the underlying string slice
    fn as_str(&self) -> &str;

    /// Gets all segments as a slice
    fn segments(&self) -> &[Seg];

    /// Gets the total display width of the string
    fn display_width(&self) -> ColWidth;

    /// Gets the number of grapheme cluster segments
    fn segment_count(&self) -> SegLength;

    /// Gets the size in bytes
    fn byte_size(&self) -> ChUnit;

    // Segment navigation.

    /// Gets a segment by index
    fn get_seg(&self, index: SegIndex) -> Option<Seg>;

    /// Checks if a column position falls in the middle of a grapheme cluster
    fn check_is_in_middle_of_grapheme(&self, col: ColIndex) -> Option<Seg>;

    // Zero-copy segment content access.

    /// Gets segment content at a specific column position
    fn get_seg_at(&self, col: ColIndex) -> Option<SegContent<'_>>;

    /// Gets segment content to the right of a column position
    fn get_seg_right_of(&self, col: ColIndex) -> Option<SegContent<'_>>;

    /// Gets segment content to the left of a column position
    fn get_seg_left_of(&self, col: ColIndex) -> Option<SegContent<'_>>;

    /// Gets the last segment content
    fn get_seg_at_end(&self) -> Option<SegContent<'_>>;

    // String operations using associated type.

    /// Clips the string to a range defined by start column and width
    fn clip(&self, start_col: ColIndex, width: ColWidth) -> Self::StringSlice<'_>;

    /// Truncates from the end to fit within the given width
    fn trunc_end_to_fit(&self, width: ColWidth) -> Self::StringSlice<'_>;

    /// Truncates from the end by the given width
    fn trunc_end_by(&self, width: ColWidth) -> Self::StringSlice<'_>;

    /// Truncates from the start by the given width
    fn trunc_start_by(&self, width: ColWidth) -> Self::StringSlice<'_>;

    // Iterator

    /// Gets an iterator over segments
    fn segments_iter(&self) -> Self::SegmentIterator<'_>;

    // Additional methods.

    /// Checks if the string is empty
    fn is_empty(&self) -> bool;

    /// Gets the last segment
    fn last(&self) -> Option<Seg>;

    /// Checks if the string contains wide segments (width > 1)
    fn contains_wide_segments(&self) -> ContainsWideSegments;
}

/// Mutation operations for single-line strings using associated types
/// to handle different paradigms (immutable vs mutable operations).
pub trait GraphemeStringMut: GraphemeString {
    /// Associated type for mutation results - handles paradigm differences elegantly
    type MutResult;

    /// Inserts text at a specific column position
    fn insert_text(&mut self, col: ColIndex, text: &str) -> Option<Self::MutResult>;

    /// Deletes a range of text between two column positions
    fn delete_range(&mut self, start: ColIndex, end: ColIndex)
    -> Option<Self::MutResult>;

    /// Replaces a range of text with new text
    fn replace_range(
        &mut self,
        start: ColIndex,
        end: ColIndex,
        text: &str,
    ) -> Option<Self::MutResult>;

    /// Truncates the string at a specific column position
    fn truncate(&mut self, col: ColIndex) -> Option<Self::MutResult>;
}
