// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Generates `Add`, `AddAssign`, `Sub`, and `SubAssign` implementations for numeric types.
///
/// This macro reduces boilerplate for types that need to support arithmetic operations
/// with primitive numeric types (usize, u16, i32). It's used internally by dimension
/// types like `ColIndex` and `RowIndex` to provide ergonomic arithmetic operations.
///
/// # Parameters
/// - `$type`: The struct type that will receive the implementations (e.g., `ColIndex`, `RowIndex`)
/// - `$constructor`: The constructor function that creates instances of `$type` (e.g., `col`, `row`)
/// - `[$($numeric_type:ident),*]`: Array of numeric types to implement operations for
///
/// # Generated Operations
/// For each numeric type `T` in the array, the macro generates:
/// - `impl Add<T> for $type` - Addition that returns a new instance
/// - `impl AddAssign<T> for $type` - In-place addition
/// - `impl Sub<T> for $type` - Subtraction that returns a new instance (uses `saturating_sub`)
/// - `impl SubAssign<T> for $type` - In-place subtraction
///
/// # Type Requirements
/// The target type (`$type`) must implement:
/// - `as_usize(self) -> usize` - Convert to usize for arithmetic
/// - `as_u16(self) -> u16` - Convert to u16 for arithmetic (when using u16 operations)
/// 
/// The constructor function must accept the result of arithmetic operations.
///
/// # Special Handling
/// - **Subtraction**: Uses `saturating_sub()` to prevent underflow (returns 0 instead of panicking)
/// - **i32 operations**: Negative values are treated as 0 using `rhs.max(0)` before conversion
/// - **Type conversions**: Each numeric type uses its appropriate conversion method:
///   - `usize`: Uses `as_usize()` directly
///   - `u16`: Uses `as_u16()` and converts result back via constructor
///   - `i32`: Converts to `usize` via `as_usize()` after clamping negatives to 0
///
/// # Example Usage
/// This macro is used internally by dimension types:
/// ```ignore
/// create_numeric_arithmetic_operators!(ColIndex, col, [usize, u16, i32]);
/// create_numeric_arithmetic_operators!(RowIndex, row, [usize, u16, i32]);
/// ```
///
/// The generated implementations enable operations like:
/// ```
/// use r3bl_tui::{ColIndex, col};
/// 
/// let index = col(10);
/// 
/// // Basic operations work with different numeric types
/// assert_eq!(index + 5usize, col(15));
/// assert_eq!(index + 3u16, col(13));
/// assert_eq!(index - 3usize, col(7));
/// 
/// // Special behaviors: underflow protection and negative i32 handling
/// assert_eq!(col(2) - 5usize, col(0));  // saturating_sub prevents panic
/// assert_eq!(index + (-5i32), col(10)); // negative i32 becomes 0
/// ```
macro_rules! create_numeric_arithmetic_operators {
    ($type:ty, $constructor:ident, [$($numeric_type:ident),*]) => {
        $(
            impl std::ops::Sub<$numeric_type> for $type {
                type Output = $type;

                fn sub(self, rhs: $numeric_type) -> Self::Output {
                    create_numeric_arithmetic_operators!(@sub_impl self, rhs, $constructor, $numeric_type)
                }
            }

            impl std::ops::SubAssign<$numeric_type> for $type {
                fn sub_assign(&mut self, rhs: $numeric_type) {
                    *self = *self - rhs;
                }
            }

            impl std::ops::Add<$numeric_type> for $type {
                type Output = $type;

                fn add(self, rhs: $numeric_type) -> Self::Output {
                    create_numeric_arithmetic_operators!(@add_impl self, rhs, $constructor, $numeric_type)
                }
            }

            impl std::ops::AddAssign<$numeric_type> for $type {
                fn add_assign(&mut self, rhs: $numeric_type) {
                    *self = *self + rhs;
                }
            }
        )*
    };

    // Sub implementation for usize
    (@sub_impl $self:expr, $rhs:expr, $constructor:ident, usize) => {
        $constructor($self.as_usize().saturating_sub($rhs))
    };

    // Sub implementation for u16
    (@sub_impl $self:expr, $rhs:expr, $constructor:ident, u16) => {
        $constructor($self.as_u16().saturating_sub($rhs))
    };

    // Sub implementation for i32 (treat negative as 0)
    (@sub_impl $self:expr, $rhs:expr, $constructor:ident, i32) => {
        {
            #[allow(clippy::cast_sign_loss)]
            {
                $constructor($self.as_usize().saturating_sub($rhs.max(0) as usize))
            }
        }
    };

    // Add implementation for usize
    (@add_impl $self:expr, $rhs:expr, $constructor:ident, usize) => {
        $constructor($self.as_usize() + $rhs)
    };

    // Add implementation for u16
    (@add_impl $self:expr, $rhs:expr, $constructor:ident, u16) => {
        $constructor($self.as_u16() + $rhs)
    };

    // Add implementation for i32 (treat negative as 0)
    (@add_impl $self:expr, $rhs:expr, $constructor:ident, i32) => {
        {
            #[allow(clippy::cast_sign_loss)]
            {
                $constructor($self.as_usize() + $rhs.max(0) as usize)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    use super::*;

    // Define a test type to validate the macro
    #[derive(Copy, Clone, PartialEq, Debug)]
    struct TestIndex(usize);

    impl TestIndex {
        fn as_usize(self) -> usize { self.0 }
        fn as_u16(self) -> u16 { self.0 as u16 }
    }

    fn test_index(value: impl Into<TestIndex>) -> TestIndex { value.into() }

    impl From<usize> for TestIndex {
        fn from(value: usize) -> Self { TestIndex(value) }
    }

    impl From<u16> for TestIndex {
        fn from(value: u16) -> Self { TestIndex(value as usize) }
    }

    // Use the macro to generate operations for our test type
    create_numeric_arithmetic_operators!(TestIndex, test_index, [usize, u16, i32]);

    #[test]
    fn test_macro_generated_add_operations() {
        let index = TestIndex(10);

        // Test Add<usize>
        let result = index + 5usize;
        assert_eq!(result, TestIndex(15));

        // Test Add<u16>
        let result = index + 3u16;
        assert_eq!(result, TestIndex(13));

        // Test Add<i32> with positive value
        let result = index + 7i32;
        assert_eq!(result, TestIndex(17));

        // Test Add<i32> with negative value (should become 0)
        let result = index + (-5i32);
        assert_eq!(result, TestIndex(10)); // -5 becomes 0
    }

    #[test]
    fn test_macro_generated_sub_operations() {
        let index = TestIndex(10);

        // Test Sub<usize> with normal subtraction
        let result = index - 3usize;
        assert_eq!(result, TestIndex(7));

        // Test Sub<usize> with saturating subtraction (underflow)
        let result = TestIndex(3) - 10usize;
        assert_eq!(result, TestIndex(0)); // saturating_sub prevents underflow

        // Test Sub<u16>
        let result = index - 2u16;
        assert_eq!(result, TestIndex(8));

        // Test Sub<i32> with positive value
        let result = index - 4i32;
        assert_eq!(result, TestIndex(6));

        // Test Sub<i32> with negative value (should become 0)
        let result = index - (-5i32);
        assert_eq!(result, TestIndex(10)); // -5 becomes 0, so no change
    }

    #[test]
    fn test_macro_generated_assign_operations() {
        // Test AddAssign
        let mut index = TestIndex(10);
        index += 5usize;
        assert_eq!(index, TestIndex(15));

        // Test SubAssign
        let mut index = TestIndex(10);
        index -= 3usize;
        assert_eq!(index, TestIndex(7));

        // Test SubAssign with underflow protection
        let mut index = TestIndex(2);
        index -= 5usize;
        assert_eq!(index, TestIndex(0)); // saturating_sub prevents underflow
    }
}
