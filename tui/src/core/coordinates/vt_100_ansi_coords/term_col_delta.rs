// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Relative horizontal cursor movement for VT-100 ANSI sequences.
//!
//! [`TermColDelta`] represents **how many columns** to move the cursor, as opposed to
//! [`TermCol`] which represents **which column** the cursor is on.
//!
//! # The CSI Zero Problem
//!
//! ANSI cursor movement commands (CUF, CUB) interpret parameter 0 as 1:
//! - `CSI 0 C` (`CursorForward` with n=0) moves the cursor **1 column right**, not 0
//! - `CSI 0 D` (`CursorBackward` with n=0) moves the cursor **1 column left**, not 0
//!
//! This type provides [`as_nonzero_u16()`] to guard against this bug.
//!
//! # Example
//!
//! ```rust
//! use r3bl_tui::{TermColDelta, term_col_delta, CsiSequence};
//!
//! let position: u16 = 240;
//! let term_width: u16 = 80;
//!
//! let cols_right = term_col_delta(position % term_width); // 0 cols
//!
//! // Safe emission: only emit if non-zero
//! if let Some(n) = cols_right.as_nonzero_u16() {
//!     // This branch is NOT taken because cols_right is 0
//!     let _ = CsiSequence::CursorForward(n);
//! }
//! ```
//!
//! [`TermCol`]: super::TermCol
//! [`as_nonzero_u16()`]: TermColDelta::as_nonzero_u16

use crate::{ColWidth, NumericConversions};
use std::fmt::{Display, Formatter};

/// Relative horizontal cursor movement (column delta).
///
/// Represents how many columns to move left or right. Used with
/// [`CsiSequence::CursorForward`] and [`CsiSequence::CursorBackward`].
///
/// # Safety Against CSI Zero Bug
///
/// Use [`as_nonzero_u16()`] to safely emit cursor movement commands:
///
/// ```rust
/// use r3bl_tui::{term_col_delta, CsiSequence};
/// use std::io::Write;
///
/// let delta = term_col_delta(0);
///
/// // Safe: only emit if non-zero
/// if let Some(n) = delta.as_nonzero_u16() {
///     let seq = CsiSequence::CursorForward(n);
///     // write seq to terminal...
/// }
/// // For delta=0, no sequence is emitted (correct behavior)
/// ```
///
/// [`CsiSequence::CursorForward`]: crate::CsiSequence::CursorForward
/// [`CsiSequence::CursorBackward`]: crate::CsiSequence::CursorBackward
/// [`as_nonzero_u16()`]: Self::as_nonzero_u16
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TermColDelta(u16);

impl NumericConversions for TermColDelta {
    fn as_usize(&self) -> usize { self.0 as usize }
    fn as_u16(&self) -> u16 { self.0 }
}

impl Display for TermColDelta {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TermColDelta({})", self.0)
    }
}

impl TermColDelta {
    /// Create a new column delta from a raw value.
    ///
    /// Zero is a valid value (meaning "don't move").
    #[must_use]
    pub const fn new(value: u16) -> Self { Self(value) }

    /// Get the raw delta value.
    #[must_use]
    pub const fn value(self) -> u16 { self.0 }

    /// Returns `true` if the delta is zero (no movement).
    #[must_use]
    pub const fn is_zero(self) -> bool { self.0 == 0 }

    /// Returns the value as `Option<u16>`, returning `None` if zero.
    ///
    /// This is the **recommended** way to use delta values with CSI sequences.
    /// It prevents the CSI zero bug where `CSI 0 C` is interpreted as `CSI 1 C`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use r3bl_tui::{term_col_delta, CsiSequence};
    ///
    /// let delta = term_col_delta(5);
    /// if let Some(n) = delta.as_nonzero_u16() {
    ///     let _ = CsiSequence::CursorForward(n);
    /// }
    /// ```
    #[must_use]
    pub const fn as_nonzero_u16(self) -> Option<u16> {
        if self.0 == 0 { None } else { Some(self.0) }
    }
}

impl From<u16> for TermColDelta {
    fn from(value: u16) -> Self { Self::new(value) }
}

impl From<ColWidth> for TermColDelta {
    /// Convert a [`ColWidth`] to a [`TermColDelta`].
    ///
    /// This allows natural conversion when a column width value needs to be used
    /// as a relative cursor movement amount.
    ///
    /// # Example
    ///
    /// ```rust
    /// use r3bl_tui::{width, TermColDelta, CsiSequence};
    ///
    /// let col_width = width(40);
    /// let delta = TermColDelta::from(col_width);
    ///
    /// if let Some(n) = delta.as_nonzero_u16() {
    ///     let _ = CsiSequence::CursorForward(n);
    /// }
    /// ```
    fn from(value: ColWidth) -> Self { Self::new(value.as_u16()) }
}

/// Create a [`TermColDelta`] from a raw value.
///
/// This is the preferred constructor for creating column deltas.
///
/// # Example
///
/// ```rust
/// use r3bl_tui::term_col_delta;
///
/// let delta = term_col_delta(5);
/// assert_eq!(delta.value(), 5);
/// ```
#[must_use]
pub const fn term_col_delta(value: u16) -> TermColDelta { TermColDelta::new(value) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_term_col_delta_zero() {
        let delta = term_col_delta(0);
        assert!(delta.is_zero());
        assert_eq!(delta.as_nonzero_u16(), None);
        assert_eq!(delta.value(), 0);
    }

    #[test]
    fn test_term_col_delta_nonzero() {
        let delta = term_col_delta(5);
        assert!(!delta.is_zero());
        assert_eq!(delta.as_nonzero_u16(), Some(5));
        assert_eq!(delta.value(), 5);
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", term_col_delta(5)), "TermColDelta(5)");
    }

    #[test]
    fn test_from_u16() {
        let delta: TermColDelta = 20.into();
        assert_eq!(delta.value(), 20);
    }

    #[test]
    fn test_from_col_width() {
        use crate::width;

        let col_width = width(40);
        let delta = TermColDelta::from(col_width);
        assert_eq!(delta.value(), 40);

        // Also test zero width.
        let zero_width = width(0);
        let zero_delta = TermColDelta::from(zero_width);
        assert!(zero_delta.is_zero());
        assert_eq!(zero_delta.as_nonzero_u16(), None);
    }

    #[test]
    fn test_default() {
        let delta = TermColDelta::default();
        assert!(delta.is_zero());
    }

    #[test]
    fn test_numeric_conversions() {
        let delta = term_col_delta(200);
        assert_eq!(delta.as_usize(), 200_usize);
        assert_eq!(delta.as_u16(), 200_u16);
    }

    /// Regression test: verify the CSI zero guard at column boundary.
    ///
    /// Position 240 on 80-col terminal = 240 % 80 = 0 cols (exactly at row start).
    /// This is the critical case that causes the off-by-one bug if not guarded.
    #[test]
    fn test_csi_zero_guard_at_column_boundary() {
        let position: u16 = 240;
        let term_width: u16 = 80;

        let cols = term_col_delta(position % term_width); // 0

        // Cols should NOT emit (this prevents the off-by-one bug)
        assert_eq!(cols.as_nonzero_u16(), None);

        // Position 245 = 5 cols right
        let some_cols = term_col_delta(245 % term_width);
        assert_eq!(some_cols.as_nonzero_u16(), Some(5));
    }
}
