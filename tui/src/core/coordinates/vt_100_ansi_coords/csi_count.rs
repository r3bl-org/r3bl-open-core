// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Count parameter for CSI sequences that operate on lines or characters.
//!
//! [`CsiCount`] represents **how many** lines or characters to insert, delete, or erase.
//! It wraps [`NonZeroU16`] internally, making it **impossible** to create a zero count.
//!
//! # Make Illegal States Unrepresentable
//!
//! ANSI CSI count parameters interpret 0 as 1:
//! - `CSI 0 L` (insert 0 lines) → inserts **1 line**
//! - `CSI 0 P` (delete 0 chars) → deletes **1 character**
//!
//! By making zero counts unrepresentable, we eliminate this class of bugs entirely.
//!
//! # Example
//!
//! ```rust
//! use r3bl_tui::{CsiCount, CsiSequence};
//!
//! // Fallible construction - must handle the None case
//! if let Some(count) = CsiCount::new(5) {
//!     let _ = CsiSequence::InsertLine(count);
//! }
//!
//! // Use the ONE constant for single operations
//! let delete_one = CsiSequence::DeleteChar(CsiCount::ONE);
//! ```
//!
//! [`NonZeroU16`]: std::num::NonZeroU16

use crate::NumericConversions;
use std::{fmt::{Display, Formatter}, num::NonZeroU16};

/// Count of lines or characters for CSI operations (always >= 1).
///
/// Used with [`CsiSequence::InsertLine`], [`CsiSequence::DeleteLine`],
/// [`CsiSequence::InsertChar`], [`CsiSequence::DeleteChar`], and
/// [`CsiSequence::EraseChar`].
///
/// # Make Illegal States Unrepresentable
///
/// This type wraps [`NonZeroU16`] internally. A zero count **cannot exist**,
/// preventing the CSI zero bug at compile time:
///
/// ```rust
/// use r3bl_tui::CsiCount;
///
/// // Fallible construction - returns None for zero
/// assert!(CsiCount::new(0).is_none());
/// assert!(CsiCount::new(5).is_some());
///
/// // If you have a CsiCount, it's guaranteed non-zero
/// if let Some(count) = CsiCount::new(3) {
///     // Safe to emit - count is guaranteed non-zero
///     assert_eq!(count.get(), 3);
/// }
/// ```
///
/// [`NonZeroU16`]: std::num::NonZeroU16
/// [`CsiSequence::InsertLine`]: crate::CsiSequence::InsertLine
/// [`CsiSequence::DeleteLine`]: crate::CsiSequence::DeleteLine
/// [`CsiSequence::InsertChar`]: crate::CsiSequence::InsertChar
/// [`CsiSequence::DeleteChar`]: crate::CsiSequence::DeleteChar
/// [`CsiSequence::EraseChar`]: crate::CsiSequence::EraseChar
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CsiCount(NonZeroU16);

impl NumericConversions for CsiCount {
    fn as_usize(&self) -> usize { self.0.get() as usize }
    fn as_u16(&self) -> u16 { self.0.get() }
}

impl Display for CsiCount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "CsiCount({})", self.0)
    }
}

impl CsiCount {
    /// A count of 1 - the most common count value.
    ///
    /// Use this constant instead of `CsiCount::new(1).unwrap()` to avoid
    /// panic documentation requirements and make intent clear.
    pub const ONE: Self = Self(NonZeroU16::new(1).unwrap());

    /// Create a new count from a raw value.
    ///
    /// Returns `None` if the value is zero, since zero counts are not
    /// representable (they would cause the CSI zero bug).
    ///
    /// # Example
    ///
    /// ```rust
    /// use r3bl_tui::CsiCount;
    ///
    /// assert!(CsiCount::new(0).is_none());  // Zero not allowed
    /// assert!(CsiCount::new(5).is_some());  // Non-zero OK
    /// ```
    #[must_use]
    pub const fn new(value: u16) -> Option<Self> {
        match NonZeroU16::new(value) {
            Some(nz) => Some(Self(nz)),
            None => None,
        }
    }

    /// Create a count from a [`NonZeroU16`] value.
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

impl From<NonZeroU16> for CsiCount {
    fn from(value: NonZeroU16) -> Self { Self::from_non_zero(value) }
}

/// Create a [`CsiCount`] from a raw value.
///
/// Returns `None` if the value is zero.
///
/// # Example
///
/// ```rust
/// use r3bl_tui::csi_count;
///
/// assert!(csi_count(0).is_none());
/// assert_eq!(csi_count(3).map(|c| c.get()), Some(3));
/// ```
#[must_use]
pub const fn csi_count(value: u16) -> Option<CsiCount> { CsiCount::new(value) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_zero_returns_none() {
        assert!(CsiCount::new(0).is_none());
        assert!(csi_count(0).is_none());
    }

    #[test]
    fn test_new_non_zero_returns_some() {
        let count = CsiCount::new(3).unwrap();
        assert_eq!(count.get(), 3);

        let count2 = csi_count(5).unwrap();
        assert_eq!(count2.get(), 5);
    }

    #[test]
    fn test_from_non_zero() {
        let nz = NonZeroU16::new(7).unwrap();
        let count = CsiCount::from_non_zero(nz);
        assert_eq!(count.get(), 7);
    }

    #[test]
    fn test_from_trait() {
        let nz = NonZeroU16::new(10).unwrap();
        let count: CsiCount = nz.into();
        assert_eq!(count.get(), 10);
    }

    #[test]
    fn test_one_constant() {
        assert_eq!(CsiCount::ONE.get(), 1);
    }

    #[test]
    fn test_display() {
        let count = CsiCount::new(3).unwrap();
        assert_eq!(format!("{count}"), "CsiCount(3)");
    }

    #[test]
    fn test_numeric_conversions() {
        let count = CsiCount::new(100).unwrap();
        assert_eq!(count.as_usize(), 100_usize);
        assert_eq!(count.as_u16(), 100_u16);
    }

    #[test]
    fn test_value_returns_non_zero_u16() {
        let count = CsiCount::new(5).unwrap();
        let nz: NonZeroU16 = count.value();
        assert_eq!(nz.get(), 5);
    }

    /// Regression test: verify CSI zero bug is prevented at type level.
    #[test]
    fn test_csi_zero_bug_prevented() {
        // 0 lines/chars - should be None (prevents CSI zero bug).
        assert!(csi_count(0).is_none());

        // 1 line/char - should succeed.
        assert_eq!(csi_count(1).map(CsiCount::get), Some(1));

        // Multiple lines/chars - should succeed.
        assert_eq!(csi_count(5).map(CsiCount::get), Some(5));
    }
}
