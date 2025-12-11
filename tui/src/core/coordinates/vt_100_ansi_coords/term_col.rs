// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ColIndex, CsiSequence, NumericConversions, TermRow};
use std::{fmt::{Display, Formatter},
          num::NonZeroU16,
          ops::Add};

/// 1-based column coordinate for terminal ANSI sequences.
///
/// Uses [`NonZeroU16`] as mandated by the VT-100 specification, which defines terminal
/// coordinates as 16-bit unsigned integers with valid values ranging from 1 to 65,535.
///
/// # Construction
///
/// This type uses private fields and explicit constructors. Use these methods to create
/// `TermCol` values:
/// - [`from_raw_non_zero_value()`] - Wrap external `NonZeroU16` data (ANSI parameters)
/// - [`from_zero_based()`] - Convert from 0-based [`ColIndex`] to 1-based terminal
///   coordinate
/// - [`term_col()`] - Convenience helper (primarily for tests)
///
/// # Coordinate System
///
/// `TermCol` represents **1-based terminal coordinates** used in ANSI escape sequences.
/// This is distinct from:
/// - [`ColIndex`] - 0-based buffer/array positions
///
/// # Example
///
/// ```rust
/// use r3bl_tui::{TermCol, ColIndex, term_col};
/// use std::num::NonZeroU16;
///
/// // Create from ANSI parameter
/// let from_ansi = TermCol::from_raw_non_zero_value(NonZeroU16::new(10).unwrap());
///
/// // Convert from buffer index (0-based → 1-based)
/// let from_buffer = TermCol::from_zero_based(ColIndex::from(9));
/// assert_eq!(from_ansi, from_buffer); // Both represent column 10
///
/// // Convert to buffer index (1-based → 0-based)
/// let buffer_idx = from_ansi.to_zero_based();
/// assert_eq!(buffer_idx, ColIndex::from(9));
/// ```
///
/// [`from_raw_non_zero_value()`]: Self::from_raw_non_zero_value
/// [`from_zero_based()`]: Self::from_zero_based
/// [`term_col()`]: term_col
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermCol(NonZeroU16);

impl NumericConversions for TermCol {
    fn as_usize(&self) -> usize { self.0.get() as usize }
    fn as_u16(&self) -> u16 { self.0.get() }
}

impl Display for TermCol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.get())
    }
}

impl TermCol {
    /// Column 1 - the first (leftmost) column of the terminal.
    ///
    /// Use this constant instead of manually constructing a column 1 value.
    /// This is commonly used for operations like "move cursor to beginning of
    /// line".
    ///
    /// # Example
    ///
    /// ```rust
    /// use r3bl_tui::{TermCol, CsiSequence};
    ///
    /// // Move cursor to column 1 (beginning of line)
    /// let seq = CsiSequence::CursorHorizontalAbsolute(TermCol::ONE);
    /// ```
    pub const ONE: Self = Self(NonZeroU16::MIN);

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

    /// Convert from 0-based `ColIndex` to 1-based terminal coordinate.
    #[must_use]
    pub fn from_zero_based(index: ColIndex) -> Self {
        let nz_value = index.as_u16() + 1;
        // SAFETY: 0-based `ColIndex` + 1 is always >= 1
        debug_assert!(nz_value >= 1);
        Self::from_raw_non_zero_value(unsafe { NonZeroU16::new_unchecked(nz_value) })
    }

    /// Convert to 0-based `ColIndex` for buffer operations.
    #[must_use]
    pub fn to_zero_based(&self) -> ColIndex {
        ColIndex::from(self.as_u16().saturating_sub(1))
    }
}

impl From<ColIndex> for TermCol {
    /// Convert from 0-based [`ColIndex`] to 1-based [`TermCol`].
    ///
    /// This is always safe because the conversion adds 1, guaranteeing a non-zero
    /// value.
    fn from(value: ColIndex) -> Self { Self::from_zero_based(value) }
}

/// Add [`TermRow`] to [`TermCol`] to create a cursor position.
///
/// # Example
///
/// ```rust
/// use r3bl_tui::{TermCol, TermRow, term_col, term_row, CsiSequence};
/// use std::num::NonZeroU16;
///
/// let col = term_col(NonZeroU16::new(20).unwrap());
/// let row = term_row(NonZeroU16::new(10).unwrap());
/// let cursor_pos = col + row;
/// ```
///
/// [`TermRow`]: crate::TermRow
impl Add<TermRow> for TermCol {
    type Output = CsiSequence;

    fn add(self, rhs: TermRow) -> Self::Output {
        CsiSequence::CursorPosition {
            row: rhs,
            col: self,
        }
    }
}

/// Create a [`TermCol`] from a [`NonZeroU16`] value.
#[must_use]
pub const fn term_col(value: NonZeroU16) -> TermCol {
    TermCol::from_raw_non_zero_value(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ansi::vt_100_pty_output_parser::vt_100_pty_output_conformance_tests::test_fixtures_vt_100_ansi_conformance::nz;
    use crate::term_row;
    use std::hash::{DefaultHasher, Hash, Hasher};

    #[test]
    fn test_term_col_new() {
        let col = TermCol::from_raw_non_zero_value(nz(8));
        assert_eq!(col.as_u16(), 8);
    }

    #[test]
    fn test_term_col_helper_function() {
        let col = term_col(nz(9));
        assert_eq!(col, TermCol::from_raw_non_zero_value(nz(9)));
    }

    #[test]
    fn test_term_col_inner_field_access() {
        let col = term_col(nz(25));
        assert_eq!(col.value(), nz(25));
    }

    #[test]
    fn test_term_col_as_usize() {
        let col = term_col(nz(150));
        assert_eq!(col.as_usize(), 150_usize);
    }

    #[test]
    fn test_term_col_as_u16() {
        let col = term_col(nz(250));
        assert_eq!(col.as_u16(), 250_u16);
    }

    #[test]
    fn test_term_col_to_zero_based() {
        let col = term_col(nz(10)); // 1-based: col 10
        let index = col.to_zero_based(); // 0-based: index 9
        assert_eq!(index, ColIndex::from(9));
    }

    #[test]
    fn test_term_col_from_zero_based() {
        let index = ColIndex::from(19); // 0-based: index 19
        let col = TermCol::from_zero_based(index); // 1-based: col 20
        assert_eq!(col.as_u16(), 20);
    }

    #[test]
    fn test_term_col_from_col_index() {
        let index = ColIndex::from(14); // 0-based: index 14
        let col = TermCol::from(index); // 1-based: col 15
        assert_eq!(col.as_u16(), 15);
    }

    #[test]
    fn test_term_col_round_trip_conversion() {
        let original = term_col(nz(77));
        let zero_based = original.to_zero_based();
        let back_to_one_based = TermCol::from_zero_based(zero_based);
        assert_eq!(original, back_to_one_based);
    }

    #[test]
    fn test_term_col_display() {
        let col = term_col(nz(10));
        assert_eq!(format!("{col}"), "10");
    }

    #[test]
    fn test_term_col_display_large_value() {
        let col = term_col(nz(65535));
        assert_eq!(format!("{col}"), "65535");
    }

    #[test]
    fn test_term_col_minimum_value() {
        let col = term_col(nz(1));
        assert_eq!(col.as_u16(), 1);
        assert_eq!(col.to_zero_based(), ColIndex::from(0));
    }

    #[test]
    fn test_term_col_from_zero_index() {
        let index = ColIndex::from(0);
        let col = TermCol::from_zero_based(index);
        assert_eq!(col.as_u16(), 1);
    }

    #[test]
    fn test_term_col_maximum_value() {
        let col = term_col(nz(65535));
        assert_eq!(col.as_u16(), 65535);
        assert_eq!(col.to_zero_based(), ColIndex::from(65534));
    }

    #[test]
    fn test_term_col_from_max_index() {
        let index = ColIndex::from(65534);
        let col = TermCol::from_zero_based(index);
        assert_eq!(col.as_u16(), 65535);
    }

    #[test]
    fn test_term_col_equality() {
        let col1 = term_col(nz(10));
        let col2 = term_col(nz(10));
        let col3 = term_col(nz(11));

        assert_eq!(col1, col2);
        assert_ne!(col1, col3);
    }

    #[test]
    fn test_term_col_hash() {
        let col1 = term_col(nz(10));
        let col2 = term_col(nz(10));

        let mut hasher1 = DefaultHasher::new();
        col1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        col2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_eq!(hash1, hash2);
    }

    // ========================================================================
    // Conversion Boundary Testing
    // ========================================================================

    #[test]
    fn test_col_conversion_preserves_off_by_one_semantics() {
        // Terminal (1) should map to buffer [0]
        let term_col_1 = term_col(nz(1));
        assert_eq!(term_col_1.to_zero_based(), ColIndex::from(0));

        // Buffer [0] should map to terminal (1)
        let col_idx_0 = ColIndex::from(0);
        assert_eq!(TermCol::from_zero_based(col_idx_0).as_u16(), 1);
    }

    #[test]
    fn test_col_typical_terminal_coordinates() {
        // Test typical terminal size (80 columns)
        let col_80 = term_col(nz(80));
        assert_eq!(col_80.to_zero_based(), ColIndex::from(79));
    }

    #[test]
    fn test_col_large_terminal_coordinates() {
        // Test large terminal (200 columns)
        let col_200 = term_col(nz(200));
        assert_eq!(col_200.to_zero_based(), ColIndex::from(199));
    }

    // ========================================================================
    // Add Operations: Creating CsiSequence
    // ========================================================================

    #[test]
    fn test_term_col_add_term_row() {
        let col = term_col(nz(15));
        let row = term_row(nz(20));
        let result = col + row;

        match result {
            CsiSequence::CursorPosition { row: r, col: c } => {
                assert_eq!(r.as_u16(), 20);
                assert_eq!(c.as_u16(), 15);
            }
            _ => panic!("Expected CursorPosition variant"),
        }
    }
}
