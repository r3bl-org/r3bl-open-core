// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::ops::{Deref, DerefMut};

use crate::{ChUnit, ch, LengthMarker, UnitCompare};
use super::seg_index::{SegIndex, seg_index};

/// Represents a count of the number of grapheme segments inside of
/// [`crate::GCStringOwned`]. The width is max index (zero based) + 1.
#[derive(Debug, Copy, Clone, Default, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct SegWidth(pub ChUnit);

pub fn seg_width(arg_seg_width: impl Into<SegWidth>) -> SegWidth { arg_seg_width.into() }

mod seg_width_impl_block {
    use super::{ChUnit, Deref, DerefMut, SegIndex, SegWidth, ch, seg_index};

    impl SegWidth {
        /// Converts the width to a segment index, by subtracting 1.
        #[must_use]
        pub fn convert_to_seg_index(&self) -> SegIndex {
            let width = self.0;
            let index = width - 1;
            seg_index(index)
        }

        #[must_use]
        pub fn as_usize(&self) -> usize { self.0.as_usize() }
    }

    impl Deref for SegWidth {
        type Target = ChUnit;
        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for SegWidth {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl From<usize> for SegWidth {
        fn from(it: usize) -> Self { Self(ch(it)) }
    }

    impl From<ChUnit> for SegWidth {
        fn from(it: ChUnit) -> Self { Self(it) }
    }

    impl From<u16> for SegWidth {
        fn from(it: u16) -> Self { Self(ch(it)) }
    }

    impl From<i32> for SegWidth {
        fn from(it: i32) -> Self { Self(ch(it)) }
    }

    impl From<SegIndex> for SegWidth {
        fn from(other: SegIndex) -> Self { other.convert_to_seg_width() }
    }

}

// Implement bounds checking traits for SegWidth
impl UnitCompare for SegWidth {
    fn as_usize(&self) -> usize { self.as_usize() }
    fn as_u16(&self) -> u16 { self.0.value }
}

impl LengthMarker for SegWidth {
    type IndexType = SegIndex;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seg_width_conversions() {
        let width = seg_width(1);
        let index = width.convert_to_seg_index();
        assert_eq!(index, seg_index(0));
        let width = index.convert_to_seg_width();
        assert_eq!(width, seg_width(1));
    }

    #[test]
    fn seg_width_as_usize() {
        let width = seg_width(1);
        assert_eq!(width.as_usize(), 1);
    }
}