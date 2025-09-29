// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Base trait for numeric conversions in the bounds checking system.
///
/// This trait provides the fundamental capability that enables all unit types
/// in the bounds checking system to convert to common numeric types for comparison
/// operations. It forms the foundation that allows generic implementations
/// of bounds checking across different unit types.
///
/// ## Purpose
///
/// The [`UnitMarker`] trait serves a single, focused purpose: standardized numeric
/// conversion for comparison operations. It enables generic bounds checking code
/// to work with any unit type without knowing the specific type's implementation details.
///
/// ## Key Functionality
///
/// - **Numeric conversions**: Convert unit types to [`usize`] and [`u16`] for operations
/// - **Zero checking**: Standardized way to test if a unit value represents zero
/// - **Generic foundation**: Enables type-safe generic bounds checking implementations
///
/// ## Implementation Requirements
///
/// Types implementing this trait must provide:
/// - Conversion to [`usize`] via [`as_usize()`] method (for array indexing)
/// - Conversion to [`u16`] via [`as_u16()`] method (for terminal/PTY operations)
/// - Construction from [`usize`] and [`u16`] via `From<usize>` and `From<u16>` trait
///   bounds
///
/// These requirements are enforced by the trait definition itself—implementations cannot
/// compile without providing all required methods and trait bounds.
///
/// ## Usage in Bounds Checking
///
/// This trait is implemented by all index and length marker types, allowing the
/// bounds checking system to perform numeric comparisons without losing type safety.
///
/// ## Design Philosophy
///
/// This trait embodies the principle of "mechanism, not policy":
/// - **Mechanism**: Provides the ability to convert to numeric types
/// - **Policy**: Other traits define what comparisons mean and when they're valid
///
/// The [`UnitMarker`] trait doesn't know about bounds checking semantics—it simply
/// provides the tools that enable higher-level traits to implement those semantics.
///
/// ## See Also
///
/// - [`IndexMarker`] - Builds on this trait to provide index-specific operations
/// - [`LengthMarker`] - Builds on this trait to provide length-specific operations
/// - [Module documentation] - Overview of the complete bounds checking system
///
/// [`IndexMarker`]: crate::IndexMarker
/// [`LengthMarker`]: crate::LengthMarker
/// [Module documentation]: mod@crate::core::units::bounds_check
/// [`as_usize()`]: Self::as_usize
/// [`as_u16()`]: Self::as_u16
pub trait UnitMarker: From<usize> + From<u16> + Copy + Ord + Sized {
    /// Convert the unit to a usize value for numeric comparison.
    ///
    /// See [trait-level documentation] for usage guidelines and design rationale.
    /// Prefer this over [`as_u16()`] for most operations due to its flexibility
    /// and compatibility with Rust's standard library.
    ///
    /// ## Common Use Cases
    ///
    /// - Array indexing in Vec, arrays, or buffers
    /// - Size calculations for lengths, capacities, or offsets
    /// - Loop bounds and iteration logic
    /// - Memory operations and layout calculations
    ///
    /// [`as_u16()`]: Self::as_u16
    /// [trait-level documentation]: UnitMarker
    fn as_usize(&self) -> usize;

    /// Convert the unit to a u16 value for terminal and PTY operations.
    ///
    /// See [trait-level documentation] for usage guidelines and design rationale.
    /// Use this specifically when interfacing with terminal libraries (like crossterm)
    /// or PTY operations that require 16-bit values.
    ///
    /// ## Common Use Cases
    ///
    /// - Terminal operations with crossterm and similar libraries
    /// - PTY communication and pseudo-terminal devices
    /// - Network protocols with 16-bit size limits
    /// - Memory-constrained embedded systems
    ///
    /// [trait-level documentation]: UnitMarker
    fn as_u16(&self) -> u16;

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
    /// [trait-level documentation]: UnitMarker
    fn is_zero(&self) -> bool { self.as_usize() == 0 }
}

#[cfg(test)]
mod tests {
    use std::ops::{Add, Sub};

    use super::*;

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

    impl UnitMarker for TestUnit {
        fn as_usize(&self) -> usize { self.0 }

        #[allow(clippy::cast_possible_truncation)]
        fn as_u16(&self) -> u16 { self.0 as u16 }
    }

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
