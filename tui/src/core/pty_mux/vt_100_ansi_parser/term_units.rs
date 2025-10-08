// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Terminal coordinate units for 1-based positioning.
//!
//! This module provides type-safe coordinate types for terminal operations that use
//! 1-based indexing, as opposed to buffer operations which use 0-based indexing.
//!
//! # Implementation Note: Intentional Use of Raw `usize`
//!
//! This file contains `.as_usize()` calls exclusively in doctest examples for
//! demonstration purposes. All production code uses type-safe conversions via
//! `from_zero_based()` and `to_zero_based()` methods.
//!
//! ## Coordinate Systems
//!
//! Understanding the distinction between coordinate systems is crucial for terminal
//! applications:
//!
//! ### Terminal Coordinates (1-based)
//! - Used by ANSI escape sequences like `ESC[row;colH`
//! - Top-left corner is (1, 1)
//! - Examples: `ESC[1;1H` moves to top-left, `ESC[5;10H` moves to row 5, column 10
//! - Represented by [`TermRow`] and [`TermCol`] types
//!
//! ### Buffer Coordinates (0-based)
//! - Used internally by [`OffscreenBuffer`] and similar data structures
//! - Top-left corner is (0, 0)
//! - Standard array/vector indexing
//! - Represented by [`Row`] and [`Col`] types
//!
//! ## Type Safety
//!
//! These newtype wrappers prevent accidentally mixing coordinate systems:
//!
//! ```rust
//! use r3bl_tui::vt_100_ansi_parser::term_units::{TermRow, TermCol, term_row, term_col};
//! use r3bl_tui::{Row, Col};
//!
//! // Clear intent - terminal coordinates
//! let term_pos = (term_row(5), term_col(10)); // Row 5, Col 10 in terminal
//!
//! // Convert to buffer coordinates when needed
//! let buffer_row = term_pos.0.to_zero_based().unwrap(); // Row 4 in buffer (0-based)
//! let buffer_col = term_pos.1.to_zero_based().unwrap(); // Col 9 in buffer (0-based)
//! ```
//!
//! ## Common Patterns
//!
//! ### Creating Terminal Coordinates
//! ```rust
//! use r3bl_tui::vt_100_ansi_parser::term_units::{TermRow, TermCol, term_row, term_col};
//!
//! let row = term_row(5);      // Terminal row 5 (1-based)
//! let col = term_col(10);     // Terminal column 10 (1-based)
//! ```
//!
//! ### Converting Between Systems
//! ```rust
//! use r3bl_tui::{Row, vt_100_ansi_parser::term_units::{TermRow, term_row}};
//!
//! // From buffer to terminal coordinates
//! let buffer_row = Row::new(4);   // 0-based
//! let term_row_from_buf = TermRow::from_zero_based(buffer_row); // Now 5 (1-based)
//!
//! // From terminal to buffer coordinates
//! let term_row_direct = term_row(5); // 1-based
//! let buffer_row_2 = term_row_direct.to_zero_based().unwrap(); // Now 4 (0-based)
//! ```
//!
//! [`OffscreenBuffer`]: crate::OffscreenBuffer
//! [`Row`]: crate::Row
//! [`Col`]: crate::Col

use super::super::protocols::csi_codes::CsiSequence;
use crate::{Col, ColIndex, Row, RowIndex, col, row};
use std::{fmt::Display, ops::Add};

pub fn term_row(arg: impl Into<TermRow>) -> TermRow { arg.into() }

/// 1-based row index for terminal coordinates (CSI/ESC sequences).
///
/// Terminal sequences like `ESC[5;10H` use 1-based indexing where row 1, col 1
/// is the top-left corner. This is different from buffer coordinates which are 0-based.
///
/// # Examples
///
/// ```rust
/// use r3bl_tui::vt_100_ansi_parser::term_units::{TermRow, term_row};
/// use r3bl_tui::Row;
///
/// // Create a terminal row (1-based)
/// let term_row = term_row(5); // Terminal row 5
/// assert_eq!(term_row.as_u16(), 5);
///
/// // Convert to buffer coordinates (0-based)
/// let buffer_row = term_row.to_zero_based().unwrap();
/// assert_eq!(buffer_row.as_usize(), 4); // Buffer row 4
///
/// // Convert from buffer coordinates
/// let buffer_row = Row::new(9);
/// let term_row = TermRow::from_zero_based(buffer_row);
/// assert_eq!(term_row.as_u16(), 10); // Terminal row 10
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermRow(pub u16);

mod term_row_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl TermRow {
        /// Create a new `TermRow` with 1-based indexing.
        ///
        /// # Arguments
        /// * `value` - The 1-based row number (must be >= 1 for valid terminal
        ///   coordinates)
        ///
        /// # Examples
        /// ```rust
        /// use r3bl_tui::vt_100_ansi_parser::term_units::{TermRow, term_row};
        ///
        /// let row = term_row(5);
        /// assert_eq!(row.as_u16(), 5);
        /// ```
        #[must_use]
        pub const fn new(value: u16) -> Self { Self(value) }

        /// Get the raw 1-based value.
        ///
        /// # Examples
        /// ```rust
        /// use r3bl_tui::vt_100_ansi_parser::term_units::{TermRow, term_row};
        ///
        /// let row = term_row(42);
        /// assert_eq!(row.as_u16(), 42);
        /// ```
        #[must_use]
        pub const fn as_u16(self) -> u16 { self.0 }

        /// Convert from 0-based Row to 1-based `TermRow`.
        ///
        /// # Arguments
        /// * `row` - The 0-based row from buffer coordinates
        ///
        /// # Examples
        /// ```rust
        /// use r3bl_tui::vt_100_ansi_parser::term_units::TermRow;
        /// use r3bl_tui::Row;
        ///
        /// let buffer_row = Row::new(4); // Buffer row 4 (0-based)
        /// let term_row = TermRow::from_zero_based(buffer_row);
        /// assert_eq!(term_row.as_u16(), 5); // Terminal row 5 (1-based)
        /// ```
        #[must_use]
        pub fn from_zero_based(row: Row) -> Self { Self(row.as_u16() + 1) }

        /// Convert to 0-based Row. Returns None if the value is 0 (invalid for 1-based).
        ///
        /// # Returns
        /// * `Some(Row)` - If the terminal row is valid (>= 1)
        /// * `None` - If the terminal row is 0 (invalid for 1-based coordinates)
        ///
        /// # Examples
        /// ```rust
        /// use r3bl_tui::vt_100_ansi_parser::term_units::{TermRow, term_row};
        ///
        /// let row = term_row(5);
        /// let buffer_row = row.to_zero_based().unwrap();
        /// assert_eq!(buffer_row.as_usize(), 4);
        ///
        /// // Invalid terminal coordinate
        /// let invalid_row = term_row(0);
        /// assert!(invalid_row.to_zero_based().is_none());
        /// ```
        #[must_use]
        pub fn to_zero_based(self) -> Option<Row> {
            if self.0 == 0 {
                None
            } else {
                Some(row(self.0 - 1))
            }
        }
    }

    impl Display for TermRow {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
}

pub fn term_col(arg: impl Into<TermCol>) -> TermCol { arg.into() }

/// 1-based column index for terminal coordinates (CSI/ESC sequences).
///
/// Terminal sequences like `ESC[5;10H` use 1-based indexing where row 1, col 1
/// is the top-left corner. This is different from buffer coordinates which are 0-based.
///
/// # Examples
///
/// ```rust
/// use r3bl_tui::vt_100_ansi_parser::term_units::{TermCol, term_col};
/// use r3bl_tui::Col;
///
/// // Create a terminal column (1-based)
/// let term_col = term_col(10); // Terminal column 10
/// assert_eq!(term_col.as_u16(), 10);
///
/// // Convert to buffer coordinates (0-based)
/// let buffer_col = term_col.to_zero_based().unwrap();
/// assert_eq!(buffer_col.as_usize(), 9); // Buffer column 9
///
/// // Convert from buffer coordinates
/// let buffer_col = Col::new(19);
/// let term_col = TermCol::from_zero_based(buffer_col);
/// assert_eq!(term_col.as_u16(), 20); // Terminal column 20
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermCol(pub u16);

mod term_col_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl TermCol {
        /// Create a new `TermCol` with 1-based indexing.
        ///
        /// # Arguments
        /// * `value` - The 1-based column number (must be >= 1 for valid terminal
        ///   coordinates)
        ///
        /// # Examples
        /// ```rust
        /// use r3bl_tui::vt_100_ansi_parser::term_units::{TermCol, term_col};
        ///
        /// let col = term_col(10);
        /// assert_eq!(col.as_u16(), 10);
        /// ```
        #[must_use]
        pub const fn new(value: u16) -> Self { Self(value) }

        /// Get the raw 1-based value.
        ///
        /// # Examples
        /// ```rust
        /// use r3bl_tui::vt_100_ansi_parser::term_units::{TermCol, term_col};
        ///
        /// let col = term_col(80);
        /// assert_eq!(col.as_u16(), 80);
        /// ```
        #[must_use]
        pub const fn as_u16(self) -> u16 { self.0 }

        /// Convert from 0-based Col to 1-based `TermCol`.
        ///
        /// # Arguments
        /// * `col` - The 0-based column from buffer coordinates
        ///
        /// # Examples
        /// ```rust
        /// use r3bl_tui::vt_100_ansi_parser::term_units::TermCol;
        /// use r3bl_tui::Col;
        ///
        /// let buffer_col = Col::new(9); // Buffer column 9 (0-based)
        /// let term_col = TermCol::from_zero_based(buffer_col);
        /// assert_eq!(term_col.as_u16(), 10); // Terminal column 10 (1-based)
        /// ```
        #[must_use]
        pub fn from_zero_based(col: Col) -> Self { Self(col.as_u16() + 1) }

        /// Convert to 0-based Col. Returns None if the value is 0 (invalid for 1-based).
        ///
        /// # Returns
        /// * `Some(Col)` - If the terminal column is valid (>= 1)
        /// * `None` - If the terminal column is 0 (invalid for 1-based coordinates)
        ///
        /// # Examples
        /// ```rust
        /// use r3bl_tui::vt_100_ansi_parser::term_units::{TermCol, term_col};
        ///
        /// let col = term_col(10);
        /// let buffer_col = col.to_zero_based().unwrap();
        /// assert_eq!(buffer_col.as_usize(), 9);
        ///
        /// // Invalid terminal coordinate
        /// let invalid_col = term_col(0);
        /// assert!(invalid_col.to_zero_based().is_none());
        /// ```
        #[must_use]
        pub fn to_zero_based(self) -> Option<Col> {
            if self.0 == 0 {
                None
            } else {
                Some(col(self.0 - 1))
            }
        }
    }

    impl Display for TermCol {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
}

mod convenience_conversions {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<i32> for TermRow {
        fn from(value: i32) -> Self {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            {
                Self::new(value as u16)
            }
        }
    }

    impl From<i32> for TermCol {
        fn from(value: i32) -> Self {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            {
                Self::new(value as u16)
            }
        }
    }

    impl From<usize> for TermRow {
        fn from(value: usize) -> Self {
            #[allow(clippy::cast_possible_truncation)]
            {
                Self::new(value as u16)
            }
        }
    }

    impl From<usize> for TermCol {
        fn from(value: usize) -> Self {
            #[allow(clippy::cast_possible_truncation)]
            {
                Self::new(value as u16)
            }
        }
    }

    impl From<u16> for TermRow {
        fn from(value: u16) -> Self { Self::new(value) }
    }

    impl From<u16> for TermCol {
        fn from(value: u16) -> Self { Self::new(value) }
    }

    impl From<RowIndex> for TermRow {
        fn from(value: RowIndex) -> Self { Self::from_zero_based(Row::from(value)) }
    }

    impl From<ColIndex> for TermCol {
        fn from(value: ColIndex) -> Self { Self::from_zero_based(Col::from(value)) }
    }
}

mod add_ops_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Add `TermCol` to `TermRow` to create a cursor position.
    ///
    /// # Examples
    /// ```rust
    /// use r3bl_tui::vt_100_ansi_parser::term_units::{term_row, term_col};
    ///
    /// let position = term_row(5) + term_col(10);
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

    /// Add `TermRow` to `TermCol` to create a cursor position.
    ///
    /// # Examples
    /// ```rust
    /// use r3bl_tui::vt_100_ansi_parser::term_units::{term_row, term_col};
    ///
    /// let position = term_col(10) + term_row(5);
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
