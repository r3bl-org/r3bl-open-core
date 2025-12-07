// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Relative vertical cursor movement for VT-100 ANSI sequences.
//!
//! [`TermRowDelta`] represents **how many rows** to move the cursor, as opposed to
//! [`TermRow`] which represents **which row** the cursor is on.
//!
//! # Make Illegal States Unrepresentable
//!
//! This type wraps [`NonZeroU16`] internally, making it **impossible** to create a
//! zero-valued delta. This prevents the CSI zero bug at the type level.
//!
//! ANSI cursor movement commands (CUU, CUD) interpret parameter 0 as 1:
//! - `CSI 0 A` (`CursorUp` with n=0) moves the cursor **1 row up**, not 0
//! - `CSI 0 B` (`CursorDown` with n=0) moves the cursor **1 row down**, not 0
//!
//! By making zero deltas unrepresentable, we eliminate this class of bugs entirely.
//!
//! # Example
//!
//! ```rust
//! use r3bl_tui::{TermRowDelta, CsiSequence};
//!
//! let position: u16 = 240;
//! let term_width: u16 = 80;
//!
//! let rows_down = position / term_width; // 3 rows
//!
//! // Fallible construction - must handle the None case
//! if let Some(delta) = TermRowDelta::new(rows_down) {
//!     let _ = CsiSequence::CursorDown(delta);
//! }
//! // For rows_down=0, new() returns None, so no sequence is emitted
//! ```
//!
//! [`TermRow`]: super::TermRow
//! [`NonZeroU16`]: std::num::NonZeroU16

use crate::NumericConversions;
use std::{fmt::{Display, Formatter}, num::NonZeroU16};

/// Relative vertical cursor movement (row delta).
///
/// Represents how many rows to move up or down. Used with [`CsiSequence::CursorUp`]
/// and [`CsiSequence::CursorDown`].
///
/// # Make Illegal States Unrepresentable
///
/// This type wraps [`NonZeroU16`] internally. A zero-valued delta **cannot exist**,
/// preventing the CSI zero bug at compile time:
///
/// ```rust
/// use r3bl_tui::{TermRowDelta, CsiSequence};
///
/// // Fallible construction - returns None for zero
/// assert!(TermRowDelta::new(0).is_none());
/// assert!(TermRowDelta::new(5).is_some());
///
/// // If you have a TermRowDelta, it's guaranteed non-zero
/// if let Some(delta) = TermRowDelta::new(3) {
///     // Safe to emit - delta is guaranteed non-zero
///     let seq = CsiSequence::CursorDown(delta);
/// }
/// ```
///
/// [`NonZeroU16`]: std::num::NonZeroU16
/// [`CsiSequence::CursorUp`]: crate::CsiSequence::CursorUp
/// [`CsiSequence::CursorDown`]: crate::CsiSequence::CursorDown
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermRowDelta(NonZeroU16);

impl NumericConversions for TermRowDelta {
    fn as_usize(&self) -> usize { self.0.get() as usize }
    fn as_u16(&self) -> u16 { self.0.get() }
}

impl Display for TermRowDelta {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TermRowDelta({})", self.0)
    }
}

impl TermRowDelta {
    /// A delta of 1 row - the most common cursor movement amount.
    ///
    /// Use this constant instead of `TermRowDelta::new(1).unwrap()` to avoid
    /// panic documentation requirements and make intent clear.
    pub const ONE: Self = Self(NonZeroU16::new(1).unwrap());

    /// Create a new row delta from a raw value.
    ///
    /// Returns `None` if the value is zero, since zero-valued deltas are not
    /// representable (they would cause the CSI zero bug).
    ///
    /// # Example
    ///
    /// ```rust
    /// use r3bl_tui::TermRowDelta;
    ///
    /// assert!(TermRowDelta::new(0).is_none());  // Zero not allowed
    /// assert!(TermRowDelta::new(5).is_some());  // Non-zero OK
    /// ```
    #[must_use]
    pub const fn new(value: u16) -> Option<Self> {
        match NonZeroU16::new(value) {
            Some(nz) => Some(Self(nz)),
            None => None,
        }
    }

    /// Create a row delta from a [`NonZeroU16`] value.
    ///
    /// This is an infallible constructor since [`NonZeroU16`] is already guaranteed
    /// to be non-zero.
    #[must_use]
    pub const fn from_non_zero(value: NonZeroU16) -> Self { Self(value) }

    /// Get the inner [`NonZeroU16`] value.
    #[must_use]
    pub const fn value(self) -> NonZeroU16 { self.0 }

    /// Get the raw `u16` value (guaranteed to be >= 1).
    #[must_use]
    pub const fn get(self) -> u16 { self.0.get() }
}

impl From<NonZeroU16> for TermRowDelta {
    fn from(value: NonZeroU16) -> Self { Self::from_non_zero(value) }
}

/// Create a [`TermRowDelta`] from a raw value.
///
/// Returns `None` if the value is zero.
///
/// # Example
///
/// ```rust
/// use r3bl_tui::term_row_delta;
///
/// assert!(term_row_delta(0).is_none());
/// assert_eq!(term_row_delta(3).map(|d| d.get()), Some(3));
/// ```
#[must_use]
pub const fn term_row_delta(value: u16) -> Option<TermRowDelta> { TermRowDelta::new(value) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_zero_returns_none() {
        assert!(TermRowDelta::new(0).is_none());
        assert!(term_row_delta(0).is_none());
    }

    #[test]
    fn test_new_non_zero_returns_some() {
        let delta = TermRowDelta::new(3).unwrap();
        assert_eq!(delta.get(), 3);

        let delta2 = term_row_delta(5).unwrap();
        assert_eq!(delta2.get(), 5);
    }

    #[test]
    fn test_from_non_zero() {
        let nz = NonZeroU16::new(7).unwrap();
        let delta = TermRowDelta::from_non_zero(nz);
        assert_eq!(delta.get(), 7);
    }

    #[test]
    fn test_from_trait() {
        let nz = NonZeroU16::new(10).unwrap();
        let delta: TermRowDelta = nz.into();
        assert_eq!(delta.get(), 10);
    }

    #[test]
    fn test_display() {
        let delta = TermRowDelta::new(3).unwrap();
        assert_eq!(format!("{delta}"), "TermRowDelta(3)");
    }

    #[test]
    fn test_numeric_conversions() {
        let delta = TermRowDelta::new(100).unwrap();
        assert_eq!(delta.as_usize(), 100_usize);
        assert_eq!(delta.as_u16(), 100_u16);
    }

    #[test]
    fn test_value_returns_non_zero_u16() {
        let delta = TermRowDelta::new(5).unwrap();
        let nz: NonZeroU16 = delta.value();
        assert_eq!(nz.get(), 5);
    }

    /// Regression test: verify CSI zero bug is prevented at type level.
    ///
    /// Position 240 on 80-col terminal = exactly 3 rows down.
    /// Position 80 on 80-col terminal = exactly 1 row down.
    /// Position 0 on 80-col terminal = 0 rows (None).
    #[test]
    fn test_csi_zero_bug_prevented() {
        let term_width: u16 = 80;

        // 240 / 80 = 3 rows - should succeed.
        let rows_3 = term_row_delta(240 / term_width);
        assert_eq!(rows_3.map(super::TermRowDelta::get), Some(3));

        // 80 / 80 = 1 row - should succeed.
        let rows_1 = term_row_delta(80 / term_width);
        assert_eq!(rows_1.map(super::TermRowDelta::get), Some(1));

        // 0 / 80 = 0 rows - should be None (prevents CSI zero bug).
        #[allow(clippy::erasing_op)]
        let rows_0 = term_row_delta(0 / term_width);
        assert!(rows_0.is_none());

        // 79 / 80 = 0 rows - should be None.
        let rows_partial = term_row_delta(79 / term_width);
        assert!(rows_partial.is_none());
    }
}
