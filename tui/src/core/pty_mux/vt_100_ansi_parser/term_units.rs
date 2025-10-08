// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Type-safe 1-based terminal coordinates for ANSI escape sequences.
//!
//! This module provides the [`TermRow`] and [`TermCol`] types that implement the
//! [`TermUnit`] trait. See trait documentation for detailed usage and examples.

use super::super::protocols::csi_codes::CsiSequence;
use crate::{ColIndex, IndexOps, NumericConversions, NumericValue, RowIndex};
use std::{fmt::Display, num::NonZeroU16, ops::Add};

/// Common behavior for 1-based terminal coordinate types.
///
/// This trait provides default implementations for coordinate conversion and display,
/// eliminating code duplication between [`TermRow`] and [`TermCol`].
///
/// # Core Concept
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
/// # Usage
///
/// ```rust
/// use r3bl_tui::{
///     vt_100_ansi_parser::term_units::{term_row, term_col, TermRow, TermUnit},
///     RowIndex,
/// };
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
///
/// > <div class="warning">
/// >
/// > `TermUnit` can't extend [`NumericValue`] because [`NumericValue`] requires
/// > [`From<u16>`] which would allow constructing zero values, and `TermUnit` is non
/// > zero. Instead `TermUnit` extends [`NumericConversions`] (for reading values) and
/// > requires [`From<NonZeroU16>`] (for safe construction).
/// >
/// > </div>
///
/// [`OffscreenBuffer`]: crate::OffscreenBuffer
pub trait TermUnit: NumericConversions + From<NonZeroU16> {
    /// The corresponding 0-based index type (e.g., [`RowIndex`] or [`ColIndex`]).
    type ZeroBasedIndex: IndexOps;

    /// Access the wrapped [`NonZeroU16`] value.
    fn inner(&self) -> NonZeroU16;

    /// Wrap a [`NonZeroU16`] to create this terminal unit.
    fn wrap(value: NonZeroU16) -> Self;

    /// Create a new terminal coordinate (1-based).
    #[must_use]
    #[allow(dead_code)]
    fn new(value: NonZeroU16) -> Self { Self::wrap(value) }

    /// Get the raw 1-based value.
    #[must_use]
    #[allow(dead_code)]
    fn as_u16(&self) -> u16 { self.inner().get() }

    /// Convert from 0-based index to 1-based terminal coordinate.
    #[must_use]
    fn from_zero_based(index: Self::ZeroBasedIndex) -> Self
    where
        Self::ZeroBasedIndex: NumericValue,
    {
        let value = index.as_u16() + 1;
        debug_assert!(value > 0);
        // SAFETY: 0-based index + 1 is always >= 1
        let nonzero = unsafe { NonZeroU16::new_unchecked(value) };
        Self::wrap(nonzero)
    }

    /// Convert to 0-based index for buffer operations.
    #[must_use]
    fn to_zero_based(&self) -> Self::ZeroBasedIndex {
        Self::ZeroBasedIndex::from(self.inner().get() - 1)
    }
}

/// Create a [`TermRow`] from a [`NonZeroU16`] value.
#[must_use]
pub const fn term_row(value: NonZeroU16) -> TermRow { TermRow::new(value) }

/// 1-based row coordinate for terminal ANSI sequences.
///
/// Uses [`NonZeroU16`] as mandated by the VT-100 specification, which defines terminal
/// coordinates as 16-bit unsigned integers with valid values ranging from 1 to 65,535.
///
/// See [module documentation] for coordinate system details and usage examples.
///
/// [module documentation]: mod@super
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermRow(pub NonZeroU16);

mod term_row_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl NumericConversions for TermRow {
        fn as_usize(&self) -> usize { self.0.get() as usize }
        fn as_u16(&self) -> u16 { self.0.get() }
    }

    impl From<NonZeroU16> for TermRow {
        fn from(value: NonZeroU16) -> Self { Self(value) }
    }

    impl TermUnit for TermRow {
        type ZeroBasedIndex = RowIndex;

        fn inner(&self) -> NonZeroU16 { self.0 }

        fn wrap(value: NonZeroU16) -> Self { Self(value) }
    }

    impl TermRow {
        /// Create a new 1-based terminal row.
        #[must_use]
        pub const fn new(value: NonZeroU16) -> Self { Self(value) }

        /// Get the raw 1-based row value.
        #[must_use]
        pub const fn as_u16(self) -> u16 { self.0.get() }
    }

    impl Display for TermRow {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0.get())
        }
    }
}

/// Create a [`TermCol`] from a [`NonZeroU16`] value.
#[must_use]
pub const fn term_col(value: NonZeroU16) -> TermCol { TermCol::new(value) }

/// 1-based column coordinate for terminal ANSI sequences.
///
/// Uses [`NonZeroU16`] as mandated by the VT-100 specification, which defines terminal
/// coordinates as 16-bit unsigned integers with valid values ranging from 1 to 65,535.
///
/// See [module documentation] for coordinate system details and usage examples.
///
/// [module documentation]: mod@super
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermCol(pub NonZeroU16);

mod term_col_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl NumericConversions for TermCol {
        fn as_usize(&self) -> usize { self.0.get() as usize }
        fn as_u16(&self) -> u16 { self.0.get() }
    }

    impl From<NonZeroU16> for TermCol {
        fn from(value: NonZeroU16) -> Self { Self(value) }
    }

    impl TermUnit for TermCol {
        type ZeroBasedIndex = ColIndex;

        fn inner(&self) -> NonZeroU16 { self.0 }

        fn wrap(value: NonZeroU16) -> Self { Self(value) }
    }

    impl TermCol {
        /// Create a new 1-based terminal column.
        #[must_use]
        pub const fn new(value: NonZeroU16) -> Self { Self(value) }

        /// Get the raw 1-based column value.
        #[must_use]
        pub const fn as_u16(self) -> u16 { self.0.get() }
    }

    impl Display for TermCol {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0.get())
        }
    }
}

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
    /// use r3bl_tui::vt_100_ansi_parser::term_units::{term_row, term_col};
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
    /// use r3bl_tui::vt_100_ansi_parser::term_units::{term_row, term_col};
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
