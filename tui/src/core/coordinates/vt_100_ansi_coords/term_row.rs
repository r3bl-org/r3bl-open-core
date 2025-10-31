// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CsiSequence, NumericConversions, RowIndex, TermCol};
use std::{fmt::{Display, Formatter},
          num::NonZeroU16,
          ops::Add};

/// 1-based row coordinate for terminal ANSI sequences.
///
/// Uses [`NonZeroU16`] as mandated by the VT-100 specification, which defines terminal
/// coordinates as 16-bit unsigned integers with valid values ranging from 1 to 65,535.
///
/// # Construction
///
/// This type uses private fields and explicit constructors. Use these methods to create
/// `TermRow` values:
/// - [`from_raw_non_zero_value()`] - Wrap external `NonZeroU16` data (ANSI parameters)
/// - [`from_zero_based()`] - Convert from 0-based [`RowIndex`] to 1-based terminal
///   coordinate
/// - [`term_row()`] - Convenience helper (primarily for tests)
///
/// # Coordinate System
///
/// `TermRow` represents **1-based terminal coordinates** used in ANSI escape sequences.
/// This is distinct from:
/// - [`RowIndex`] - 0-based buffer/array positions
///
/// # Example
///
/// ```rust
/// use r3bl_tui::{TermRow, RowIndex, term_row};
/// use std::num::NonZeroU16;
///
/// // Create from ANSI parameter
/// let from_ansi = TermRow::from_raw_non_zero_value(NonZeroU16::new(5).unwrap());
///
/// // Convert from buffer index (0-based → 1-based)
/// let from_buffer = TermRow::from_zero_based(RowIndex::from(4));
/// assert_eq!(from_ansi, from_buffer); // Both represent row 5
///
/// // Convert to buffer index (1-based → 0-based)
/// let buffer_idx = from_ansi.to_zero_based();
/// assert_eq!(buffer_idx, RowIndex::from(4));
/// ```
///
/// [`from_raw_non_zero_value()`]: Self::from_raw_non_zero_value
/// [`from_zero_based()`]: Self::from_zero_based
/// [`term_row()`]: term_row
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermRow(NonZeroU16);

// Implementation is provided by the macro in the parent module, but we define
// the helper function here for convenience.

impl NumericConversions for TermRow {
    fn as_usize(&self) -> usize { self.0.get() as usize }
    fn as_u16(&self) -> u16 { self.0.get() }
}

impl Display for TermRow {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.get())
    }
}

impl TermRow {
    /// Create a 1-based terminal coordinate from a raw [`NonZeroU16`] value.
    ///
    /// Use this constructor when wrapping external [`NonZeroU16`] data, such as
    /// values parsed from ANSI escape sequence parameters.
    #[must_use]
    pub const fn from_raw_non_zero_value(value: NonZeroU16) -> Self { Self(value) }

    /// Get the inner [`NonZeroU16`] value.
    ///
    /// This provides access to the raw 1-based terminal coordinate value.
    /// Use this when you need the [`NonZeroU16`] representation, for example
    /// when serializing or passing to external APIs.
    #[must_use]
    pub const fn value(self) -> NonZeroU16 { self.0 }

    /// Get the raw 1-based value as a [`u16`].
    ///
    /// This is a convenience method that extracts the underlying [`u16`] from
    /// the [`NonZeroU16`] wrapper. Most code should use [`as_usize()`] for
    /// general numeric operations or [`value()`] for accessing the
    /// [`NonZeroU16`].
    ///
    /// [`as_usize()`]: Self::as_usize
    /// [`value()`]: Self::value
    #[must_use]
    pub const fn as_u16(self) -> u16 { self.0.get() }

    /// Convert from 0-based `RowIndex` to 1-based terminal coordinate.
    #[must_use]
    pub fn from_zero_based(index: RowIndex) -> Self {
        let nz_value = index.as_u16() + 1;
        // SAFETY: 0-based `RowIndex` + 1 is always >= 1
        debug_assert!(nz_value >= 1);
        Self::from_raw_non_zero_value(unsafe { NonZeroU16::new_unchecked(nz_value) })
    }

    /// Convert to 0-based `RowIndex` for buffer operations.
    #[must_use]
    pub fn to_zero_based(&self) -> RowIndex {
        RowIndex::from(self.as_u16().saturating_sub(1))
    }
}

impl From<RowIndex> for TermRow {
    /// Convert from 0-based [`RowIndex`] to 1-based [`TermRow`].
    ///
    /// This is always safe because the conversion adds 1, guaranteeing a non-zero
    /// value.
    fn from(value: RowIndex) -> Self { Self::from_zero_based(value) }
}

/// Add [`TermCol`] to [`TermRow`] to create a cursor position.
///
/// # Example
///
/// ```rust
/// use r3bl_tui::{TermRow, TermCol, term_row, term_col, CsiSequence};
/// use std::num::NonZeroU16;
///
/// let row = term_row(NonZeroU16::new(10).unwrap());
/// let col = term_col(NonZeroU16::new(20).unwrap());
/// let cursor_pos = row + col;
/// ```
///
/// [`TermCol`]: crate::TermCol
impl Add<TermCol> for TermRow {
    type Output = CsiSequence;

    fn add(self, rhs: TermCol) -> Self::Output {
        CsiSequence::CursorPosition {
            row: self,
            col: rhs,
        }
    }
}

/// Create a [`TermRow`] from a [`NonZeroU16`] value.
#[must_use]
pub const fn term_row(value: NonZeroU16) -> TermRow {
    TermRow::from_raw_non_zero_value(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColIndex, term_col};
    use std::hash::{DefaultHasher, Hash, Hasher};

    fn nz(value: u16) -> NonZeroU16 {
        NonZeroU16::new(value).expect("NonZeroU16 creation failed")
    }

    #[test]
    fn test_term_row_new() {
        let row = TermRow::from_raw_non_zero_value(nz(5));
        assert_eq!(row.as_u16(), 5);
    }

    #[test]
    fn test_term_row_helper_function() {
        let row = term_row(nz(7));
        assert_eq!(row, TermRow::from_raw_non_zero_value(nz(7)));
    }

    #[test]
    fn test_term_row_inner_field_access() {
        let row = term_row(nz(15));
        assert_eq!(row.value(), nz(15));
    }

    #[test]
    fn test_term_row_as_usize() {
        let row = term_row(nz(100));
        assert_eq!(row.as_usize(), 100_usize);
    }

    #[test]
    fn test_term_row_as_u16() {
        let row = term_row(nz(200));
        assert_eq!(row.as_u16(), 200_u16);
    }

    #[test]
    fn test_term_row_to_zero_based() {
        let row = term_row(nz(5)); // 1-based: row 5
        let index = row.to_zero_based(); // 0-based: index 4
        assert_eq!(index, RowIndex::from(4));
    }

    #[test]
    fn test_term_row_from_zero_based() {
        let index = RowIndex::from(4); // 0-based: index 4
        let row = TermRow::from_zero_based(index); // 1-based: row 5
        assert_eq!(row.as_u16(), 5);
    }

    #[test]
    fn test_term_row_from_row_index() {
        let index = RowIndex::from(9); // 0-based: index 9
        let row = TermRow::from(index); // 1-based: row 10
        assert_eq!(row.as_u16(), 10);
    }

    #[test]
    fn test_term_row_round_trip_conversion() {
        let original = term_row(nz(42));
        let zero_based = original.to_zero_based();
        let back_to_one_based = TermRow::from_zero_based(zero_based);
        assert_eq!(original, back_to_one_based);
    }

    #[test]
    fn test_term_row_display() {
        let row = term_row(nz(5));
        assert_eq!(format!("{row}"), "5");
    }

    #[test]
    fn test_term_row_display_large_value() {
        let row = term_row(nz(65535));
        assert_eq!(format!("{row}"), "65535");
    }

    #[test]
    fn test_term_row_minimum_value() {
        let row = term_row(nz(1));
        assert_eq!(row.as_u16(), 1);
        assert_eq!(row.to_zero_based(), RowIndex::from(0));
    }

    #[test]
    fn test_term_row_from_zero_index() {
        let index = RowIndex::from(0);
        let row = TermRow::from_zero_based(index);
        assert_eq!(row.as_u16(), 1);
    }

    #[test]
    fn test_term_row_maximum_value() {
        let row = term_row(nz(65535));
        assert_eq!(row.as_u16(), 65535);
        assert_eq!(row.to_zero_based(), RowIndex::from(65534));
    }

    #[test]
    fn test_term_row_from_max_index() {
        let index = RowIndex::from(65534);
        let row = TermRow::from_zero_based(index);
        assert_eq!(row.as_u16(), 65535);
    }

    #[test]
    fn test_term_row_equality() {
        let row1 = term_row(nz(5));
        let row2 = term_row(nz(5));
        let row3 = term_row(nz(6));

        assert_eq!(row1, row2);
        assert_ne!(row1, row3);
    }

    #[test]
    fn test_term_row_hash() {
        let row1 = term_row(nz(5));
        let row2 = term_row(nz(5));

        let mut hasher1 = DefaultHasher::new();
        row1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        row2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_term_row_debug() {
        let row = term_row(nz(42));
        let debug_str = format!("{row:?}");
        assert_eq!(debug_str, "TermRow(42)");
    }

    // ========================================================================
    // Conversion Boundary Testing
    // ========================================================================

    #[test]
    fn test_row_conversion_preserves_off_by_one_semantics() {
        // Terminal (1) should map to buffer [0]
        let term_row_1 = term_row(nz(1));
        assert_eq!(term_row_1.to_zero_based(), RowIndex::from(0));

        // Buffer [0] should map to terminal (1)
        let row_idx_0 = RowIndex::from(0);
        assert_eq!(TermRow::from_zero_based(row_idx_0).as_u16(), 1);
    }

    #[test]
    fn test_row_typical_terminal_coordinates() {
        // Test typical terminal size (24 rows)
        let row_24 = term_row(nz(24));
        assert_eq!(row_24.to_zero_based(), RowIndex::from(23));
    }

    #[test]
    fn test_row_large_terminal_coordinates() {
        // Test large terminal (100 rows)
        let row_100 = term_row(nz(100));
        assert_eq!(row_100.to_zero_based(), RowIndex::from(99));
    }

    // ========================================================================
    // Add Operations: Creating CsiSequence
    // ========================================================================

    #[test]
    fn test_term_row_add_term_col() {
        let row = term_row(nz(5));
        let col = term_col(nz(10));
        let result = row + col;

        match result {
            CsiSequence::CursorPosition { row: r, col: c } => {
                assert_eq!(r.as_u16(), 5);
                assert_eq!(c.as_u16(), 10);
            }
            _ => panic!("Expected CursorPosition variant"),
        }
    }

    #[test]
    fn test_cursor_position_creation_at_origin() {
        let row = term_row(nz(1));
        let col = term_col(nz(1));
        let result = row + col;

        match result {
            CsiSequence::CursorPosition { row: r, col: c } => {
                assert_eq!(r.as_u16(), 1);
                assert_eq!(c.as_u16(), 1);
            }
            _ => panic!("Expected CursorPosition variant"),
        }
    }

    #[test]
    fn test_cursor_position_creation_max_values() {
        let row = term_row(nz(65535));
        let col = term_col(nz(65535));
        let result = row + col;

        match result {
            CsiSequence::CursorPosition { row: r, col: c } => {
                assert_eq!(r.as_u16(), 65535);
                assert_eq!(c.as_u16(), 65535);
            }
            _ => panic!("Expected CursorPosition variant"),
        }
    }

    #[test]
    fn test_cursor_position_workflow() {
        // Create cursor position using addition
        let row = term_row(nz(10));
        let col = term_col(nz(20));
        let cursor_pos = row + col;

        // Verify the cursor position
        match cursor_pos {
            CsiSequence::CursorPosition { row: r, col: c } => {
                // Extract coordinates from cursor position
                assert_eq!(r.as_u16(), 10);
                assert_eq!(c.as_u16(), 20);

                // Convert to buffer coordinates for internal use
                let buffer_row = r.to_zero_based();
                let buffer_col = c.to_zero_based();

                assert_eq!(buffer_row, RowIndex::from(9));
                assert_eq!(buffer_col, ColIndex::from(19));
            }
            _ => panic!("Expected CursorPosition variant"),
        }
    }
}
