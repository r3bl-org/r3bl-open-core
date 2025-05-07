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

use std::{fmt::Debug,
          hash::Hash,
          ops::{Add, AddAssign, Deref, DerefMut, Mul, Sub, SubAssign}};

use super::{ChUnit, Length};

#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct Index(pub ChUnit);

pub fn idx(arg_index: impl Into<Index>) -> Index { arg_index.into() }

impl Debug for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Index({:?})", self.0)
    }
}

mod construct {
    use super::*;

    impl Index {
        pub fn new(arg_col_index: impl Into<Index>) -> Self { arg_col_index.into() }

        pub fn as_usize(&self) -> usize { self.0.into() }

        /// This is for use with [crossterm] crate.
        pub fn as_u16(&self) -> u16 { self.0.into() }

        /// Add 1 to the index to convert it to a length. The intention of this function
        /// is to meaningfully convert a [Index] to a [Length]. This is useful in
        /// situations where you need to find what the length is at this index.
        pub fn convert_to_length(&self) -> ChUnit { self.0 + 1 }
    }

    impl From<ChUnit> for Index {
        fn from(ch_unit: ChUnit) -> Self { Index(ch_unit) }
    }

    impl From<usize> for Index {
        fn from(val: usize) -> Self { Index(val.into()) }
    }

    impl From<Index> for usize {
        fn from(col: Index) -> Self { col.as_usize() }
    }

    impl From<u16> for Index {
        fn from(val: u16) -> Self { Index(val.into()) }
    }

    impl From<i32> for Index {
        fn from(val: i32) -> Self { Index(val.into()) }
    }
}

mod ops {
    use super::*;

    impl Deref for Index {
        type Target = ChUnit;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for Index {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl Add<Index> for Index {
        type Output = Index;

        fn add(self, rhs: Index) -> Self::Output {
            let mut self_copy = self;
            self_copy.0 += rhs.0;
            self_copy
        }
    }

    impl AddAssign for Index {
        fn add_assign(&mut self, rhs: Self) { self.0 += rhs.0; }
    }

    impl Sub<Index> for Index {
        type Output = Index;

        fn sub(self, rhs: Index) -> Self::Output {
            let mut self_copy = self;
            self_copy.0 -= rhs.0;
            self_copy
        }
    }

    impl SubAssign<Index> for Index {
        fn sub_assign(&mut self, rhs: Index) { self.0 -= rhs.0; }
    }

    impl Sub<Length> for Index {
        type Output = Index;

        fn sub(self, rhs: Length) -> Self::Output {
            let mut self_copy = self;
            self_copy.0 -= rhs.0;
            self_copy
        }
    }

    impl SubAssign<Length> for Index {
        fn sub_assign(&mut self, rhs: Length) { self.0 -= rhs.0; }
    }

    impl Add<Length> for Index {
        type Output = Index;

        fn add(self, rhs: Length) -> Self::Output {
            let mut self_copy = self;
            self_copy.0 += rhs.0;
            self_copy
        }
    }

    impl AddAssign<Length> for Index {
        fn add_assign(&mut self, rhs: Length) { self.0 += rhs.0; }
    }

    impl Mul<Length> for Index {
        type Output = Index;

        fn mul(self, rhs: Length) -> Self::Output {
            let mut self_copy = self;
            self_copy.0 *= rhs.0;
            self_copy
        }
    }
}
#[cfg(test)]
mod tests {
    use std::hash::{DefaultHasher, Hasher as _};

    use super::*;
    use crate::{ch, len};

    #[test]
    fn test_index_add() {
        let index1 = idx(10);
        let index2 = idx(5);
        let result = index1 + index2;
        assert_eq!(result, idx(15));
    }

    #[test]
    fn test_index_add_assign() {
        let mut index1 = idx(10);
        let index2 = idx(5);
        index1 += index2;
        assert_eq!(index1, idx(15));
    }

    #[test]
    fn test_index_sub() {
        let index1 = idx(10);
        let index2 = idx(5);
        let result = index1 - index2;
        assert_eq!(result, idx(5));
    }

    #[test]
    fn test_index_sub_assign() {
        let mut index1 = idx(10);
        let index2 = idx(5);
        index1 -= index2;
        assert_eq!(index1, idx(5));
    }

    #[test]
    fn test_index_from_ch_unit() {
        let ch_unit = ch(10);
        let index = Index::from(ch_unit);
        assert_eq!(index, idx(10));
    }

    #[test]
    fn test_index_from_usize() {
        let val = 10_usize;
        let index = Index::from(val);
        assert_eq!(index, idx(10));
    }

    #[test]
    fn test_index_from_u16() {
        let val = 10_u16;
        let index = Index::from(val);
        assert_eq!(index, idx(10));
    }

    #[test]
    fn test_index_from_i32() {
        let val = 10_i32;
        let index = Index::from(val);
        assert_eq!(index, idx(10));
    }

    #[test]
    fn test_index_as_usize() {
        let index = idx(10);
        let val = index.as_usize();
        assert_eq!(val, 10_usize);
    }

    #[test]
    fn test_index_as_u16() {
        let index = idx(10);
        let val = index.as_u16();
        assert_eq!(val, 10_u16);
    }

    #[test]
    fn test_index_convert_to_width() {
        let index = idx(9);
        let width = index.convert_to_length();
        assert_eq!(width, ch(10));
    }

    #[test]
    fn test_index_deref() {
        let index = idx(10);
        let value = *index;
        assert_eq!(value, ch(10));
    }

    #[test]
    fn test_index_deref_mut() {
        let mut index = idx(10);
        *index = ch(20);
        assert_eq!(index, idx(20));
    }

    #[test]
    fn test_index_sub_length() {
        let index = idx(10);
        let length = len(3);
        let result = index - length;
        assert_eq!(result, idx(7));
    }

    #[test]
    fn test_index_sub_assign_length() {
        let mut index = idx(10);
        let length = len(3);
        index -= length;
        assert_eq!(index, idx(7));
    }

    #[test]
    fn test_index_add_length() {
        let index = idx(10);
        let length = len(3);
        let result = index + length;
        assert_eq!(result, idx(13));
    }

    #[test]
    fn test_index_add_assign_length() {
        let mut index = idx(10);
        let length = len(3);
        index += length;
        assert_eq!(index, idx(13));
    }

    #[test]
    fn test_index_mul_length() {
        let index = idx(10);
        let length = len(3);
        let result = index * length;
        assert_eq!(result, idx(30));
    }

    #[test]
    fn test_index_into_usize() {
        let index = idx(10);
        let result: usize = index.into();
        assert_eq!(result, 10);
    }

    #[test]
    fn test_index_debug_fmt() {
        let index = idx(10);
        let debug_string = format!("{index:?}");
        assert_eq!(debug_string, "Index(10)");
    }

    #[test]
    fn test_index_partial_ord() {
        let index1 = idx(10);
        let index2 = idx(5);
        assert!(index1 > index2);
        assert!(index2 < index1);
        assert!(index1 >= index2);
        assert!(index2 <= index1);
    }

    #[test]
    fn test_index_ord() {
        let index1 = idx(10);
        let index2 = idx(5);
        assert!(index1 > index2);
        assert!(index2 < index1);
    }

    #[test]
    fn test_index_eq() {
        let index1 = idx(10);
        let index2 = idx(10);
        assert_eq!(index1, index2);
    }

    #[test]
    fn test_index_ne() {
        let index1 = idx(10);
        let index2 = idx(5);
        assert_ne!(index1, index2);
    }

    #[test]
    fn test_index_hash() {
        let index1 = idx(10);
        let index2 = idx(10);

        let mut hasher1 = DefaultHasher::new();
        index1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        index2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_idx_fn() {
        let index = Index(ch(10));
        assert_eq!(index, idx(10));
    }
}
