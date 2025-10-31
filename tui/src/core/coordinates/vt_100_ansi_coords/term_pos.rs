// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! 1-based terminal position combining column and row coordinates.

use super::{TermCol, TermRow};
use std::num::NonZeroU16;

/// 1-based terminal position combining column and row coordinates.
///
/// Represents a position on the terminal using VT-100 1-based coordinates where both
/// the column and row are `NonZeroU16` values (valid range: 1 to 65,535).
///
/// # Coordinate Order
///
/// Note the `(col, row)` field order, which matches VT-100 ANSI convention for mouse
/// events and cursor positioning. This differs from buffer coordinates which use `(row,
/// col)` order.
///
/// # Construction
///
/// Use [`TermPos::from_one_based()`] to construct from raw 1-based coordinate values:
///
/// ```rust
/// use r3bl_tui::TermPos;
///
/// let pos = TermPos::from_one_based(10, 5); // column 10, row 5
/// assert_eq!(pos.col.as_u16(), 10);
/// assert_eq!(pos.row.as_u16(), 5);
/// ```
///
/// # Validation
///
/// Mouse event coordinates defined by this type have been validated against **real
/// terminal emulator behavior** through interactive observation testing. See the
/// [`observe_real_interactive_terminal_input_events`] test module
/// for ground truth testing methodology and results that establish the correctness of
/// these 1-based coordinates.
///
/// This means:
/// - ✅ Coordinates are VT-100 spec-compliant (1-based, not 0-based)
/// - ✅ Mouse click positions match actual terminal emulator output
/// - ✅ Tested against multiple terminal emulators
/// - ✅ Validated through real-world interactive testing
///
/// # Use Cases
///
/// - Mouse event positions in ANSI input sequences
/// - Any protocol layer that needs a combined coordinate pair
///
/// [`TermPos::from_one_based()`]: Self::from_one_based
/// [`observe_real_interactive_terminal_input_events`]: mod@crate::core::ansi::vt_100_terminal_input_parser::validation_tests::observe_real_interactive_terminal_input_events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TermPos {
    pub col: TermCol,
    pub row: TermRow,
}

impl TermPos {
    /// Construct a terminal position from raw 1-based coordinate values.
    ///
    /// This is the primary constructor for ANSI sequence parsing where coordinates
    /// are received as raw `u16` values that are known to be 1-based and non-zero.
    ///
    /// # Panics
    ///
    /// Panics if either coordinate is zero (invalid VT-100 coordinate).
    ///
    /// # Example
    ///
    /// ```rust
    /// use r3bl_tui::TermPos;
    ///
    /// // Mouse event at column 10, row 5 (1-based coordinates)
    /// let pos = TermPos::from_one_based(10, 5);
    /// assert_eq!(pos.col.as_u16(), 10);
    /// assert_eq!(pos.row.as_u16(), 5);
    /// ```
    #[must_use]
    pub fn from_one_based(col: u16, row: u16) -> Self {
        let col_nz = NonZeroU16::new(col).expect("Column must be non-zero (1-based)");
        let row_nz = NonZeroU16::new(row).expect("Row must be non-zero (1-based)");

        Self {
            col: TermCol::from_raw_non_zero_value(col_nz),
            row: TermRow::from_raw_non_zero_value(row_nz),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_term_pos_from_one_based() {
        let pos = TermPos::from_one_based(10, 5);
        assert_eq!(pos.col.as_u16(), 10);
        assert_eq!(pos.row.as_u16(), 5);
    }

    #[test]
    fn test_term_pos_minimum_values() {
        let pos = TermPos::from_one_based(1, 1);
        assert_eq!(pos.col.as_u16(), 1);
        assert_eq!(pos.row.as_u16(), 1);
    }

    #[test]
    fn test_term_pos_maximum_values() {
        let pos = TermPos::from_one_based(65535, 65535);
        assert_eq!(pos.col.as_u16(), 65535);
        assert_eq!(pos.row.as_u16(), 65535);
    }

    #[test]
    fn test_term_pos_equality() {
        let pos1 = TermPos::from_one_based(10, 5);
        let pos2 = TermPos::from_one_based(10, 5);
        let pos3 = TermPos::from_one_based(10, 6);

        assert_eq!(pos1, pos2);
        assert_ne!(pos1, pos3);
    }

    #[test]
    fn test_term_pos_debug() {
        let pos = TermPos::from_one_based(10, 5);
        let debug_str = format!("{pos:?}");
        assert!(debug_str.contains("10") && debug_str.contains("5"));
    }

    #[test]
    #[should_panic(expected = "Column must be non-zero")]
    fn test_term_pos_panics_on_zero_column() { let _unused = TermPos::from_one_based(0, 5); }

    #[test]
    #[should_panic(expected = "Row must be non-zero")]
    fn test_term_pos_panics_on_zero_row() { let _unused = TermPos::from_one_based(10, 0); }
}
