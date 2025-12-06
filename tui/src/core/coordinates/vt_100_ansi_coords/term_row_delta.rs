// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Relative vertical cursor movement for VT-100 ANSI sequences.
//!
//! [`TermRowDelta`] represents **how many rows** to move the cursor, as opposed to
//! [`TermRow`] which represents **which row** the cursor is on.
//!
//! # The CSI Zero Problem
//!
//! ANSI cursor movement commands (CUU, CUD) interpret parameter 0 as 1:
//! - `CSI 0 A` (`CursorUp` with n=0) moves the cursor **1 row up**, not 0
//! - `CSI 0 B` (`CursorDown` with n=0) moves the cursor **1 row down**, not 0
//!
//! This type provides [`as_nonzero_u16()`] to guard against this bug.
//!
//! # Example
//!
//! ```rust
//! use r3bl_tui::{TermRowDelta, term_row_delta, CsiSequence};
//!
//! let position: u16 = 240;
//! let term_width: u16 = 80;
//!
//! let rows_down = term_row_delta(position / term_width); // 3 rows
//!
//! // Safe emission: only emit if non-zero
//! if let Some(n) = rows_down.as_nonzero_u16() {
//!     let _ = CsiSequence::CursorDown(n);
//! }
//! ```
//!
//! [`TermRow`]: super::TermRow
//! [`as_nonzero_u16()`]: TermRowDelta::as_nonzero_u16

use crate::{NumericConversions, RowHeight};
use std::fmt::{Display, Formatter};

/// Relative vertical cursor movement (row delta).
///
/// Represents how many rows to move up or down. Used with [`CsiSequence::CursorUp`]
/// and [`CsiSequence::CursorDown`].
///
/// # Safety Against CSI Zero Bug
///
/// Use [`as_nonzero_u16()`] to safely emit cursor movement commands:
///
/// ```rust
/// use r3bl_tui::{term_row_delta, CsiSequence};
/// use std::io::Write;
///
/// let delta = term_row_delta(0);
///
/// // Safe: only emit if non-zero
/// if let Some(n) = delta.as_nonzero_u16() {
///     let seq = CsiSequence::CursorDown(n);
///     // write seq to terminal...
/// }
/// // For delta=0, no sequence is emitted (correct behavior)
/// ```
///
/// [`CsiSequence::CursorUp`]: crate::CsiSequence::CursorUp
/// [`CsiSequence::CursorDown`]: crate::CsiSequence::CursorDown
/// [`as_nonzero_u16()`]: Self::as_nonzero_u16
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TermRowDelta(u16);

impl NumericConversions for TermRowDelta {
    fn as_usize(&self) -> usize { self.0 as usize }
    fn as_u16(&self) -> u16 { self.0 }
}

impl Display for TermRowDelta {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TermRowDelta({})", self.0)
    }
}

impl TermRowDelta {
    /// Create a new row delta from a raw value.
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
    /// It prevents the CSI zero bug where `CSI 0 A` is interpreted as `CSI 1 A`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use r3bl_tui::{term_row_delta, CsiSequence};
    ///
    /// let delta = term_row_delta(3);
    /// if let Some(n) = delta.as_nonzero_u16() {
    ///     let _ = CsiSequence::CursorDown(n);
    /// }
    /// ```
    #[must_use]
    pub const fn as_nonzero_u16(self) -> Option<u16> {
        if self.0 == 0 { None } else { Some(self.0) }
    }
}

impl From<u16> for TermRowDelta {
    fn from(value: u16) -> Self { Self::new(value) }
}

impl From<RowHeight> for TermRowDelta {
    /// Convert a [`RowHeight`] to a [`TermRowDelta`].
    ///
    /// This allows natural conversion when a row height value needs to be used
    /// as a relative cursor movement amount.
    ///
    /// # Example
    ///
    /// ```rust
    /// use r3bl_tui::{height, TermRowDelta, CsiSequence};
    ///
    /// let row_height = height(10);
    /// let delta = TermRowDelta::from(row_height);
    ///
    /// if let Some(n) = delta.as_nonzero_u16() {
    ///     let _ = CsiSequence::CursorDown(n);
    /// }
    /// ```
    fn from(value: RowHeight) -> Self { Self::new(value.as_u16()) }
}

/// Create a [`TermRowDelta`] from a raw value.
///
/// This is the preferred constructor for creating row deltas.
///
/// # Example
///
/// ```rust
/// use r3bl_tui::term_row_delta;
///
/// let delta = term_row_delta(3);
/// assert_eq!(delta.value(), 3);
/// ```
#[must_use]
pub const fn term_row_delta(value: u16) -> TermRowDelta { TermRowDelta::new(value) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_term_row_delta_zero() {
        let delta = term_row_delta(0);
        assert!(delta.is_zero());
        assert_eq!(delta.as_nonzero_u16(), None);
        assert_eq!(delta.value(), 0);
    }

    #[test]
    fn test_term_row_delta_nonzero() {
        let delta = term_row_delta(3);
        assert!(!delta.is_zero());
        assert_eq!(delta.as_nonzero_u16(), Some(3));
        assert_eq!(delta.value(), 3);
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", term_row_delta(3)), "TermRowDelta(3)");
    }

    #[test]
    fn test_from_u16() {
        let delta: TermRowDelta = 10.into();
        assert_eq!(delta.value(), 10);
    }

    #[test]
    fn test_from_row_height() {
        use crate::height;

        let row_height = height(10);
        let delta = TermRowDelta::from(row_height);
        assert_eq!(delta.value(), 10);

        // Also test zero height.
        let zero_height = height(0);
        let zero_delta = TermRowDelta::from(zero_height);
        assert!(zero_delta.is_zero());
        assert_eq!(zero_delta.as_nonzero_u16(), None);
    }

    #[test]
    fn test_default() {
        let delta = TermRowDelta::default();
        assert!(delta.is_zero());
    }

    #[test]
    fn test_numeric_conversions() {
        let delta = term_row_delta(100);
        assert_eq!(delta.as_usize(), 100_usize);
        assert_eq!(delta.as_u16(), 100_u16);
    }

    /// Regression test: verify the CSI zero guard works correctly.
    #[test]
    fn test_csi_zero_guard_at_row_boundary() {
        // Position 240 on 80-col terminal = exactly 3 rows down
        let position: u16 = 240;
        let term_width: u16 = 80;

        let rows = term_row_delta(position / term_width); // 3

        // Rows should emit
        assert_eq!(rows.as_nonzero_u16(), Some(3));

        // Position 0 = no movement
        let zero_rows = term_row_delta(0);
        assert_eq!(zero_rows.as_nonzero_u16(), None);
    }
}
