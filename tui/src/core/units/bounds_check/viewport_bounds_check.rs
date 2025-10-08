// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Viewport visibility checking for rendering - see [`ViewportBoundsCheck`] trait.

use super::{index_ops::IndexOps, numeric_value::NumericConversions};
use crate::RangeBoundsResult;

/// Viewport visibility checking for rendering and UI operations.
///
/// This trait handles spatial visibility checks where we need to know if content
/// falls within a visible window or viewport. It provides essential operations for
/// rendering optimization, scroll calculations, and determining what content needs
/// to be displayed.
///
/// ## Purpose
///
/// This trait answers the question: **"Is this content visible in my viewport?"**
///
/// Viewport bounds checking is fundamentally different from array bounds or cursor
/// positioning because it's about rendering and visibility rather than safety or
/// editing semantics.
///
/// ## Key Trait Capabilities
///
/// - **Visibility checking**: Determine if content falls within viewport bounds via
///   [`check_viewport_bounds()`]
/// - **Three-state result**: Returns underflow/within/overflow for precise positioning
/// - **Exclusive upper bounds**: Uses `[start, start+size)` semantics for rendering
/// - **Automatic implementation**: Available for all [`IndexOps`] types via blanket impl
///
/// ## Viewport Geometry
///
/// Viewports naturally express their geometry as position + size rather than
/// start/end indices. This is because UI systems think in terms of:
/// - "Show me 20 columns starting at column 5"
/// - "Render a window 800x600 at position (100, 50)"
///
/// ```text
/// Horizontal Viewport Example:
/// Full content is 50 columns wide, viewport shows columns [10, 30)
///
///      viewport_start=10           viewport_end=30 (exclusive)
///               ↓                           ↓
/// Column:   8   9  10  11  12  ...  28  29  30  31  32
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///         │   │   │ ▓ │ ▓ │ ▓ │...│ ▓ │ ▓ │   │   │   │
///         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
///                 ╰────── viewport area ──╯
///
/// ▓ : Visible content within viewport
///
/// Vertical Viewport Example:
/// Full content is 50 rows tall, viewport shows rows [10, 30)
///
///      Row 8      ┌───────────────────────┐
///      Row 9      │                       │
///      Row 10  ←  ├───────────────────────┤  ← viewport_start=10
///      Row 11     │ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ │
///      Row 12     │ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ │
///       ...       │ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ │  Visible content
///      Row 28     │ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ │  (viewport area)
///      Row 29     │ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ │
///      Row 30  ←  ├───────────────────────┤  ← viewport_end=30 (exclusive)
///      Row 31     │                       │
///      Row 32     │                       │
///                 └───────────────────────┘
///
/// ▓ : Visible content within viewport
/// ```
///
/// See the [Interval Notation] section in the module documentation for notation
/// details.
///
/// ## Primary Use Cases
///
/// - Terminal viewport scrolling: Determining which lines are visible
/// - Window clipping regions: Checking if UI elements need rendering
/// - Visible content determination: Optimizing what to draw
/// - Render optimization: Skip processing for off-screen elements
/// - Scroll calculations: Determining scroll positions and ranges
///
/// ## Key Distinction from Other Bounds Traits
///
/// Unlike array or cursor bounds which are about safety and editing, viewport
/// bounds are about rendering and visibility:
///
/// | Trait                         | Rule                          | Use Case      | Example                                              |
/// |-------------------------------|-------------------------------|---------------|------------------------------------------------------|
/// | [`ArrayBoundsCheck`]          | `index < length`              | Index safety  | `buffer[5]` needs `5 < buffer.len()`                 |
/// | [`CursorBoundsCheck`]         | `index <= length`             | Text editing  | Cursor can be at position `length` (after last char) |
/// | `ViewportBoundsCheck`📍       | `start <= index < start+size` | Rendering     | Content visibility in windows                        |
/// | [`RangeBoundsExt`]            | `start <= end <= length`      | Iteration     | Range object structural validation                   |
///
/// ## Exclusive Upper Bound Semantics
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
/// ## Method Selection Guide
///
/// ### When to Use [`check_viewport_bounds()`]
/// - **Pattern matching**: When you need to handle underflow/overflow differently
/// - **Detailed status**: When the specific type of bounds violation matters
/// - **Complex logic**: When you need more than just within/not-within information
/// - **Scroll calculations**: When determining scroll direction (left/right/up/down)
///
/// ## Examples
///
/// This trait provides comprehensive visibility checking:
/// ```rust
/// use r3bl_tui::{ViewportBoundsCheck, RangeBoundsResult, col, width};
///
/// let viewport_start = col(10);
/// let viewport_width = width(20);
/// let content_col = col(15);
///
/// // Simple boolean check - most common pattern
/// if content_col.check_viewport_bounds(viewport_start, viewport_width) ==
/// RangeBoundsResult::Within {     // Content is visible, render it
/// }
///
/// // Detailed status for complex logic
/// match content_col.check_viewport_bounds(viewport_start, viewport_width) {
///     RangeBoundsResult::Underflowed => println!("Scroll right to see"),
///     RangeBoundsResult::Within => println!("Content visible"),
///     RangeBoundsResult::Overflowed => println!("Scroll left to see"),
/// }
///
/// // Render loop optimization
/// for col_index in 0..50 {
///     let column = col(col_index);
///     if column.check_viewport_bounds(viewport_start, viewport_width) ==
/// RangeBoundsResult::Within {         // Only render visible columns
///     }
/// }
/// ```
///
/// ## See Also
///
/// - [`IndexOps`] - Index-to-index comparisons and basic bounds checking
/// - [`ArrayBoundsCheck`] - Array index validation for correct content access
/// - [`CursorBoundsCheck`] - Cursor positioning for text editing
/// - [`RangeBoundsExt`] - Range validation for iteration and algorithms
/// - [Module documentation] - Overview of the complete bounds checking architecture
///
/// [`IndexOps`]: crate::IndexOps
/// [`ArrayBoundsCheck`]: crate::ArrayBoundsCheck
/// [`CursorBoundsCheck`]: crate::CursorBoundsCheck
/// [`RangeBoundsExt`]: crate::RangeBoundsExt
/// [Module documentation]: mod@crate::core::units::bounds_check
/// [Interval Notation]: mod@crate::core::units::bounds_check#interval-notation
/// [`check_viewport_bounds()`]: Self::check_viewport_bounds
pub trait ViewportBoundsCheck: IndexOps {
    /// Check if this index is within a viewport window.
    ///
    /// Returns a three-state result indicating whether the index falls before
    /// (underflowed), within, or after (overflowed) the viewport bounds defined by
    /// `[start, start+size)`.
    ///
    /// See the [trait documentation][Self] for viewport geometry, exclusive upper bound
    /// semantics, design rationale, usage patterns, and examples.
    ///
    /// # Parameters
    /// - `arg_viewport_start`: The starting index of the viewport
    /// - `arg_viewport_size`: The size/length of the viewport
    ///
    /// # Returns
    /// - [`RangeBoundsResult::Underflowed`] if `index < start`
    /// - [`RangeBoundsResult::Within`] if `start <= index < start+size`
    /// - [`RangeBoundsResult::Overflowed`] if `index >= start+size`
    ///
    /// [`RangeBoundsResult::Underflowed`]: crate::RangeBoundsResult::Underflowed
    /// [`RangeBoundsResult::Within`]: crate::RangeBoundsResult::Within
    /// [`RangeBoundsResult::Overflowed`]: crate::RangeBoundsResult::Overflowed
    fn check_viewport_bounds(
        &self,
        arg_viewport_start: impl Into<Self>,
        arg_viewport_size: impl Into<Self::LengthType>,
    ) -> RangeBoundsResult {
        let start_bound: Self = arg_viewport_start.into();
        let viewport_size: Self::LengthType = arg_viewport_size.into();

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
}

/// Blanket implementation that automatically implements [`ViewportBoundsCheck`] for all
/// types that implement [`IndexOps`].
///
/// This eliminates the need to write individual empty impl blocks like:
/// ```rust,compile_fail
/// impl ViewportBoundsCheck for ColIndex {}
/// impl ViewportBoundsCheck for RowIndex {}
/// impl ViewportBoundsCheck for Index {}
/// ```
///
/// All method implementations are provided as defaults in the trait definition,
/// so this impl block is empty - it simply activates the trait for all [`IndexOps`]
/// types.
///
/// This pattern is only possible because [`ViewportBoundsCheck`] is not parameterized
/// over type parameters. For comparison, see [`ArrayBoundsCheck`] which cannot use a
/// blanket impl due to its `<LengthType>` type parameter.
///
/// [`ArrayBoundsCheck`]: crate::ArrayBoundsCheck
impl<T> ViewportBoundsCheck for T where T: IndexOps {}

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
        assert!(
            idx(9).check_viewport_bounds(viewport_start, viewport_size)
                != RangeBoundsResult::Within
        );
        assert!(
            idx(30).check_viewport_bounds(viewport_start, viewport_size)
                != RangeBoundsResult::Within
        );
        assert!(
            idx(35).check_viewport_bounds(viewport_start, viewport_size)
                != RangeBoundsResult::Within
        );

        // Test positions within viewport [10, 30)
        assert!(
            idx(10).check_viewport_bounds(viewport_start, viewport_size)
                == RangeBoundsResult::Within
        );
        assert!(
            idx(15).check_viewport_bounds(viewport_start, viewport_size)
                == RangeBoundsResult::Within
        );
        assert!(
            idx(25).check_viewport_bounds(viewport_start, viewport_size)
                == RangeBoundsResult::Within
        );
        assert!(
            idx(29).check_viewport_bounds(viewport_start, viewport_size)
                == RangeBoundsResult::Within
        );
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
        assert!(
            idx(5).check_viewport_bounds(viewport_start, zero_size)
                != RangeBoundsResult::Within
        );
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
        assert!(
            idx(2).check_viewport_bounds(viewport_start, unit_size)
                != RangeBoundsResult::Within
        );
        assert!(
            idx(3).check_viewport_bounds(viewport_start, unit_size)
                == RangeBoundsResult::Within
        );
        assert!(
            idx(4).check_viewport_bounds(viewport_start, unit_size)
                != RangeBoundsResult::Within
        );
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
        assert!(
            idx(0).check_viewport_bounds(viewport_start, viewport_size)
                == RangeBoundsResult::Within
        );
        assert!(
            idx(4).check_viewport_bounds(viewport_start, viewport_size)
                == RangeBoundsResult::Within
        );
        assert!(
            idx(5).check_viewport_bounds(viewport_start, viewport_size)
                != RangeBoundsResult::Within
        );
    }
}
