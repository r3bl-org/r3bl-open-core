// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{index_marker::IndexMarker, unit_marker::UnitMarker};
use crate::RangeBoundsResult;

/// Viewport and window visibility checking for rendering and UI operations.
///
/// This trait handles spatial visibility checks where we need to know if content
/// falls within a visible window or viewport. This is fundamentally different from
/// array bounds or cursor positioning because it's about rendering and visibility
/// rather than safety or editing semantics.
///
/// ## Core Purpose
///
/// Use case: "Is this content visible in my viewport?"
///
/// Viewport bounds checking answers questions about what's currently visible
/// on screen, what needs to be rendered, and what can be skipped for performance.
///
/// # Key Distinction from Other Bounds Traits
///
/// Unlike array or cursor bounds which are about safety and editing, viewport
/// bounds are about rendering and visibility:
///
/// | Trait                         | Rule                          | Use Case      | Example                                              |
/// |-------------------------------|-------------------------------|---------------|------------------------------------------------------|
/// | [`ArrayBoundsCheck`]          | `index < length`              | Index safety  | `buffer[5]` needs `5 < buffer.len()`                 |
/// | [`CursorBoundsCheck`]         | `index <= length`             | Text editing  | Cursor can be at position `length` (after last char) |
/// | `ViewportBoundsCheck`📍       | `start <= index < start+size` | Rendering     | Content visibility in windows                        |
/// | [`RangeBoundsCheck`]          | `start <= end <= length`      | Iteration     | Range object structural validation                   |
///
/// ## Viewport Geometry
///
/// Viewports naturally express their geometry as position + size rather than
/// start/end indices. This is because UI systems think in terms of:
/// - "Show me 20 columns starting at column 5"
/// - "Render a window 800x600 at position (100, 50)"
///
/// ```text
/// Viewport Example:
/// Full content is 50 columns wide, viewport shows columns [10, 30)
///
///      viewport_start=10          viewport_end=30 (exclusive)
///               ↓                          ↓
/// Column:   8   9  10  11  12 ... 28  29  30  31  32
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///         │   │   │ ▓ │ ▓ │ ▓ │...│ ▓ │ ▓ │   │   │   │
///         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
///                 ╰────── viewport area ──────╯
/// ```
///
/// ## Primary Use Cases
///
/// - Terminal viewport scrolling: Determining which lines are visible
/// - Window clipping regions: Checking if UI elements need rendering
/// - Visible content determination: Optimizing what to draw
/// - Render optimization: Skip processing for off-screen elements
/// - Scroll calculations: Determining scroll positions and ranges
///
/// ## Exclusive Upper Bound Semantics
///
/// See the [Interval Notation] section in the module documentation for notation
/// details.
///
/// Viewport bounds use exclusive upper bounds `[start, start+size)` because:
/// 1. Natural for iteration: `for i in start..end` in Rust
/// 2. Size-based thinking: "Show me N items starting here"
/// 3. Pixel-perfect rendering: Avoids off-by-one errors in graphics
/// 4. Performance optimization: Clean range checks without edge case handling
///
/// ## Design Rationale
///
/// Viewport operations are hybrid operations that combine:
/// - Start position: An index indicating where the viewport begins
/// - Viewport size: A length indicating how many units the viewport covers
/// - Exclusive upper bound: The viewport covers `[start, start+size)`
///
/// This pattern is distinct enough from pure index-to-index or index-to-length
/// comparisons to warrant its own trait.
///
/// ## Key Methods
///
/// - [`check_viewport_bounds()`] - Returns detailed status for pattern matching
/// - [`is_in_viewport()`] - Returns boolean for simple visibility checks
///
/// Both methods implement the same logic with different return types to support
/// different usage patterns.
///
/// ## Relationship to Other Bounds Checking
///
/// This trait complements but doesn't replace other bounds checking:
/// - Before rendering: Use viewport bounds to check visibility
/// - During rendering: Use array bounds to ensure correct content access
/// - For cursor: Use cursor bounds to validate text editing positions
///
/// ## See Also
///
/// - [`IndexMarker`] - Index-to-index comparisons and basic bounds checking
/// - [`ArrayBoundsCheck`] - Array index validation for correct content access
/// - [`CursorBoundsCheck`] - Cursor positioning for text editing
/// - [`RangeBoundsCheck`] - Range validation for iteration and algorithms
/// - [Module documentation] - Overview of the complete bounds checking architecture
///
/// [`IndexMarker`]: crate::IndexMarker
/// [`ArrayBoundsCheck`]: crate::ArrayBoundsCheck
/// [`CursorBoundsCheck`]: crate::CursorBoundsCheck
/// [`RangeBoundsCheck`]: crate::RangeBoundsCheck
/// [Module documentation]: mod@crate::core::units::bounds_check
/// [Interval Notation]: mod@crate::core::units::bounds_check#interval-notation
/// [`check_viewport_bounds()`]: Self::check_viewport_bounds
/// [`is_in_viewport()`]: Self::is_in_viewport
pub trait ViewportBoundsCheck: IndexMarker {
    /// Check if this index is within a viewport window with exclusive upper bound.
    ///
    /// This provides comprehensive bounds checking that can detect underflow,
    /// valid positions, and overflow in a single operation with exclusive upper bound
    /// semantics suitable for viewport and window bounds.
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
    ///                 ╰──── within range ────╯
    ///
    /// U = Underflowed (index < start)
    /// W = Within (start <= index < start+size)
    /// O = Overflowed (index >= start+size)
    ///
    /// check_viewport_bounds(col(1), col(2), width(6)) = Underflowed
    /// check_viewport_bounds(col(2), col(2), width(6)) = Within
    /// check_viewport_bounds(col(7), col(2), width(6)) = Within
    /// check_viewport_bounds(col(8), col(2), width(6)) = Overflowed
    /// ```
    ///
    /// # Use Cases
    /// - Viewport bounds checking: `[viewport_start, viewport_start+viewport_size)`
    /// - Buffer array access: `[0, buffer_length)` for safe indexing
    /// - Window visibility: Checking if content is within a scrollable window
    /// - Index range validation: Ensuring indices stay within content bounds
    ///
    /// # When to Use This Method vs Semantic Aliases
    /// - Use this method when you need detailed status information (underflow/overflow
    ///   handling)
    /// - Use [`is_in_viewport()`] for simple boolean viewport containment checks
    /// - For pattern matching: When you need to handle underflow/overflow differently
    /// - For complex logic: When the specific type of bounds violation matters
    ///
    /// ```rust
    /// use r3bl_tui::{ViewportBoundsCheck, RangeBoundsResult, col, width};
    ///
    /// let viewport_start = col(10);
    /// let viewport_width = width(20);
    /// let caret_col = col(15);
    ///
    /// // Use this core method for detailed handling:
    /// match caret_col.check_viewport_bounds(viewport_start, viewport_width) {
    ///     RangeBoundsResult::Underflowed => println!("Need to scroll right"),
    ///     RangeBoundsResult::Within => println!("Cursor visible at col {}", caret_col.as_usize()),
    ///     RangeBoundsResult::Overflowed => println!("Need to scroll left"),
    /// }
    ///
    /// // Use semantic alias for simple checks:
    /// if caret_col.is_in_viewport(viewport_start, viewport_width) {
    ///     println!("Rendering cursor at position {}", caret_col.as_usize());
    /// }
    /// ```
    ///
    /// # Returns
    /// - [`RangeBoundsResult::Underflowed`] if index < start
    /// - [`RangeBoundsResult::Within`] if start <= index < start+size
    /// - [`RangeBoundsResult::Overflowed`] if index >= start+size
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{ViewportBoundsCheck, RangeBoundsResult, col, width};
    ///
    /// let viewport_start = col(2);
    /// let viewport_width = width(6);
    ///
    /// // Viewport covers [2, 8) - column 8 is NOT included
    /// assert_eq!(col(1).check_viewport_bounds(viewport_start, viewport_width), RangeBoundsResult::Underflowed);
    /// assert_eq!(col(5).check_viewport_bounds(viewport_start, viewport_width), RangeBoundsResult::Within);
    /// assert_eq!(col(8).check_viewport_bounds(viewport_start, viewport_width), RangeBoundsResult::Overflowed);
    /// ```
    ///
    /// [`RangeBoundsResult::Underflowed`]: crate::RangeBoundsResult::Underflowed
    /// [`RangeBoundsResult::Within`]: crate::RangeBoundsResult::Within
    /// [`RangeBoundsResult::Overflowed`]: crate::RangeBoundsResult::Overflowed
    /// [`is_in_viewport()`]: Self::is_in_viewport
    fn check_viewport_bounds(
        &self,
        viewport_start: impl Into<Self>,
        viewport_size: Self::LengthType,
    ) -> RangeBoundsResult
    where
        Self: Sized,
    {
        let start_bound: Self = viewport_start.into();

        if *self < start_bound {
            RangeBoundsResult::Underflowed
        } else {
            // Calculate the exclusive upper bound: start + size (using usize arithmetic)
            let start_as_usize = start_bound.as_usize();
            let size_as_usize = viewport_size.as_usize();
            let end_bound_usize = start_as_usize + size_as_usize;
            let self_as_usize = self.as_usize();

            if self_as_usize >= end_bound_usize {
                RangeBoundsResult::Overflowed
            } else {
                RangeBoundsResult::Within
            }
        }
    }

    /// Check if this index is visible within a viewport window.
    ///
    /// This is a semantic alias for [`check_viewport_bounds()`] that returns
    /// a boolean result. Use this when you need a simple true/false answer for viewport
    /// containment checking.
    ///
    /// A viewport defines a rectangular window showing a portion of larger content,
    /// with exclusive upper bounds: `[start, start+size)`.
    ///
    /// ```text
    /// Viewport Window Example:
    /// Full content is 50 columns wide, viewport shows columns [10, 30)
    ///
    ///      viewport_start=10          viewport_end=30 (exclusive)
    ///               ↓                          ↓
    /// Column:   8   9  10  11  12 ... 28  29  30  31  32
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///         │   │   │ ▓ │ ▓ │ ▓ │...│ ▓ │ ▓ │   │   │   │
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
    /// - Simple boolean checks: When you only need true/false for viewport visibility
    /// - Conditional rendering: Deciding whether to draw/process elements
    /// - Scroll calculations: Checking if content is currently visible
    ///
    /// # When to Use Core Methods Instead
    /// - Pattern matching: When you need to handle underflow/overflow differently
    /// - Detailed status: When the specific type of bounds violation matters
    /// - Complex logic: When you need more than just within/not-within information
    ///
    /// ```rust
    /// use r3bl_tui::{ViewportBoundsCheck, RangeBoundsResult, col, width};
    ///
    /// let viewport_start = col(5);
    /// let viewport_width = width(10);
    /// let caret_col = col(8);
    ///
    /// // Use this semantic alias for simple checks:
    /// if caret_col.is_in_viewport(viewport_start, viewport_width) {
    ///     println!("Cursor at {} is visible", caret_col.as_usize());
    /// }
    ///
    /// // Use core method for detailed handling:
    /// match caret_col.check_viewport_bounds(viewport_start, viewport_width) {
    ///     RangeBoundsResult::Underflowed => println!("Need to scroll right"),
    ///     RangeBoundsResult::Within => println!("Cursor visible"),
    ///     RangeBoundsResult::Overflowed => println!("Need to scroll left"),
    /// }
    /// ```
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{ViewportBoundsCheck, col, width};
    ///
    /// let viewport_start = col(10);
    /// let viewport_width = width(20);
    ///
    /// // Simple boolean viewport checks
    /// assert!(!col(9).is_in_viewport(viewport_start, viewport_width));   // Before viewport
    /// assert!(col(10).is_in_viewport(viewport_start, viewport_width));   // At viewport start
    /// assert!(col(25).is_in_viewport(viewport_start, viewport_width));   // Within viewport
    /// assert!(col(29).is_in_viewport(viewport_start, viewport_width));   // At viewport end-1
    /// assert!(!col(30).is_in_viewport(viewport_start, viewport_width));  // After viewport
    ///
    /// // Usage in rendering logic
    /// for col_val in 0..50 {
    ///     let column_index = col(col_val);
    ///     if column_index.is_in_viewport(viewport_start, viewport_width) {
    ///         // render_column(column_index);
    ///     }
    ///     // Skip rendering for columns outside viewport
    /// }
    /// ```
    ///
    /// [`check_viewport_bounds()`]: Self::check_viewport_bounds
    fn is_in_viewport(
        &self,
        viewport_start: impl Into<Self>,
        viewport_size: Self::LengthType,
    ) -> bool
    where
        Self: Sized,
    {
        matches!(
            self.check_viewport_bounds(viewport_start, viewport_size),
            RangeBoundsResult::Within
        )
    }
}

// Automatic implementation for all IndexMarker types
impl<T> ViewportBoundsCheck for T where T: IndexMarker {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{idx, len};

    #[test]
    fn test_check_viewport_bounds() {
        let viewport_start = idx(2);
        let viewport_size = len(6);

        // Test underflow
        assert_eq!(
            idx(0).check_viewport_bounds(viewport_start, viewport_size),
            RangeBoundsResult::Underflowed
        );
        assert_eq!(
            idx(1).check_viewport_bounds(viewport_start, viewport_size),
            RangeBoundsResult::Underflowed
        );

        // Test within bounds [2, 8)
        assert_eq!(
            idx(2).check_viewport_bounds(viewport_start, viewport_size),
            RangeBoundsResult::Within
        );
        assert_eq!(
            idx(5).check_viewport_bounds(viewport_start, viewport_size),
            RangeBoundsResult::Within
        );
        assert_eq!(
            idx(7).check_viewport_bounds(viewport_start, viewport_size),
            RangeBoundsResult::Within
        );

        // Test overflow (8 and beyond)
        assert_eq!(
            idx(8).check_viewport_bounds(viewport_start, viewport_size),
            RangeBoundsResult::Overflowed
        );
        assert_eq!(
            idx(10).check_viewport_bounds(viewport_start, viewport_size),
            RangeBoundsResult::Overflowed
        );
    }

    #[test]
    fn test_is_in_viewport() {
        let viewport_start = idx(10);
        let viewport_size = len(20);

        // Test positions outside viewport
        assert!(!idx(9).is_in_viewport(viewport_start, viewport_size));
        assert!(!idx(30).is_in_viewport(viewport_start, viewport_size));
        assert!(!idx(35).is_in_viewport(viewport_start, viewport_size));

        // Test positions within viewport [10, 30)
        assert!(idx(10).is_in_viewport(viewport_start, viewport_size));
        assert!(idx(15).is_in_viewport(viewport_start, viewport_size));
        assert!(idx(25).is_in_viewport(viewport_start, viewport_size));
        assert!(idx(29).is_in_viewport(viewport_start, viewport_size));
    }

    #[test]
    fn test_zero_size_viewport() {
        let viewport_start = idx(5);
        let zero_size = len(0);

        // Zero-size viewport should not contain any indices
        assert_eq!(
            idx(5).check_viewport_bounds(viewport_start, zero_size),
            RangeBoundsResult::Overflowed
        );
        assert!(!idx(5).is_in_viewport(viewport_start, zero_size));
    }

    #[test]
    fn test_unit_size_viewport() {
        let viewport_start = idx(3);
        let unit_size = len(1);

        // Unit-size viewport [3, 4) should contain only index 3
        assert_eq!(
            idx(2).check_viewport_bounds(viewport_start, unit_size),
            RangeBoundsResult::Underflowed
        );
        assert_eq!(
            idx(3).check_viewport_bounds(viewport_start, unit_size),
            RangeBoundsResult::Within
        );
        assert_eq!(
            idx(4).check_viewport_bounds(viewport_start, unit_size),
            RangeBoundsResult::Overflowed
        );

        // Boolean tests
        assert!(!idx(2).is_in_viewport(viewport_start, unit_size));
        assert!(idx(3).is_in_viewport(viewport_start, unit_size));
        assert!(!idx(4).is_in_viewport(viewport_start, unit_size));
    }

    #[test]
    fn test_zero_start_viewport() {
        let viewport_start = idx(0);
        let viewport_size = len(5);

        // Viewport starting at zero [0, 5)
        assert_eq!(
            idx(0).check_viewport_bounds(viewport_start, viewport_size),
            RangeBoundsResult::Within
        );
        assert_eq!(
            idx(4).check_viewport_bounds(viewport_start, viewport_size),
            RangeBoundsResult::Within
        );
        assert_eq!(
            idx(5).check_viewport_bounds(viewport_start, viewport_size),
            RangeBoundsResult::Overflowed
        );

        // Boolean tests
        assert!(idx(0).is_in_viewport(viewport_start, viewport_size));
        assert!(idx(4).is_in_viewport(viewport_start, viewport_size));
        assert!(!idx(5).is_in_viewport(viewport_start, viewport_size));
    }
}
