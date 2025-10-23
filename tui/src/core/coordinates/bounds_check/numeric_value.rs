// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Base traits for numeric conversions - see [`NumericValue`] and [`NumericConversions`].

/// Base trait for reading numeric values from wrapper types.
///
/// `NumericConversions` provides the foundational conversion methods that enable all
/// numeric types in the bounds checking system to convert to standard Rust integer
/// types. It separates the concern of "reading values" from "constructing values".
///
/// ## Purpose
///
/// This trait serves as the minimal interface for types that wrap numeric
/// values and need to expose those values as [`usize`] or [`u16`]. This trait is extended
/// by [`NumericValue`] (which adds construction from integers) and is also used by
/// types that cannot be constructed from arbitrary integers (like terminal coordinates
/// that must be non-zero).
///
/// ## Implementing Types
///
/// This trait is implemented by:
/// - All index and length types (via [`NumericValue`])
/// - Terminal coordinate types ([`TermRow`], [`TermCol`]) that wrap [`NonZeroU16`]
///
/// ## Design Rationale
///
/// By separating reading ([`as_usize`], [`as_u16`]) from construction ([`From<usize>`],
/// [`From<u16>`]), we allow types with construction constraints (like non-zero values)
/// to participate in generic numeric operations without violating their invariants.
///
/// [`TermRow`]: crate::TermRow
/// [`TermCol`]: crate::TermCol
/// [`NonZeroU16`]: std::num::NonZeroU16
/// [`as_usize`]: Self::as_usize
/// [`as_u16`]: Self::as_u16
/// [`From<usize>`]: std::convert::From
/// [`From<u16>`]: std::convert::From
pub trait NumericConversions: Copy + Sized {
    /// Convert to a [`usize`] value for array indexing and size calculations.
    ///
    /// This is the preferred conversion method for most operations due to its
    /// flexibility and compatibility with Rust's standard library.
    fn as_usize(&self) -> usize;

    /// Convert to a [`u16`] value for terminal and PTY operations.
    ///
    /// Use this when interfacing with terminal libraries or PTY operations
    /// that require 16-bit values.
    fn as_u16(&self) -> u16;
}

/// Base trait for numeric conversions in the bounds checking system.
///
/// `NumericValue` provides standardized numeric conversion capabilities for any type that
/// represents a numeric value. It enables generic implementations that can work with
/// diverse numeric types without knowing their specific implementation details.
///
/// ## Purpose
///
/// This trait serves a single, focused purpose: standardized numeric conversion for
/// comparison operations. Any type that wraps a numeric value and needs to participate in
/// generic numeric operations can implement this trait.
///
/// ## Key Trait Capabilities
///
/// - **Numeric conversions**: Convert to [`usize`] and [`u16`] via [`as_usize()`] and
///   [`as_u16()`]
/// - **Instance construction**: Create instances from [`usize`] and [`u16`] via
///   `From<usize>` and `From<u16>`
/// - **Zero checking**: Test if a value represents zero via [`is_zero()`]
/// - **Generic foundation**: Enables type-safe generic implementations across numeric
///   types
///
/// ## Implementing Types
///
/// While this trait is general-purpose, it is currently implemented by all index and
/// length types in the bounds checking system:
///
/// **Index types** (0-based positions):
/// - [`Index`] - Generic position (dimension-agnostic)
/// - [`RowIndex`] - Vertical position in terminal grid
/// - [`ColIndex`] - Horizontal position in terminal grid
/// - [`ByteIndex`] - Byte position in UTF-8 strings
/// - [`SegIndex`] - Grapheme segment position
///
/// **Length types** (1-based sizes):
/// - [`Length`] - Generic size (dimension-agnostic)
/// - [`RowHeight`] - Vertical size in terminal grid
/// - [`ColWidth`] - Horizontal size in terminal grid
/// - [`ByteLength`] - Byte count in UTF-8 strings
/// - [`SegLength`] - Grapheme segment count
///
/// **Other numeric types**:
/// - [`ChUnit`] - Character unit for text measurement
///
/// ## Examples
///
/// The [`NumericValue`] trait provides standardized numeric conversions for all
/// index and length types:
///
/// ```rust
/// use r3bl_tui::{col, width, ColIndex, ColWidth, NumericValue};
///
/// let index = col(42);
/// let length = width(100);
///
/// // Convert to numeric types
/// let buffer_pos: usize = index.as_usize(); // For array indexing
/// let terminal_col: u16 = index.as_u16();   // For terminal/PTY operations
/// assert_eq!(buffer_pos, 42);
/// assert_eq!(terminal_col, 42);
///
/// // Create from numeric types
/// let from_usize = ColIndex::from(42_usize);
/// let from_u16 = ColIndex::from(42_u16);
/// assert_eq!(index, from_usize);
/// assert_eq!(index, from_u16);
///
/// // Check for zero values
/// let zero_length = width(0);
/// let non_zero_length = width(10);
/// assert!(zero_length.is_zero());
/// assert!(!non_zero_length.is_zero());
/// ```
///
/// [`as_usize()`]: NumericConversions::as_usize
/// [`as_u16()`]: NumericConversions::as_u16
/// [`is_zero()`]: Self::is_zero
/// [`Index`]: crate::Index
/// [`RowIndex`]: crate::RowIndex
/// [`ColIndex`]: crate::ColIndex
/// [`ByteIndex`]: crate::ByteIndex
/// [`SegIndex`]: crate::SegIndex
/// [`Length`]: crate::Length
/// [`RowHeight`]: crate::RowHeight
/// [`ColWidth`]: crate::ColWidth
/// [`ByteLength`]: crate::ByteLength
/// [`SegLength`]: crate::SegLength
/// [`ChUnit`]: crate::ChUnit
pub trait NumericValue: NumericConversions + From<usize> + From<u16> + Ord {
    /// Check if the unit value is zero.
    ///
    /// See [trait-level documentation] for usage guidelines. The default
    /// implementation uses `as_usize() == 0`. Types with special zero semantics
    /// can override this method if needed.
    ///
    /// ## Common Use Cases
    ///
    /// - Empty container checks
    /// - Origin position tests
    /// - Validation and edge case handling
    /// - Initialization verification
    ///
    /// [trait-level documentation]: NumericValue
    fn is_zero(&self) -> bool { self.as_usize() == 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::{Add, Sub};

    // Test implementation for unit testing
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    struct TestUnit(usize);

    impl From<usize> for TestUnit {
        fn from(value: usize) -> Self { TestUnit(value) }
    }

    impl From<u16> for TestUnit {
        fn from(value: u16) -> Self { TestUnit(value as usize) }
    }

    impl Add for TestUnit {
        type Output = Self;
        fn add(self, other: Self) -> Self { TestUnit(self.0.saturating_add(other.0)) }
    }

    impl Sub for TestUnit {
        type Output = Self;
        fn sub(self, other: Self) -> Self { TestUnit(self.0.saturating_sub(other.0)) }
    }

    impl NumericConversions for TestUnit {
        fn as_usize(&self) -> usize { self.0 }

        #[allow(clippy::cast_possible_truncation)]
        fn as_u16(&self) -> u16 { self.0 as u16 }
    }

    impl NumericValue for TestUnit {}

    #[test]
    fn test_as_usize_conversion() {
        let unit = TestUnit::from(42usize);
        assert_eq!(unit.as_usize(), 42);
    }

    #[test]
    fn test_as_u16_conversion() {
        let unit = TestUnit::from(42usize);
        assert_eq!(unit.as_u16(), 42u16);
    }

    #[test]
    fn test_from_usize() {
        let unit = TestUnit::from(123usize);
        assert_eq!(unit.as_usize(), 123);
    }

    #[test]
    fn test_from_u16() {
        let unit = TestUnit::from(456u16);
        assert_eq!(unit.as_usize(), 456);
    }

    #[test]
    fn test_is_zero_default_implementation() {
        let zero_unit = TestUnit::from(0usize);
        let non_zero_unit = TestUnit::from(42usize);

        assert!(zero_unit.is_zero());
        assert!(!non_zero_unit.is_zero());
    }

    #[test]
    fn test_zero_edge_cases() {
        // Test conversion edge cases for zero
        let zero_from_usize = TestUnit::from(0usize);
        let zero_from_u16 = TestUnit::from(0u16);

        assert!(zero_from_usize.is_zero());
        assert!(zero_from_u16.is_zero());
        assert_eq!(zero_from_usize.as_usize(), 0);
        assert_eq!(zero_from_u16.as_u16(), 0);
    }

    #[test]
    fn test_large_values() {
        // Test with larger values to ensure conversion stability
        let large_value = 65535usize;
        let unit = TestUnit::from(large_value);

        assert_eq!(unit.as_usize(), large_value);
        #[allow(clippy::cast_possible_truncation)]
        let expected_u16 = large_value as u16;
        assert_eq!(unit.as_u16(), expected_u16);
        assert!(!unit.is_zero());
    }

    #[test]
    fn test_u16_overflow_edge_case() {
        // Test what happens when usize value exceeds u16 range
        let large_value = 70000usize; // Exceeds u16::MAX (65535)
        let unit = TestUnit::from(large_value);

        assert_eq!(unit.as_usize(), large_value);
        // This should truncate to fit in u16
        #[allow(clippy::cast_possible_truncation)]
        let expected_u16 = large_value as u16;
        assert_eq!(unit.as_u16(), expected_u16);
        assert!(!unit.is_zero());
    }
}
