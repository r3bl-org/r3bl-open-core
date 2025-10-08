// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Relative byte displacement from a reference point - see [`ByteOffset`] type.

use super::ByteIndex;
use crate::{ChUnit, Index, Length, RowIndex};
use std::ops::{Add, Deref, DerefMut, Sub};

/// Represents a byte offset within a line or buffer segment.
///
/// A `ByteOffset` represents a relative distance in bytes from a starting position,
/// as opposed to [`ByteIndex`] which represents an absolute position within a buffer.
/// This distinction is crucial for maintaining semantic correctness in operations.
///
/// > This newtype struct does not use [`ChUnit`] like other unit types because
/// > offsets are inherently [`usize`].
///
/// # Type System Disambiguation
///
/// `ByteOffset` is conceptually distinct from both indices and lengths in the type
/// system:
/// - **Not an Index**: Unlike [`ByteIndex`], it doesn't represent an absolute position
/// - **Not a Length**: Unlike length types (e.g., [`LengthOps`]), it doesn't represent an
///   extent or size
/// - **Is a Displacement**: Represents a relative displacement/distance from a reference
///   point
///
/// Think of it as:
/// - Index = absolute coordinate (like "3rd Street")
/// - Length = extent/size (like "10 blocks long")
/// - Offset = displacement (like "5 blocks east from here")
///
/// # Semantic Usage
/// - Use `ByteOffset` for positions relative to line start (0-based within line)
/// - Use `ByteIndex` for absolute positions in the global buffer
/// - Arithmetic: `ByteIndex + ByteOffset = ByteIndex` (position + distance = new
///   position)
/// - Arithmetic: `ByteIndex - ByteIndex = ByteOffset` (position - position = distance)
///
/// # Examples
///
/// ```rust
/// use r3bl_tui::{ByteIndex, ByteOffset, byte_index, byte_offset};
///
/// // Line starts at byte 100 in buffer
/// let line_start = byte_index(100);
///
/// // Want to insert at byte 5 within the line
/// let position_in_line = byte_offset(5);
///
/// // Calculate absolute insertion position
/// let absolute_position = line_start + position_in_line;
/// assert_eq!(absolute_position.as_usize(), 105);
/// ```
///
/// [`LengthOps`]: crate::LengthOps
#[derive(Debug, Copy, Clone, Default, PartialEq, Ord, PartialOrd, Eq, Hash)]
pub struct ByteOffset(pub usize);

impl ByteOffset {
    #[must_use]
    pub fn as_usize(&self) -> usize { self.0 }
}

/// Creates a new [`ByteOffset`] from any type that can be converted into it.
///
/// This is a convenience function that is equivalent to calling [`ByteOffset::from`].
///
/// # Examples
///
/// ```rust
/// use r3bl_tui::{ByteOffset, byte_offset};
///
/// let offset = byte_offset(42);
/// assert_eq!(offset, ByteOffset::from(42usize));
/// ```
pub fn byte_offset(arg_byte_offset: impl Into<ByteOffset>) -> ByteOffset {
    arg_byte_offset.into()
}

impl Deref for ByteOffset {
    type Target = usize;
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for ByteOffset {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl From<usize> for ByteOffset {
    fn from(it: usize) -> Self { Self(it) }
}

impl From<ChUnit> for ByteOffset {
    fn from(it: ChUnit) -> Self { Self(crate::usize(it)) }
}

impl From<ByteOffset> for Index {
    fn from(it: ByteOffset) -> Self { Self::from(it.0) }
}

impl From<ByteOffset> for RowIndex {
    fn from(it: ByteOffset) -> Self { RowIndex::from(Index::from(it)) }
}

impl From<ByteIndex> for ByteOffset {
    fn from(it: ByteIndex) -> Self { Self(it.as_usize()) }
}

impl From<Length> for ByteOffset {
    fn from(it: Length) -> Self { Self(it.as_usize()) }
}

// Arithmetic operations between ByteIndex and ByteOffset.
impl Add<ByteOffset> for ByteIndex {
    type Output = ByteIndex;

    /// Add a byte offset to an absolute byte position.
    ///
    /// This represents moving forward from an absolute position by a relative distance.
    /// Semantically: `absolute_position + offset = new_absolute_position`
    fn add(self, rhs: ByteOffset) -> Self::Output {
        ByteIndex::from(self.as_usize() + rhs.as_usize())
    }
}

impl Sub<ByteOffset> for ByteIndex {
    type Output = ByteIndex;

    /// Subtract a byte offset from an absolute byte position.
    ///
    /// This represents moving backward from an absolute position by a relative distance.
    /// Semantically: `absolute_position - offset = new_absolute_position`
    fn sub(self, rhs: ByteOffset) -> Self::Output {
        ByteIndex::from(self.as_usize().saturating_sub(rhs.as_usize()))
    }
}

impl Sub<ByteIndex> for ByteIndex {
    type Output = ByteOffset;

    /// Calculate the distance between two absolute byte positions.
    ///
    /// This represents finding the offset/distance from one position to another.
    /// Semantically: `position - position = distance`
    fn sub(self, rhs: ByteIndex) -> Self::Output {
        ByteOffset::from(self.as_usize().saturating_sub(rhs.as_usize()))
    }
}

impl Add<ByteOffset> for ByteOffset {
    type Output = ByteOffset;

    /// Add two byte offsets together.
    ///
    /// This represents combining two relative distances.
    /// Semantically: `offset + offset = combined_offset`
    fn add(self, rhs: ByteOffset) -> Self::Output {
        ByteOffset::from(self.as_usize() + rhs.as_usize())
    }
}

impl Sub<ByteOffset> for ByteOffset {
    type Output = ByteOffset;

    /// Subtract one byte offset from another.
    ///
    /// This represents finding the difference between two relative distances.
    /// Semantically: `offset - offset = offset_difference`
    fn sub(self, rhs: ByteOffset) -> Self::Output {
        ByteOffset::from(self.as_usize().saturating_sub(rhs.as_usize()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{byte_index, ch};

    // Basic construction and conversion tests.
    #[test]
    fn test_byte_offset_from_usize() {
        let offset = ByteOffset::from(42usize);
        assert_eq!(offset.as_usize(), 42);
    }

    #[test]
    fn test_byte_offset_from_ch_unit() {
        let ch_unit = ch(10);
        let offset = ByteOffset::from(ch_unit);
        assert_eq!(offset.as_usize(), 10);
    }

    #[test]
    fn test_byte_offset_as_usize() {
        let offset = byte_offset(25);
        assert_eq!(offset.as_usize(), 25);
    }

    #[test]
    fn test_byte_offset_deref() {
        let offset = byte_offset(15);
        let value = *offset;
        assert_eq!(value, 15);
    }

    #[test]
    fn test_byte_offset_deref_mut() {
        let mut offset = byte_offset(20);
        *offset = 30;
        assert_eq!(offset.as_usize(), 30);
    }

    // Conversion tests.
    #[test]
    fn test_byte_offset_to_index() {
        let offset = byte_offset(42);
        let index: Index = offset.into();
        assert_eq!(index.as_usize(), 42);
    }

    #[test]
    fn test_byte_offset_to_row_index() {
        let offset = byte_offset(42);
        let row_index: RowIndex = offset.into();
        assert_eq!(row_index.as_usize(), 42);
    }

    // ByteOffset + ByteOffset arithmetic tests.
    #[test]
    fn test_offset_add_offset() {
        let offset1 = byte_offset(10);
        let offset2 = byte_offset(15);
        let result = offset1 + offset2;
        assert_eq!(result, byte_offset(25));
    }

    #[test]
    fn test_offset_sub_offset() {
        let offset1 = byte_offset(20);
        let offset2 = byte_offset(5);
        let result = offset1 - offset2;
        assert_eq!(result, byte_offset(15));
    }

    #[test]
    fn test_offset_sub_saturating() {
        let offset1 = byte_offset(5);
        let offset2 = byte_offset(10);
        let result = offset1 - offset2;
        assert_eq!(result, byte_offset(0)); // saturating_sub prevents underflow
    }

    // Critical cross-type arithmetic tests (ByteIndex + ByteOffset).
    #[test]
    fn test_index_add_offset() {
        let index = byte_index(100);
        let offset = byte_offset(50);
        let result = index + offset;
        assert_eq!(result, byte_index(150));
    }

    #[test]
    fn test_index_sub_offset() {
        let index = byte_index(100);
        let offset = byte_offset(30);
        let result = index - offset;
        assert_eq!(result, byte_index(70));
    }

    #[test]
    fn test_index_sub_offset_saturating() {
        let index = byte_index(20);
        let offset = byte_offset(50);
        let result = index - offset;
        assert_eq!(result, byte_index(0)); // saturating_sub prevents underflow
    }

    #[test]
    fn test_index_sub_index_gives_offset() {
        let index1 = byte_index(100);
        let index2 = byte_index(40);
        let offset: ByteOffset = index1 - index2;
        assert_eq!(offset, byte_offset(60));
    }

    #[test]
    fn test_index_sub_index_gives_offset_saturating() {
        let index1 = byte_index(30);
        let index2 = byte_index(50);
        let offset: ByteOffset = index1 - index2;
        assert_eq!(offset, byte_offset(0)); // saturating_sub prevents underflow
    }

    // Edge case tests.
    #[test]
    fn test_zero_offset_operations() {
        let zero_offset = byte_offset(0);
        let some_offset = byte_offset(42);
        let some_index = byte_index(100);

        // Adding zero should not change value.
        assert_eq!(some_index + zero_offset, some_index);
        assert_eq!(some_offset + zero_offset, some_offset);

        // Subtracting zero should not change value.
        assert_eq!(some_index - zero_offset, some_index);
        assert_eq!(some_offset - zero_offset, some_offset);
    }

    #[test]
    fn test_large_offset_operations() {
        let large_offset = byte_offset(usize::MAX / 2);
        let another_offset = byte_offset(10);

        // Test that large operations don't panic.
        _ = large_offset + another_offset;
        _ = large_offset - another_offset;
    }

    // Trait implementation tests.
    #[test]
    fn test_debug_format() {
        let offset = byte_offset(42);
        let debug_str = format!("{offset:?}");
        assert!(debug_str.contains("ByteOffset"));
        assert!(debug_str.contains("42"));
    }

    #[test]
    fn test_clone() {
        let offset1 = byte_offset(42);
        let offset2 = offset1;
        assert_eq!(offset1, offset2);
    }

    #[test]
    fn test_copy() {
        let offset1 = byte_offset(42);
        let offset2 = offset1; // Copy semantics
        assert_eq!(offset1, offset2);
    }

    #[test]
    fn test_equality() {
        let offset1 = byte_offset(42);
        let offset2 = byte_offset(42);
        let offset3 = byte_offset(24);

        assert_eq!(offset1, offset2);
        assert_ne!(offset1, offset3);
    }

    #[test]
    fn test_ordering() {
        let offset1 = byte_offset(10);
        let offset2 = byte_offset(20);
        let offset3 = byte_offset(10);

        assert!(offset1 < offset2);
        assert!(offset2 > offset1);
        assert!(offset1 <= offset3);
        assert!(offset1 >= offset3);
    }

    #[test]
    fn test_default() {
        let offset = ByteOffset::default();
        assert_eq!(offset, byte_offset(0));
    }

    #[test]
    fn test_from_index_conversion() {
        let offset = byte_offset(42);
        let index: Index = offset.into();
        assert_eq!(index.as_usize(), 42);
    }

    // ByteIndex <-> ByteOffset conversion tests.
    #[test]
    fn test_byte_offset_from_byte_index() {
        let byte_index = byte_index(100);
        let offset = ByteOffset::from(byte_index);
        assert_eq!(offset.as_usize(), 100);
    }

    #[test]
    fn test_byte_offset_from_byte_index_into() {
        let byte_index = byte_index(50);
        let offset: ByteOffset = byte_index.into();
        assert_eq!(offset, byte_offset(50));
    }

    #[test]
    fn test_byte_index_to_byte_offset_semantic_conversion() {
        // Converting absolute position within line to relative offset.
        let segment_start_position = byte_index(25); // Position 25 in line content
        let as_line_relative_offset: ByteOffset = segment_start_position.into();

        assert_eq!(as_line_relative_offset.as_usize(), 25);

        // Should be usable with line position arithmetic.
        let line_buffer_start = byte_index(1000);
        let absolute_position = line_buffer_start + as_line_relative_offset;
        assert_eq!(absolute_position, byte_index(1025));
    }

    #[test]
    fn test_byte_index_to_byte_offset_roundtrip_values() {
        let original_positions = [0, 1, 10, 100, 1000, usize::MAX / 2];

        for pos in original_positions {
            let byte_index = byte_index(pos);
            let byte_offset: ByteOffset = byte_index.into();

            assert_eq!(byte_index.as_usize(), pos);
            assert_eq!(byte_offset.as_usize(), pos);
            assert_eq!(byte_index.as_usize(), byte_offset.as_usize());
        }
    }

    #[test]
    fn test_byte_index_to_byte_offset_zero_case() {
        let zero_index = byte_index(0);
        let zero_offset: ByteOffset = zero_index.into();

        assert_eq!(zero_index.as_usize(), 0);
        assert_eq!(zero_offset.as_usize(), 0);
        assert_eq!(zero_offset, byte_offset(0));
    }

    // Semantic correctness tests - demonstrating the type safety improvements.
    #[test]
    fn test_semantic_position_plus_distance() {
        // Semantic test: absolute position + relative distance = new absolute position.
        let line_start_position = byte_index(1000); // Position in global buffer
        let position_within_line = byte_offset(25); // Distance from line start
        let absolute_position = line_start_position + position_within_line;

        assert_eq!(absolute_position, byte_index(1025));
    }

    #[test]
    fn test_semantic_distance_between_positions() {
        // Semantic test: position - position = distance.
        let end_position = byte_index(150);
        let start_position = byte_index(100);
        let distance: ByteOffset = end_position - start_position;

        assert_eq!(distance, byte_offset(50));
    }
}
