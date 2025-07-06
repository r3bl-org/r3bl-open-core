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

use std::ops::{Deref, DerefMut};

use crate::{ch, ChUnit};

/// Represents a grapheme segment index inside of [`super::GCString`].
#[derive(Debug, Copy, Clone, Default, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct SegIndex(pub ChUnit);

pub fn seg_index(arg_seg_index: impl Into<SegIndex>) -> SegIndex { arg_seg_index.into() }

mod seg_index_impl_block {
    use super::{ch, seg_width, ChUnit, Deref, DerefMut, SegIndex, SegWidth};

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

    impl From<SegWidth> for SegIndex {
        fn from(other: SegWidth) -> Self { other.convert_to_seg_index() }
    }
}

/// Represents a count of the number of grapheme segments inside of
/// [`super::GCString`]. The width is max index (zero based) + 1.
#[derive(Debug, Copy, Clone, Default, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct SegWidth(pub ChUnit);

pub fn seg_width(arg_seg_width: impl Into<SegWidth>) -> SegWidth { arg_seg_width.into() }

mod seg_width_impl_block {
    use super::{ch, seg_index, ChUnit, Deref, DerefMut, SegIndex, SegWidth};

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

    impl From<SegIndex> for SegWidth {
        fn from(other: SegIndex) -> Self { other.convert_to_seg_width() }
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
    fn seg_width_conversions() {
        let width = seg_width(1);
        let index = width.convert_to_seg_index();
        assert_eq!(index, seg_index(0));
        let width = index.convert_to_seg_width();
        assert_eq!(width, seg_width(1));
    }

    #[test]
    fn seg_index_as_usize() {
        let index = seg_index(0);
        assert_eq!(index.as_usize(), 0);
    }

    #[test]
    fn seg_width_as_usize() {
        let width = seg_width(1);
        assert_eq!(width.as_usize(), 1);
    }
}
