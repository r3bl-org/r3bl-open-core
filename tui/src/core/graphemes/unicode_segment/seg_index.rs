// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::seg_length::{SegLength, seg_length};
use crate::{ChUnit, IndexOps, VPIndex, generate_index_type_impl};
use std::fmt::{Display, Formatter};

/// Represents a grapheme segment index inside of [`crate::GCStringOwned`].
#[derive(Copy, Clone, Default, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct SegIndex(ChUnit);
generate_index_type_impl!(
    SegIndex,   // Add impl for this type
    SegLength,  // Use this associated type
    seg_index,  // Make this constructor fn
    seg_length  // Use this constructor fn
);

impl Display for SegIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_usize())
    }
}

impl SegIndex {
    /// Converts the segment index to a length, by adding 1.
    #[must_use]
    pub fn convert_to_seg_length(&self) -> SegLength { IndexOps::convert_to_length(self) }
}

impl From<SegLength> for SegIndex {
    fn from(other: SegLength) -> SegIndex { other.convert_to_seg_index() }
}

impl From<VPIndex> for SegIndex {
    fn from(it: VPIndex) -> SegIndex { SegIndex(*it) }
}

impl From<SegIndex> for VPIndex {
    fn from(it: SegIndex) -> VPIndex { VPIndex::from(*it) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seg_index_conversions() {
        let index = seg_index(0);
        let length = index.convert_to_seg_length();
        assert_eq!(length, seg_length(1u16));
        let index = length.convert_to_seg_index();
        assert_eq!(index, seg_index(0));
    }

    #[test]
    fn seg_index_as_usize() {
        let index = seg_index(0);
        assert_eq!(index.as_usize(), 0);
    }

    #[test]
    fn seg_index_addition() {
        let index1 = seg_index(5);
        let index2 = seg_index(3);
        let result = index1 + index2;
        assert_eq!(result.as_usize(), 8);

        // Test with zero
        let zero = seg_index(0);
        let index = seg_index(10);
        assert_eq!((zero + index).as_usize(), 10);
        assert_eq!((index + zero).as_usize(), 10);
    }

    #[test]
    fn seg_index_subtraction() {
        let index1 = seg_index(10);
        let index2 = seg_index(3);
        let result = index1 - index2;
        assert_eq!(result.as_usize(), 7);

        // Test saturating subtraction (underflow protection).
        let small = seg_index(3);
        let large = seg_index(10);
        let result = small - large;
        assert_eq!(result.as_usize(), 0);

        // Test subtraction with zero.
        let index = seg_index(10);
        let zero = seg_index(0);
        assert_eq!((index - zero).as_usize(), 10);
    }

    #[test]
    fn seg_index_sub_assign() {
        let mut index = seg_index(10);
        index -= seg_index(3);
        assert_eq!(index.as_usize(), 7);

        // Test saturating sub_assign.
        let mut small = seg_index(3);
        small -= seg_index(10);
        assert_eq!(small.as_usize(), 0);
    }

    #[test]
    fn seg_index_sub_length() {
        let index = seg_index(10);
        let length = seg_length(3u16);
        let result = index - length;
        assert_eq!(result.as_usize(), 7);

        // Test saturating subtraction with length.
        let small_index = seg_index(3);
        let large_length = seg_length(10u16);
        let result = small_index - large_length;
        assert_eq!(result.as_usize(), 0);
    }

    #[test]
    fn seg_index_sub_assign_length() {
        let mut index = seg_index(10);
        let length = seg_length(3u16);
        index -= length;
        assert_eq!(index.as_usize(), 7);
    }

    #[test]
    fn seg_index_range_boundary_compatibility() {
        use crate::{RangeBoundsExt, RangeValidityStatus};
        use std::ops::Range;

        let start = seg_index(2);
        let end = seg_index(5);
        let range: Range<SegIndex> = start..end;
        let length = seg_length(10u16);

        // Test that RangeBoundsExt works with SegIndex now that Add is implemented
        assert_eq!(
            range.check_range_is_valid_for_length(length),
            RangeValidityStatus::Valid
        );

        // Test invalid range - end is out of bounds
        let invalid_range: Range<SegIndex> = seg_index(8)..seg_index(12);
        assert_eq!(
            invalid_range.check_range_is_valid_for_length(length),
            RangeValidityStatus::EndOutOfBounds
        );
    }
}
