// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Cursor positioning for text editing - see [`CursorBoundsCheck`] trait.

use super::{length_marker::LengthMarker, result_enums::CursorPositionBoundsStatus,
            unit_marker::UnitMarker};

/// Cursor end-of-line positioning semantics for text editing.
///
/// This trait provides cursor positioning utilities specifically for text editing
/// contexts where cursors can be placed at the end-of-line position (index == length).
///
/// > See [Interval Notation] in the [Module Documentation] for the mathematical range
/// > notation used in the diagrams.
///
/// ## Core Purpose
///
/// Use case: "Can a text cursor be placed at this position?"
///
/// This trait handles the special case in text editing where a cursor position
/// can equal the content length, representing the position "after" the last character.
/// This is distinct from array bounds checking where such positions are invalid.
///
/// # Key Distinction from Other Bounds Traits
///
/// | Trait                         | Rule                          | Use Case      | Example                                              |
/// |-------------------------------|-------------------------------|---------------|------------------------------------------------------|
/// | [`ArrayBoundsCheck`]          | `index < length`              | Index safety  | `buffer[5]` needs `5 < buffer.len()`                 |
/// | `CursorBoundsCheck`📍         | `index <= length`             | Text editing  | Cursor can be at position `length` (after last char) |
/// | [`ViewportBoundsCheck`]       | `start <= index < start+size` | Rendering     | Content visibility in windows                        |
/// | [`RangeBoundsCheck`]          | `start <= end <= length`      | Iteration     | Range object structural validation                   |
///
/// ## Cursor Positioning Semantics
///
/// In text editing, cursors have special positioning rules that differ from array access:
///
/// ```text
/// Text content: "hello" (length=5)
///
///             ╭── length=5 ───╮
///             │   (1-based)   │
/// Index:      0   1   2   3   4   5
/// (0-based) ┌───┬───┬───┬───┬───┬───┐
/// Content:  │ h │ e │ l │ l │ o │ ▓ │
///           └───┴───┴───┴───┴───┴───┘
///             ╰─valid indices─╯   │
///             ╰─valid cursor──────╯
///               positions         ↑
///                        "after last position"
///
/// Array access: indices 0-4 are valid (index < length)
/// Cursor positions: indices 0-5 are valid (index <= length)
/// ```
///
/// Position 5 is invalid for array access (`text[5]` would panic) but valid for
/// cursor placement (cursor after the last character).
///
/// # Semantic Meaning
///
/// For content of length N, valid indices are 0..N-1 for content access, but
/// position N is valid for cursor placement and range boundaries:
///
/// ```text
///           ╭── length=5 ───╮
/// Index:    0   1   2   3   4   5
///         ┌───┬───┬───┬───┬───┬───┐
/// Content:│ h │ e │ l │ l │ o │ ! │
///         └───┴───┴───┴───┴───┴───┘
///           ╰─valid indices─╯   │
///           ╰───────────────────╯ valid cursor positions
///                               ↑
///                      "after last position"
/// ```
///
/// ## Primary Use Cases
///
/// This trait is essential for:
/// - Cursor positioning after the last character in text editing
/// - Range operations with exclusive end semantics
/// - Natural text editor interactions where cursors sit after content
/// - Text cursor positioning: Where to place the text insertion cursor
/// - Range end boundaries: Exclusive range ends in text processing
/// - Navigation operations: End key, append operations
/// - Selection boundaries: Text selection endpoint validation
///
/// ## Relationship to Other Bounds Checking
///
/// This trait provides the foundation for cursor-aware operations:
/// - Before text operations: Use cursor bounds to validate positions
/// - During array access: Use array bounds to ensure correct content operations
/// - For range operations: Use range validation to ensure structural validity
///
/// ## See Also
///
/// - [`ArrayBoundsCheck`] - Array access safety with strict boundaries
/// - [`ViewportBoundsCheck`] - Viewport visibility checking
/// - [`RangeBoundsCheck`] - Range validation for iteration and algorithms
/// - [`IndexMarker`] - Index types that use cursor positioning
/// - [Module documentation] - Overview of the complete bounds checking architecture
///
/// [`ArrayBoundsCheck`]: crate::ArrayBoundsCheck
/// [`ViewportBoundsCheck`]: crate::ViewportBoundsCheck
/// [`RangeBoundsCheck`]: crate::RangeBoundsCheck
/// [`IndexMarker`]: crate::IndexMarker
/// [Module documentation]: mod@crate::core::units::bounds_check
/// [Interval Notation]: mod@crate::core::units::bounds_check#interval-notation
///
/// # Examples
///
/// ```
/// use r3bl_tui::{CursorBoundsCheck, ColWidth, col, width};
///
/// let w = width(5);
/// assert_eq!(w.eol_cursor_position(), col(5));
///
/// let zero_w = width(0);
/// assert_eq!(zero_w.eol_cursor_position(), col(0));
/// ```
pub trait CursorBoundsCheck: LengthMarker {
    /// Get the cursor position at end-of-line (after the last character).
    ///
    /// This is the position where a cursor can be placed to continue typing,
    /// equivalent to where the cursor sits after pressing End in a text editor.
    /// For content of length N, this returns position N.
    ///
    /// This position is fundamental for text editing operations as it represents
    /// the natural place where new text would be appended to existing content.
    ///
    /// Returns the position where index equals the content length.
    fn eol_cursor_position(&self) -> Self::IndexType;

    /// Check if a cursor position is valid for this line/buffer.
    ///
    /// See the [Interval Notation] section in the module documentation for notation
    /// details.
    ///
    /// Returns true for positions in the range [0, length] (inclusive of EOL position).
    /// This allows cursors to be positioned anywhere from the start to after the last
    /// character.
    ///
    /// [Interval Notation]: mod@crate::core::units::bounds_check#interval-notation
    fn is_valid_cursor_position(&self, pos: Self::IndexType) -> bool;

    /// Clamp a cursor position to valid bounds for this line/buffer.
    ///
    /// Ensures the cursor position is valid for text editing operations.
    /// Positions beyond the EOL are clamped to the EOL position.
    fn clamp_cursor_position(&self, pos: Self::IndexType) -> Self::IndexType;

    /// Performs cursor position bounds checking for text editing contexts.
    ///
    /// This method validates whether a position is suitable for cursor placement,
    /// allowing the cursor to be positioned at the end-of-line (after the last
    /// character). This is distinct from array access checking where such positions
    /// would be invalid.
    ///
    /// ```text
    /// Cursor position checking:
    ///
    /// Self (Length)
    /// Position:   0   1   2   3   4   5   6   7   8   9   10  11
    /// (0-based) ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///           │ S │ W │ W │ W │ W │ W │ W │ W │ W │ W │ E │ B │
    ///           ├─▲─┴─▲─┴───┴───┴───┴───┴───┴───┴───┴─▲─┴─▲─┼─▲─┘
    ///           │ │   │                               │   │ │ │
    ///           │Start│                               │  End│Beyond
    ///           │     └────────── Within ─────────────┘     │
    ///           └───────────── content_length=10 ───────────┘
    ///
    /// S = AtStart (position=0)
    /// W = Within (1 ≤ position < 10)
    /// E = AtEnd (position=10)
    /// B = Beyond (position > 10)
    /// ```
    ///
    /// # Returns
    /// [`CursorPositionBoundsStatus`] indicating whether the position is within content,
    /// at a content boundary, or beyond content boundaries.
    fn check_cursor_position_bounds(
        &self,
        pos: Self::IndexType,
    ) -> CursorPositionBoundsStatus;
}

/// Blanket implementation for all types that implement `LengthMarker`.
///
/// This provides consistent EOL cursor positioning for all length types
/// (`Length`, `ColWidth`, `RowHeight`) without code duplication.
impl<T: LengthMarker> CursorBoundsCheck for T
where
    T: Copy,
    T::IndexType: From<usize>
        + std::ops::Add<Output = T::IndexType>
        + PartialOrd
        + Copy
        + UnitMarker,
{
    fn eol_cursor_position(&self) -> Self::IndexType {
        let length_val = self.as_usize();

        if length_val == 0 {
            // Use From<usize> for type-safe construction.
            T::IndexType::from(0_usize)
        } else {
            // Normal case: last valid index + 1.
            self.convert_to_index() + T::IndexType::from(1_usize)
        }
    }

    fn is_valid_cursor_position(&self, pos: Self::IndexType) -> bool {
        // Position is valid if it's not beyond the boundary
        self.check_cursor_position_bounds(pos) != CursorPositionBoundsStatus::Beyond
    }

    fn clamp_cursor_position(&self, pos: Self::IndexType) -> Self::IndexType {
        if self.is_valid_cursor_position(pos) {
            pos
        } else {
            self.eol_cursor_position()
        }
    }

    fn check_cursor_position_bounds(
        &self,
        pos: Self::IndexType,
    ) -> CursorPositionBoundsStatus {
        let position = pos.as_usize();
        let length = self.as_usize();

        if position > length {
            CursorPositionBoundsStatus::Beyond
        } else if position == 0 {
            CursorPositionBoundsStatus::AtStart
        } else if position == length {
            CursorPositionBoundsStatus::AtEnd
        } else {
            CursorPositionBoundsStatus::Within
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColIndex, ColWidth, RowHeight, RowIndex, idx, len};

    mod eol_cursor_position_tests {
        use super::*;

        #[test]
        fn test_eol_cursor_position_trait() {
            // Test with ColWidth.
            {
                let width_5 = ColWidth::new(5);
                assert_eq!(
                    width_5.eol_cursor_position(),
                    ColIndex::new(5),
                    "Width 5 should give boundary position at col 5"
                );

                let width_0 = ColWidth::new(0);
                assert_eq!(
                    width_0.eol_cursor_position(),
                    ColIndex::new(0),
                    "Zero width should give position 0"
                );

                let width_1 = ColWidth::new(1);
                assert_eq!(
                    width_1.eol_cursor_position(),
                    ColIndex::new(1),
                    "Width 1 should give boundary position at col 1"
                );
            }

            // Test with RowHeight.
            {
                let height_3 = RowHeight::new(3);
                assert_eq!(
                    height_3.eol_cursor_position(),
                    RowIndex::new(3),
                    "Height 3 should give boundary position at row 3"
                );

                let height_0 = RowHeight::new(0);
                assert_eq!(
                    height_0.eol_cursor_position(),
                    RowIndex::new(0),
                    "Zero height should give position 0"
                );
            }

            // Test with generic Length.
            {
                let len_10 = len(10);
                assert_eq!(
                    len_10.eol_cursor_position(),
                    idx(10),
                    "Length 10 should give boundary position at index 10"
                );

                let len_0 = len(0);
                assert_eq!(
                    len_0.eol_cursor_position(),
                    idx(0),
                    "Zero length should give position 0"
                );
            }
        }

        #[test]
        fn test_is_valid_cursor_position_trait() {
            let content_length = len(5);

            // Within boundary
            assert!(content_length.is_valid_cursor_position(idx(0)));
            assert!(content_length.is_valid_cursor_position(idx(3)));
            assert!(content_length.is_valid_cursor_position(idx(5))); // EOL position

            // Beyond boundary
            assert!(!content_length.is_valid_cursor_position(idx(6)));
            assert!(!content_length.is_valid_cursor_position(idx(10)));
        }

        #[test]
        fn test_clamp_cursor_position_trait() {
            let content_length = len(5);

            // Within boundary - no change
            assert_eq!(content_length.clamp_cursor_position(idx(0)), idx(0));
            assert_eq!(content_length.clamp_cursor_position(idx(3)), idx(3));
            assert_eq!(content_length.clamp_cursor_position(idx(5)), idx(5));

            // Beyond boundary - clamp to boundary
            assert_eq!(content_length.clamp_cursor_position(idx(6)), idx(5));
            assert_eq!(content_length.clamp_cursor_position(idx(10)), idx(5));
        }

        #[test]
        fn test_boundary_semantic_equivalence() {
            // Verify the semantic: eol_cursor_position() == convert_to_index()
            // + 1 (for non-zero).
            for i in 1..=10 {
                let w = ColWidth::new(i);
                let expected = w.convert_to_index() + ColIndex::new(1);
                let actual = w.eol_cursor_position();
                assert_eq!(
                    actual, expected,
                    "For width {i}, boundary should be convert_to_index() + 1"
                );
            }

            // Verify zero edge case
            let zero_width = ColWidth::new(0);
            assert_eq!(
                zero_width.eol_cursor_position(),
                ColIndex::new(0),
                "Zero width should give position 0, not -1 or error"
            );
        }
    }

    mod check_cursor_position_bounds_tests {
        use super::*;

        #[test]
        fn test_check_cursor_position_bounds_basic() {
            let content_length = len(5);

            // At start.
            assert_eq!(
                content_length.check_cursor_position_bounds(idx(0)),
                CursorPositionBoundsStatus::AtStart
            );

            // Within content.
            assert_eq!(
                content_length.check_cursor_position_bounds(idx(2)),
                CursorPositionBoundsStatus::Within
            );
            assert_eq!(
                content_length.check_cursor_position_bounds(idx(4)),
                CursorPositionBoundsStatus::Within
            );

            // At end boundary.
            assert_eq!(
                content_length.check_cursor_position_bounds(idx(5)),
                CursorPositionBoundsStatus::AtEnd
            );

            // Beyond content.
            assert_eq!(
                content_length.check_cursor_position_bounds(idx(6)),
                CursorPositionBoundsStatus::Beyond
            );
            assert_eq!(
                content_length.check_cursor_position_bounds(idx(10)),
                CursorPositionBoundsStatus::Beyond
            );
        }

        #[test]
        fn test_check_cursor_position_bounds_edge_cases() {
            // Zero-length content - AtStart takes precedence.
            let zero_length = len(0);
            assert_eq!(
                zero_length.check_cursor_position_bounds(idx(0)),
                CursorPositionBoundsStatus::AtStart
            );
            assert_eq!(
                zero_length.check_cursor_position_bounds(idx(1)),
                CursorPositionBoundsStatus::Beyond
            );

            // Single element content.
            let single_length = len(1);
            assert_eq!(
                single_length.check_cursor_position_bounds(idx(0)),
                CursorPositionBoundsStatus::AtStart
            );
            assert_eq!(
                single_length.check_cursor_position_bounds(idx(1)),
                CursorPositionBoundsStatus::AtEnd
            );
            assert_eq!(
                single_length.check_cursor_position_bounds(idx(2)),
                CursorPositionBoundsStatus::Beyond
            );
        }

        #[test]
        fn test_check_cursor_position_bounds_with_typed_indices() {
            // Test with ColIndex/ColWidth.
            let col_width = ColWidth::new(3);
            assert_eq!(
                col_width.check_cursor_position_bounds(ColIndex::new(0)),
                CursorPositionBoundsStatus::AtStart
            );
            assert_eq!(
                col_width.check_cursor_position_bounds(ColIndex::new(2)),
                CursorPositionBoundsStatus::Within
            );
            assert_eq!(
                col_width.check_cursor_position_bounds(ColIndex::new(3)),
                CursorPositionBoundsStatus::AtEnd
            );
            assert_eq!(
                col_width.check_cursor_position_bounds(ColIndex::new(4)),
                CursorPositionBoundsStatus::Beyond
            );

            // Test with RowIndex/RowHeight.
            let row_height = RowHeight::new(2);
            assert_eq!(
                row_height.check_cursor_position_bounds(RowIndex::new(0)),
                CursorPositionBoundsStatus::AtStart
            );
            assert_eq!(
                row_height.check_cursor_position_bounds(RowIndex::new(1)),
                CursorPositionBoundsStatus::Within
            );
            assert_eq!(
                row_height.check_cursor_position_bounds(RowIndex::new(2)),
                CursorPositionBoundsStatus::AtEnd
            );
            assert_eq!(
                row_height.check_cursor_position_bounds(RowIndex::new(3)),
                CursorPositionBoundsStatus::Beyond
            );
        }
    }
}
