// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! After-last position utilities for cursor positioning.
//!
//! This module provides the [`AfterLastPosition`] trait for converting length values
//! to their "after last" position, essential for cursor/caret positioning in text
//! editors and terminal emulators.
//!
//! The "after last" position (index == length) allows cursors to be placed after
//! the last character in a line, which is critical for text editing operations.

use super::length_and_index_markers::LengthMarker;

/// Trait for converting length values to their "after last" position.
///
/// The "after last" position (index == length) is essential for cursor/caret
/// positioning in text editors and terminal emulators, where the cursor can
/// be placed after the last character in a line.
///
/// # Semantic Meaning
///
/// For a length of N, valid indices are 0..N-1 for content access, but
/// position N is valid for cursor placement:
///
/// ```text
/// Content: "hello"
/// Length:  5
/// Valid indices:     0, 1, 2, 3, 4  (for character access)
/// Valid positions:   0, 1, 2, 3, 4, 5  (for cursor placement)
///                                   ↑
///                          "after last" position
/// ```
///
/// Here's an example:
/// ```text
/// R ┌──────────┐
/// 0 ❱hello░    │
///   └─────⮬────┘
///   C0123456789
/// ```
///
/// # Edge Case Handling
///
/// This trait handles the zero-length edge case specially:
/// - For length > 0: Returns `convert_to_index() + 1`
/// - For length == 0: Returns index 0 (the only valid position)
///
/// # Examples
///
/// ```
/// use r3bl_tui::{ColWidth, AfterLastPosition, width, col};
///
/// let w = width(5);
/// assert_eq!(w.to_after_last_position(), col(5));
///
/// let zero_w = width(0);
/// assert_eq!(zero_w.to_after_last_position(), col(0));
/// ```
pub trait AfterLastPosition: LengthMarker {
    /// Convert this length to its "after last" position.
    ///
    /// Returns the position where a cursor can be placed after all content,
    /// which is `index == length` for non-empty content.
    fn to_after_last_position(&self) -> Self::IndexType;
}

/// Blanket implementation for all types that implement `LengthMarker`.
///
/// This provides consistent "after last" position calculation for all
/// length types (Length, `ColWidth`, `RowHeight`) without code duplication.
impl<T: LengthMarker> AfterLastPosition for T
where
    T::IndexType: From<usize> + std::ops::Add<Output = T::IndexType>,
{
    fn to_after_last_position(&self) -> Self::IndexType {
        let length_val = self.as_usize();

        if length_val == 0 {
            // Use From<usize> for type-safe construction.
            T::IndexType::from(0)
        } else {
            // Normal case: last valid index + 1.
            self.convert_to_index() + T::IndexType::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColIndex, ColWidth, RowHeight, RowIndex, idx, len};

    #[test]
    fn test_after_last_position_trait() {
        // Test with ColWidth.
        {
            let width_5 = ColWidth::new(5);
            assert_eq!(
                width_5.to_after_last_position(),
                ColIndex::new(5),
                "Width 5 should give after-last position at col 5"
            );

            let width_0 = ColWidth::new(0);
            assert_eq!(
                width_0.to_after_last_position(),
                ColIndex::new(0),
                "Zero width should give position 0"
            );

            let width_1 = ColWidth::new(1);
            assert_eq!(
                width_1.to_after_last_position(),
                ColIndex::new(1),
                "Width 1 should give after-last position at col 1"
            );
        }

        // Test with RowHeight.
        {
            let height_3 = RowHeight::new(3);
            assert_eq!(
                height_3.to_after_last_position(),
                RowIndex::new(3),
                "Height 3 should give after-last position at row 3"
            );

            let height_0 = RowHeight::new(0);
            assert_eq!(
                height_0.to_after_last_position(),
                RowIndex::new(0),
                "Zero height should give position 0"
            );
        }

        // Test with generic Length.
        {
            let len_10 = len(10);
            assert_eq!(
                len_10.to_after_last_position(),
                idx(10),
                "Length 10 should give after-last position at index 10"
            );

            let len_0 = len(0);
            assert_eq!(
                len_0.to_after_last_position(),
                idx(0),
                "Zero length should give position 0"
            );
        }

        // Edge case: very large values (within u16 range).
        {
            let large_width = ColWidth::new(1000);
            assert_eq!(
                large_width.to_after_last_position(),
                ColIndex::new(1000),
                "Large width should still calculate correctly"
            );
        }
    }

    #[test]
    fn test_after_last_position_semantic_equivalence() {
        // Verify the semantic: to_after_last_position() == convert_to_index() + 1 (for
        // non-zero).
        for i in 1..=10 {
            let w = ColWidth::new(i);
            let expected = w.convert_to_index() + ColIndex::new(1);
            let actual = w.to_after_last_position();
            assert_eq!(
                actual, expected,
                "For width {i}, after-last should be convert_to_index() + 1"
            );
        }

        // Verify zero edge case
        let zero_width = ColWidth::new(0);
        assert_eq!(
            zero_width.to_after_last_position(),
            ColIndex::new(0),
            "Zero width should give position 0, not -1 or error"
        );
    }
}
