// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::ops::{Deref, DerefMut};

use crate::{ChUnit, ch, IndexMarker, UnitCompare};
use super::seg_width::{SegWidth, seg_width};

/// Represents a grapheme segment index inside of [`crate::GCStringOwned`].
#[derive(Debug, Copy, Clone, Default, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct SegIndex(pub ChUnit);

pub fn seg_index(arg_seg_index: impl Into<SegIndex>) -> SegIndex { arg_seg_index.into() }

mod seg_index_impl_block {
    use super::{ChUnit, Deref, DerefMut, SegIndex, SegWidth, ch, seg_width};

    impl SegIndex {
        /// Converts the segment index to a width, by adding 1.
        #[must_use]
        pub fn convert_to_seg_width(&self) -> SegWidth {
            let index = self.0;
            let width = index + 1;
            seg_width(width)
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

    impl From<SegWidth> for SegIndex {
        fn from(other: SegWidth) -> Self { other.convert_to_seg_index() }
    }

}

// Implement bounds checking traits for SegIndex
impl UnitCompare for SegIndex {
    fn as_usize(&self) -> usize { self.as_usize() }
    fn as_u16(&self) -> u16 { self.0.value }
}

impl IndexMarker for SegIndex {
    type LengthType = SegWidth;

    fn convert_to_length(&self) -> Self::LengthType {
        self.convert_to_seg_width()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seg_index_conversions() {
        let index = seg_index(0);
        let width = index.convert_to_seg_width();
        assert_eq!(width, seg_width(1));
        let index = width.convert_to_seg_index();
        assert_eq!(index, seg_index(0));
    }

    #[test]
    fn seg_index_as_usize() {
        let index = seg_index(0);
        assert_eq!(index.as_usize(), 0);
    }
}
