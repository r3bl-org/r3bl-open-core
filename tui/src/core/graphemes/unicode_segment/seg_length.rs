// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::seg_index::{SegIndex, seg_index};
use crate::{ChUnit, LengthOps, NumericConversions, NumericValue, ch};
use std::ops::{Add, Deref, DerefMut};

/// Represents a count of the number of grapheme segments inside of
/// [`crate::GCStringOwned`]. The length is max index (zero based) + 1.
#[derive(Debug, Copy, Clone, Default, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct SegLength(pub ChUnit);

pub fn seg_length(arg_seg_length: impl Into<SegLength>) -> SegLength {
    arg_seg_length.into()
}

mod seg_length_impl_block {
    use super::{ChUnit, Deref, DerefMut, SegIndex, SegLength, ch, seg_index};

    impl SegLength {
        /// Converts the length to a segment index, by subtracting 1.
        #[must_use]
        pub fn convert_to_seg_index(&self) -> SegIndex {
            let length = self.0;
            let index = length - 1;
            seg_index(index)
        }

        #[must_use]
        pub fn as_usize(&self) -> usize { self.0.as_usize() }
    }

    impl Deref for SegLength {
        type Target = ChUnit;
        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for SegLength {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl From<usize> for SegLength {
        fn from(it: usize) -> Self { Self(ch(it)) }
    }

    impl From<ChUnit> for SegLength {
        fn from(it: ChUnit) -> Self { Self(it) }
    }

    impl From<u16> for SegLength {
        fn from(it: u16) -> Self { Self(ch(it)) }
    }

    impl From<i32> for SegLength {
        fn from(it: i32) -> Self { Self(ch(it)) }
    }

    impl From<SegIndex> for SegLength {
        fn from(other: SegIndex) -> Self { other.convert_to_seg_length() }
    }
}

// Implement bounds checking traits for SegLength.
impl Add for SegLength {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        SegLength(ChUnit::from(self.0.value + other.0.value))
    }
}

impl NumericConversions for SegLength {
    fn as_usize(&self) -> usize { self.0.as_usize() }
    fn as_u16(&self) -> u16 { self.0.as_u16() }
}

impl NumericValue for SegLength {}

impl LengthOps for SegLength {
    type IndexType = SegIndex;

    fn convert_to_index(&self) -> Self::IndexType {
        if self.0.value == 0 {
            SegIndex(ChUnit::from(0))
        } else {
            SegIndex(ChUnit::from(self.0.value - 1))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IndexOps;

    #[test]
    fn seg_length_conversions() {
        let length = seg_length(1);
        let index = length.convert_to_seg_index();
        assert_eq!(index, seg_index(0));
        let length = index.convert_to_seg_length();
        assert_eq!(length, seg_length(1));
    }

    #[test]
    fn seg_length_as_usize() {
        let length = seg_length(1);
        assert_eq!(length.as_usize(), 1);
    }

    #[test]
    fn seg_length_from_various_types() {
        // Test From<usize>
        let length = SegLength::from(5usize);
        assert_eq!(length.as_usize(), 5);

        // Test From<u16>
        let length = SegLength::from(3u16);
        assert_eq!(length.as_usize(), 3);

        // Test From<i32>
        let length = SegLength::from(7i32);
        assert_eq!(length.as_usize(), 7);

        // Test seg_length function
        let length = seg_length(10);
        assert_eq!(length.as_usize(), 10);
    }

    #[test]
    fn seg_length_edge_cases() {
        let length = seg_length(0);
        assert_eq!(length.as_usize(), 0);

        // Test with length 1 (should give index 0)
        let length = seg_length(1);
        let index = length.convert_to_seg_index();
        assert_eq!(index.as_usize(), 0);

        // Test larger values
        let length = seg_length(100);
        let index = length.convert_to_seg_index();
        assert_eq!(index.as_usize(), 99);
    }

    #[test]
    fn seg_length_bounds_checking_traits() {
        let length = seg_length(100);

        // Test NumericValue trait
        assert_eq!(length.as_usize(), 100);
        assert_eq!(length.as_u16(), 100);

        // Test LengthOps trait through IndexOps conversion
        let index = seg_index(99);
        let converted_length = index.convert_to_length();
        assert_eq!(converted_length.as_usize(), 100);
    }
}
