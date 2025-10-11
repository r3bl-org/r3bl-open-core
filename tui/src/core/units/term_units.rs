// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Type-safe 1-based terminal coordinates for ANSI escape sequences.
//!
//! This module provides the [`TermRow`] and [`TermCol`] types for terminal coordinates.
//! See type documentation for detailed usage and examples.
//! The [`vt_100_ansi_parser`] is the primary consumer of these types, along with
//! the [`offscreen_buffer`] module which uses them for VT 100 related operations.
//!
//! [`vt_100_ansi_parser`]: crate::core::pty_mux::vt_100_ansi_parser
//! [`offscreen_buffer`]: crate::tui::terminal_lib_backends::offscreen_buffer

use crate::{ColIndex, NumericConversions, RowIndex,
            vt_100_ansi_parser::protocols::csi_codes::CsiSequence};
use std::{num::NonZeroU16, ops::Add};

/// Internal macro to implement all necessary traits and methods for terminal coordinate
/// types.
///
/// This macro generates:
/// - [`NumericConversions`] trait implementation for reading values
/// - [`From<NonZeroU16>`] for construction
/// - [`Display`] trait for formatting
/// - [`new()`] constructor for creating instances
/// - [`as_u16()`] for extracting the raw 1-based value
/// - [`from_zero_based()`] for converting from 0-based buffer indices
/// - [`to_zero_based()`] for converting to 0-based buffer indices
///
/// [`NumericConversions`]: crate::core::units::NumericConversions
/// [`From<NonZeroU16>`]: std::convert::From
/// [`Display`]: std::fmt::Display
/// [`new()`]: Self::new
/// [`as_u16()`]: Self::as_u16
/// [`from_zero_based()`]: Self::from_zero_based
/// [`to_zero_based()`]: Self::to_zero_based
macro_rules! generate_impl_term_unit {
    ($term_index_type:ty, $index_type:ty) => {
        impl NumericConversions for $term_index_type {
            fn as_usize(&self) -> usize { self.0.get() as usize }
            fn as_u16(&self) -> u16 { self.0.get() }
        }

        impl ::std::fmt::Display for $term_index_type {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}", self.0.get())
            }
        }

        impl $term_index_type {
            /// Create a new 1-based terminal coordinate.
            #[must_use]
            pub const fn new(value: NonZeroU16) -> Self { Self(value) }

            /// Get the raw 1-based value.
            #[must_use]
            pub const fn as_u16(self) -> u16 { self.0.get() }

            /// Convert from 0-based index_type to 1-based terminal coordinate.
            #[must_use]
            pub fn from_zero_based(index: $index_type) -> Self {
                let nz_value = index.as_u16() + 1;
                // SAFETY: 0-based index_type + 1 is always >= 1
                debug_assert!(nz_value >= 1);
                Self::new(unsafe { NonZeroU16::new_unchecked(nz_value) })
            }

            /// Convert to 0-based index_type for buffer operations.
            #[must_use]
            pub fn to_zero_based(&self) -> $index_type {
                <$index_type>::from(self.as_u16().saturating_sub(1))
            }
        }
    };
}

/// # Core Concept: Two Coordinate Systems
///
/// Terminal operations use two distinct coordinate systems that must never be mixed:
///
/// ```text
/// Terminal Coordinates (1-based)    Buffer Coordinates (0-based)
/// ┌─────────────────────────┐      ┌─────────────────────────┐
/// │ (1,1) (1,2) (1,3) ...   │      │ (0,0) (0,1) (0,2) ...   │
/// │ (2,1) (2,2) (2,3) ...   │      │ (1,0) (1,1) (1,2) ...   │
/// │ (3,1) (3,2) (3,3) ...   │      │ (2,0) (2,1) (2,2) ...   │
/// │ ...                     │      │ ...                     │
/// └─────────────────────────┘      └─────────────────────────┘
///   ANSI sequences                   Arrays/buffers/vectors
///   ESC[row;colH                     vec[row_idx][col_idx]
/// ```
///
/// **Why This Matters**: ANSI escape sequences like `ESC[5;10H` use 1-based indexing
/// where `(1,1)` is the top-left corner. Internal data structures use 0-based indexing
/// where `(0,0)` is the top-left. Mixing these systems causes off-by-one errors.
///
/// # Usage Example
///
/// ```rust
/// use r3bl_tui::{term_col, term_row, RowIndex, TermRow};
/// use std::num::NonZeroU16;
///
/// // Create terminal coordinates for ANSI sequences
/// let term_pos = (
///     term_row(NonZeroU16::new(5).unwrap()),
///     term_col(NonZeroU16::new(10).unwrap())
/// );
/// // Generates: ESC[5;10H (row 5, col 10 in terminal)
///
/// // Convert to buffer coordinates for array access
/// let buffer_row = term_pos.0.to_zero_based(); // RowIndex(4)
/// let buffer_col = term_pos.1.to_zero_based(); // ColIndex(9)
/// // Now safe to use: buffer[buffer_row.as_usize()][buffer_col.as_usize()]
///
/// // Convert from buffer back to terminal
/// let buffer_idx = RowIndex::new(4);
/// let term_row = TermRow::from_zero_based(buffer_idx); // TermRow(5)
/// ```
///
/// # Common Pitfalls
///
/// - **Off-by-one errors**: Always convert explicitly, never manually add/subtract 1
/// - **Type confusion**: Use [`TermRow`]/[`TermCol`] for ANSI, [`RowIndex`]/[`ColIndex`]
///   for buffers
/// - **Missing conversion**: Converting to buffer coords is infallible (always safe), but
///   forgetting to convert leads to accessing wrong cells

/// Create a [`TermRow`] from a [`NonZeroU16`] value.
#[must_use]
pub const fn term_row(value: NonZeroU16) -> TermRow { TermRow::new(value) }

/// 1-based row coordinate for terminal ANSI sequences.
///
/// Uses [`NonZeroU16`] as mandated by the VT-100 specification, which defines terminal
/// coordinates as 16-bit unsigned integers with valid values ranging from 1 to 65,535.
///
/// See [self module documentation] for primary consumers of these types.
/// See [module documentation] for coordinate system details and usage examples.
///
/// [self module documentation]: mod@self
/// [module documentation]: mod@super
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermRow(pub NonZeroU16);
generate_impl_term_unit!(TermRow, RowIndex);

/// Create a [`TermCol`] from a [`NonZeroU16`] value.
#[must_use]
pub const fn term_col(value: NonZeroU16) -> TermCol { TermCol::new(value) }

/// 1-based column coordinate for terminal ANSI sequences.
///
/// Uses [`NonZeroU16`] as mandated by the VT-100 specification, which defines terminal
/// coordinates as 16-bit unsigned integers with valid values ranging from 1 to 65,535.
///
/// See [self module documentation] for primary consumers of these types.
/// See [module documentation] for coordinate system details and usage examples.
///
/// [self module documentation]: mod@self
/// [module documentation]: mod@super
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermCol(pub NonZeroU16);
generate_impl_term_unit!(TermCol, ColIndex);

/// Safe conversions from buffer coordinates (0-based) to terminal coordinates (1-based).
mod from_buffer_coords {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<RowIndex> for TermRow {
        /// Convert from 0-based [`RowIndex`] to 1-based [`TermRow`].
        ///
        /// This is always safe because the conversion adds 1, guaranteeing a non-zero
        /// value.
        fn from(value: RowIndex) -> Self { Self::from_zero_based(value) }
    }

    impl From<ColIndex> for TermCol {
        /// Convert from 0-based [`ColIndex`] to 1-based [`TermCol`].
        ///
        /// This is always safe because the conversion adds 1, guaranteeing a non-zero
        /// value.
        fn from(value: ColIndex) -> Self { Self::from_zero_based(value) }
    }
}

mod add_ops_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Add [`TermCol`] to [`TermRow`] to create a cursor position.
    ///
    /// # Examples
    /// ```rust
    /// use r3bl_tui::{term_col, term_row};
    /// use std::num::NonZeroU16;
    ///
    /// let position = term_row(NonZeroU16::new(5).unwrap()) + term_col(NonZeroU16::new(10).unwrap());
    /// // This creates a CsiSequence::CursorPosition { row: TermRow(5), col: TermCol(10) }
    /// ```
    impl Add<TermCol> for TermRow {
        type Output = CsiSequence;

        fn add(self, rhs: TermCol) -> Self::Output {
            CsiSequence::CursorPosition {
                row: self,
                col: rhs,
            }
        }
    }

    /// Add [`TermRow`] to [`TermCol`] to create a cursor position.
    ///
    /// # Examples
    /// ```rust
    /// use r3bl_tui::{term_col, term_row};
    /// use std::num::NonZeroU16;
    ///
    /// let position = term_col(NonZeroU16::new(10).unwrap()) + term_row(NonZeroU16::new(5).unwrap());
    /// // This creates a CsiSequence::CursorPosition { row: TermRow(5), col: TermCol(10) }
    /// ```
    impl Add<TermRow> for TermCol {
        type Output = CsiSequence;

        fn add(self, rhs: TermRow) -> Self::Output {
            CsiSequence::CursorPosition {
                row: rhs,
                col: self,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::{DefaultHasher, Hash, Hasher};

    // ========================================================================
    // Helper functions for creating test values
    // ========================================================================

    fn nz(value: u16) -> NonZeroU16 {
        NonZeroU16::new(value).expect("NonZeroU16 creation failed")
    }

    // ========================================================================
    // TermRow: Construction and Basic Operations
    // ========================================================================

    #[test]
    fn test_term_row_new() {
        let row = TermRow::new(nz(5));
        assert_eq!(row.as_u16(), 5);
    }

    #[test]
    fn test_term_row_helper_function() {
        let row = term_row(nz(7));
        assert_eq!(row, TermRow::new(nz(7)));
    }

    #[test]
    fn test_term_row_inner_field_access() {
        let row = term_row(nz(15));
        assert_eq!(row.0, nz(15));
    }

    // ========================================================================
    // TermCol: Construction and Basic Operations
    // ========================================================================

    #[test]
    fn test_term_col_new() {
        let col = TermCol::new(nz(8));
        assert_eq!(col.as_u16(), 8);
    }

    #[test]
    fn test_term_col_helper_function() {
        let col = term_col(nz(9));
        assert_eq!(col, TermCol::new(nz(9)));
    }

    #[test]
    fn test_term_col_inner_field_access() {
        let col = term_col(nz(25));
        assert_eq!(col.0, nz(25));
    }

    // ========================================================================
    // TermRow: NumericConversions Trait
    // ========================================================================

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

    // ========================================================================
    // TermCol: NumericConversions Trait
    // ========================================================================

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

    // ========================================================================
    // TermRow: Coordinate Conversions
    // ========================================================================

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

    // ========================================================================
    // TermCol: Coordinate Conversions
    // ========================================================================

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

    // ========================================================================
    // Display Trait Implementation
    // ========================================================================

    #[test]
    fn test_term_row_display() {
        let row = term_row(nz(5));
        assert_eq!(format!("{row}"), "5");
    }

    #[test]
    fn test_term_col_display() {
        let col = term_col(nz(10));
        assert_eq!(format!("{col}"), "10");
    }

    #[test]
    fn test_term_row_display_large_value() {
        let row = term_row(nz(65535));
        assert_eq!(format!("{row}"), "65535");
    }

    #[test]
    fn test_term_col_display_large_value() {
        let col = term_col(nz(65535));
        assert_eq!(format!("{col}"), "65535");
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

    // ========================================================================
    // Edge Cases: Minimum Values
    // ========================================================================

    #[test]
    fn test_term_row_minimum_value() {
        let row = term_row(nz(1));
        assert_eq!(row.as_u16(), 1);
        assert_eq!(row.to_zero_based(), RowIndex::from(0));
    }

    #[test]
    fn test_term_col_minimum_value() {
        let col = term_col(nz(1));
        assert_eq!(col.as_u16(), 1);
        assert_eq!(col.to_zero_based(), ColIndex::from(0));
    }

    #[test]
    fn test_term_row_from_zero_index() {
        let index = RowIndex::from(0);
        let row = TermRow::from_zero_based(index);
        assert_eq!(row.as_u16(), 1);
    }

    #[test]
    fn test_term_col_from_zero_index() {
        let index = ColIndex::from(0);
        let col = TermCol::from_zero_based(index);
        assert_eq!(col.as_u16(), 1);
    }

    // ========================================================================
    // Edge Cases: Maximum Values
    // ========================================================================

    #[test]
    fn test_term_row_maximum_value() {
        let row = term_row(nz(65535));
        assert_eq!(row.as_u16(), 65535);
        assert_eq!(row.to_zero_based(), RowIndex::from(65534));
    }

    #[test]
    fn test_term_col_maximum_value() {
        let col = term_col(nz(65535));
        assert_eq!(col.as_u16(), 65535);
        assert_eq!(col.to_zero_based(), ColIndex::from(65534));
    }

    #[test]
    fn test_term_row_from_max_index() {
        let index = RowIndex::from(65534);
        let row = TermRow::from_zero_based(index);
        assert_eq!(row.as_u16(), 65535);
    }

    #[test]
    fn test_term_col_from_max_index() {
        let index = ColIndex::from(65534);
        let col = TermCol::from_zero_based(index);
        assert_eq!(col.as_u16(), 65535);
    }

    // ========================================================================
    // Equality and Comparison
    // ========================================================================

    #[test]
    fn test_term_row_equality() {
        let row1 = term_row(nz(5));
        let row2 = term_row(nz(5));
        let row3 = term_row(nz(6));

        assert_eq!(row1, row2);
        assert_ne!(row1, row3);
    }

    #[test]
    fn test_term_col_equality() {
        let col1 = term_col(nz(10));
        let col2 = term_col(nz(10));
        let col3 = term_col(nz(11));

        assert_eq!(col1, col2);
        assert_ne!(col1, col3);
    }

    // ========================================================================
    // Hash Implementation
    // ========================================================================

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
    // Debug Trait Implementation
    // ========================================================================

    #[test]
    fn test_term_row_debug() {
        let row = term_row(nz(42));
        let debug_str = format!("{row:?}");
        assert_eq!(debug_str, "TermRow(42)");
    }

    #[test]
    fn test_term_col_debug() {
        let col = term_col(nz(84));
        let debug_str = format!("{col:?}");
        assert_eq!(debug_str, "TermCol(84)");
    }

    // ========================================================================
    // Clone and Copy
    // ========================================================================

    #[test]
    fn test_term_row_clone() {
        let row1 = term_row(nz(5));
        let row2 = row1;
        assert_eq!(row1, row2);
    }

    #[test]
    fn test_term_col_clone() {
        let col1 = term_col(nz(10));
        let col2 = col1;
        assert_eq!(col1, col2);
    }

    // ========================================================================
    // Conversion Boundary Testing
    // ========================================================================

    #[test]
    fn test_conversion_preserves_off_by_one_semantics() {
        // Terminal (1,1) should map to buffer [0][0]
        let term_row_1 = term_row(nz(1));
        let term_col_1 = term_col(nz(1));

        assert_eq!(term_row_1.to_zero_based(), RowIndex::from(0));
        assert_eq!(term_col_1.to_zero_based(), ColIndex::from(0));

        // Buffer [0][0] should map to terminal (1,1)
        let row_idx_0 = RowIndex::from(0);
        let col_idx_0 = ColIndex::from(0);

        assert_eq!(TermRow::from_zero_based(row_idx_0).as_u16(), 1);
        assert_eq!(TermCol::from_zero_based(col_idx_0).as_u16(), 1);
    }

    #[test]
    fn test_typical_terminal_coordinates() {
        // Test typical terminal size (80x24)
        let row_24 = term_row(nz(24));
        let col_80 = term_col(nz(80));

        assert_eq!(row_24.to_zero_based(), RowIndex::from(23));
        assert_eq!(col_80.to_zero_based(), ColIndex::from(79));
    }

    #[test]
    fn test_large_terminal_coordinates() {
        // Test large terminal (200x100)
        let row_100 = term_row(nz(100));
        let col_200 = term_col(nz(200));

        assert_eq!(row_100.to_zero_based(), RowIndex::from(99));
        assert_eq!(col_200.to_zero_based(), ColIndex::from(199));
    }

    // ========================================================================
    // TermUnit Trait Method Tests
    // ========================================================================

    #[test]
    fn test_term_row_term_unit_methods() {
        let row = term_row(nz(10));
        assert_eq!(row.0, nz(10));
        assert_eq!(row.as_u16(), 10);
    }

    #[test]
    fn test_term_col_term_unit_methods() {
        let col = term_col(nz(25));
        assert_eq!(col.0, nz(25));
        assert_eq!(col.as_u16(), 25);
    }

    // ========================================================================
    // Integration Tests: Full Workflow
    // ========================================================================

    #[test]
    fn test_full_workflow_ansi_to_buffer_to_ansi() {
        // Start with ANSI coordinates ESC[5;10H
        let ansi_row = term_row(nz(5));
        let ansi_col = term_col(nz(10));

        // Convert to buffer coordinates
        let buffer_row = ansi_row.to_zero_based(); // RowIndex(4)
        let buffer_col = ansi_col.to_zero_based(); // ColIndex(9)

        assert_eq!(buffer_row, RowIndex::from(4));
        assert_eq!(buffer_col, ColIndex::from(9));

        // Convert back to ANSI coordinates
        let back_to_ansi_row = TermRow::from_zero_based(buffer_row);
        let back_to_ansi_col = TermCol::from_zero_based(buffer_col);

        assert_eq!(back_to_ansi_row, ansi_row);
        assert_eq!(back_to_ansi_col, ansi_col);
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
