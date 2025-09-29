// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Core traits for type-safe bounds checking operations.
//!
//! This module defines the foundational traits that enable the bounds checking system:
//! - [`UnitCompare`] - Numeric conversion operations for unit types
//! - [`IndexMarker`] - Identifies 0-based position types (like [`ColIndex`],
//!   [`RowIndex`])
//! - [`LengthMarker`] - Identifies 1-based size types (like [`ColWidth`], [`RowHeight`])
//!
//! These traits work together to provide type safety and prevent incorrect
//! comparisons between incompatible types (e.g., row vs column indices).
//!
//! See the [module documentation] for details on the type system and safety guarantees.
//!
//! [module documentation]: mod@crate::core::units::bounds_check
//! [`UnitCompare`]: crate::UnitCompare
//! [`IndexMarker`]: crate::IndexMarker
//! [`LengthMarker`]: crate::LengthMarker
//! [`ColIndex`]: crate::ColIndex
//! [`RowIndex`]: crate::RowIndex
//! [`ColWidth`]: crate::ColWidth
//! [`RowHeight`]: crate::RowHeight

use std::{cmp::min, ops::Sub};

use crate::{ArrayAccessBoundsStatus, Length, len};

/// Core trait for unit comparison operations.
///
/// Provides standardized methods to convert unit types to common numeric types
/// for comparison operations. This trait enables generic implementations of
/// bounds checking across different unit types.
pub trait UnitCompare: From<usize> + From<u16> {
    /// Convert the unit to a usize value for numeric comparison, usually for array
    /// indexing operations.
    fn as_usize(&self) -> usize;

    /// Convert the unit to a u16 value for crossterm compatibility and other terminal and
    /// pty based operations.
    fn as_u16(&self) -> u16;

    /// Check if the unit value is zero.
    fn is_zero(&self) -> bool { self.as_usize() == 0 }
}

/// Marker trait for index-type units (0-based position indicators).
///
/// This trait identifies types that represent positions or indices within
/// content, such as [`RowIndex`], [`ColIndex`], and [`Index`]. These are 0-based
/// values where the first position is index 0.
///
/// Each index type has a corresponding length type via [`LengthType`](Self::LengthType),
/// enabling safe bounds checking operations in both directions.
///
/// See the [module documentation] "Type System"
/// section for details on how index types relate to length types and the type safety
/// guarantees.
///
/// ## Method Parameter Types Explained
///
/// The clamp methods have carefully chosen parameter types to reflect real-world usage:
///
/// - **`clamp_to_max_length(max_length: LengthType)`**: Takes a **length** because upper
///   bounds are naturally expressed as container sizes ("How many items are there?")
/// - **`clamp_to_min_index(min_bound: Self)`**: Takes an **index** because lower bounds
///   are naturally expressed as minimum positions ("What's the lowest position allowed?")
///
/// This asymmetry makes the API more ergonomic by matching how these operations are
/// typically used in practice.
///
/// [module documentation]: mod@crate::core::units::bounds_check
/// [`RowIndex`]: crate::RowIndex
/// [`ColIndex`]: crate::ColIndex
/// [`Index`]: crate::Index
pub trait IndexMarker: UnitCompare {
    /// The corresponding length type for this index type.
    ///
    /// The constraint `LengthMarker<IndexType = Self>` creates a bidirectional
    /// relationship: this ensures that the length type's `IndexType` points back to
    /// this same index type, preventing type mismatches like [`ColIndex`] ↔
    /// [`RowHeight`].
    ///
    /// Note: For special cases like [`ByteIndex`] that need to work with [`Length`],
    /// we provide convenience methods that convert to compatible types rather than
    /// breaking the bidirectional constraint system.
    ///
    /// [`ColIndex`]: crate::ColIndex
    /// [`RowHeight`]: crate::RowHeight
    /// [`ByteIndex`]: crate::ByteIndex
    /// [`Length`]: crate::Length
    type LengthType: LengthMarker<IndexType = Self>;

    /// Convert this index to the corresponding length type.
    ///
    /// This typically involves adding 1 to the index value since
    /// indices are 0-based and lengths are 1-based.
    ///
    /// ```text
    /// Index=5 (0-based) to length (1-based) conversion:
    ///
    ///                           index=5 (0-based)
    ///                                 ↓
    /// Index:      0   1   2   3   4   5   6   7   8   9
    /// (0-based) ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///           │   │   │   │   │   │   │   │   │   │   │
    ///           └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
    /// Length:     1   2   3   4   5   6   7   8   9   10
    /// (1-based)                       ↑
    ///                  convert_to_length() = 6 (1-based)
    /// ```
    fn convert_to_length(&self) -> Self::LengthType;

    /// Answers the question: "Does this index overflow this length?"
    ///
    /// Check if this index overflows the given length's bounds.
    /// This is the inverse of [`LengthMarker::is_overflowed_by`] and provides
    /// a natural way to express bounds checking from the index's perspective.
    ///
    /// ```text
    /// Checking if index overflows length:
    ///
    ///                           index=5 (0-based)   index=10 (0-based)
    ///                                 ↓                   ↓
    /// Index:      0   1   2   3   4   5   6   7   8   9 │ 10  11  12
    /// (1-based) ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┼───┬───┬───┐
    ///           │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ × │ × │ × │
    ///           ├───┴───┴───┴───┴───┴───┴───┴───┴───┴───┼───┴───┴───┤
    ///           ├──────────── within bounds ────────────┼─ overflow ┘
    ///           └────────── length=10 (1-based) ────────┘
    ///
    /// overflows(length=10) = true (index 10 overflows length 10)
    /// overflows(length=5)  = false (index 5 within length 10)
    /// ```
    ///
    /// # Returns
    /// true if the index is greater than or equal to the length.
    ///
    /// # See Also
    /// For detailed status information with pattern matching capabilities, use
    /// [`check_array_access_bounds`] which returns a
    /// [`ArrayAccessBoundsStatus`] enum. This method is a convenience wrapper
    /// designed for simple boolean conditions.
    ///
    /// Both methods are semantically equivalent:
    /// - `index.overflows(length)` returns `bool`
    /// - `index.check_array_access_bounds(length) == ArrayAccessBoundsStatus::Overflowed`
    ///   returns `bool`
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{IndexMarker, BoundsCheck, ArrayAccessBoundsStatus, col, width};
    ///
    /// let index = col(10);
    /// let max_width = width(10);
    ///
    /// // Simple boolean check - use this method:
    /// if index.overflows(max_width) {
    ///     println!("Index out of bounds");
    /// }
    ///
    /// // For pattern matching - use check_array_access_bounds():
    /// match index.check_array_access_bounds(max_width) {
    ///     ArrayAccessBoundsStatus::Within => println!("Safe to access"),
    ///     ArrayAccessBoundsStatus::Overflowed => println!("Index out of bounds"),
    ///     ArrayAccessBoundsStatus::Underflowed => println!("Index underflowed"),
    /// }
    ///
    /// let smaller_index = col(5);
    /// assert!(!smaller_index.overflows(max_width));  // Within bounds
    /// ```
    ///
    /// [`check_array_access_bounds`]: crate::BoundsCheck::check_array_access_bounds
    /// [`ArrayAccessBoundsStatus`]: crate::ArrayAccessBoundsStatus
    /// [`LengthMarker::is_overflowed_by`]: crate::LengthMarker::is_overflowed_by
    fn overflows(&self, arg_length: impl Into<Self::LengthType>) -> bool
    where
        Self: PartialOrd + Sized + Copy,
    {
        let length: Self::LengthType = arg_length.into();
        // Single source of truth for bounds checking logic
        // Special case: empty collection (length 0) has no valid indices.
        if length.as_usize() == 0 {
            return true;
        }
        *self > length.convert_to_index()
    }

    /// Check if this index underflows (goes below) the given minimum bound.
    ///
    /// This is useful for checking if a position would go negative or below
    /// a starting position when moving backwards, such as in scrolling logic.
    ///
    /// ```text
    /// Checking if index underflows minimum:
    ///
    ///           min_bound=3
    ///                ↓
    /// Index:   0   1   2   3   4   5   6   7   8   9
    ///        ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///        │ × │ × │ × │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │
    ///        ├───┴───┴───┼───┴───┴───┴───┴───┴───┴───┤
    ///        └ underflow ┼──── valid range ──────────┘
    ///
    /// underflows(min=3) for index=2  = true  (below minimum)
    /// underflows(min=3) for index=3  = false (at minimum, valid)
    /// underflows(min=3) for index=5  = false (above minimum)
    /// ```
    ///
    /// # Returns
    /// Returns true if this index is less than the minimum bound.
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{IndexMarker, col, row};
    ///
    /// let min_col = col(3);
    /// assert!(col(0).underflows(min_col));  // 0 < 3
    /// assert!(col(2).underflows(min_col));  // 2 < 3
    /// assert!(!col(3).underflows(min_col)); // 3 == 3 (at boundary)
    /// assert!(!col(5).underflows(min_col)); // 5 > 3
    /// ```
    fn underflows(&self, min_bound: impl Into<Self>) -> bool
    where
        Self: PartialOrd + Sized,
    {
        let min: Self = min_bound.into();
        *self < min
    }

    /// Check if this index is within a range [start, start+size).
    /// The upper bound is EXCLUSIVE, making this suitable for viewport and window bounds.
    ///
    /// This provides comprehensive bounds checking that can detect underflow,
    /// valid positions, and overflow in a single operation with exclusive upper bound
    /// semantics.
    ///
    /// **Note on interval notation:**
    /// - `[` means the boundary is INCLUDED (closed)
    /// - `)` means the boundary is EXCLUDED (open)
    /// - Example: `[2, 8)` includes 2,3,4,5,6,7 but excludes 8
    ///
    /// ```text
    /// Example with start=2, size=6:
    /// Viewport covers [2, 8) - index 8 is NOT included
    ///
    ///       start=2                start+size=8 (exclusive)
    ///           ↓                          ↓
    /// Index:    0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///         │ U │ U │ W │ W │ W │ W │ W │ W │ O │ O │
    ///         └───┴───┼───┴───┴───┴───┴───┴───┼───┴───┘
    ///                 ╰───── within range ────╯
    ///
    /// U = Underflowed (index < start)
    /// W = Within (start <= index < start+size)
    /// O = Overflowed (index >= start+size)
    ///
    /// check_range_bounds_exclusive_end(col(1), col(2), width(6)) = Underflowed
    /// check_range_bounds_exclusive_end(col(2), col(2), width(6)) = Within
    /// check_range_bounds_exclusive_end(col(7), col(2), width(6)) = Within
    /// check_range_bounds_exclusive_end(col(8), col(2), width(6)) = Overflowed
    /// ```
    ///
    /// # Use Cases
    /// - **Viewport bounds checking**: `[viewport_start, viewport_start+viewport_size)`
    /// - **Buffer array access**: `[0, buffer_length)` for safe indexing
    /// - **Window visibility**: Checking if content is within a scrollable window
    /// - **Memory range validation**: Ensuring pointers stay within allocated bounds
    ///
    /// # When to Use This Method vs Semantic Aliases
    /// - **Use this method** when you need detailed status information
    ///   (underflow/overflow handling)
    /// - **Use [`is_in_viewport()`]** for simple boolean viewport containment checks
    /// - **For pattern matching**: When you need to handle underflow/overflow differently
    /// - **For complex logic**: When the specific type of bounds violation matters
    ///
    /// ```rust
    /// // Use this core method for detailed handling:
    /// match caret_col.check_range_bounds_exclusive_end(viewport_start, viewport_width) {
    ///     ArrayAccessBoundsStatus::Underflowed => scroll_right(),
    ///     ArrayAccessBoundsStatus::Within => render_cursor(),
    ///     ArrayAccessBoundsStatus::Overflowed => scroll_left(),
    /// }
    ///
    /// // Use semantic alias for simple checks:
    /// if caret_col.is_in_viewport(viewport_start, viewport_width) {
    ///     render_cursor();
    /// }
    /// ```
    ///
    /// # Returns
    /// - [`ArrayAccessBoundsStatus::Underflowed`] if index < start
    /// - [`ArrayAccessBoundsStatus::Within`] if start <= index < start+size
    /// - [`ArrayAccessBoundsStatus::Overflowed`] if index >= start+size
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{IndexMarker, ArrayAccessBoundsStatus, col, width};
    ///
    /// let viewport_start = col(2);
    /// let viewport_width = width(6);
    ///
    /// // Viewport covers [2, 8) - column 8 is NOT included
    /// assert_eq!(col(1).check_range_bounds_exclusive_end(viewport_start, viewport_width), ArrayAccessBoundsStatus::Underflowed);
    /// assert_eq!(col(5).check_range_bounds_exclusive_end(viewport_start, viewport_width), ArrayAccessBoundsStatus::Within);
    /// assert_eq!(col(8).check_range_bounds_exclusive_end(viewport_start, viewport_width), ArrayAccessBoundsStatus::Overflowed);
    /// ```
    ///
    /// [`ArrayAccessBoundsStatus::Underflowed`]: crate::ArrayAccessBoundsStatus::Underflowed
    /// [`ArrayAccessBoundsStatus::Within`]: crate::ArrayAccessBoundsStatus::Within
    /// [`ArrayAccessBoundsStatus::Overflowed`]: crate::ArrayAccessBoundsStatus::Overflowed
    /// [`is_in_viewport()`]: Self::is_in_viewport
    fn check_range_bounds_exclusive_end(
        &self,
        arg_start: impl Into<Self>,
        size: Self::LengthType,
    ) -> ArrayAccessBoundsStatus
    where
        Self: PartialOrd + Sized + Copy,
    {
        let start_bound: Self = arg_start.into();

        if *self < start_bound {
            ArrayAccessBoundsStatus::Underflowed
        } else {
            // Calculate the exclusive upper bound: start + size (using usize arithmetic)
            let start_as_usize = start_bound.as_usize();
            let size_as_usize = size.as_usize();
            let end_bound_usize = start_as_usize + size_as_usize;
            let self_as_usize = self.as_usize();

            if self_as_usize >= end_bound_usize {
                ArrayAccessBoundsStatus::Overflowed
            } else {
                ArrayAccessBoundsStatus::Within
            }
        }
    }

    /// Check if this index is within an inclusive range [min_index, max_index].
    /// Both the lower and upper bounds are INCLUSIVE, making this suitable for scroll
    /// regions and selections.
    ///
    /// This is useful for checking membership in regions defined by two indices,
    /// such as scroll regions, selection ranges, or viewport bounds. Both endpoints
    /// are included in the valid range.
    ///
    /// **Note on interval notation:**
    /// - `[` and `]` mean the boundaries are INCLUDED (closed)
    /// - Example: `[2, 7]` includes 2,3,4,5,6,7 (both 2 and 7 are included)
    ///
    /// ```text
    /// Example with min_index=2, max_index=7:
    ///
    ///            min_index=2           max_index=7
    ///                  ↓                   ↓
    /// Index:   0   1   2   3   4   5   6   7   8   9
    ///        ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///        │ U │ U │ W │ W │ W │ W │ W │ W │ O │ O │
    ///        └───┴───┼───┴───┴───┴───┴───┴───┼───┴───┘
    ///                ╰─── within range ───╯
    ///
    /// U = Underflowed (index < min_index)
    /// W = Within (min_index <= index <= max_index)
    /// O = Overflowed (index > max_index)
    ///
    /// check_range_bounds_inclusive_end(row(1), row(2), row(7)) = Underflowed
    /// check_range_bounds_inclusive_end(row(2), row(2), row(7)) = Within
    /// check_range_bounds_inclusive_end(row(5), row(2), row(7)) = Within
    /// check_range_bounds_inclusive_end(row(7), row(2), row(7)) = Within
    /// check_range_bounds_inclusive_end(row(8), row(2), row(7)) = Overflowed
    /// ```
    ///
    /// # Use Cases
    /// - **VT-100 scroll region checking**: `[scroll_top, scroll_bottom]` for terminal
    ///   operations
    /// - **Text selection bounds**: `[selection_start, selection_end]` for highlighting
    /// - **Inclusive range validation**: Any range where both endpoints are meaningful
    ///   positions
    /// - **Region membership testing**: Checking if a position falls within defined
    ///   boundaries
    ///
    /// # When to Use This Method vs Semantic Aliases
    /// - **Use this method** when you need detailed status information for pattern
    ///   matching
    /// - **Use [`is_in_inclusive_range()`]** for simple boolean range membership checks
    /// - **Use [`is_in_scroll_region()`]** for VT-100 terminal scroll region operations
    /// - **Use [`is_in_selection_range()`]** for text editor selection operations
    /// - **For pattern matching**: When you need to handle underflow/overflow differently
    /// - **For complex logic**: When the specific type of bounds violation matters
    ///
    /// ```rust
    /// // Use this core method for detailed handling:
    /// match row_index.check_range_bounds_inclusive_end(scroll_top, scroll_bottom) {
    ///     ArrayAccessBoundsStatus::Within => {
    ///         // Process operation within scroll region
    ///         perform_scroll_operation();
    ///     }
    ///     _ => {
    ///         // Skip operation - outside scroll region
    ///         return;
    ///     }
    /// }
    ///
    /// // Use semantic aliases for simple checks:
    /// if row_index.is_in_scroll_region(scroll_top, scroll_bottom) {
    ///     perform_scroll_operation();
    /// }
    /// ```
    ///
    /// # Arguments
    /// - `min_index`: Lower bound of the range (inclusive)
    /// - `max_index`: Upper bound of the range (inclusive)
    ///
    /// # Returns
    /// - [`ArrayAccessBoundsStatus::Underflowed`] if index < min_index
    /// - [`ArrayAccessBoundsStatus::Within`] if min_index <= index <= max_index
    /// - [`ArrayAccessBoundsStatus::Overflowed`] if index > max_index
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{IndexMarker, ArrayAccessBoundsStatus, row, col};
    ///
    /// // Scroll region bounds checking
    /// let scroll_top = row(2);
    /// let scroll_bottom = row(7);
    ///
    /// assert_eq!(row(1).check_range_bounds_inclusive_end(scroll_top, scroll_bottom), ArrayAccessBoundsStatus::Underflowed);
    /// assert_eq!(row(2).check_range_bounds_inclusive_end(scroll_top, scroll_bottom), ArrayAccessBoundsStatus::Within);
    /// assert_eq!(row(5).check_range_bounds_inclusive_end(scroll_top, scroll_bottom), ArrayAccessBoundsStatus::Within);
    /// assert_eq!(row(7).check_range_bounds_inclusive_end(scroll_top, scroll_bottom), ArrayAccessBoundsStatus::Within);
    /// assert_eq!(row(8).check_range_bounds_inclusive_end(scroll_top, scroll_bottom), ArrayAccessBoundsStatus::Overflowed);
    ///
    /// // Usage in conditional logic
    /// let row_index = row(5); // Example row index
    /// match row_index.check_range_bounds_inclusive_end(scroll_top, scroll_bottom) {
    ///     ArrayAccessBoundsStatus::Within => {
    ///         // Process operation within scroll region
    ///     }
    ///     _ => {
    ///         // Skip operation - outside scroll region
    ///         return;
    ///     }
    /// }
    /// ```
    ///
    /// [`ArrayAccessBoundsStatus::Underflowed`]: crate::ArrayAccessBoundsStatus::Underflowed
    /// [`ArrayAccessBoundsStatus::Within`]: crate::ArrayAccessBoundsStatus::Within
    /// [`ArrayAccessBoundsStatus::Overflowed`]: crate::ArrayAccessBoundsStatus::Overflowed
    /// [`is_in_inclusive_range()`]: Self::is_in_inclusive_range
    /// [`is_in_scroll_region()`]: Self::is_in_scroll_region
    /// [`is_in_selection_range()`]: Self::is_in_selection_range
    fn check_range_bounds_inclusive_end(
        &self,
        min_index: Self,
        max_index: Self,
    ) -> ArrayAccessBoundsStatus
    where
        Self: PartialOrd + Copy,
    {
        if *self < min_index {
            ArrayAccessBoundsStatus::Underflowed
        } else if *self > max_index {
            ArrayAccessBoundsStatus::Overflowed
        } else {
            ArrayAccessBoundsStatus::Within
        }
    }

    /// Clamp this index to stay within the bounds defined by a container length.
    ///
    /// **Why this method takes a `LengthType` parameter:**
    /// Upper bounds are naturally expressed as container sizes (how many items exist).
    /// This contrasts with `clamp_to_min_index()` which takes an index parameter,
    /// since lower bounds are naturally expressed as minimum positions.
    ///
    /// Returns the index unchanged if within bounds, or the maximum valid index
    /// if it would overflow. This method provides a convenient way to ensure
    /// indices stay within valid array/buffer boundaries.
    ///
    /// ```text
    /// Clamping operation with max_length=10:
    ///
    ///                           index=5 (within bounds)     index=15 (overflows)
    ///                                 ↓                                       ↓
    /// Index:      0   1   2   3   4   5   6   7   8   9 │ 10  11  12  13  14  15
    /// (0-based) ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┼───┬───┬───┬───┬───┬───┐
    ///           │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ × │ × │ × │ × │ × │ × │
    ///           ├───┴───┴───┴───┴───┴───┴───┴───┴───┴───┼───┴───┴───┴───┴───┴───┤
    ///           ├────────── valid indices ──────────────┼─── overflow ──────────┘
    ///           └────────── length=10 (1-based) ────────┘
    ///
    /// clamp_to_max_length(index=5, max_length=10)  = 5 (unchanged - within bounds)
    /// clamp_to_max_length(index=15, max_length=10) = 9 (clamped to max valid index)
    /// ```
    ///
    /// # Returns
    ///
    /// The index unchanged if it's within bounds, or the maximum valid index
    /// (length - 1) if the index would overflow the bounds.
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{IndexMarker, col, width};
    ///
    /// let max_width = width(10);
    ///
    /// // Index within bounds - returned unchanged
    /// assert_eq!(col(5).clamp_to_max_length(max_width), col(5));
    ///
    /// // Index at boundary - returned unchanged (9 is valid for width 10)
    /// assert_eq!(col(9).clamp_to_max_length(max_width), col(9));
    ///
    /// // Index overflows - clamped to maximum valid index
    /// assert_eq!(col(15).clamp_to_max_length(max_width), col(9));
    ///
    /// // Zero index - always valid
    /// assert_eq!(col(0).clamp_to_max_length(max_width), col(0));
    /// ```
    #[must_use]
    fn clamp_to_max_length(&self, max_length: Self::LengthType) -> Self
    where
        Self: Copy + Sized + PartialOrd,
        Self::LengthType: Copy,
    {
        if self.overflows(max_length) {
            max_length.convert_to_index()
        } else {
            *self
        }
    }

    /// Ensures this index is at least the given minimum bound.
    ///
    /// **Why this method takes an index (`Self`) parameter:**
    /// Lower bounds are naturally expressed as minimum positions (index-to-index).
    /// This contrasts with `clamp_to_max_length()` which takes a length parameter,
    /// since upper bounds are naturally expressed as container sizes.
    ///
    /// Returns the minimum bound if this index is less than it,
    /// otherwise returns self unchanged. This is useful for ensuring
    /// indices don't go below a starting position, such as in scrolling logic.
    ///
    /// ```text
    /// Clamping operation with min_bound=3:
    ///
    ///                 min_bound=3
    ///    current index=2   │   current index=7
    ///                  ↓   ↓               ↓
    /// Index:   0   1   2   3   4   5   6   7   8   9
    ///        ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///        │ × │ × │ × │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │
    ///        ├───┴───┴───┼───┴───┴───┴───┴───┴───┴───┤
    ///        └ underflow ┴──── valid range ──────────┘
    ///
    /// clamp_to_min_index(index=2, min_index=3) = 3 (clamped up to minimum)
    /// clamp_to_min_index(index=7, min_index=3) = 7 (unchanged - above minimum)
    /// ```
    ///
    /// # Returns
    ///
    /// The larger of this index or the minimum bound. This ensures the result
    /// is never below the minimum bound.
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{IndexMarker, col, row};
    ///
    /// let min_col = col(3);
    ///
    /// // Index below minimum - clamped up
    /// assert_eq!(col(1).clamp_to_min_index(min_col), col(3));
    ///
    /// // Index above minimum - unchanged
    /// assert_eq!(col(5).clamp_to_min_index(min_col), col(5));
    ///
    /// // Index at minimum - unchanged
    /// assert_eq!(col(3).clamp_to_min_index(min_col), col(3));
    ///
    /// // Zero index below minimum - clamped up
    /// assert_eq!(col(0).clamp_to_min_index(min_col), col(3));
    /// ```
    #[must_use]
    fn clamp_to_min_index(&self, min_bound: impl Into<Self>) -> Self
    where
        Self: Ord + Copy,
    {
        let min: Self = min_bound.into();
        (*self).max(min)
    }

    // ======================================================================================
    // Semantic Aliases - Boolean convenience methods for common use cases
    // ======================================================================================
    //
    // These methods provide self-documenting names for specific bounds checking
    // scenarios. They wrap the core methods above to provide clear, domain-specific
    // boolean checks.
    //
    // USAGE GUIDANCE:
    // - Use semantic aliases for simple boolean conditions in business logic
    // - Use core methods when you need pattern matching or detailed status information
    // - Choose the alias that best describes your specific use case

    /// Check if this index is visible within a viewport window.
    ///
    /// This is a semantic alias for `check_range_bounds_exclusive_end()` that returns
    /// a boolean result. Use this when you need a simple true/false answer for viewport
    /// containment checking.
    ///
    /// A viewport defines a rectangular window showing a portion of larger content,
    /// with exclusive upper bounds: `[start, start+size)`.
    ///
    /// **Note on interval notation:**
    /// - `[` means the boundary is INCLUDED (closed)
    /// - `)` means the boundary is EXCLUDED (open)
    /// - Example: `[10, 30)` includes 10,11,12,...,29 but excludes 30
    ///
    /// ```text
    /// Viewport Window Example:
    /// Full content is 50 columns wide, viewport shows columns [10, 30)
    ///
    ///          viewport_start=10       viewport_end=30 (exclusive)
    ///                   ↓                       ↓
    /// Column:   8   9   10  11  12 ...  28  29  30  31  32
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///         │   │   │ ▓ │ ▓ │ ▓ │...│ ▓ │ ▓ │ X │   │   │
    ///         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
    ///                 ╰────── viewport area ──────╯
    ///
    /// is_in_viewport(col(9),  start=col(10), size=width(20)) → false
    /// is_in_viewport(col(10), start=col(10), size=width(20)) → true
    /// is_in_viewport(col(29), start=col(10), size=width(20)) → true
    /// is_in_viewport(col(30), start=col(10), size=width(20)) → false
    /// ```
    ///
    /// # When to Use This Method
    /// - **Simple boolean checks**: When you only need true/false for viewport visibility
    /// - **Conditional rendering**: Deciding whether to draw/process elements
    /// - **Scroll calculations**: Checking if content is currently visible
    ///
    /// # When to Use Core Methods Instead
    /// - **Pattern matching**: When you need to handle underflow/overflow differently
    /// - **Detailed status**: When the specific type of bounds violation matters
    /// - **Complex logic**: When you need more than just within/not-within information
    ///
    /// ```rust
    /// // Use this semantic alias for simple checks:
    /// if caret_col.is_in_viewport(viewport_start, viewport_width) {
    ///     render_cursor();
    /// }
    ///
    /// // Use core method for detailed handling:
    /// match caret_col.check_range_bounds_exclusive_end(viewport_start, viewport_width) {
    ///     ArrayAccessBoundsStatus::Underflowed => scroll_right(),
    ///     ArrayAccessBoundsStatus::Within => render_cursor(),
    ///     ArrayAccessBoundsStatus::Overflowed => scroll_left(),
    /// }
    /// ```
    fn is_in_viewport(
        &self,
        viewport_start: impl Into<Self>,
        viewport_size: Self::LengthType,
    ) -> bool
    where
        Self: PartialOrd + Sized + Copy,
    {
        matches!(
            self.check_range_bounds_exclusive_end(viewport_start, viewport_size),
            ArrayAccessBoundsStatus::Within
        )
    }

    /// Check if this index is within an inclusive range.
    ///
    /// This is a semantic alias for `check_range_bounds_inclusive_end()` that returns
    /// a boolean result. Use this for general-purpose inclusive range membership testing
    /// where both endpoints are meaningful positions.
    ///
    /// **Note on interval notation:**
    /// - `[` and `]` mean the boundaries are INCLUDED (closed)
    /// - Example: `[3, 7]` includes 3,4,5,6,7 (both 3 and 7 are included)
    ///
    /// ```text
    /// Generic Inclusive Range [min=3, max=7]:
    ///
    ///       min_index=3              max_index=7
    ///           ↓                          ↓
    /// Index:    0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///         │   │   │   │ ● │ ● │ ● │ ● │ ● │   │   │
    ///         └───┴───┴───┼───┴───┴───┴───┴───┼───┴───┘
    ///                     ╰─── within range ───╯
    ///                     (both ends included)
    ///
    /// is_in_inclusive_range(2, min=3, max=7) → false
    /// is_in_inclusive_range(3, min=3, max=7) → true
    /// is_in_inclusive_range(5, min=3, max=7) → true
    /// is_in_inclusive_range(7, min=3, max=7) → true
    /// is_in_inclusive_range(8, min=3, max=7) → false
    /// ```
    ///
    /// # When to Use This Method
    /// - **Simple membership tests**: When you only need true/false for range containment
    /// - **General range checking**: For any inclusive range where endpoints matter
    /// - **Algorithm logic**: Simple bounds checking in loops or calculations
    ///
    /// # When to Use More Specific Aliases
    /// - **Scroll regions**: Use `is_in_scroll_region()` for VT-100 terminal operations
    /// - **Text selections**: Use `is_in_selection_range()` for text editing operations
    ///
    /// ```rust
    /// // Use this for general inclusive range checks:
    /// if row_index.is_in_inclusive_range(region_start, region_end) {
    ///     process_within_region();
    /// }
    ///
    /// // Use more specific aliases for domain-specific operations:
    /// if row_index.is_in_scroll_region(scroll_top, scroll_bottom) {
    ///     apply_scroll_operation();
    /// }
    /// ```
    fn is_in_inclusive_range(&self, start_index: Self, end_index: Self) -> bool
    where
        Self: PartialOrd + Copy,
    {
        matches!(
            self.check_range_bounds_inclusive_end(start_index, end_index),
            ArrayAccessBoundsStatus::Within
        )
    }

    /// Check if this index is within a VT-100 terminal scroll region.
    ///
    /// This is a semantic alias for `check_range_bounds_inclusive_end()` specifically
    /// for VT-100 terminal scroll region operations. VT-100 scroll regions use
    /// inclusive bounds where both the top and bottom lines are part of the scrollable
    /// area.
    ///
    /// **Note on interval notation:**
    /// - `[` and `]` mean the boundaries are INCLUDED (closed)
    /// - Example: `[2, 5]` includes rows 2,3,4,5 (both 2 and 5 are included)
    ///
    /// ```text
    /// Terminal Buffer with Scroll Region:
    ///               Row: 0-based
    /// max_height=8 ╮  ▼  ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Fixed Header (outside scroll)       │
    ///              │  1  │ Status Bar (outside scroll)         │
    ///              │     ├─────────────────────────────────────┤ ← scroll_top=2
    ///              │  2  │ ▓▓▓ Scrollable Line 1 ▓▓▓          │ ╮
    ///              │  3  │ ▓▓▓ Scrollable Line 2 ▓▓▓          │ │ Scroll
    ///              │  4  │ ▓▓▓ Scrollable Line 3 ▓▓▓          │ │ Region
    ///              │  5  │ ▓▓▓ Scrollable Line 4 ▓▓▓          │ ╯ [2,5]
    ///              │     ├─────────────────────────────────────┤ ← scroll_bottom=5
    ///              │  6  │ Fixed Footer (outside scroll)       │
    ///              ╰  7  │ Command Line (outside scroll)       │
    ///                    └─────────────────────────────────────┘
    ///
    /// is_in_scroll_region(row(1), top=row(2), bottom=row(5)) → false
    /// is_in_scroll_region(row(2), top=row(2), bottom=row(5)) → true
    /// is_in_scroll_region(row(4), top=row(2), bottom=row(5)) → true
    /// is_in_scroll_region(row(5), top=row(2), bottom=row(5)) → true
    /// is_in_scroll_region(row(6), top=row(2), bottom=row(5)) → false
    /// ```
    ///
    /// # When to Use This Method
    /// - **VT-100 operations**: Line insertion, deletion, and scrolling operations
    /// - **Terminal emulation**: Implementing CSI sequences like IL (Insert Line), DL
    ///   (Delete Line)
    /// - **Scroll region logic**: Determining if operations should affect a line
    ///
    /// # VT-100 Context
    /// In VT-100 terminals, scroll regions define which lines participate in scrolling
    /// operations:
    /// - **Insert Line (IL)**: Only affects lines within the scroll region
    /// - **Delete Line (DL)**: Only affects lines within the scroll region
    /// - **Scroll Up (SU)**: Only scrolls content within the region
    /// - **Scroll Down (SD)**: Only scrolls content within the region
    ///
    /// ```rust
    /// // Use this in VT-100 terminal operations:
    /// if cursor_row.is_in_scroll_region(scroll_top, scroll_bottom) {
    ///     perform_line_operation();
    /// } else {
    ///     skip_operation(); // Outside scroll region
    /// }
    ///
    /// // Example from actual VT-100 parser:
    /// match row_index.check_range_bounds_inclusive_end(scroll_top, scroll_bottom) {
    ///     ArrayAccessBoundsStatus::Within => {
    ///         // Continue with line insertion/deletion
    ///     }
    ///     _ => {
    ///         // Skip - cursor outside scroll region
    ///         return;
    ///     }
    /// }
    /// ```
    fn is_in_scroll_region(&self, scroll_top: Self, scroll_bottom: Self) -> bool
    where
        Self: PartialOrd + Copy,
    {
        matches!(
            self.check_range_bounds_inclusive_end(scroll_top, scroll_bottom),
            ArrayAccessBoundsStatus::Within
        )
    }

    /// Check if this index is within a text selection range.
    ///
    /// This is a semantic alias for `check_range_bounds_inclusive_end()` specifically
    /// for text selection operations. Text selections use inclusive bounds where both
    /// the start and end positions are part of the selected content.
    ///
    /// **Note on interval notation:**
    /// - `[` and `]` mean the boundaries are INCLUDED (closed)
    /// - Example: `[4, 14]` includes indices 4,5,6,7,8,9,10,11,12,13,14 (both 4 and 14
    ///   are included)
    ///
    /// ```text
    /// Text Selection Example:
    /// Original text: "The quick brown fox jumps"
    /// Selected text: "quick brown" (indices 4-14 inclusive)
    ///
    ///       selection_start=4                      selection_end=14
    ///               ↓                                      ↓
    /// Index:    0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    /// Char:   │ T │ h │ e │   │ q │ u │ i │ c │ k │   │ b │ r │ o │ w │ n │   │ f │ o │ x │
    ///         └───┴───┴───┴───┼───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┼───┴───┴───┴───┘
    ///                         ╰───────────── selected range ──────────────╯
    ///
    /// is_in_selection_range(idx(3),  start=idx(4), end=idx(14)) → false
    /// is_in_selection_range(idx(4),  start=idx(4), end=idx(14)) → true
    /// is_in_selection_range(idx(14), start=idx(4), end=idx(14)) → true
    /// is_in_selection_range(idx(15), start=idx(4), end=idx(14)) → false
    /// ```
    ///
    /// # When to Use This Method
    /// - **Text editing**: Determining if a position is within selected text
    /// - **Highlighting logic**: Deciding whether to apply selection styling
    /// - **Copy/cut operations**: Identifying which content to include
    /// - **Selection manipulation**: Expanding, contracting, or moving selections
    ///
    /// # Text Selection Context
    /// In text editors, selections define ranges of content for operations:
    /// - **Visual feedback**: Highlighting selected text with different colors
    /// - **Clipboard operations**: Copy/cut only affects selected content
    /// - **Bulk editing**: Apply formatting or transformations to selection
    /// - **Navigation**: Jump between selection boundaries
    ///
    /// ```rust
    /// // Use this in text editor operations:
    /// if char_index.is_in_selection_range(selection_start, selection_end) {
    ///     apply_selection_highlight();
    /// }
    ///
    /// // Example selection operation:
    /// for (index, character) in text.chars().enumerate() {
    ///     let char_index = CharIndex::from(index);
    ///     if char_index.is_in_selection_range(sel_start, sel_end) {
    ///         selected_text.push(character);
    ///     }
    /// }
    /// ```
    fn is_in_selection_range(&self, selection_start: Self, selection_end: Self) -> bool
    where
        Self: PartialOrd + Copy,
    {
        matches!(
            self.check_range_bounds_inclusive_end(selection_start, selection_end),
            ArrayAccessBoundsStatus::Within
        )
    }
}

/// Marker trait for length-type units (1-based size measurements).
///
/// This trait identifies types that represent sizes or lengths of content,
/// such as [`RowHeight`], [`ColWidth`], and [`Length`]. These are 1-based values
/// where a length of 1 means "one unit of size".
///
/// Each length type has a corresponding index type via [`IndexType`](Self::IndexType),
/// enabling safe bounds checking operations.
///
/// See the [module documentation](crate::core::units::bounds_check) "Type System"
/// section for details on how length types relate to index types and the type safety
/// guarantees.
///
/// [`RowHeight`]: crate::RowHeight
/// [`ColWidth`]: crate::ColWidth
/// [`Length`]: crate::Length
pub trait LengthMarker: UnitCompare {
    /// The corresponding index type for this length type.
    ///
    /// The constraint `IndexMarker<LengthType = Self>` creates a bidirectional
    /// relationship: this ensures that the index type's `LengthType` points back to
    /// this same length type, preventing type mismatches like [`ColWidth`] ↔
    /// [`RowIndex`].
    ///
    /// [`ColWidth`]: crate::ColWidth
    /// [`RowIndex`]: crate::RowIndex
    type IndexType: IndexMarker<LengthType = Self>;

    /// Convert this length to the corresponding index type.
    ///
    /// This typically involves subtracting 1 from the length value since
    /// lengths are 1-based and indices are 0-based.
    ///
    /// ```text
    /// Length=10 to index conversion:
    ///           ┌────────── length=10 (1-based) ────────┐
    /// Length:     1   2   3   4   5   6   7   8   9   10
    /// (1-based) ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///           │   │   │   │   │   │   │   │   │ x │   │
    ///           └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
    /// Index:      0   1   2   3   4   5   6   7   8   9
    /// (0-based)                                       ↑
    ///                                         convert_to_index() = 9
    /// ```
    fn convert_to_index(&self) -> Self::IndexType {
        let value = self.as_usize().saturating_sub(1);
        Self::IndexType::from(value)
    }

    /// Answers the question: "Does this length get overflowed by this index?"
    ///
    /// Check if the given index would overflow this length's bounds.
    ///
    /// Example - Checking overflow for length=10
    ///
    /// ```text
    ///                                             boundary
    ///                                                 │
    /// Index:    0   1   2   3   4   5   6   7   8   9 │ 10  11  12
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┼───┬───┬───┐
    ///         │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✗ │ ✗ │ ✗ │
    ///         ├───┴───┴───┴───┴───┴───┴───┴───┴───┴───┼───┴───┴───┤
    ///         ├──────────── valid indices ────────────┼─ overflow ┘
    ///         └────────── length=10 (1-based) ────────┘
    ///
    /// is_overflowed_by(5)  = false (within bounds)
    /// is_overflowed_by(9)  = false (last valid index)
    /// is_overflowed_by(10) = true (at boundary)
    /// is_overflowed_by(11) = true (beyond boundary)
    /// ```
    ///
    /// # Returns
    ///
    /// Returns true if the index is greater than or equal to the length.
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{LengthMarker, col, row, width};
    ///
    /// let max_col = width(10);
    /// assert!(!max_col.is_overflowed_by(col(5)));  // Within bounds
    /// assert!(max_col.is_overflowed_by(col(10)));  // At boundary - overflows
    /// assert!(max_col.is_overflowed_by(col(15)));  // Beyond boundary
    ///
    /// // Pos (row + col) automatically converts to ColIndex
    /// assert!(!max_col.is_overflowed_by(row(0) + col(5)));  // Pos converts to ColIndex
    /// assert!(max_col.is_overflowed_by(row(2) + col(10)));  // Pos at boundary - overflows
    /// ```
    fn is_overflowed_by(&self, arg_index: impl Into<Self::IndexType>) -> bool
    where
        Self::IndexType: PartialOrd + Copy,
        Self: Copy,
    {
        let index: Self::IndexType = arg_index.into();
        // Delegate to overflows() for single source of truth
        index.overflows(*self)
    }

    /// Calculate the remaining space from the given index to the end of this length.
    ///
    /// ```text
    /// With max_width=10:
    ///
    ///                 index=3 (0-based)
    ///                       ↓
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///         │   │   │   │ × │ × │ × │ × │ × │ × │ × │
    ///         ├───┴───┴───┼───┴───┴───┴───┴───┴───┴───┤
    ///         │           └───── 7 chars remain ──────┤
    ///         └────────── width=10 (1-based) ─────────┘
    ///
    /// remaining_from(3)  = 7 (chars from index 3 to 9)
    /// remaining_from(9)  = 1 (only position 9 remains)
    /// remaining_from(10) = 0 (at boundary, nothing remains)
    /// ```
    ///
    /// # Returns
    /// The number of units between the index and the boundary defined by this
    /// length. For example, if this is a [`ColWidth`] of 10 and the index is at column 3,
    /// this returns a [`Length`] of 7 (columns 3-9, inclusive).
    ///
    /// Returns [`Length`](0) if the index is at or beyond the boundary.
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{LengthMarker, col, row, width, len};
    ///
    /// let max_width = width(10);
    /// assert_eq!(max_width.remaining_from(col(3)), len(7));  // 7 columns remain
    /// assert_eq!(max_width.remaining_from(col(10)), len(0)); // At boundary
    /// assert_eq!(max_width.remaining_from(col(15)), len(0)); // Beyond boundary
    ///
    /// // Pos (row + col) automatically converts to ColIndex
    /// assert_eq!(max_width.remaining_from(row(0) + col(3)), len(7));  // Pos converts to ColIndex
    /// assert_eq!(max_width.remaining_from(row(1) + col(10)), len(0)); // Pos at boundary
    /// ```
    ///
    /// [`ColWidth`]: crate::ColWidth
    /// [`Length`]: crate::Length
    fn remaining_from(&self, arg_index: impl Into<Self::IndexType>) -> Length
    where
        Self::IndexType: PartialOrd + Sub<Output = Self::IndexType> + Copy,
        <Self::IndexType as IndexMarker>::LengthType: Into<Length>,
        Self: Copy,
    {
        let index: Self::IndexType = arg_index.into();
        if self.is_overflowed_by(index) {
            len(0)
        } else {
            // Get max index for this length.
            let max_index = self.convert_to_index();
            // Calculate num of chars from cursor to boundary (as index difference).
            let chars_remaining_as_index = max_index - index;
            // Convert from 0-based index difference to 1-based length.
            chars_remaining_as_index.convert_to_length().into()
        }
    }

    /// Clamps this length to a maximum value.
    ///
    /// ```text
    /// Clamping operation with max_length=7:
    ///
    /// Case 1: length=5 (within bounds)
    /// ┌───── length=5 ────┐
    /// │ 1   2   3   4   5 │ 6   7 ← max_length boundary
    /// ├───┬───┬───┬───┬───┼───┬───┤
    /// │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │   │   │
    /// └───┴───┴───┴───┴───┴───┴───┘
    ///
    /// Result: clamp_to_max_length(5, max=7) = 5 (no change - within bounds)
    ///
    /// Case 2: length=10 (exceeds bounds)
    /// ┌───────────── length=10 ───────────────┐
    /// │ 1   2   3   4   5   6   7 │ 8   9   10 (trimmed)
    /// ├───┬───┬───┬───┬───┬───┬───┼───┬───┬───┤
    /// │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ × │ × │ × │
    /// └───┴───┴───┴───┴───┴───┴───┼───┴───┴───┘
    ///                             └─ max_length=7 boundary
    ///
    /// Result: clamp_to_max_length(10, max=7) = 7 (clamped to maximum)
    /// ```
    ///
    /// # Returns
    ///
    /// The smaller of this length or the maximum length provided.
    /// This is commonly used when constraining operations to available space
    /// or buffer boundaries.
    ///
    /// # Examples
    ///
    /// ```
    /// use r3bl_tui::{LengthMarker, len};
    ///
    /// // Length within bounds - no change
    /// let small_length = len(5);
    /// let max_allowed = len(10);
    /// assert_eq!(small_length.clamp_to_max(max_allowed), len(5));
    ///
    /// // Length exceeds bounds - gets clamped
    /// let large_length = len(15);
    /// let max_allowed = len(10);
    /// assert_eq!(large_length.clamp_to_max(max_allowed), len(10));
    ///
    /// // Equal lengths - returns the same value
    /// let equal_length = len(8);
    /// let max_allowed = len(8);
    /// assert_eq!(equal_length.clamp_to_max(max_allowed), len(8));
    /// ```
    #[must_use]
    fn clamp_to_max(&self, arg_max_length: impl Into<Self>) -> Self
    where
        Self: Copy + Ord,
    {
        let max_length: Self = arg_max_length.into();
        min(*self, max_length)
    }
}

#[cfg(test)]
mod overflow_operations_tests {
    use super::*;
    use crate::{BoundsCheck, ColIndex, ColWidth, RowHeight, RowIndex, idx};

    #[test]
    fn test_is_overflowed_by() {
        // Test basic cases with Index/Length.
        assert!(!len(3).is_overflowed_by(idx(1)), "Within bounds");
        assert!(len(3).is_overflowed_by(idx(3)), "At boundary");
        assert!(len(3).is_overflowed_by(idx(5)), "Beyond bounds");
        assert!(
            len(0).is_overflowed_by(idx(0)),
            "Empty collection edge case"
        );

        // Test with typed dimensions.
        assert!(
            !ColWidth::new(10).is_overflowed_by(ColIndex::new(5)),
            "Typed indices within bounds"
        );
        assert!(
            ColWidth::new(10).is_overflowed_by(ColIndex::new(10)),
            "Typed indices at boundary"
        );
        assert!(
            !RowHeight::new(5).is_overflowed_by(RowIndex::new(3)),
            "Row indices within bounds"
        );
        assert!(
            RowHeight::new(5).is_overflowed_by(RowIndex::new(5)),
            "Row indices at boundary"
        );

        // Verify method matches existing check_overflows behavior.
        let test_cases = [(0, 1), (1, 1), (5, 10), (10, 10)];
        for (index_val, length_val) in test_cases {
            let index = idx(index_val);
            let length = len(length_val);
            assert_eq!(
                length.is_overflowed_by(index),
                index.check_array_access_bounds(length)
                    == ArrayAccessBoundsStatus::Overflowed,
                "New method should match existing behavior for index {index_val} and length {length_val}"
            );
        }
    }

    #[test]
    fn test_underflows() {
        use crate::{col, row};

        // Test column underflow
        let min_col = col(3);
        assert!(col(0).underflows(min_col)); // 0 < 3
        assert!(col(2).underflows(min_col)); // 2 < 3
        assert!(!col(3).underflows(min_col)); // 3 == 3 (at boundary)
        assert!(!col(5).underflows(min_col)); // 5 > 3

        // Test row underflow
        let min_row = row(5);
        assert!(row(4).underflows(min_row)); // 4 < 5
        assert!(!row(5).underflows(min_row)); // 5 == 5
        assert!(!row(10).underflows(min_row)); // 10 > 5

        // Test with Index/Length
        let min_index = idx(7);
        assert!(idx(3).underflows(min_index)); // 3 < 7
        assert!(idx(6).underflows(min_index)); // 6 < 7
        assert!(!idx(7).underflows(min_index)); // 7 == 7
        assert!(!idx(10).underflows(min_index)); // 10 > 7
    }

    #[test]
    fn test_check_range_bounds_exclusive_end() {
        use crate::{ArrayAccessBoundsStatus, col, width};

        let min_col = col(2);
        let max_width = width(6);

        // Test underflow (before start)
        assert_eq!(
            col(0).check_range_bounds_exclusive_end(min_col, max_width),
            ArrayAccessBoundsStatus::Underflowed
        );
        assert_eq!(
            col(1).check_range_bounds_exclusive_end(min_col, max_width),
            ArrayAccessBoundsStatus::Underflowed
        );

        // Test within bounds [2, 8) - range is [start, start+size)
        assert_eq!(
            col(2).check_range_bounds_exclusive_end(min_col, max_width),
            ArrayAccessBoundsStatus::Within
        );
        assert_eq!(
            col(5).check_range_bounds_exclusive_end(min_col, max_width),
            ArrayAccessBoundsStatus::Within
        );
        assert_eq!(
            col(7).check_range_bounds_exclusive_end(min_col, max_width),
            ArrayAccessBoundsStatus::Within
        );

        // Test overflow (at or beyond exclusive end)
        assert_eq!(
            col(8).check_range_bounds_exclusive_end(min_col, max_width),
            ArrayAccessBoundsStatus::Overflowed
        );
        assert_eq!(
            col(10).check_range_bounds_exclusive_end(min_col, max_width),
            ArrayAccessBoundsStatus::Overflowed
        );

        // Test edge cases with zero minimum
        let min_zero = col(0);
        assert_eq!(
            col(0).check_range_bounds_exclusive_end(min_zero, max_width),
            ArrayAccessBoundsStatus::Within
        );
        assert_eq!(
            col(5).check_range_bounds_exclusive_end(min_zero, max_width),
            ArrayAccessBoundsStatus::Within
        );
        assert_eq!(
            col(6).check_range_bounds_exclusive_end(min_zero, max_width),
            ArrayAccessBoundsStatus::Overflowed
        );
    }

    #[test]
    fn test_overflows() {
        // Test basic cases with Index/Length - should mirror is_overflowed_by results
        assert!(!idx(1).overflows(len(3)), "Within bounds");
        assert!(idx(3).overflows(len(3)), "At boundary");
        assert!(idx(5).overflows(len(3)), "Beyond bounds");
        assert!(idx(0).overflows(len(0)), "Empty collection edge case");

        // Test with typed dimensions.
        assert!(
            !ColIndex::new(5).overflows(ColWidth::new(10)),
            "Typed indices within bounds"
        );
        assert!(
            ColIndex::new(10).overflows(ColWidth::new(10)),
            "Typed indices at boundary"
        );
        assert!(
            !RowIndex::new(3).overflows(RowHeight::new(5)),
            "Row indices within bounds"
        );
        assert!(
            RowIndex::new(5).overflows(RowHeight::new(5)),
            "Row indices at boundary"
        );

        // Verify method matches is_overflowed_by behavior (inverse relationship)
        let test_cases = [(0, 1), (1, 1), (5, 10), (10, 10)];
        for (index_val, length_val) in test_cases {
            let index = idx(index_val);
            let length = len(length_val);
            assert_eq!(
                index.overflows(length),
                length.is_overflowed_by(index),
                "overflows() should match is_overflowed_by() for index {index_val} and length {length_val}"
            );
        }

        // Test with specific typed combinations.
        let col_cases = [(0, 5), (4, 5), (5, 5), (6, 5)];
        for (index_val, width_val) in col_cases {
            let col_index = ColIndex::new(index_val);
            let col_width = ColWidth::new(width_val);
            assert_eq!(
                col_index.overflows(col_width),
                col_width.is_overflowed_by(col_index),
                "ColIndex::overflows should match ColWidth::is_overflowed_by for index {index_val} and width {width_val}"
            );
        }

        let row_cases = [(0, 3), (2, 3), (3, 3), (4, 3)];
        for (index_val, height_val) in row_cases {
            let row_index = RowIndex::new(index_val);
            let row_height = RowHeight::new(height_val);
            assert_eq!(
                row_index.overflows(row_height),
                row_height.is_overflowed_by(row_index),
                "RowIndex::overflows should match RowHeight::is_overflowed_by for index {index_val} and height {height_val}"
            );
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_index_clamp_to_max_length() {
        use crate::{col, height, row, width};

        // Test basic Index/Length clamping scenarios
        assert_eq!(
            idx(5).clamp_to_max_length(len(10)),
            idx(5),
            "Index within bounds - returned unchanged"
        );
        assert_eq!(
            idx(9).clamp_to_max_length(len(10)),
            idx(9),
            "Index at max valid position - returned unchanged"
        );
        assert_eq!(
            idx(15).clamp_to_max_length(len(10)),
            idx(9),
            "Index overflows - clamped to max valid index (length-1)"
        );
        assert_eq!(
            idx(0).clamp_to_max_length(len(10)),
            idx(0),
            "Zero index - always valid"
        );

        // Test with ColIndex/ColWidth typed dimensions
        assert_eq!(
            col(5).clamp_to_max_length(width(10)),
            col(5),
            "ColIndex within bounds - returned unchanged"
        );
        assert_eq!(
            col(9).clamp_to_max_length(width(10)),
            col(9),
            "ColIndex at boundary - returned unchanged (9 is valid for width 10)"
        );
        assert_eq!(
            col(10).clamp_to_max_length(width(10)),
            col(9),
            "ColIndex at boundary - clamped to max valid index"
        );
        assert_eq!(
            col(15).clamp_to_max_length(width(10)),
            col(9),
            "ColIndex overflows - clamped to max valid index"
        );
        assert_eq!(
            col(0).clamp_to_max_length(width(10)),
            col(0),
            "ColIndex zero - always valid"
        );

        // Test with RowIndex/RowHeight typed dimensions
        assert_eq!(
            row(3).clamp_to_max_length(height(5)),
            row(3),
            "RowIndex within bounds - returned unchanged"
        );
        assert_eq!(
            row(4).clamp_to_max_length(height(5)),
            row(4),
            "RowIndex at max valid position - returned unchanged"
        );
        assert_eq!(
            row(5).clamp_to_max_length(height(5)),
            row(4),
            "RowIndex at boundary - clamped to max valid index"
        );
        assert_eq!(
            row(8).clamp_to_max_length(height(5)),
            row(4),
            "RowIndex overflows - clamped to max valid index"
        );

        // Test edge case: empty collection (length 0)
        assert_eq!(
            idx(0).clamp_to_max_length(len(0)),
            idx(0),
            "Empty collection edge case - index 0 should remain 0"
        );
        assert_eq!(
            idx(5).clamp_to_max_length(len(0)),
            idx(0),
            "Empty collection with overflow - should clamp to 0"
        );

        // Test single element case (length 1)
        assert_eq!(
            idx(0).clamp_to_max_length(len(1)),
            idx(0),
            "Single element case - index 0 valid"
        );
        assert_eq!(
            idx(1).clamp_to_max_length(len(1)),
            idx(0),
            "Single element case - index 1 clamped to 0"
        );

        // Property test: result should always be <= max valid index
        let test_cases = [(5, 10), (10, 10), (15, 10), (0, 5), (100, 20)];
        for (index_val, length_val) in test_cases {
            let index = idx(index_val);
            let length = len(length_val);
            let result = index.clamp_to_max_length(length);
            let max_valid_index = if length_val > 0 { length_val - 1 } else { 0 };

            assert!(
                result.as_usize() <= max_valid_index,
                "clamp_to_max_length({index_val}, {length_val}) = {} should be <= {max_valid_index}",
                result.as_usize()
            );

            // If within bounds, should return original value
            if index_val < length_val {
                assert_eq!(
                    result, index,
                    "clamp_to_max_length({index_val}, {length_val}) should preserve value when within bounds"
                );
            }
        }

        // Test with large values
        assert_eq!(
            col(999).clamp_to_max_length(width(10)),
            col(9),
            "Large index value clamped to small max"
        );

        // Test consistency: clamp_to_max should match overflows() behavior
        let consistency_cases = [(5, 10), (9, 10), (10, 10), (15, 10)];
        for (index_val, length_val) in consistency_cases {
            let index = idx(index_val);
            let length = len(length_val);
            let clamped = index.clamp_to_max_length(length);
            let overflows = index.overflows(length);

            if overflows {
                // If it overflows, clamp_to_max should return max valid index
                assert_eq!(
                    clamped,
                    length.convert_to_index(),
                    "When index overflows, clamp_to_max should return max valid index"
                );
            } else {
                // If it doesn't overflow, clamp_to_max should return original
                assert_eq!(
                    clamped, index,
                    "When index doesn't overflow, clamp_to_max should preserve value"
                );
            }
        }
    }

    #[test]
    fn test_index_clamp_to_min_index() {
        use crate::{col, row};

        // Test basic Index clamping to minimum
        assert_eq!(
            idx(2).clamp_to_min_index(idx(5)),
            idx(5),
            "Index below minimum - clamped up to minimum"
        );
        assert_eq!(
            idx(7).clamp_to_min_index(idx(5)),
            idx(7),
            "Index above minimum - returned unchanged"
        );
        assert_eq!(
            idx(5).clamp_to_min_index(idx(5)),
            idx(5),
            "Index at minimum - returned unchanged"
        );
        assert_eq!(
            idx(0).clamp_to_min_index(idx(3)),
            idx(3),
            "Zero index below minimum - clamped up"
        );

        // Test with ColIndex typed dimensions
        assert_eq!(
            col(1).clamp_to_min_index(col(3)),
            col(3),
            "ColIndex below minimum - clamped up"
        );
        assert_eq!(
            col(5).clamp_to_min_index(col(3)),
            col(5),
            "ColIndex above minimum - unchanged"
        );
        assert_eq!(
            col(3).clamp_to_min_index(col(3)),
            col(3),
            "ColIndex at minimum - unchanged"
        );
        assert_eq!(
            col(0).clamp_to_min_index(col(2)),
            col(2),
            "ColIndex zero below minimum - clamped up"
        );

        // Test with RowIndex typed dimensions
        assert_eq!(
            row(2).clamp_to_min_index(row(4)),
            row(4),
            "RowIndex below minimum - clamped up"
        );
        assert_eq!(
            row(6).clamp_to_min_index(row(4)),
            row(6),
            "RowIndex above minimum - unchanged"
        );
        assert_eq!(
            row(4).clamp_to_min_index(row(4)),
            row(4),
            "RowIndex at minimum - unchanged"
        );

        // Test edge cases
        assert_eq!(
            idx(0).clamp_to_min_index(idx(0)),
            idx(0),
            "Zero index with zero minimum"
        );
        assert_eq!(
            idx(10).clamp_to_min_index(idx(0)),
            idx(10),
            "Any index with zero minimum should be unchanged"
        );

        // Property test: result should always be >= minimum
        let test_cases = [(2, 5), (5, 5), (7, 5), (0, 3), (10, 1)];
        for (index_val, min_val) in test_cases {
            let index = idx(index_val);
            let min_bound = idx(min_val);
            let result = index.clamp_to_min_index(min_bound);

            assert!(
                result.as_usize() >= min_bound.as_usize(),
                "clamp_to_min_index({index_val}, {min_val}) = {} should be >= {min_val}",
                result.as_usize()
            );

            // If above minimum, should return original value
            if index_val >= min_val {
                assert_eq!(
                    result, index,
                    "clamp_to_min_index({index_val}, {min_val}) should preserve value when above minimum"
                );
            } else {
                assert_eq!(
                    result, min_bound,
                    "clamp_to_min_index({index_val}, {min_val}) should return minimum when below"
                );
            }
        }

        // Test with large values
        assert_eq!(
            col(0).clamp_to_min_index(col(100)),
            col(100),
            "Small index clamped to large minimum"
        );
        assert_eq!(
            col(200).clamp_to_min_index(col(100)),
            col(200),
            "Large index unchanged when above minimum"
        );
    }

    #[test]
    fn test_check_range_bounds_inclusive_end() {
        use crate::{ArrayAccessBoundsStatus, col, row};

        // Test basic Index types
        assert_eq!(
            idx(1).check_range_bounds_inclusive_end(idx(2), idx(7)),
            ArrayAccessBoundsStatus::Underflowed,
            "Index below range"
        );
        assert_eq!(
            idx(2).check_range_bounds_inclusive_end(idx(2), idx(7)),
            ArrayAccessBoundsStatus::Within,
            "Index at range start"
        );
        assert_eq!(
            idx(5).check_range_bounds_inclusive_end(idx(2), idx(7)),
            ArrayAccessBoundsStatus::Within,
            "Index within range"
        );
        assert_eq!(
            idx(7).check_range_bounds_inclusive_end(idx(2), idx(7)),
            ArrayAccessBoundsStatus::Within,
            "Index at range end"
        );
        assert_eq!(
            idx(8).check_range_bounds_inclusive_end(idx(2), idx(7)),
            ArrayAccessBoundsStatus::Overflowed,
            "Index above range"
        );

        // Test edge case: single element range
        assert_eq!(
            idx(5).check_range_bounds_inclusive_end(idx(5), idx(5)),
            ArrayAccessBoundsStatus::Within,
            "Single element range - at element"
        );
        assert_eq!(
            idx(4).check_range_bounds_inclusive_end(idx(5), idx(5)),
            ArrayAccessBoundsStatus::Underflowed,
            "Single element range - below"
        );
        assert_eq!(
            idx(6).check_range_bounds_inclusive_end(idx(5), idx(5)),
            ArrayAccessBoundsStatus::Overflowed,
            "Single element range - above"
        );

        // Test with ColIndex (scroll regions use case)
        let scroll_top = col(10);
        let scroll_bottom = col(20);

        assert_eq!(
            col(9).check_range_bounds_inclusive_end(scroll_top, scroll_bottom),
            ArrayAccessBoundsStatus::Underflowed,
            "Col below scroll region"
        );
        assert_eq!(
            col(10).check_range_bounds_inclusive_end(scroll_top, scroll_bottom),
            ArrayAccessBoundsStatus::Within,
            "Col at scroll start"
        );
        assert_eq!(
            col(15).check_range_bounds_inclusive_end(scroll_top, scroll_bottom),
            ArrayAccessBoundsStatus::Within,
            "Col within scroll region"
        );
        assert_eq!(
            col(20).check_range_bounds_inclusive_end(scroll_top, scroll_bottom),
            ArrayAccessBoundsStatus::Within,
            "Col at scroll end"
        );
        assert_eq!(
            col(21).check_range_bounds_inclusive_end(scroll_top, scroll_bottom),
            ArrayAccessBoundsStatus::Overflowed,
            "Col above scroll region"
        );

        // Test with RowIndex (typical VT-100 scroll region scenario)
        let vt_scroll_top = row(5);
        let vt_scroll_bottom = row(15);

        assert_eq!(
            row(4).check_range_bounds_inclusive_end(vt_scroll_top, vt_scroll_bottom),
            ArrayAccessBoundsStatus::Underflowed,
            "Row above scroll region"
        );
        assert_eq!(
            row(5).check_range_bounds_inclusive_end(vt_scroll_top, vt_scroll_bottom),
            ArrayAccessBoundsStatus::Within,
            "Row at scroll top"
        );
        assert_eq!(
            row(10).check_range_bounds_inclusive_end(vt_scroll_top, vt_scroll_bottom),
            ArrayAccessBoundsStatus::Within,
            "Row within scroll region"
        );
        assert_eq!(
            row(15).check_range_bounds_inclusive_end(vt_scroll_top, vt_scroll_bottom),
            ArrayAccessBoundsStatus::Within,
            "Row at scroll bottom"
        );
        assert_eq!(
            row(16).check_range_bounds_inclusive_end(vt_scroll_top, vt_scroll_bottom),
            ArrayAccessBoundsStatus::Overflowed,
            "Row below scroll region"
        );

        // Test zero-based ranges
        assert_eq!(
            idx(0).check_range_bounds_inclusive_end(idx(0), idx(3)),
            ArrayAccessBoundsStatus::Within,
            "Zero at start of zero-based range"
        );
        assert_eq!(
            idx(3).check_range_bounds_inclusive_end(idx(0), idx(3)),
            ArrayAccessBoundsStatus::Within,
            "End of zero-based range"
        );
        assert_eq!(
            idx(4).check_range_bounds_inclusive_end(idx(0), idx(3)),
            ArrayAccessBoundsStatus::Overflowed,
            "Beyond zero-based range"
        );

        // Test property: consistency with manual bounds checking
        let test_cases = [
            (0, 0, 10),   // At start
            (5, 0, 10),   // Within
            (10, 0, 10),  // At end
            (15, 10, 20), // Within larger range
            (1, 1, 1),    // Single element range
        ];

        for (index_val, min_val, max_val) in test_cases {
            let index = idx(index_val);
            let min_index = idx(min_val);
            let max_index = idx(max_val);

            let status = index.check_range_bounds_inclusive_end(min_index, max_index);
            let not_underflow = !index.underflows(min_index);
            let not_overflow = index <= max_index;
            let expected_within = not_underflow && not_overflow;

            let is_within = status == ArrayAccessBoundsStatus::Within;
            assert_eq!(
                is_within, expected_within,
                "check_range_bounds_inclusive_end({index_val}, {min_val}, {max_val}) = {status:?} should match manual bounds checking"
            );
        }

        // Test inverted range (min > max) - should always return Overflowed since index <
        // min but also > max
        assert_eq!(
            idx(5).check_range_bounds_inclusive_end(idx(10), idx(5)),
            ArrayAccessBoundsStatus::Underflowed,
            "Inverted range: 5 < 10 (min)"
        );
        assert_eq!(
            idx(10).check_range_bounds_inclusive_end(idx(10), idx(5)),
            ArrayAccessBoundsStatus::Overflowed,
            "Inverted range: 10 > 5 (max)"
        );
        assert_eq!(
            idx(0).check_range_bounds_inclusive_end(idx(10), idx(5)),
            ArrayAccessBoundsStatus::Underflowed,
            "Inverted range: 0 < 10 (min)"
        );

        // Test large values
        let large_min = idx(1000);
        let large_max = idx(2000);
        assert_eq!(
            idx(999).check_range_bounds_inclusive_end(large_min, large_max),
            ArrayAccessBoundsStatus::Underflowed,
            "Below large range"
        );
        assert_eq!(
            idx(1000).check_range_bounds_inclusive_end(large_min, large_max),
            ArrayAccessBoundsStatus::Within,
            "At large range start"
        );
        assert_eq!(
            idx(1500).check_range_bounds_inclusive_end(large_min, large_max),
            ArrayAccessBoundsStatus::Within,
            "Within large range"
        );
        assert_eq!(
            idx(2000).check_range_bounds_inclusive_end(large_min, large_max),
            ArrayAccessBoundsStatus::Within,
            "At large range end"
        );
        assert_eq!(
            idx(2001).check_range_bounds_inclusive_end(large_min, large_max),
            ArrayAccessBoundsStatus::Overflowed,
            "Above large range"
        );

        // Test typed consistency between ColIndex and RowIndex
        assert_eq!(
            col(5).check_range_bounds_inclusive_end(col(3), col(7)),
            ArrayAccessBoundsStatus::Within,
            "ColIndex range check"
        );
        assert_eq!(
            row(5).check_range_bounds_inclusive_end(row(3), row(7)),
            ArrayAccessBoundsStatus::Within,
            "RowIndex range check"
        );
        assert_eq!(
            col(2).check_range_bounds_inclusive_end(col(3), col(7)),
            ArrayAccessBoundsStatus::Underflowed,
            "ColIndex below range"
        );
        assert_eq!(
            row(8).check_range_bounds_inclusive_end(row(3), row(7)),
            ArrayAccessBoundsStatus::Overflowed,
            "RowIndex above range"
        );
    }
}

#[cfg(test)]
mod clamping_tests {
    use super::*;
    use crate::{height, idx, width};

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_index_clamp_to_max_length() {
        use crate::{col, height, row, width};

        // Test basic Index/Length clamping scenarios
        assert_eq!(
            idx(5).clamp_to_max_length(len(10)),
            idx(5),
            "Index within bounds - returned unchanged"
        );
        assert_eq!(
            idx(9).clamp_to_max_length(len(10)),
            idx(9),
            "Index at max valid position - returned unchanged"
        );
        assert_eq!(
            idx(15).clamp_to_max_length(len(10)),
            idx(9),
            "Index overflows - clamped to max valid index (length-1)"
        );
        assert_eq!(
            idx(0).clamp_to_max_length(len(10)),
            idx(0),
            "Zero index - always valid"
        );

        // Test with ColIndex/ColWidth typed dimensions
        assert_eq!(
            col(5).clamp_to_max_length(width(10)),
            col(5),
            "ColIndex within bounds - returned unchanged"
        );
        assert_eq!(
            col(9).clamp_to_max_length(width(10)),
            col(9),
            "ColIndex at boundary - returned unchanged (9 is valid for width 10)"
        );
        assert_eq!(
            col(10).clamp_to_max_length(width(10)),
            col(9),
            "ColIndex at boundary - clamped to max valid index"
        );
        assert_eq!(
            col(15).clamp_to_max_length(width(10)),
            col(9),
            "ColIndex overflows - clamped to max valid index"
        );
        assert_eq!(
            col(0).clamp_to_max_length(width(10)),
            col(0),
            "ColIndex zero - always valid"
        );

        // Test with RowIndex/RowHeight typed dimensions
        assert_eq!(
            row(3).clamp_to_max_length(height(5)),
            row(3),
            "RowIndex within bounds - returned unchanged"
        );
        assert_eq!(
            row(4).clamp_to_max_length(height(5)),
            row(4),
            "RowIndex at max valid position - returned unchanged"
        );
        assert_eq!(
            row(5).clamp_to_max_length(height(5)),
            row(4),
            "RowIndex at boundary - clamped to max valid index"
        );
        assert_eq!(
            row(8).clamp_to_max_length(height(5)),
            row(4),
            "RowIndex overflows - clamped to max valid index"
        );

        // Test edge case: empty collection (length 0)
        assert_eq!(
            idx(0).clamp_to_max_length(len(0)),
            idx(0),
            "Empty collection edge case - index 0 should remain 0"
        );
        assert_eq!(
            idx(5).clamp_to_max_length(len(0)),
            idx(0),
            "Empty collection with overflow - should clamp to 0"
        );

        // Test single element case (length 1)
        assert_eq!(
            idx(0).clamp_to_max_length(len(1)),
            idx(0),
            "Single element case - index 0 valid"
        );
        assert_eq!(
            idx(1).clamp_to_max_length(len(1)),
            idx(0),
            "Single element case - index 1 clamped to 0"
        );

        // Property test: result should always be <= max valid index
        let test_cases = [(5, 10), (10, 10), (15, 10), (0, 5), (100, 20)];
        for (index_val, length_val) in test_cases {
            let index = idx(index_val);
            let length = len(length_val);
            let result = index.clamp_to_max_length(length);
            let max_valid_index = if length_val > 0 { length_val - 1 } else { 0 };

            assert!(
                result.as_usize() <= max_valid_index,
                "clamp_to_max_length({index_val}, {length_val}) = {} should be <= {max_valid_index}",
                result.as_usize()
            );

            // If within bounds, should return original value
            if index_val < length_val {
                assert_eq!(
                    result, index,
                    "clamp_to_max_length({index_val}, {length_val}) should preserve value when within bounds"
                );
            }
        }

        // Test with large values
        assert_eq!(
            col(999).clamp_to_max_length(width(10)),
            col(9),
            "Large index value clamped to small max"
        );

        // Test consistency: clamp_to_max should match overflows() behavior
        let consistency_cases = [(5, 10), (9, 10), (10, 10), (15, 10)];
        for (index_val, length_val) in consistency_cases {
            let index = idx(index_val);
            let length = len(length_val);
            let clamped = index.clamp_to_max_length(length);
            let overflows = index.overflows(length);

            if overflows {
                // If it overflows, clamp_to_max should return max valid index
                assert_eq!(
                    clamped,
                    length.convert_to_index(),
                    "When index overflows, clamp_to_max should return max valid index"
                );
            } else {
                // If it doesn't overflow, clamp_to_max should return original
                assert_eq!(
                    clamped, index,
                    "When index doesn't overflow, clamp_to_max should preserve value"
                );
            }
        }
    }

    #[test]
    fn test_index_clamp_to_min_index() {
        use crate::{col, row};

        // Test basic Index clamping to minimum
        assert_eq!(
            idx(5).clamp_to_min_index(idx(3)),
            idx(5),
            "Index above minimum - returned unchanged"
        );
        assert_eq!(
            idx(3).clamp_to_min_index(idx(3)),
            idx(3),
            "Index equals minimum - returned unchanged"
        );
        assert_eq!(
            idx(1).clamp_to_min_index(idx(3)),
            idx(3),
            "Index below minimum - clamped to minimum"
        );
        assert_eq!(
            idx(0).clamp_to_min_index(idx(0)),
            idx(0),
            "Zero index with zero minimum - no change"
        );

        // Test with ColIndex typed dimensions
        assert_eq!(
            col(10).clamp_to_min_index(col(5)),
            col(10),
            "ColIndex above minimum - returned unchanged"
        );
        assert_eq!(
            col(5).clamp_to_min_index(col(5)),
            col(5),
            "ColIndex equals minimum - returned unchanged"
        );
        assert_eq!(
            col(2).clamp_to_min_index(col(5)),
            col(5),
            "ColIndex below minimum - clamped to minimum"
        );
        assert_eq!(
            col(0).clamp_to_min_index(col(3)),
            col(3),
            "ColIndex zero below minimum - clamped to minimum"
        );

        // Test with RowIndex typed dimensions
        assert_eq!(
            row(8).clamp_to_min_index(row(5)),
            row(8),
            "RowIndex above minimum - returned unchanged"
        );
        assert_eq!(
            row(5).clamp_to_min_index(row(5)),
            row(5),
            "RowIndex equals minimum - returned unchanged"
        );
        assert_eq!(
            row(3).clamp_to_min_index(row(5)),
            row(5),
            "RowIndex below minimum - clamped to minimum"
        );

        // Property test: result should always be >= minimum
        let test_cases = [(0, 3), (2, 3), (3, 3), (5, 3), (10, 8)];
        for (index_val, min_val) in test_cases {
            let index = idx(index_val);
            let min_bound = idx(min_val);
            let result = index.clamp_to_min_index(min_bound);

            assert!(
                result.as_usize() >= min_val,
                "clamp_to_min_index({index_val}, {min_val}) = {} should be >= {min_val}",
                result.as_usize()
            );

            // If above minimum, should return original value
            if index_val >= min_val {
                assert_eq!(
                    result, index,
                    "clamp_to_min_index({index_val}, {min_val}) should preserve value when above minimum"
                );
            } else {
                assert_eq!(
                    result, min_bound,
                    "clamp_to_min_index({index_val}, {min_val}) should return minimum when below"
                );
            }
        }

        // Test with large values
        assert_eq!(
            col(0).clamp_to_min_index(col(100)),
            col(100),
            "Small index clamped to large minimum"
        );
        assert_eq!(
            col(200).clamp_to_min_index(col(100)),
            col(200),
            "Large index unchanged when above minimum"
        );
    }

    #[test]
    fn test_clamp_to_max_length() {
        use super::*;

        // Test basic clamp operations with Length/Length.
        assert_eq!(
            LengthMarker::clamp_to_max(&len(5), len(10)),
            len(5),
            "Length within bounds - no change"
        );
        assert_eq!(
            LengthMarker::clamp_to_max(&len(15), len(10)),
            len(10),
            "Length exceeds bounds - gets clamped"
        );
        assert_eq!(
            LengthMarker::clamp_to_max(&len(8), len(8)),
            len(8),
            "Length equals bounds - no change"
        );
        assert_eq!(
            LengthMarker::clamp_to_max(&len(0), len(5)),
            len(0),
            "Zero length - no change"
        );

        // Test with typed dimensions
        assert_eq!(
            LengthMarker::clamp_to_max(&width(15), width(10)),
            width(10),
            "ColWidth exceeds bounds - gets clamped"
        );
        assert_eq!(
            LengthMarker::clamp_to_max(&height(8), height(12)),
            height(8),
            "RowHeight within bounds - no change"
        );

        // Property test: result should always be <= max
        let test_cases = [(5, 10), (10, 10), (15, 10), (0, 5)];
        for (length_val, max_val) in test_cases {
            let length = len(length_val);
            let max = len(max_val);
            let result = LengthMarker::clamp_to_max(&length, max);

            assert!(
                result.as_usize() <= max_val,
                "clamp_to_max({length_val}, {max_val}) = {} should be <= {max_val}",
                result.as_usize()
            );

            // If within bounds, should return original value
            if length_val <= max_val {
                assert_eq!(
                    result, length,
                    "clamp_to_max({length_val}, {max_val}) should preserve value when within bounds"
                );
            }
        }
    }
}

#[cfg(test)]
mod length_operations_tests {
    use super::*;
    use crate::{ColIndex, ColWidth, RowHeight, RowIndex, idx};

    #[test]
    fn test_remaining_from() {
        // Test basic cases with Length/Index.
        assert_eq!(
            len(10).remaining_from(idx(3)),
            len(7),
            "Normal case: 7 chars remain from index 3 to 9"
        );
        assert_eq!(
            len(10).remaining_from(idx(9)),
            len(1),
            "Edge case: only 1 char remains at last position"
        );
        assert_eq!(
            len(10).remaining_from(idx(10)),
            len(0),
            "Boundary case: at boundary, nothing remains"
        );
        assert_eq!(
            len(10).remaining_from(idx(15)),
            len(0),
            "Overflow case: beyond boundary, nothing remains"
        );

        // Test edge case: empty length.
        assert_eq!(
            len(0).remaining_from(idx(0)),
            len(0),
            "Empty collection: no chars remain"
        );
        assert_eq!(
            len(0).remaining_from(idx(5)),
            len(0),
            "Empty collection with overflow: no chars remain"
        );

        // Test with typed dimensions - ColWidth/ColIndex.
        let col_width = ColWidth::new(10);
        assert_eq!(
            col_width.remaining_from(ColIndex::new(3)),
            len(7),
            "ColWidth: 7 chars remain from col 3"
        );
        assert_eq!(
            col_width.remaining_from(ColIndex::new(9)),
            len(1),
            "ColWidth: 1 char remains at last col"
        );
        assert_eq!(
            col_width.remaining_from(ColIndex::new(10)),
            len(0),
            "ColWidth: at boundary"
        );
        assert_eq!(
            col_width.remaining_from(ColIndex::new(15)),
            len(0),
            "ColWidth: beyond boundary"
        );

        // Test with typed dimensions - RowHeight/RowIndex.
        let row_height = RowHeight::new(5);
        assert_eq!(
            row_height.remaining_from(RowIndex::new(2)),
            len(3),
            "RowHeight: 3 rows remain from row 2"
        );
        assert_eq!(
            row_height.remaining_from(RowIndex::new(4)),
            len(1),
            "RowHeight: 1 row remains at last row"
        );
        assert_eq!(
            row_height.remaining_from(RowIndex::new(5)),
            len(0),
            "RowHeight: at boundary"
        );
        assert_eq!(
            row_height.remaining_from(RowIndex::new(10)),
            len(0),
            "RowHeight: beyond boundary"
        );

        // Test single element case.
        assert_eq!(
            len(1).remaining_from(idx(0)),
            len(1),
            "Single element: 1 char remains from start"
        );
        assert_eq!(
            len(1).remaining_from(idx(1)),
            len(0),
            "Single element: at boundary"
        );

        // Test specific examples from documentation.
        let max_width = ColWidth::new(10);
        assert_eq!(
            max_width.remaining_from(ColIndex::new(3)),
            len(7),
            "Doc example: remaining_from(3) = 7"
        );
        assert_eq!(
            max_width.remaining_from(ColIndex::new(9)),
            len(1),
            "Doc example: remaining_from(9) = 1"
        );
        assert_eq!(
            max_width.remaining_from(ColIndex::new(10)),
            len(0),
            "Doc example: remaining_from(10) = 0"
        );
    }
}

#[cfg(test)]
mod conversion_tests {
    use super::*;
    use crate::{ColIndex, ColWidth, RowHeight, RowIndex, idx};

    #[test]
    fn test_convert_to_length() {
        // Test basic index to length conversion (0-based to 1-based).
        assert_eq!(
            idx(0).convert_to_length(),
            len(1),
            "Index 0 converts to length 1"
        );
        assert_eq!(
            idx(5).convert_to_length(),
            len(6),
            "Index 5 converts to length 6"
        );
        assert_eq!(
            idx(9).convert_to_length(),
            len(10),
            "Index 9 converts to length 10"
        );
        assert_eq!(
            idx(100).convert_to_length(),
            len(101),
            "Index 100 converts to length 101"
        );

        // Test with typed dimensions - ColIndex to ColWidth.
        assert_eq!(
            ColIndex::new(0).convert_to_length(),
            ColWidth::new(1),
            "ColIndex 0 to ColWidth 1"
        );
        assert_eq!(
            ColIndex::new(5).convert_to_length(),
            ColWidth::new(6),
            "ColIndex 5 to ColWidth 6"
        );
        assert_eq!(
            ColIndex::new(9).convert_to_length(),
            ColWidth::new(10),
            "ColIndex 9 to ColWidth 10"
        );
        assert_eq!(
            ColIndex::new(999).convert_to_length(),
            ColWidth::new(1000),
            "ColIndex 999 to ColWidth 1000"
        );

        // Test with typed dimensions - RowIndex to RowHeight.
        assert_eq!(
            RowIndex::new(0).convert_to_length(),
            RowHeight::new(1),
            "RowIndex 0 to RowHeight 1"
        );
        assert_eq!(
            RowIndex::new(2).convert_to_length(),
            RowHeight::new(3),
            "RowIndex 2 to RowHeight 3"
        );
        assert_eq!(
            RowIndex::new(4).convert_to_length(),
            RowHeight::new(5),
            "RowIndex 4 to RowHeight 5"
        );
        assert_eq!(
            RowIndex::new(49).convert_to_length(),
            RowHeight::new(50),
            "RowIndex 49 to RowHeight 50"
        );

        // Test that the conversion is consistent - converting back should work.
        let original_index = idx(42);
        let converted_length = original_index.convert_to_length();
        let back_to_index = converted_length.convert_to_index();
        assert_eq!(
            back_to_index, original_index,
            "Round-trip conversion should be consistent"
        );

        // Test with typed round-trip conversions.
        let col_index = ColIndex::new(7);
        let col_width = col_index.convert_to_length();
        let back_to_col_index = col_width.convert_to_index();
        assert_eq!(
            back_to_col_index, col_index,
            "ColIndex round-trip should be consistent"
        );

        let row_index = RowIndex::new(3);
        let row_height = row_index.convert_to_length();
        let back_to_row_index = row_height.convert_to_index();
        assert_eq!(
            back_to_row_index, row_index,
            "RowIndex round-trip should be consistent"
        );
    }

    #[test]
    fn test_convert_to_index() {
        // Test basic length to index conversion (1-based to 0-based).
        assert_eq!(
            len(1).convert_to_index(),
            idx(0),
            "Length 1 converts to index 0"
        );
        assert_eq!(
            len(6).convert_to_index(),
            idx(5),
            "Length 6 converts to index 5"
        );
        assert_eq!(
            len(10).convert_to_index(),
            idx(9),
            "Length 10 converts to index 9"
        );
        assert_eq!(
            len(101).convert_to_index(),
            idx(100),
            "Length 101 converts to index 100"
        );

        // Test with typed dimensions - ColWidth to ColIndex.
        assert_eq!(
            ColWidth::new(1).convert_to_index(),
            ColIndex::new(0),
            "ColWidth 1 to ColIndex 0"
        );
        assert_eq!(
            ColWidth::new(6).convert_to_index(),
            ColIndex::new(5),
            "ColWidth 6 to ColIndex 5"
        );
        assert_eq!(
            ColWidth::new(10).convert_to_index(),
            ColIndex::new(9),
            "ColWidth 10 to ColIndex 9"
        );
        assert_eq!(
            ColWidth::new(1000).convert_to_index(),
            ColIndex::new(999),
            "ColWidth 1000 to ColIndex 999"
        );

        // Test with typed dimensions - RowHeight to RowIndex.
        assert_eq!(
            RowHeight::new(1).convert_to_index(),
            RowIndex::new(0),
            "RowHeight 1 to RowIndex 0"
        );
        assert_eq!(
            RowHeight::new(3).convert_to_index(),
            RowIndex::new(2),
            "RowHeight 3 to RowIndex 2"
        );
        assert_eq!(
            RowHeight::new(5).convert_to_index(),
            RowIndex::new(4),
            "RowHeight 5 to RowIndex 4"
        );
        assert_eq!(
            RowHeight::new(50).convert_to_index(),
            RowIndex::new(49),
            "RowHeight 50 to RowIndex 49"
        );

        // Test that the conversion is consistent - converting back should work.
        let original_length = len(42);
        let converted_index = original_length.convert_to_index();
        let back_to_length = converted_index.convert_to_length();
        assert_eq!(
            back_to_length, original_length,
            "Round-trip conversion should be consistent"
        );

        // Test with typed round-trip conversions.
        let col_width = ColWidth::new(8);
        let col_index = col_width.convert_to_index();
        let back_to_col_width = col_index.convert_to_length();
        assert_eq!(
            back_to_col_width, col_width,
            "ColWidth round-trip should be consistent"
        );

        let row_height = RowHeight::new(4);
        let row_index = row_height.convert_to_index();
        let back_to_row_height = row_index.convert_to_length();
        assert_eq!(
            back_to_row_height, row_height,
            "RowHeight round-trip should be consistent"
        );

        // Test edge case: Length 0 should convert to... well, this might not be
        // implemented but if it is, it should be consistent with the type system.
        // Note: Length 0 might be a special case that needs separate handling.
    }

    #[test]
    fn test_as_usize() {
        // Test basic index types conversion to usize.
        assert_eq!(idx(0).as_usize(), 0, "Index 0 as usize");
        assert_eq!(idx(5).as_usize(), 5, "Index 5 as usize");
        assert_eq!(idx(100).as_usize(), 100, "Index 100 as usize");
        assert_eq!(idx(999).as_usize(), 999, "Index 999 as usize");

        // Test basic length types conversion to usize.
        assert_eq!(len(1).as_usize(), 1, "Length 1 as usize");
        assert_eq!(len(6).as_usize(), 6, "Length 6 as usize");
        assert_eq!(len(10).as_usize(), 10, "Length 10 as usize");
        assert_eq!(len(1000).as_usize(), 1000, "Length 1000 as usize");

        // Test typed index conversions - ColIndex.
        assert_eq!(ColIndex::new(0).as_usize(), 0, "ColIndex 0 as usize");
        assert_eq!(ColIndex::new(5).as_usize(), 5, "ColIndex 5 as usize");
        assert_eq!(ColIndex::new(80).as_usize(), 80, "ColIndex 80 as usize");
        assert_eq!(
            ColIndex::new(1024).as_usize(),
            1024,
            "ColIndex 1024 as usize"
        );

        // Test typed index conversions - RowIndex.
        assert_eq!(RowIndex::new(0).as_usize(), 0, "RowIndex 0 as usize");
        assert_eq!(RowIndex::new(3).as_usize(), 3, "RowIndex 3 as usize");
        assert_eq!(RowIndex::new(25).as_usize(), 25, "RowIndex 25 as usize");
        assert_eq!(RowIndex::new(768).as_usize(), 768, "RowIndex 768 as usize");

        // Test typed length conversions - ColWidth.
        assert_eq!(ColWidth::new(1).as_usize(), 1, "ColWidth 1 as usize");
        assert_eq!(ColWidth::new(10).as_usize(), 10, "ColWidth 10 as usize");
        assert_eq!(ColWidth::new(80).as_usize(), 80, "ColWidth 80 as usize");
        assert_eq!(
            ColWidth::new(1920).as_usize(),
            1920,
            "ColWidth 1920 as usize"
        );

        // Test typed length conversions - RowHeight.
        assert_eq!(RowHeight::new(1).as_usize(), 1, "RowHeight 1 as usize");
        assert_eq!(RowHeight::new(5).as_usize(), 5, "RowHeight 5 as usize");
        assert_eq!(RowHeight::new(30).as_usize(), 30, "RowHeight 30 as usize");
        assert_eq!(
            RowHeight::new(1080).as_usize(),
            1080,
            "RowHeight 1080 as usize"
        );

        // Test edge cases.
        assert_eq!(len(0).as_usize(), 0, "Length 0 as usize");
        assert_eq!(ColWidth::new(0).as_usize(), 0, "ColWidth 0 as usize");
        assert_eq!(RowHeight::new(0).as_usize(), 0, "RowHeight 0 as usize");

        // Test that as_usize preserves the underlying numeric value.
        for value in [0, 1, 5, 10, 42, 100, 999] {
            assert_eq!(
                idx(value).as_usize(),
                value,
                "Index {value} preserves value"
            );
            assert_eq!(
                len(value).as_usize(),
                value,
                "Length {value} preserves value"
            );
            assert_eq!(
                ColIndex::new(value).as_usize(),
                value,
                "ColIndex {value} preserves value"
            );
            assert_eq!(
                ColWidth::new(value).as_usize(),
                value,
                "ColWidth {value} preserves value"
            );
            assert_eq!(
                RowIndex::new(value).as_usize(),
                value,
                "RowIndex {value} preserves value"
            );
            assert_eq!(
                RowHeight::new(value).as_usize(),
                value,
                "RowHeight {value} preserves value"
            );
        }
    }

    #[test]
    fn test_clamp_to_max_length() {
        // Test basic clamp operations with Length/Length.
        assert_eq!(
            LengthMarker::clamp_to_max(&len(5), len(10)),
            len(5),
            "Length within bounds - no change"
        );
        assert_eq!(
            LengthMarker::clamp_to_max(&len(15), len(10)),
            len(10),
            "Length exceeds bounds - gets clamped"
        );
        assert_eq!(
            LengthMarker::clamp_to_max(&len(8), len(8)),
            len(8),
            "Equal lengths - returns the same value"
        );
        assert_eq!(
            LengthMarker::clamp_to_max(&len(0), len(5)),
            len(0),
            "Zero length within bounds"
        );
        assert_eq!(
            LengthMarker::clamp_to_max(&len(0), len(0)),
            len(0),
            "Zero length with zero max"
        );

        // Test with typed length dimensions - ColWidth.
        let col_width_5 = ColWidth::new(5);
        let col_width_10 = ColWidth::new(10);
        let col_width_15 = ColWidth::new(15);

        assert_eq!(
            LengthMarker::clamp_to_max(&col_width_5, col_width_10),
            col_width_5,
            "ColWidth within bounds - no change"
        );
        assert_eq!(
            LengthMarker::clamp_to_max(&col_width_15, col_width_10),
            col_width_10,
            "ColWidth exceeds bounds - gets clamped"
        );
        assert_eq!(
            LengthMarker::clamp_to_max(&col_width_10, col_width_10),
            col_width_10,
            "ColWidth equals bounds - returns max"
        );

        // Test with typed length dimensions - RowHeight.
        let row_height_3 = RowHeight::new(3);
        let row_height_5 = RowHeight::new(5);
        let row_height_7 = RowHeight::new(7);
        let row_height_15 = RowHeight::new(15);
        let row_height_20 = RowHeight::new(20);

        assert_eq!(
            LengthMarker::clamp_to_max(&row_height_3, row_height_15),
            row_height_3,
            "RowHeight within bounds - no change"
        );
        assert_eq!(
            LengthMarker::clamp_to_max(&row_height_7, row_height_5),
            row_height_5,
            "RowHeight exceeds smaller bounds - gets clamped"
        );
        assert_eq!(
            LengthMarker::clamp_to_max(&row_height_20, row_height_15),
            row_height_15,
            "RowHeight exceeds larger bounds - gets clamped"
        );

        // Test edge cases.
        assert_eq!(
            LengthMarker::clamp_to_max(&len(1), len(1)),
            len(1),
            "Single element case"
        );
        assert_eq!(
            LengthMarker::clamp_to_max(&len(100), len(1)),
            len(1),
            "Large value clamped to small max"
        );

        // Test that clamp_to always returns a value <= both inputs.
        let test_cases = [(5, 10), (10, 5), (0, 10), (10, 0), (7, 7), (100, 50)];
        for (length_val, max_val) in test_cases {
            let length = len(length_val);
            let max_length = len(max_val);
            let result = LengthMarker::clamp_to_max(&length, max_length);

            assert!(
                result.as_usize() <= length.as_usize(),
                "clamp_to_max_length({length_val}, {max_val}) result should be <= original length"
            );
            assert!(
                result.as_usize() <= max_length.as_usize(),
                "clamp_to_max_length({length_val}, {max_val}) result should be <= max_length"
            );
        }
    }

    #[test]
    fn test_as_u16() {
        // Test basic index types conversion to u16.
        assert_eq!(idx(0).as_u16(), 0, "Index 0 as u16");
        assert_eq!(idx(5).as_u16(), 5, "Index 5 as u16");
        assert_eq!(idx(100).as_u16(), 100, "Index 100 as u16");
        assert_eq!(idx(999).as_u16(), 999, "Index 999 as u16");

        // Test basic length types conversion to u16.
        assert_eq!(len(1).as_u16(), 1, "Length 1 as u16");
        assert_eq!(len(6).as_u16(), 6, "Length 6 as u16");
        assert_eq!(len(10).as_u16(), 10, "Length 10 as u16");
        assert_eq!(len(1000).as_u16(), 1000, "Length 1000 as u16");

        // Test typed index conversions - ColIndex.
        assert_eq!(ColIndex::new(0).as_u16(), 0, "ColIndex 0 as u16");
        assert_eq!(ColIndex::new(5).as_u16(), 5, "ColIndex 5 as u16");
        assert_eq!(ColIndex::new(80).as_u16(), 80, "ColIndex 80 as u16");
        assert_eq!(ColIndex::new(1024).as_u16(), 1024, "ColIndex 1024 as u16");

        // Test typed index conversions - RowIndex.
        assert_eq!(RowIndex::new(0).as_u16(), 0, "RowIndex 0 as u16");
        assert_eq!(RowIndex::new(3).as_u16(), 3, "RowIndex 3 as u16");
        assert_eq!(RowIndex::new(25).as_u16(), 25, "RowIndex 25 as u16");
        assert_eq!(RowIndex::new(768).as_u16(), 768, "RowIndex 768 as u16");

        // Test typed length conversions - ColWidth.
        assert_eq!(ColWidth::new(1).as_u16(), 1, "ColWidth 1 as u16");
        assert_eq!(ColWidth::new(10).as_u16(), 10, "ColWidth 10 as u16");
        assert_eq!(ColWidth::new(80).as_u16(), 80, "ColWidth 80 as u16");
        assert_eq!(ColWidth::new(1920).as_u16(), 1920, "ColWidth 1920 as u16");

        // Test typed length conversions - RowHeight.
        assert_eq!(RowHeight::new(1).as_u16(), 1, "RowHeight 1 as u16");
        assert_eq!(RowHeight::new(5).as_u16(), 5, "RowHeight 5 as u16");
        assert_eq!(RowHeight::new(30).as_u16(), 30, "RowHeight 30 as u16");
        assert_eq!(RowHeight::new(1080).as_u16(), 1080, "RowHeight 1080 as u16");

        // Test edge cases.
        assert_eq!(len(0).as_u16(), 0, "Length 0 as u16");
        assert_eq!(ColWidth::new(0).as_u16(), 0, "ColWidth 0 as u16");
        assert_eq!(RowHeight::new(0).as_u16(), 0, "RowHeight 0 as u16");

        // Test terminal-typical values (crossterm compatibility).
        assert_eq!(ColWidth::new(80).as_u16(), 80, "Standard terminal width 80");
        assert_eq!(ColWidth::new(120).as_u16(), 120, "Wide terminal width 120");
        assert_eq!(
            RowHeight::new(24).as_u16(),
            24,
            "Standard terminal height 24"
        );
        assert_eq!(RowHeight::new(50).as_u16(), 50, "Tall terminal height 50");

        // Test u16 max boundary (65535).
        assert_eq!(len(65535).as_u16(), 65535, "Length u16::MAX as u16");
        assert_eq!(
            ColWidth::new(65535).as_u16(),
            65535,
            "ColWidth u16::MAX as u16"
        );
        assert_eq!(
            RowHeight::new(65535).as_u16(),
            65535,
            "RowHeight u16::MAX as u16"
        );

        // Test that as_u16 preserves the underlying numeric value for typical ranges.
        for value in [0, 1, 5, 10, 42, 80, 100, 120, 1024] {
            assert_eq!(
                idx(value).as_u16(),
                u16::try_from(value).unwrap(),
                "Index {value} preserves value"
            );
            assert_eq!(
                len(value).as_u16(),
                u16::try_from(value).unwrap(),
                "Length {value} preserves value"
            );
            assert_eq!(
                ColIndex::new(value).as_u16(),
                u16::try_from(value).unwrap(),
                "ColIndex {value} preserves value"
            );
            assert_eq!(
                ColWidth::new(value).as_u16(),
                u16::try_from(value).unwrap(),
                "ColWidth {value} preserves value"
            );
            assert_eq!(
                RowIndex::new(value).as_u16(),
                u16::try_from(value).unwrap(),
                "RowIndex {value} preserves value"
            );
            assert_eq!(
                RowHeight::new(value).as_u16(),
                u16::try_from(value).unwrap(),
                "RowHeight {value} preserves value"
            );
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_is_zero() {
        // Test basic index types - zero values.
        assert!(idx(0).is_zero(), "Index 0 should be zero");
        assert!(!idx(1).is_zero(), "Index 1 should not be zero");
        assert!(!idx(5).is_zero(), "Index 5 should not be zero");
        assert!(!idx(100).is_zero(), "Index 100 should not be zero");

        // Test basic length types - zero and non-zero values.
        assert!(len(0).is_zero(), "Length 0 should be zero");
        assert!(!len(1).is_zero(), "Length 1 should not be zero");
        assert!(!len(5).is_zero(), "Length 5 should not be zero");
        assert!(!len(100).is_zero(), "Length 100 should not be zero");

        // Test typed index types - ColIndex.
        assert!(ColIndex::new(0).is_zero(), "ColIndex 0 should be zero");
        assert!(!ColIndex::new(1).is_zero(), "ColIndex 1 should not be zero");
        assert!(
            !ColIndex::new(10).is_zero(),
            "ColIndex 10 should not be zero"
        );
        assert!(
            !ColIndex::new(80).is_zero(),
            "ColIndex 80 should not be zero"
        );

        // Test typed index types - RowIndex.
        assert!(RowIndex::new(0).is_zero(), "RowIndex 0 should be zero");
        assert!(!RowIndex::new(1).is_zero(), "RowIndex 1 should not be zero");
        assert!(!RowIndex::new(5).is_zero(), "RowIndex 5 should not be zero");
        assert!(
            !RowIndex::new(25).is_zero(),
            "RowIndex 25 should not be zero"
        );

        // Test typed length types - ColWidth.
        assert!(ColWidth::new(0).is_zero(), "ColWidth 0 should be zero");
        assert!(!ColWidth::new(1).is_zero(), "ColWidth 1 should not be zero");
        assert!(
            !ColWidth::new(10).is_zero(),
            "ColWidth 10 should not be zero"
        );
        assert!(
            !ColWidth::new(80).is_zero(),
            "ColWidth 80 should not be zero"
        );
        assert!(
            !ColWidth::new(120).is_zero(),
            "ColWidth 120 should not be zero"
        );

        // Test typed length types - RowHeight.
        assert!(RowHeight::new(0).is_zero(), "RowHeight 0 should be zero");
        assert!(
            !RowHeight::new(1).is_zero(),
            "RowHeight 1 should not be zero"
        );
        assert!(
            !RowHeight::new(5).is_zero(),
            "RowHeight 5 should not be zero"
        );
        assert!(
            !RowHeight::new(24).is_zero(),
            "RowHeight 24 should not be zero"
        );
        assert!(
            !RowHeight::new(50).is_zero(),
            "RowHeight 50 should not be zero"
        );

        // Test edge cases and boundary values.
        assert!(
            !idx(usize::MAX).is_zero(),
            "Index usize::MAX should not be zero"
        );
        assert!(
            !len(usize::MAX).is_zero(),
            "Length usize::MAX should not be zero"
        );
        assert!(
            !ColIndex::new(u16::MAX as usize).is_zero(),
            "ColIndex u16::MAX should not be zero"
        );
        assert!(
            !RowIndex::new(u16::MAX as usize).is_zero(),
            "RowIndex u16::MAX should not be zero"
        );
        assert!(
            !ColWidth::new(u16::MAX as usize).is_zero(),
            "ColWidth u16::MAX should not be zero"
        );
        assert!(
            !RowHeight::new(u16::MAX as usize).is_zero(),
            "RowHeight u16::MAX should not be zero"
        );

        // Test consistency with as_usize() == 0 (the implementation).
        for value in [0, 1, 5, 10, 42, 100, 999] {
            assert_eq!(
                idx(value).is_zero(),
                idx(value).as_usize() == 0,
                "Index {value} is_zero should match as_usize() == 0"
            );
            assert_eq!(
                len(value).is_zero(),
                len(value).as_usize() == 0,
                "Length {value} is_zero should match as_usize() == 0"
            );
            assert_eq!(
                ColIndex::new(value).is_zero(),
                ColIndex::new(value).as_usize() == 0,
                "ColIndex {value} is_zero should match as_usize() == 0"
            );
            assert_eq!(
                ColWidth::new(value).is_zero(),
                ColWidth::new(value).as_usize() == 0,
                "ColWidth {value} is_zero should match as_usize() == 0"
            );
            assert_eq!(
                RowIndex::new(value).is_zero(),
                RowIndex::new(value).as_usize() == 0,
                "RowIndex {value} is_zero should match as_usize() == 0"
            );
            assert_eq!(
                RowHeight::new(value).is_zero(),
                RowHeight::new(value).as_usize() == 0,
                "RowHeight {value} is_zero should match as_usize() == 0"
            );
        }
    }
}
