// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Relative horizontal cursor movement for VT-100 ANSI sequences.
//!
//! [`TermColDelta`] represents **how many columns** to move the cursor, as opposed to
//! [`TermCol`] which represents **which column** the cursor is on.
//!
//! # Make Illegal States Unrepresentable
//!
//! This type wraps [`NonZeroU16`] internally, making it **impossible** to create a
//! zero-valued delta. This prevents the CSI zero bug at the type level.
//!
//! ANSI cursor movement commands (CUF, CUB) interpret parameter 0 as 1:
//! - `CSI 0 C` (`CursorForward` with n=0) moves the cursor **1 column right**, not 0
//! - `CSI 0 D` (`CursorBackward` with n=0) moves the cursor **1 column left**, not 0
//!
//! By making zero deltas unrepresentable, we eliminate this class of bugs entirely.
//!
//! # Example
//!
//! ```rust
//! use r3bl_tui::{TermColDelta, CsiSequence};
//!
//! let position: u16 = 240;
//! let term_width: u16 = 80;
//!
//! let cols_right = position % term_width; // 0 cols (exactly at row boundary)
//!
//! // Fallible construction - must handle the None case
//! if let Some(delta) = TermColDelta::new(cols_right) {
//!     let _ = CsiSequence::CursorForward(delta);
//! }
//! // For cols_right=0, new() returns None, so no sequence is emitted
//! ```
//!
//! [`TermCol`]: super::TermCol
//! [`NonZeroU16`]: std::num::NonZeroU16

use crate::NumericConversions;
use std::{fmt::{Display, Formatter}, num::NonZeroU16};

/// Relative horizontal cursor movement (column delta).
///
/// Represents how many columns to move left or right. Used with
/// [`CsiSequence::CursorForward`] and [`CsiSequence::CursorBackward`].
///
/// # Make Illegal States Unrepresentable
///
/// This type wraps [`NonZeroU16`] internally. A zero-valued delta **cannot exist**,
/// preventing the CSI zero bug at compile time:
///
/// ```rust
/// use r3bl_tui::{TermColDelta, CsiSequence};
///
/// // Fallible construction - returns None for zero
/// assert!(TermColDelta::new(0).is_none());
/// assert!(TermColDelta::new(5).is_some());
///
/// // If you have a TermColDelta, it's guaranteed non-zero
/// if let Some(delta) = TermColDelta::new(5) {
///     // Safe to emit - delta is guaranteed non-zero
///     let seq = CsiSequence::CursorForward(delta);
/// }
/// ```
///
/// [`NonZeroU16`]: std::num::NonZeroU16
/// [`CsiSequence::CursorForward`]: crate::CsiSequence::CursorForward
/// [`CsiSequence::CursorBackward`]: crate::CsiSequence::CursorBackward
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TermColDelta(NonZeroU16);

impl NumericConversions for TermColDelta {
    fn as_usize(&self) -> usize { self.0.get() as usize }
    fn as_u16(&self) -> u16 { self.0.get() }
}

impl Display for TermColDelta {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TermColDelta({})", self.0)
    }
}

impl TermColDelta {
    /// A delta of 1 column - the most common cursor movement amount.
    ///
    /// Use this constant instead of `TermColDelta::new(1).unwrap()` to avoid
    /// panic documentation requirements and make intent clear.
    pub const ONE: Self = Self(NonZeroU16::new(1).unwrap());

    /// Create a new column delta from a raw value.
    ///
    /// Returns `None` if the value is zero, since zero-valued deltas are not
    /// representable (they would cause the CSI zero bug).
    ///
    /// # Example
    ///
    /// ```rust
    /// use r3bl_tui::TermColDelta;
    ///
    /// assert!(TermColDelta::new(0).is_none());  // Zero not allowed
    /// assert!(TermColDelta::new(5).is_some());  // Non-zero OK
    /// ```
    #[must_use]
    pub const fn new(value: u16) -> Option<Self> {
        match NonZeroU16::new(value) {
            Some(nz) => Some(Self(nz)),
            None => None,
        }
    }

    /// Create a column delta from a [`NonZeroU16`] value.
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

impl From<NonZeroU16> for TermColDelta {
    fn from(value: NonZeroU16) -> Self { Self::from_non_zero(value) }
}

/// Create a [`TermColDelta`] from a raw value.
///
/// Returns `None` if the value is zero.
///
/// # Example
///
/// ```rust
/// use r3bl_tui::term_col_delta;
///
/// assert!(term_col_delta(0).is_none());
/// assert_eq!(term_col_delta(5).map(|d| d.get()), Some(5));
/// ```
#[must_use]
pub const fn term_col_delta(value: u16) -> Option<TermColDelta> { TermColDelta::new(value) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_zero_returns_none() {
        assert!(TermColDelta::new(0).is_none());
        assert!(term_col_delta(0).is_none());
    }

    #[test]
    fn test_new_non_zero_returns_some() {
        let delta = TermColDelta::new(5).unwrap();
        assert_eq!(delta.get(), 5);

        let delta2 = term_col_delta(10).unwrap();
        assert_eq!(delta2.get(), 10);
    }

    #[test]
    fn test_from_non_zero() {
        let nz = NonZeroU16::new(7).unwrap();
        let delta = TermColDelta::from_non_zero(nz);
        assert_eq!(delta.get(), 7);
    }

    #[test]
    fn test_from_trait() {
        let nz = NonZeroU16::new(20).unwrap();
        let delta: TermColDelta = nz.into();
        assert_eq!(delta.get(), 20);
    }

    #[test]
    fn test_display() {
        let delta = TermColDelta::new(5).unwrap();
        assert_eq!(format!("{delta}"), "TermColDelta(5)");
    }

    #[test]
    fn test_numeric_conversions() {
        let delta = TermColDelta::new(200).unwrap();
        assert_eq!(delta.as_usize(), 200_usize);
        assert_eq!(delta.as_u16(), 200_u16);
    }

    #[test]
    fn test_value_returns_non_zero_u16() {
        let delta = TermColDelta::new(5).unwrap();
        let nz: NonZeroU16 = delta.value();
        assert_eq!(nz.get(), 5);
    }

    /// Regression test: verify CSI zero bug is prevented at type level.
    ///
    /// Position 240 on 80-col terminal = 240 % 80 = 0 cols (exactly at row start).
    /// This is the critical case that causes the off-by-one bug if not guarded.
    #[test]
    fn test_csi_zero_bug_prevented_at_column_boundary() {
        let term_width: u16 = 80;

        // 240 % 80 = 0 cols - should be None (prevents CSI zero bug).
        let cols_0 = term_col_delta(240 % term_width);
        assert!(cols_0.is_none());

        // 245 % 80 = 5 cols - should succeed.
        let cols_5 = term_col_delta(245 % term_width);
        assert_eq!(cols_5.map(super::TermColDelta::get), Some(5));

        // 80 % 80 = 0 cols - should be None.
        let cols_boundary = term_col_delta(80 % term_width);
        assert!(cols_boundary.is_none());

        // 79 % 80 = 79 cols - should succeed.
        let cols_79 = term_col_delta(79 % term_width);
        assert_eq!(cols_79.map(super::TermColDelta::get), Some(79));
    }
}
