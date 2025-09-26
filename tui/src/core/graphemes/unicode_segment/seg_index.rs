// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::ops::{Add, Deref, DerefMut};

use super::seg_length::{SegLength, seg_length};
use crate::{ChUnit, Index, IndexMarker, UnitCompare, ch};

/// Represents a grapheme segment index inside of [`crate::GCStringOwned`].
#[derive(Debug, Copy, Clone, Default, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct SegIndex(pub ChUnit);

pub fn seg_index(arg_seg_index: impl Into<SegIndex>) -> SegIndex { arg_seg_index.into() }

mod seg_index_impl_block {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl SegIndex {
        /// Converts the segment index to a length, by adding 1.
        #[must_use]
        pub fn convert_to_seg_length(&self) -> SegLength {
            let index = self.0;
            let length = index + 1;
            seg_length(length)
        }

        #[must_use]
        pub fn as_usize(&self) -> usize { self.0.as_usize() }
    }

    impl Deref for SegIndex {
        type Target = ChUnit;
        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for SegIndex {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }
}

mod conversions {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<usize> for SegIndex {
        fn from(it: usize) -> Self { Self(ch(it)) }
    }

    impl From<ChUnit> for SegIndex {
        fn from(it: ChUnit) -> Self { Self(it) }
    }

    impl From<u16> for SegIndex {
        fn from(it: u16) -> Self { Self(ch(it)) }
    }

    impl From<i32> for SegIndex {
        fn from(it: i32) -> Self { Self(ch(it)) }
    }

    impl From<SegLength> for SegIndex {
        fn from(other: SegLength) -> Self { other.convert_to_seg_index() }
    }

    impl From<Index> for SegIndex {
        fn from(it: Index) -> Self { Self(ch(it.as_usize())) }
    }

    impl From<SegIndex> for Index {
        fn from(it: SegIndex) -> Self { Index::from(it.as_usize()) }
    }
}

// Implement arithmetic operations for SegIndex.
mod arithmetic {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Add for SegIndex {
        type Output = SegIndex;

        fn add(self, rhs: SegIndex) -> Self::Output {
            SegIndex::from(self.as_usize() + rhs.as_usize())
        }
    }
}

// Implement bounds checking traits for SegIndex.
impl UnitCompare for SegIndex {
    fn as_usize(&self) -> usize { self.as_usize() }
    fn as_u16(&self) -> u16 { self.0.value }
}

impl IndexMarker for SegIndex {
    type LengthType = SegLength;

    fn convert_to_length(&self) -> Self::LengthType { self.convert_to_seg_length() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seg_index_conversions() {
        let index = seg_index(0);
        let length = index.convert_to_seg_length();
        assert_eq!(length, seg_length(1));
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
    fn seg_index_range_boundary_compatibility() {
        use std::ops::Range;

        use crate::RangeBoundary;

        let start = seg_index(2);
        let end = seg_index(5);
        let range: Range<SegIndex> = start..end;
        let length = seg_length(10);

        // Test that RangeBoundary works with SegIndex now that Add is implemented
        assert!(range.is_valid(length));

        // Test invalid range
        let invalid_range: Range<SegIndex> = seg_index(8)..seg_index(12);
        assert!(!invalid_range.is_valid(length));
    }
}
