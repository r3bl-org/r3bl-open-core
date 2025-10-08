// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Cursor positioning for text editing - see [`CursorBoundsCheck`] trait.

use super::{length_ops::LengthOps, numeric_value::NumericValue,
            result_enums::CursorPositionBoundsStatus};
use std::ops::Add;

/// Trait for 0-based position/index types, providing text cursor positioning with
/// end-of-line semantics.
///
/// This trait provides cursor positioning utilities specifically for text editing
/// contexts where cursors can be placed at the end-of-line position (index == length).
///
/// > <div class="warning">
/// >
/// > This trait is automatically implemented for all types that implement [`LengthOps`]
/// > through a [blanket implementation](#implementors). You can use this trait on
/// > those types without writing any implementation code yourself.
/// >
/// > </div>
///
/// ## Purpose
///
/// This trait answers the question: **"Can a text cursor be placed
/// at this position?"**
///
/// This trait handles the special case in text editing where a cursor position
/// can equal the content length, which is distinct from array bounds checking where
/// such positions are invalid.
///
/// ## Key Trait Capabilities
///
/// - **EOL positioning**: Get cursor position after last character via
///   [`eol_cursor_position()`]
/// - **Position validation**: Check if positions are valid for cursors via
///   [`is_valid_cursor_position()`]
/// - **Position clamping**: Clamp positions to valid cursor bounds via
///   [`clamp_cursor_position()`]
/// - **Detailed status**: Get comprehensive bounds status via
///   [`check_cursor_position_bounds()`]
///
/// ## Implementing Types
///
/// The following length types have this trait automatically available:
///
/// - [`Length`] - Generic 1-based size (dimension-agnostic)
/// - [`RowHeight`] - Vertical size in terminal grid (number of rows)
/// - [`ColWidth`] - Horizontal size in terminal grid (number of columns)
/// - [`ByteLength`] - Byte count in UTF-8 strings
/// - [`SegLength`] - Grapheme segment count
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
/// ## Key Distinction from Other Bounds Traits
///
/// | Trait                         | Rule                          | Use Case      | Example                                              |
/// |-------------------------------|-------------------------------|---------------|------------------------------------------------------|
/// | [`ArrayBoundsCheck`]          | `index < length`              | Index safety  | `buffer[5]` needs `5 < buffer.len()`                 |
/// | `CursorBoundsCheck`📍         | `index <= length`             | Text editing  | Cursor can be at position `length` (after last char) |
/// | [`ViewportBoundsCheck`]       | `start <= index < start+size` | Rendering     | Content visibility in windows                        |
/// | [`RangeBoundsExt`]          | `start <= end <= length`      | Iteration     | Range object structural validation                   |
///
/// ## Primary Use Cases
///
/// This trait is essential for:
/// - Text cursor positioning after the last character (EOL position)
/// - Range end boundaries with exclusive end semantics
/// - Navigation operations (End key, append operations)
/// - Selection boundaries and text selection endpoint validation
///
/// ## Examples
///
/// The `CursorBoundsCheck` trait provides comprehensive cursor positioning:
///
/// ```
/// use r3bl_tui::{CursorBoundsCheck, CursorPositionBoundsStatus, width, col};
///
/// let line_width = width(5);
///
/// // Get end-of-line cursor position (after last character)
/// let eol_pos = line_width.eol_cursor_position();
/// assert_eq!(eol_pos, col(5));  // Position after index 4
///
/// // Validate cursor positions
/// assert!(line_width.is_valid_cursor_position(col(0)));  // Start
/// assert!(line_width.is_valid_cursor_position(col(3)));  // Middle
/// assert!(line_width.is_valid_cursor_position(col(5)));  // EOL (valid!)
/// assert!(!line_width.is_valid_cursor_position(col(6))); // Beyond
///
/// // Clamp positions to valid bounds
/// assert_eq!(line_width.clamp_cursor_position(col(3)), col(3));  // Within
/// assert_eq!(line_width.clamp_cursor_position(col(10)), col(5)); // Clamped to EOL
///
/// // Detailed status checking
/// match line_width.check_cursor_position_bounds(col(5)) {
///     CursorPositionBoundsStatus::AtEnd => { /* cursor at EOL */ }
///     CursorPositionBoundsStatus::Within => { /* cursor on character */ }
///     CursorPositionBoundsStatus::AtStart => { /* cursor at start */ }
///     CursorPositionBoundsStatus::Beyond => { /* invalid position */ }
/// }
/// ```
///
/// ## See Also
///
/// - [`ArrayBoundsCheck`] - Array access safety with strict boundaries
/// - [`ViewportBoundsCheck`] - Viewport visibility checking
/// - [`RangeBoundsExt`] - Range validation for iteration and algorithms
/// - [`IndexOps`] - Index types that use cursor positioning
/// - [`LengthOps`] - Length types that implement this trait
/// - [Module documentation] - Overview of the complete bounds checking architecture
///
/// [`ArrayBoundsCheck`]: crate::ArrayBoundsCheck
/// [`ViewportBoundsCheck`]: crate::ViewportBoundsCheck
/// [`RangeBoundsExt`]: crate::RangeBoundsExt
/// [`IndexOps`]: crate::IndexOps
/// [`LengthOps`]: crate::LengthOps
/// [Module documentation]: mod@crate::core::units::bounds_check
/// [Interval Notation]: mod@crate::core::units::bounds_check#interval-notation
/// [`eol_cursor_position()`]: CursorBoundsCheck::eol_cursor_position
/// [`is_valid_cursor_position()`]: CursorBoundsCheck::is_valid_cursor_position
/// [`clamp_cursor_position()`]: CursorBoundsCheck::clamp_cursor_position
/// [`check_cursor_position_bounds()`]: CursorBoundsCheck::check_cursor_position_bounds
/// [`Length`]: crate::Length
/// [`RowHeight`]: crate::RowHeight
/// [`ColWidth`]: crate::ColWidth
/// [`ByteLength`]: crate::ByteLength
/// [`SegLength`]: crate::SegLength
pub trait CursorBoundsCheck: LengthOps
where
    Self::IndexType: Add<Output = Self::IndexType>,
{
    /// Get the cursor position at end-of-line (after the last character).
    ///
    /// For content of length N, this returns position N.
    /// See the [trait documentation][Self] for cursor positioning semantics.
    fn eol_cursor_position(&self) -> Self::IndexType {
        let length_val = self.as_usize();

        if length_val == 0 {
            // Use From<usize> for type-safe construction.
            Self::IndexType::from(0_usize)
        } else {
            // Normal case: last valid index + 1.
            self.convert_to_index() + Self::IndexType::from(1_usize)
        }
    }

    /// Check if a cursor position is valid for this line/buffer.
    ///
    /// Returns true for positions in the range `[0, length]` (inclusive of EOL position).
    fn is_valid_cursor_position(&self, pos: Self::IndexType) -> bool {
        // Position is valid if it's not beyond the boundary
        self.check_cursor_position_bounds(pos) != CursorPositionBoundsStatus::Beyond
    }

    /// Clamp a cursor position to valid bounds `[0, length]`.
    ///
    /// Positions beyond EOL are clamped to the EOL position.
    fn clamp_cursor_position(&self, pos: Self::IndexType) -> Self::IndexType {
        if self.is_valid_cursor_position(pos) {
            pos
        } else {
            self.eol_cursor_position()
        }
    }

    /// Check detailed cursor position status for text editing contexts.
    ///
    /// See the [trait documentation][Self] for cursor positioning semantics and visual
    /// diagrams.
    ///
    /// # Returns
    /// - [`CursorPositionBoundsStatus::AtStart`] if position = 0
    /// - [`CursorPositionBoundsStatus::Within`] if 0 < position < length
    /// - [`CursorPositionBoundsStatus::AtEnd`] if position = length (EOL position)
    /// - [`CursorPositionBoundsStatus::Beyond`] if position > length
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

/// Blanket implementation that automatically implements [`CursorBoundsCheck`] for all
/// types that implement [`LengthOps`].
///
/// This eliminates the need to write individual empty impl blocks like:
/// ```compile_fail
/// # use r3bl_tui::{CursorBoundsCheck, ColWidth, RowHeight, Length};
/// impl CursorBoundsCheck for ColWidth {}
/// impl CursorBoundsCheck for RowHeight {}
/// impl CursorBoundsCheck for Length {}
/// // Error: only traits defined in the current crate can be implemented
/// ```
///
/// All method implementations are provided as defaults in the trait definition,
/// so this impl block is empty - it simply activates the trait for all [`LengthOps`]
/// types.
///
/// This pattern is only possible because [`CursorBoundsCheck`] is not parameterized
/// over type parameters. For comparison, see [`ArrayBoundsCheck`] which cannot use a
/// blanket impl due to its `<LengthType>` type parameter.
///
/// [`ArrayBoundsCheck`]: crate::ArrayBoundsCheck
impl<T: LengthOps> CursorBoundsCheck for T
where
    T::IndexType: Add<Output = T::IndexType>,
{
    // All methods use default implementations from the trait
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
