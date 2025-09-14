// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Extension traits for grapheme-aware string operations.

use crate::{ColIndex, GCStringOwned, GraphemeString, SegStringOwned};

/// Extension trait for when ownership is needed.
/// This trait provides convenience methods for converting borrowed
/// grapheme operations into owned types.
pub trait GraphemeStringOwnedExt: GraphemeString {
    /// Convert the entire string to an owned `GCStringOwned`.
    fn to_owned(&self) -> GCStringOwned { GCStringOwned::new(self.as_str()) }

    /// Get an owned version of the segment at a specific column position.
    fn get_seg_owned_at(&self, col: ColIndex) -> Option<SegStringOwned> {
        self.get_seg_at(col).map(|seg_content| SegStringOwned {
            string: GCStringOwned::from(seg_content.content),
            width: seg_content.seg.display_width,
            start_at: seg_content.seg.start_display_col_index,
        })
    }

    /// Get an owned version of the segment to the right of a column position.
    fn get_seg_owned_right_of(&self, col: ColIndex) -> Option<SegStringOwned> {
        self.get_seg_right_of(col)
            .map(|seg_content| SegStringOwned {
                string: GCStringOwned::from(seg_content.content),
                width: seg_content.seg.display_width,
                start_at: seg_content.seg.start_display_col_index,
            })
    }

    /// Get an owned version of the segment to the left of a column position.
    fn get_seg_owned_left_of(&self, col: ColIndex) -> Option<SegStringOwned> {
        self.get_seg_left_of(col).map(|seg_content| SegStringOwned {
            string: GCStringOwned::from(seg_content.content),
            width: seg_content.seg.display_width,
            start_at: seg_content.seg.start_display_col_index,
        })
    }

    /// Get an owned version of the last segment.
    fn get_seg_owned_at_end(&self) -> Option<SegStringOwned> {
        self.get_seg_at_end().map(|seg_content| SegStringOwned {
            string: GCStringOwned::from(seg_content.content),
            width: seg_content.seg.display_width,
            start_at: seg_content.seg.start_display_col_index,
        })
    }

    /// Clone all segments into a Vec of owned segments.
    fn segments_to_vec(&self) -> Vec<crate::Seg> { self.segments().to_vec() }
}

// Auto-implement the extension for all GraphemeString types.
impl<T: GraphemeString + ?Sized> GraphemeStringOwnedExt for T {}
