// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! One-based byte size measurements - see [`ByteLength`] type.
use crate::{ArrayBoundsCheck, ByteIndex, ChUnit, NarrowingCastToUsize, VPLength,
            WideningCastToUsize,
            bounds_check::{LengthOps, NumericConversions, NumericValue,
                           StorageCoordinate},
            byte_index, usize};
use std::ops::{Add, Deref, DerefMut};

/// Represents a byte length measurement (1-based).
///
/// A [`ByteLength`] represents the number of bytes in a buffer, string segment, or other
/// byte-oriented structure. Unlike [`ByteIndex`] which is 0-based (representing
/// positions), [`ByteLength`] is 1-based (representing sizes/counts).
///
/// > This newtype struct does not use [`ChUnit`] like other unit types because
/// > offsets are inherently [`prim@usize`].
///
/// This type enables semantic correctness in the bounds checking system by providing
/// a proper length type that pairs with [`ByteIndex`] for byte-based operations,
/// eliminating the need for conversions to character-based [`VPLength`] types.
///
/// # Type System Integration
///
/// [`ByteLength`] implements [`LengthOps`] with [`ByteIndex`] as its associated index
/// type, creating a bidirectional relationship that allows for type-safe bounds checking
/// operations specific to byte measurements.
///
/// # Examples
///
/// ```rust
/// use r3bl_tui::{ArrayBoundsCheck, ArrayOverflowResult, ByteIndex, ByteLength, IndexOps, byte_index, byte_len};
///
/// // Create a buffer with 10 bytes
/// let buffer_size = byte_len(10);
///
/// // Check if an index is within bounds
/// let index = byte_index(5);
/// assert_eq!(index.overflows(buffer_size), ArrayOverflowResult::Within);
///
/// // Index at the boundary
/// let boundary_index = byte_index(9);
/// assert_eq!(boundary_index.overflows(buffer_size), ArrayOverflowResult::Within);
///
/// // Index beyond boundary
/// let beyond_index = byte_index(10);
/// assert_eq!(beyond_index.overflows(buffer_size), ArrayOverflowResult::Overflowed);
/// ```
///
/// [`VPLength`]: crate::VPLength
#[derive(Debug, Copy, Clone, Default, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct ByteLength(usize);

/// Creates a new [`ByteLength`] from any type that can be converted into it.
/// This is a convenience function that is equivalent to calling [`ByteLength::from`].
///
/// # Examples
///
/// ```rust
/// use r3bl_tui::{ByteLength, byte_len};
///
/// let length = byte_len(42u16);
/// assert_eq!(length, ByteLength::from(42u16));
/// ```
pub fn byte_len(arg_byte_length: impl Into<ByteLength>) -> ByteLength {
    arg_byte_length.into()
}

impl ByteLength {
    #[must_use]
    pub fn as_usize(&self) -> usize { self.0 }

    /// Converts this length to the corresponding index type (0-based).
    ///
    /// Since lengths are 1-based and indices are 0-based, this subtracts 1 to get the
    /// last valid index position.
    ///
    /// ```text
    /// Length=6 (1-based) to index (0-based) conversion:
    ///
    /// Length:     1   2   3   4   5   6
    /// (1-based) ┌───┬───┬───┬───┬───┬───┐
    ///           │   │   │   │   │   │   │
    ///           └───┴───┴───┴───┴───┴───┘
    /// Index:      0   1   2   3   4   5
    /// (0-based)                       ↑
    ///           convert_to_index() = 5 (0-based, last valid position)
    /// ```
    #[must_use]
    pub fn convert_to_index(&self) -> ByteIndex { byte_index(self.0.saturating_sub(1)) }
}

impl Deref for ByteLength {
    type Target = usize;
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for ByteLength {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl From<usize> for ByteLength {
    fn from(it: usize) -> ByteLength { ByteLength(it) }
}

impl From<ChUnit> for ByteLength {
    fn from(it: ChUnit) -> ByteLength { ByteLength(usize(it)) }
}

impl From<ByteIndex> for ByteLength {
    /// Converts a 0-based index to a 1-based length.
    ///
    /// Index 0 becomes Length 1, Index 1 becomes Length 2, etc.
    fn from(it: ByteIndex) -> ByteLength { ByteLength(it.as_usize().saturating_add(1)) }
}

impl From<u16> for ByteLength {
    fn from(it: u16) -> ByteLength { ByteLength(it.as_usize_widening()) }
}

impl From<i32> for ByteLength {
    fn from(it: i32) -> ByteLength { ByteLength(it.as_usize_narrowing()) }
}

impl From<VPLength> for ByteLength {
    /// Converts a character-based length to a byte-based length.
    ///
    /// Both types are 1-based measurements, so this is a direct value conversion. This
    /// conversion assumes that the Length value represents the same semantic measurement
    /// but in different units (characters vs bytes).
    fn from(it: VPLength) -> ByteLength { ByteLength(it.as_usize()) }
}

impl Add for ByteLength {
    type Output = Self;
    fn add(self, other: Self) -> Self { ByteLength(self.0 + other.0) }
}

impl Add<usize> for ByteLength {
    type Output = Self;
    fn add(self, other: usize) -> Self { ByteLength(self.0 + other) }
}

impl NumericConversions for ByteLength {
    /// Converts the byte length to a [`prim@usize`] value for numeric comparison.
    fn as_usize(&self) -> usize { self.0 }
}

impl NumericValue for ByteLength {}

impl LengthOps for ByteLength {
    type IndexType = ByteIndex;

    fn convert_to_index(&self) -> Self::IndexType { self.convert_to_index() }
}

impl StorageCoordinate for ByteLength {}

impl ArrayBoundsCheck<ByteLength> for ByteLength {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ch;

    // Basic construction and conversion tests.
    #[test]
    fn test_byte_length_from_usize() {
        let length = ByteLength::from(42u16);
        assert_eq!(length.as_usize(), 42);
    }

    #[test]
    fn test_byte_length_from_ch_unit() {
        let ch_unit = ch(10);
        let length = ByteLength::from(ch_unit);
        assert_eq!(length.as_usize(), 10);
    }

    #[test]
    fn test_byte_length_as_usize() {
        let length = byte_len(25);
        assert_eq!(length.as_usize(), 25);
    }

    #[test]
    fn test_byte_length_deref() {
        let length = byte_len(15);
        let value = *length;
        assert_eq!(value, 15);
    }

    #[test]
    fn test_byte_length_deref_mut() {
        let mut length = byte_len(20);
        *length = 30;
        assert_eq!(length.as_usize(), 30);
    }

    // Conversion tests.
    #[test]
    fn test_byte_length_from_byte_index() {
        let index = byte_index(5);
        let length = ByteLength::from(index);
        assert_eq!(length.as_usize(), 6); // 0-based to 1-based conversion
    }

    #[test]
    fn test_convert_to_index() {
        let length = byte_len(6);
        let index = length.convert_to_index();
        assert_eq!(index.as_usize(), 5); // 1-based to 0-based conversion
    }

    #[test]
    fn test_roundtrip_index_to_length_to_index() {
        let original_index = byte_index(10);
        let as_length = ByteLength::from(original_index);
        let back_to_index = as_length.convert_to_index();

        assert_eq!(as_length.as_usize(), 11); // 10 + 1
        assert_eq!(back_to_index, original_index);
    }

    // Edge case tests.
    #[test]
    fn test_zero_byte_length() {
        let zero_length = byte_len(0);
        assert_eq!(zero_length.as_usize(), 0);
        assert_eq!(*zero_length, 0);

        // Converting zero length to index should saturate at 0.
        let index = zero_length.convert_to_index();
        assert_eq!(index.as_usize(), 0);
    }

    #[test]
    fn test_large_byte_length() {
        let large_length = byte_len(usize::MAX / 2);
        assert_eq!(large_length.as_usize(), usize::MAX / 2);

        let index = large_length.convert_to_index();
        assert_eq!(index.as_usize(), (usize::MAX / 2) - 1);
    }

    #[test]
    fn test_clone() {
        let length1 = byte_len(42);
        let length2 = length1;
        assert_eq!(length1, length2);
    }

    #[test]
    fn test_copy() {
        let length1 = byte_len(42);
        let length2 = length1; // Copy semantics
        assert_eq!(length1, length2);
    }

    #[test]
    fn test_equality() {
        let length1 = byte_len(42);
        let length2 = byte_len(42);
        let length3 = byte_len(24);

        assert_eq!(length1, length2);
        assert_ne!(length1, length3);
    }

    #[test]
    fn test_ordering() {
        let length1 = byte_len(10);
        let length2 = byte_len(20);
        let length3 = byte_len(10);

        assert!(length1 < length2);
        assert!(length2 > length1);
        assert!(length1 <= length3);
        assert!(length1 >= length3);
    }

    #[test]
    fn test_default() {
        let length = ByteLength::default();
        assert_eq!(length, byte_len(0));
    }

    #[test]
    fn test_hash() {
        use rustc_hash::FxHashSet;

        let mut set = FxHashSet::default();
        let length1 = byte_len(42);
        let length2 = byte_len(42);
        let length3 = byte_len(24);

        set.insert(length1);
        set.insert(length2); // Should not increase set size
        set.insert(length3);

        assert_eq!(set.len(), 2); // Only two unique values
        assert!(set.contains(&length1));
        assert!(set.contains(&length2));
        assert!(set.contains(&length3));
    }

    // Semantic correctness tests.
    #[test]
    fn test_semantic_buffer_length_usage() {
        // ByteLength represents the total size of a byte buffer.
        let buffer_size = byte_len(100);
        let first_position = byte_index(0);
        let _middle_position = byte_index(50);
        let last_valid_position = byte_index(99);
        let beyond_position = byte_index(100);

        // These should be semantically correct comparisons.
        assert_eq!(buffer_size.as_usize(), 100);
        assert_eq!(first_position.as_usize(), 0);
        assert_eq!(last_valid_position, buffer_size.convert_to_index());

        // The last valid index should be length - 1.
        assert_eq!(buffer_size.convert_to_index().as_usize(), 99);

        // Beyond position should equal the length value.
        assert_eq!(beyond_position.as_usize(), buffer_size.as_usize());
    }

    // Constructor function tests.
    #[test]
    fn test_byte_len_constructor_function() {
        let length = byte_len(42usize);
        assert_eq!(length, ByteLength::from(42u16));

        let length_from_ch = byte_len(ch(10));
        assert_eq!(length_from_ch, ByteLength::from(ch(10)));
    }

    // Unit trait tests.
    #[test]
    fn test_unit_marker_implementation() {
        let length = byte_len(42);
        assert_eq!(length.as_usize(), 42);
        assert!(!length.is_zero());

        let zero_length = byte_len(0);
        assert!(zero_length.is_zero());
    }
}
