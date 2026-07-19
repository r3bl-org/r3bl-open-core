// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Zero-based byte position in strings and buffers - see [`ByteIndex`] type.

use crate::{ArrayBoundsCheck, ByteLength, ByteOffset, ChUnit, NarrowingCastToUsize,
            VPIndex, WideningCastToUsize,
            bounds_check::{IndexOps, NumericConversions, NumericValue,
                           StorageCoordinate},
            usize};
use std::ops::{Add, AddAssign, Deref, DerefMut, Range, Sub, SubAssign};

/// Represents an absolute byte position within strings and buffers (0-based).
///
/// A [`ByteIndex`] represents a specific byte position within a buffer, string, or other
/// byte-oriented structure. As a 0-based index, `ByteIndex(0)` refers to the first byte,
/// `ByteIndex(1)` to the second byte, and so on. This is distinct from [`ByteLength`]
/// which is 1-based and represents sizes/counts.
///
/// > This newtype struct does not use [`ChUnit`] like other unit types because byte
/// > positions are inherently [`prim@usize`].
///
/// This type is primarily used for byte-level operations in text processing, particularly
/// when working with [`UTF-8`] strings where character boundaries don't align with byte
/// boundaries. It provides type safety when dealing with the underlying byte
/// representation of [`crate::InlineString`] within [`crate::GCStringOwned`].
///
/// # Type System Integration
///
/// [`ByteIndex`] implements [`IndexOps`] with [`ByteLength`] as its associated length
/// type, creating a bidirectional relationship that enables type-safe bounds checking
/// operations specific to byte-level indexing.
///
/// # Type System Disambiguation
///
/// [`ByteIndex`] is conceptually distinct from related types in the type system:
/// - **vs [`ByteLength`]**: Index is 0-based position, Length is 1-based size
/// - **vs [`ByteOffset`]**: Index is absolute position, Offset is relative displacement
/// - **vs [`VPIndex`]**: [`ByteIndex`] is for byte positions, [`VPIndex`] is for
///   character positions
///
/// Think of it as:
/// - [`ByteIndex`] = absolute byte coordinate (like "byte position 42")
/// - [`ByteLength`] = byte count/size (like "10 bytes long")
/// - [`ByteOffset`] = byte displacement (like "5 bytes forward from here")
///
/// > 💡 **See also**: For complete workflows showing [`ByteIndex`] used with [`UTF-8`]
/// > string
/// > operations and bounds checking, see the [coordinates module
/// > documentation](crate::coordinates#common-workflows).
///
/// # Examples
///
/// ```rust
/// use r3bl_tui::{ByteIndex, ByteLength, byte_index, byte_len, ArrayBoundsCheck, ArrayOverflowResult};
///
/// // Create a byte index
/// let pos = byte_index(10);
/// let buffer_size = byte_len(20);
///
/// // Check if the byte position is valid for array access
/// assert_eq!(pos.overflows(buffer_size), ArrayOverflowResult::Within);
///
/// // Convert to character-based VPIndex if needed
/// use r3bl_tui::VPIndex;
/// let char_index: VPIndex = pos.into();
/// ```
///
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
#[derive(Debug, Copy, Clone, Default, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct ByteIndex(usize);

impl ByteIndex {
    #[must_use]
    pub fn as_usize(&self) -> usize { self.0 }
}

/// Helper constructor for [`ByteIndex`].
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
    fn from(it: usize) -> ByteIndex { ByteIndex(it) }
}

impl From<ChUnit> for ByteIndex {
    fn from(it: ChUnit) -> ByteIndex { ByteIndex(usize(it)) }
}

impl From<ByteIndex> for VPIndex {
    fn from(it: ByteIndex) -> VPIndex { VPIndex::from(it.0) }
}

impl From<ByteOffset> for ByteIndex {
    fn from(it: ByteOffset) -> ByteIndex { ByteIndex(it.as_usize()) }
}

impl From<ByteIndex> for usize {
    fn from(it: ByteIndex) -> usize { it.0 }
}

impl From<u16> for ByteIndex {
    fn from(it: u16) -> ByteIndex { ByteIndex(it.as_usize_widening()) }
}

impl From<i32> for ByteIndex {
    fn from(it: i32) -> ByteIndex { ByteIndex(it.as_usize_narrowing()) }
}

impl NumericConversions for ByteIndex {
    /// Converts the byte index to a usize value for numeric comparison, usually for array
    /// indexing operations.
    fn as_usize(&self) -> usize { self.0 }
}

impl NumericValue for ByteIndex {}

impl IndexOps for ByteIndex {
    type LengthType = ByteLength;

    fn convert_to_length(&self) -> Self::LengthType {
        crate::byte_len(self.as_usize().saturating_add(1))
    }
}

impl StorageCoordinate for ByteIndex {}

/// Implement `Add` trait for `ByteIndex`.
/// This allows `ByteIndex` to be used with `Range<ByteIndex>` for type-safe bounds
/// checking.
impl Add for ByteIndex {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output { Self(self.0.saturating_add(other.0)) }
}

impl Add<usize> for ByteIndex {
    type Output = Self;

    fn add(self, other: usize) -> Self::Output { Self(self.0.saturating_add(other)) }
}

/// Implement `AddAssign` trait for convenient position advancement.
/// This allows `+=` operations to increment a byte position.
impl AddAssign for ByteIndex {
    fn add_assign(&mut self, other: Self) { self.0 = self.0.saturating_add(other.0); }
}

impl AddAssign<usize> for ByteIndex {
    fn add_assign(&mut self, other: usize) { self.0 = self.0.saturating_add(other); }
}

impl Sub<usize> for ByteIndex {
    type Output = Self;

    fn sub(self, other: usize) -> Self::Output { Self(self.0.saturating_sub(other)) }
}

/// Implement `SubAssign` trait for in-place position decrement.
impl SubAssign for ByteIndex {
    fn sub_assign(&mut self, other: Self) { self.0 = self.0.saturating_sub(other.0); }
}

impl SubAssign<usize> for ByteIndex {
    fn sub_assign(&mut self, other: usize) { self.0 = self.0.saturating_sub(other); }
}

/// Extension trait to enable conversion from [`Range<ByteIndex>`] to [`Range<usize>`]
/// for slice indexing.
///
/// This works around Rust's orphan rule which prevents implementing
/// `From<Range<ByteIndex>> for Range<usize>`. The method name mimics `.into()` behavior
/// while remaining legally implementable.
///
/// # Example
/// ```
/// use r3bl_tui::{ByteIndex, byte_index};
/// use r3bl_tui::ByteIndexRangeExt;
/// use std::ops::Range;
///
/// let byte_range: Range<ByteIndex> = byte_index(5)..byte_index(10);
/// let usize_range: Range<usize> = byte_range.to_usize_range();
/// assert_eq!(usize_range, 5..10);
/// ```
///
/// [`ByteIndex`]: crate::ByteIndex
/// [`Range<ByteIndex>`]: std::ops::Range
/// [`Range<usize>`]: std::ops::Range
/// [`Range`]: std::ops::Range
/// [`std`]: std
pub trait ByteIndexRangeExt {
    /// Converts a [`Range<ByteIndex>`] to [`Range<usize>`] for slice indexing.
    ///
    /// This method provides the functionality that would ideally be available via
    /// `.into()`, but Rust's orphan rule prevents implementing the following:
    /// - `From<Range<ByteIndex>> for Range<usize>`
    /// - Why? Because the target type's head type [`Range`] is foreign (from [`std`]),
    ///   even though [`ByteIndex`] in the source type is from our crate.
    fn to_usize_range(self) -> Range<usize>;
}

impl ByteIndexRangeExt for Range<ByteIndex> {
    fn to_usize_range(self) -> Range<usize> { self.start.as_usize()..self.end.as_usize() }
}

/// Pull in [`ArrayBoundsCheck`] (default) implementation for type-safe bounds checking.
impl ArrayBoundsCheck<ByteLength> for ByteIndex {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{byte_offset, ch};

    // Basic construction and conversion tests.
    #[test]
    fn test_byte_index_from_usize() {
        let index = byte_index(42usize);
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

    // Conversion tests to other types.
    #[test]
    fn test_byte_index_to_usize() {
        let index = byte_index(42);
        let value: usize = index.into();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_byte_index_to_index() {
        let index = byte_index(42);
        let generic_index: VPIndex = index.into();
        assert_eq!(generic_index.as_usize(), 42);
    }

    // Critical ByteIndex <-> ByteOffset conversion tests.
    #[test]
    fn test_byte_index_to_byte_offset_conversion() {
        let index = byte_index(100);
        let offset: ByteOffset = index.into();
        assert_eq!(offset.as_usize(), 100);
    }

    #[test]
    fn test_byte_offset_from_byte_index_semantic() {
        // Semantic test: converting absolute position to relative offset.
        let absolute_position = byte_index(50);
        let relative_offset = byte_offset(absolute_position);
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

    // Edge case tests.
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

    #[test]
    fn test_clone() {
        let index1 = byte_index(42);
        let index2 = index1;
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
        use rustc_hash::FxHashSet;

        let mut set = FxHashSet::default();
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

    // Semantic correctness tests.
    #[test]
    fn test_semantic_absolute_position_usage() {
        // ByteIndex represents absolute positions in buffers/strings.
        let buffer_start = byte_index(0);
        let char_position = byte_index(5);
        let end_position = byte_index(100);

        assert!(buffer_start < char_position);
        assert!(char_position < end_position);

        // Converting to offset makes sense when position becomes relative.
        let relative_from_start: ByteOffset = char_position.into();
        assert_eq!(relative_from_start.as_usize(), 5);
    }

    // Constructor function tests.
    #[test]
    fn test_byte_index_constructor_function() {
        let index = byte_index(42usize);
        assert_eq!(index, ByteIndex::from(42u16));

        let index_from_ch = byte_index(ch(10));
        assert_eq!(index_from_ch, ByteIndex::from(ch(10)));
    }

    // Add trait tests.
    #[test]
    fn test_byte_index_addition() {
        let index1 = byte_index(10);
        let index2 = byte_index(20);
        let result = index1 + index2;
        assert_eq!(result, byte_index(30));
    }

    #[test]
    fn test_byte_index_add_assign() {
        let mut index = byte_index(10);
        let increment = byte_index(25);
        index += increment;
        assert_eq!(index, byte_index(35));

        // Test multiple increments
        index += byte_index(5);
        assert_eq!(index, byte_index(40));

        // Test zero increment
        index += byte_index(0);
        assert_eq!(index, byte_index(40));
    }

    #[test]
    fn test_byte_index_add_assign_byte_offset() {
        use crate::byte_offset;

        // Test basic position += displacement
        let mut position = byte_index(100);
        let displacement = byte_offset(50);
        position += displacement;
        assert_eq!(position, byte_index(150));

        // Test multiple displacements
        position += byte_offset(25);
        assert_eq!(position, byte_index(175));

        position += byte_offset(5);
        assert_eq!(position, byte_index(180));

        // Test zero displacement
        position += byte_offset(0);
        assert_eq!(position, byte_index(180));
    }

    #[test]
    fn test_byte_index_saturating_add() {
        let max_index = byte_index(usize::MAX);
        assert_eq!(max_index + 10, byte_index(usize::MAX));
        assert_eq!(max_index + byte_index(10), byte_index(usize::MAX));

        let mut index = byte_index(usize::MAX);
        index += 10;
        assert_eq!(index, byte_index(usize::MAX));
    }

    #[test]
    fn test_byte_index_subtraction() {
        use crate::byte_offset;

        let index = byte_index(30);
        assert_eq!(index - byte_index(10), byte_offset(20));
        assert_eq!(index - 10, byte_index(20));
    }

    #[test]
    fn test_byte_index_sub_assign() {
        let mut index = byte_index(30);
        index -= byte_index(10);
        assert_eq!(index, byte_index(20));

        index -= 5;
        assert_eq!(index, byte_index(15));
    }

    #[test]
    fn test_byte_index_saturating_sub() {
        use crate::byte_offset;

        let index = byte_index(10);
        assert_eq!(index - byte_index(30), byte_offset(0));
        assert_eq!(index - 30, byte_index(0));

        let mut index = byte_index(10);
        index -= 30;
        assert_eq!(index, byte_index(0));
    }

    #[test]
    fn test_byte_index_range_boundary_compatibility() {
        use crate::{RangeValidityStatus, bounds_check::RangeBoundsExt};
        use std::ops::Range;

        let start = byte_index(5);
        let end = byte_index(15);
        let length = crate::byte_len(20);

        let range: Range<ByteIndex> = start..end;
        assert_eq!(
            range.check_range_is_valid_for_length(length),
            RangeValidityStatus::Valid
        );

        let invalid_range: Range<ByteIndex> = byte_index(25)..byte_index(30);
        assert_eq!(
            invalid_range.check_range_is_valid_for_length(length),
            RangeValidityStatus::StartOutOfBounds
        );
    }

    #[test]
    fn test_range_conversion_to_usize() {
        let byte_range: Range<ByteIndex> = byte_index(5)..byte_index(10);
        let usize_range: Range<usize> = byte_range.to_usize_range();
        assert_eq!(usize_range, 5..10);

        // Test with zero start
        let zero_start_range: Range<ByteIndex> = byte_index(0)..byte_index(7);
        let zero_usize_range: Range<usize> = zero_start_range.to_usize_range();
        assert_eq!(zero_usize_range, 0..7);

        // Test empty range
        let empty_range: Range<ByteIndex> = byte_index(3)..byte_index(3);
        let empty_usize_range: Range<usize> = empty_range.to_usize_range();
        assert_eq!(empty_usize_range, 3..3);
    }
}
