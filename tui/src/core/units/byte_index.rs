// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::ops::{Deref, DerefMut};

use crate::{ChUnit, Index};

/// Represents a byte index inside of the underlying [`crate::InlineString`] of
/// [`crate::GCStringOwned`].
#[derive(Debug, Copy, Clone, Default, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct ByteIndex(pub usize);

impl ByteIndex {
    #[must_use]
    pub fn as_usize(&self) -> usize { self.0 }
}

pub fn byte_index(arg_byte_index: impl Into<ByteIndex>) -> ByteIndex {
    arg_byte_index.into()
}

impl Deref for ByteIndex {
    type Target = usize;
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for ByteIndex {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl From<usize> for ByteIndex {
    fn from(it: usize) -> Self { Self(it) }
}

impl From<ChUnit> for ByteIndex {
    fn from(it: ChUnit) -> Self { Self(crate::usize(it)) }
}

impl From<ByteIndex> for Index {
    fn from(it: ByteIndex) -> Self { Self::from(it.0) }
}

impl From<ByteIndex> for usize {
    fn from(it: ByteIndex) -> Self { it.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ByteOffset, ch, byte_offset};

    // Basic construction and conversion tests
    #[test]
    fn test_byte_index_from_usize() {
        let index = ByteIndex::from(42usize);
        assert_eq!(index.as_usize(), 42);
    }

    #[test]
    fn test_byte_index_from_ch_unit() {
        let ch_unit = ch(10);
        let index = ByteIndex::from(ch_unit);
        assert_eq!(index.as_usize(), 10);
    }

    #[test]
    fn test_byte_index_as_usize() {
        let index = byte_index(25);
        assert_eq!(index.as_usize(), 25);
    }

    #[test]
    fn test_byte_index_deref() {
        let index = byte_index(15);
        let value = *index;
        assert_eq!(value, 15);
    }

    #[test]
    fn test_byte_index_deref_mut() {
        let mut index = byte_index(20);
        *index = 30;
        assert_eq!(index.as_usize(), 30);
    }

    // Conversion tests to other types
    #[test]
    fn test_byte_index_to_usize() {
        let index = byte_index(42);
        let value: usize = index.into();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_byte_index_to_index() {
        let index = byte_index(42);
        let generic_index: Index = index.into();
        assert_eq!(generic_index.as_usize(), 42);
    }

    // Critical ByteIndex <-> ByteOffset conversion tests
    #[test]
    fn test_byte_index_to_byte_offset_conversion() {
        let index = byte_index(100);
        let offset: ByteOffset = index.into();
        assert_eq!(offset.as_usize(), 100);
    }

    #[test]
    fn test_byte_offset_from_byte_index_semantic() {
        // Semantic test: converting absolute position to relative offset
        let absolute_position = byte_index(50);
        let relative_offset = ByteOffset::from(absolute_position);
        assert_eq!(relative_offset, byte_offset(50));
    }

    #[test]
    fn test_roundtrip_byte_index_to_offset_to_usize() {
        let original = byte_index(123);
        let as_offset: ByteOffset = original.into();
        let as_usize: usize = original.into();

        assert_eq!(as_offset.as_usize(), 123);
        assert_eq!(as_usize, 123);
        assert_eq!(as_offset.as_usize(), as_usize);
    }

    // Edge case tests
    #[test]
    fn test_zero_byte_index() {
        let zero_index = byte_index(0);
        assert_eq!(zero_index.as_usize(), 0);
        assert_eq!(*zero_index, 0);

        let offset: ByteOffset = zero_index.into();
        assert_eq!(offset.as_usize(), 0);
    }

    #[test]
    fn test_large_byte_index() {
        let large_index = byte_index(usize::MAX / 2);
        assert_eq!(large_index.as_usize(), usize::MAX / 2);

        let offset: ByteOffset = large_index.into();
        assert_eq!(offset.as_usize(), usize::MAX / 2);
    }

    // Trait implementation tests
    #[test]
    fn test_debug_format() {
        let index = byte_index(42);
        let debug_str = format!("{:?}", index);
        assert!(debug_str.contains("ByteIndex"));
        assert!(debug_str.contains("42"));
    }

    #[test]
    fn test_clone() {
        let index1 = byte_index(42);
        let index2 = index1.clone();
        assert_eq!(index1, index2);
    }

    #[test]
    fn test_copy() {
        let index1 = byte_index(42);
        let index2 = index1; // Copy semantics
        assert_eq!(index1, index2);
    }

    #[test]
    fn test_equality() {
        let index1 = byte_index(42);
        let index2 = byte_index(42);
        let index3 = byte_index(24);

        assert_eq!(index1, index2);
        assert_ne!(index1, index3);
    }

    #[test]
    fn test_ordering() {
        let index1 = byte_index(10);
        let index2 = byte_index(20);
        let index3 = byte_index(10);

        assert!(index1 < index2);
        assert!(index2 > index1);
        assert!(index1 <= index3);
        assert!(index1 >= index3);
    }

    #[test]
    fn test_default() {
        let index = ByteIndex::default();
        assert_eq!(index, byte_index(0));
    }

    #[test]
    fn test_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        let index1 = byte_index(42);
        let index2 = byte_index(42);
        let index3 = byte_index(24);

        set.insert(index1);
        set.insert(index2); // Should not increase set size
        set.insert(index3);

        assert_eq!(set.len(), 2); // Only two unique values
        assert!(set.contains(&index1));
        assert!(set.contains(&index2));
        assert!(set.contains(&index3));
    }

    // Semantic correctness tests
    #[test]
    fn test_semantic_absolute_position_usage() {
        // ByteIndex represents absolute positions in buffers/strings
        let buffer_start = byte_index(0);
        let char_position = byte_index(5);
        let end_position = byte_index(100);

        assert!(buffer_start < char_position);
        assert!(char_position < end_position);

        // Converting to offset makes sense when position becomes relative
        let relative_from_start: ByteOffset = char_position.into();
        assert_eq!(relative_from_start.as_usize(), 5);
    }

    // Constructor function tests
    #[test]
    fn test_byte_index_constructor_function() {
        let index = byte_index(42usize);
        assert_eq!(index, ByteIndex::from(42usize));

        let index_from_ch = byte_index(ch(10));
        assert_eq!(index_from_ch, ByteIndex::from(ch(10)));
    }
}
