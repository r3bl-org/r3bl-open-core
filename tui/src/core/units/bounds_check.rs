// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Bounds checking utilities for terminal user interface index validation.
//!
//! This module provides a comprehensive system for validating index positions against
//! various bounds, specifically designed for TUI applications where precise position
//! validation is crucial for rendering and user interaction.
//!
//! # Core Concepts
//!
//! The module implements two distinct paradigms for bounds checking:
//!
//! ## Array-Style Bounds Checking (`check_overflows`)
//!
//! Traditional array bounds checking where an index is valid if it's less than the
//! maximum length. Returns [`Within`](BoundsOverflowStatus::Within) for safe access or
//! [`Overflowed`](BoundsOverflowStatus::Overflowed) when bounds are exceeded.
//!
//! ## Content Position Checking (`check_content_position`)
//!
//! Content-aware position checking essential for text editing and cursor positioning.
//! Returns [`ContentPositionStatus`] variants indicating the relationship between an
//! index and content boundaries, including start, within, end, and beyond positions.
//!
//! # Type System
//!
//! The bounds checking system is built around two main type categories that ensure
//! type safety and prevent incorrect comparisons:
//!
//! ## Index Types (0-based position indicators)
//!
//! Types that implement [`IndexMarker`] represent positions within content, starting
//! from 0:
//! - [`Index`] - Generic 0-based position
//! - [`RowIndex`] - Row position in a terminal grid
//! - [`ColIndex`] - Column position in a terminal grid
//!
//! ## Length Types (1-based size measurements)
//!
//! Types that implement [`LengthMarker`] represent sizes or extents, starting from 1:
//! - [`Length`] - Generic 1-based size
//! - [`RowHeight`] - Height of terminal content
//! - [`ColWidth`] - Width of terminal content
//!
//! ## Type Safety Guarantees
//!
//! The trait system enforces several important constraints:
//! - Only index types can be bounds-checked against length types
//! - Each length type has a corresponding index type via [`LengthMarker::IndexType`]
//! - Automatic conversion between compatible types via [`LengthMarker::convert_to_index`]
//! - Prevents accidental comparisons between incompatible types (e.g., row vs column)
//!
//! # Key Components
//!
//! - [`BoundsCheck`] trait: Core functionality for both checking paradigms
//! - [`BoundsOverflowStatus`] enum: Results for array-style bounds checking
//! - [`ContentPositionStatus`] enum: Results for content position checking
//! - [`LengthMarker::is_overflowed_by`] method: Convenient overflow checking from length
//!   perspective ("Does this length get overflowed by this index?")
//! - [`IndexMarker::overflows`] method: Convenient overflow checking from index
//!   perspective ("Does this index overflow this length?")
//!
//! ## Implementations
//!
//! The module provides a single generic implementation of [`BoundsCheck`] that works
//! with any index type implementing [`IndexMarker`] and any length type implementing
//! [`LengthMarker`]. This eliminates code duplication and ensures consistent behavior
//! across all unit types.
//!
//! Individual types implement the required marker traits in their respective modules:
//! - [`UnitCompare`] - Enables numeric conversions for comparison operations
//! - [`IndexMarker`] - Identifies 0-based position types
//! - [`LengthMarker`] - Identifies 1-based size types with index correspondence
//!
//! # Usage Examples
//!
//! ```
//! use r3bl_tui::{BoundsCheck, ContentPositionStatus, idx, len};
//!
//! let content_length = len(10);
//! let cursor_pos = idx(8);
//!
//! // Content position checking for text editing
//! match cursor_pos.check_content_position(content_length) {
//!     ContentPositionStatus::AtStart => println!("Cursor at start"),
//!     ContentPositionStatus::Within => println!("Cursor on content"),
//!     ContentPositionStatus::AtEnd => println!("Cursor at end"),
//!     ContentPositionStatus::Beyond => println!("Invalid position"),
//! }
//!
//! // Array-style overflow checking - two equivalent approaches:
//!
//! // Approach 1: Length perspective - "Does this length get overflowed by this index?"
//! if !content_length.is_overflowed_by(cursor_pos) {
//!     // Safe to access content[cursor_pos]
//! }
//!
//! // Approach 2: Index perspective - "Does this index overflow this length?"
//! if !cursor_pos.overflows(content_length) {
//!     // Safe to access content[cursor_pos]
//! }
//! ```
//!
//! [`RowIndex`]: crate::RowIndex
//! [`ColIndex`]: crate::ColIndex
//! [`RowHeight`]: crate::RowHeight
//! [`ColWidth`]: crate::ColWidth
//! [`Index`]: crate::Index
//! [`Length`]: crate::Length
//! [`dimens`]: crate::dimens

use std::ops::Sub;

use crate::{Length, len};

/// Result of array-style bounds checking operations.
///
/// Used with [`BoundsCheck::check_overflows`] to determine if an index can safely
/// access array content. See the [module documentation](self) for details on the
/// bounds checking paradigms.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{BoundsCheck, BoundsOverflowStatus, idx, len};
///
/// let index = idx(5);
/// let length = len(10);
/// assert_eq!(index.check_overflows(length), BoundsOverflowStatus::Within);
///
/// let large_index = idx(10);
/// assert_eq!(large_index.check_overflows(length), BoundsOverflowStatus::Overflowed);
/// ```
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BoundsOverflowStatus {
    /// Indicates that an index is within the bounds of a length.
    Within,
    /// Indicates that an index has overflowed the bounds of a length.
    Overflowed,
}

/// Concise macro for array-style bounds overflow checking.
///
/// Returns `true` if the index overflows the bounds (`index >= max`), `false` if within
/// bounds. Equivalent to `index.check_overflows(max) ==
/// BoundsOverflowStatus::Overflowed`.
///
/// See the [module documentation](self) for the difference between overflow checking
/// and content position checking.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{idx, len};
///
/// if len(10).is_overflowed_by(idx(5)) {
///     // Handle overflow case
/// }
///
/// // More convenient than verbose syntax:
/// # use r3bl_tui::{BoundsCheck, BoundsOverflowStatus, ColIndex, ColWidth};
/// # let index = ColIndex::new(5);
/// # let width = ColWidth::new(10);
/// // if index.check_overflows(width) == BoundsOverflowStatus::Overflowed { ... }
/// if width.is_overflowed_by(index) { /* ... */ }
/// ```
/// Core trait for unit comparison operations.
///
/// Provides standardized methods to convert unit types to common numeric types
/// for comparison operations. This trait enables generic implementations of
/// bounds checking across different unit types.
pub trait UnitCompare {
    /// Convert the unit to a usize value for numeric comparison, usually for array
    /// indexing operations.
    fn as_usize(&self) -> usize;

    /// Convert the unit to a u16 value for crossterm compatibility and other terminal and
    /// pty based operations.
    fn as_u16(&self) -> u16;
}

/// Marker trait for index-type units (0-based position indicators).
///
/// This trait identifies types that represent positions or indices within
/// content, such as `RowIndex`, `ColIndex`, and `Index`. These are 0-based
/// values where the first position is index 0.
///
/// Each index type has a corresponding length type via [`LengthType`](Self::LengthType),
/// enabling safe bounds checking operations in both directions.
///
/// See the [module documentation](self) "Type System" section for details on
/// how index types relate to length types and the type safety guarantees.
pub trait IndexMarker: UnitCompare {
    /// The corresponding length type for this index type.
    ///
    /// The constraint `LengthMarker<IndexType = Self>` creates a bidirectional
    /// relationship: this ensures that the length type's `IndexType` points back to
    /// this same index type, preventing type mismatches like `ColIndex` ↔
    /// `RowHeight`.
    type LengthType: LengthMarker<IndexType = Self>;

    /// Convert this index to the corresponding length type.
    ///
    /// This typically involves adding 1 to the index value since
    /// indices are 0-based and lengths are 1-based.
    ///
    /// # Visual Example
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
    /// Returns true if the index is greater than or equal to the length.
    ///
    /// This is the inverse of [`LengthMarker::is_overflowed_by`] and provides
    /// a natural way to express bounds checking from the index's perspective.
    ///
    /// # Visual Example
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
    ///           ├────────── within bounds ──────────────┼─ overflow ┘
    ///           └────────── length=10 (1-based) ────────┘
    ///
    /// overflows(length=10) = true (index 10 overflows length 10)
    /// overflows(length=5)  = false (index 5 within length 10)
    /// ```
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{col, width};
    ///
    /// let index = col(10);
    /// let max_width = width(10);
    /// assert!(index.overflows(max_width));  // At boundary - overflows
    ///
    /// let smaller_index = col(5);
    /// assert!(!smaller_index.overflows(max_width));  // Within bounds
    /// ```
    fn overflows(&self, length: Self::LengthType) -> bool
    where
        Self: PartialOrd + Sized + Copy,
    {
        length.is_overflowed_by(*self)
    }
}

/// Marker trait for length-type units (1-based size measurements).
///
/// This trait identifies types that represent sizes or lengths of content,
/// such as `RowHeight`, `ColWidth`, and `Length`. These are 1-based values
/// where a length of 1 means "one unit of size".
///
/// Each length type has a corresponding index type via [`IndexType`](Self::IndexType),
/// enabling safe bounds checking operations.
///
/// See the [module documentation](self) "Type System" section for details on
/// how length types relate to index types and the type safety guarantees.
pub trait LengthMarker: UnitCompare {
    /// The corresponding index type for this length type.
    ///
    /// The constraint `IndexMarker<LengthType = Self>` creates a bidirectional
    /// relationship: this ensures that the index type's `LengthType` points back to
    /// this same length type, preventing type mismatches like `ColWidth` ↔
    /// `RowIndex`.
    type IndexType: IndexMarker<LengthType = Self>;

    /// Convert this length to the corresponding index type.
    ///
    /// This typically involves subtracting 1 from the length value since
    /// lengths are 1-based and indices are 0-based.
    ///
    /// # Visual Example
    ///
    /// ```text
    /// Length=10 to index conversion:
    ///           ┌────────── length=10 (1-based) ────────┐
    /// Length:     1   2   3   4   5   6   7   8   9   10
    /// (1-based) ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///           │   │   │   │   │   │   │   │   │   │ ␩ │
    ///           └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
    /// Index:      0   1   2   3   4   5   6   7   8   9
    /// (0-based)                                       ↑
    ///                                         convert_to_index() = 9
    /// ```
    fn convert_to_index(&self) -> Self::IndexType;

    /// Answers the question: "Does this length get overflowed by this index?"
    ///
    /// Check if the given index would overflow this length's bounds.
    /// Returns true if the index is greater than or equal to the length.
    ///
    /// # Visual Example
    ///
    /// ```text
    /// Checking overflow for length=10:
    ///
    ///                                             boundary
    ///                                                 │
    /// Index:    0   1   2   3   4   5   6   7   8   9 │ 10  11  12
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┼───┬───┬───┐
    ///         │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✗ │ ✗ │ ✗ │
    ///         ├───┴───┴───┴───┴───┴───┴───┴───┴───┴───┼───┴───┴───┤
    ///         ├────────── valid indices ──────────────┼─ overflow ┘
    ///         └────────── length=10 (1-based) ────────┘
    ///
    /// is_overflowed_by(5)  = false (within bounds)
    /// is_overflowed_by(9)  = false (last valid index)
    /// is_overflowed_by(10) = true (at boundary)
    /// is_overflowed_by(11) = true (beyond boundary)
    /// ```
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{col, width};
    ///
    /// let max_col = width(10);
    /// assert!(!max_col.is_overflowed_by(col(5)));  // Within bounds
    /// assert!(max_col.is_overflowed_by(col(10)));  // At boundary - overflows
    /// assert!(max_col.is_overflowed_by(col(15)));  // Beyond boundary
    /// ```
    fn is_overflowed_by(&self, index: Self::IndexType) -> bool
    where
        Self::IndexType: PartialOrd,
    {
        // Special case: empty collection (length 0) has no valid indices
        if self.as_usize() == 0 {
            return true;
        }
        index > self.convert_to_index()
    }

    /// Calculate the remaining space from the given index to the end of this length.
    ///
    /// Returns the number of units between the index and the boundary defined by this
    /// length. For example, if this is a ColWidth of 10 and the index is at column 3,
    /// this returns a Length of 7 (columns 3-9, inclusive).
    ///
    /// Returns Length(0) if the index is at or beyond the boundary.
    ///
    /// # Visual Example
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
    /// # Examples
    /// ```
    /// use r3bl_tui::{col, width, len};
    ///
    /// let max_width = width(10);
    /// assert_eq!(max_width.remaining_from(col(3)), len(7));  // 7 columns remain
    /// assert_eq!(max_width.remaining_from(col(10)), len(0)); // At boundary
    /// assert_eq!(max_width.remaining_from(col(15)), len(0)); // Beyond boundary
    /// ```
    fn remaining_from(&self, index: Self::IndexType) -> Length
    where
        Self::IndexType: PartialOrd + Sub<Output = Self::IndexType> + Copy,
        <Self::IndexType as IndexMarker>::LengthType: Into<Length>,
    {
        if self.is_overflowed_by(index) {
            len(0)
        } else {
            // Get max index for this length
            let max_index = self.convert_to_index();
            // Calculate num of chars from cursor to boundary (as index difference)
            let chars_remaining_as_index = max_index - index;
            // Convert from 0-based index difference to 1-based length
            chars_remaining_as_index.convert_to_length().into()
        }
    }
}

/// Core trait for index bounds validation in TUI applications.
///
/// Provides both array-style bounds checking and content position checking.
/// See the [module documentation](self) for detailed explanations of both paradigms.
///
/// This trait is now generic over length types that implement `LengthMarker`,
/// and can only be implemented by index types that implement `IndexMarker`.
/// This ensures type safety and prevents incorrect comparisons between incompatible
/// types.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{BoundsCheck, BoundsOverflowStatus, RowIndex, RowHeight};
///
/// let row_index = RowIndex::new(5);
/// let height = RowHeight::new(5);
/// assert_eq!(row_index.check_overflows(height), BoundsOverflowStatus::Overflowed);
/// ```
pub trait BoundsCheck<LengthType: LengthMarker>
where
    Self: IndexMarker,
{
    /// Performs array-style bounds checking.
    ///
    /// Returns `BoundsOverflowStatus::Within` if the index can safely access content,
    /// `BoundsOverflowStatus::Overflowed` if the index would exceed array bounds.
    ///
    /// See the [module documentation](self) for detailed explanation of bounds checking.
    ///
    /// # Visual Example
    ///
    /// ```text
    /// Array-style bounds checking:
    ///
    ///                           index=5 (0-based)   index=10 (0-based)
    ///                                 ↓                   ↓
    /// Index:      0   1   2   3   4   5   6   7   8   9 │ 10  11  12
    /// (1-based) ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┼───┬───┬───┐
    ///           │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ × │ × │ × │
    ///           ├───┴───┴───┴───┴───┴───┴───┴───┴───┴───┼───┴───┴───┤
    ///           ├────────── within bounds ──────────────┼─ overflow ┘
    ///           └────────── length=10 (1-based) ────────┘
    ///
    /// check_overflows(length=5)  = Within
    /// check_overflows(length=10) = Overflowed
    /// ```
    fn check_overflows(&self, max: LengthType) -> BoundsOverflowStatus;

    /// Performs content position checking.
    ///
    /// Returns `ContentPositionStatus` indicating whether the index is within content,
    /// at a content boundary, or beyond content boundaries.
    ///
    /// See the [module documentation](self) for detailed explanation of content position
    /// checking.
    ///
    /// # Visual Example
    ///
    /// ```text
    /// Content position checking:
    ///
    /// Self
    /// Index:      0   1   2   3   4   5   6   7   8   9   10  11
    /// (0-based) ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///           │ S │ W │ W │ W │ W │ W │ W │ W │ W │ W │ E │ B │
    ///           ├─▲─┴─▲─┴───┴───┴───┴───┴───┴───┴───┴─▲─┴─▲─┼─▲─┘
    ///           │ │   │                               │   │ │ │
    ///           │Start│                               │  End│Beyond
    ///           │     └────────── Within ─────────────┘     │
    ///           └───────────── content_length=10 ───────────┘
    ///
    /// S = AtStart (index=0)
    /// W = Within (1 ≤ index < 10)
    /// E = AtEnd (index=10)
    /// B = Beyond (index > 10)
    /// ```
    fn check_content_position(&self, content_length: LengthType)
    -> ContentPositionStatus;
}

/// Generic implementation of [`BoundsCheck`] for any [`IndexMarker`] type with
/// [`LengthMarker`] type.
///
/// This single implementation works with all index and length types that implement the
/// required marker traits, eliminating code duplication and ensuring consistent behavior.
/// The trait system guarantees type safety by only allowing compatible index-length
/// pairs.
impl<IndexType, LengthType> BoundsCheck<LengthType> for IndexType
where
    IndexType: IndexMarker + PartialOrd + Copy,
    LengthType: LengthMarker<IndexType = IndexType>,
{
    fn check_overflows(&self, length: LengthType) -> BoundsOverflowStatus {
        let this = *self;
        let other = length.convert_to_index();
        if this > other {
            BoundsOverflowStatus::Overflowed
        } else {
            BoundsOverflowStatus::Within
        }
    }

    fn check_content_position(
        &self,
        content_length: LengthType,
    ) -> ContentPositionStatus {
        let position = self.as_usize();
        let length = content_length.as_usize();

        if position > length {
            ContentPositionStatus::Beyond
        } else if position == 0 {
            ContentPositionStatus::AtStart
        } else if position == length {
            ContentPositionStatus::AtEnd
        } else {
            ContentPositionStatus::Within
        }
    }
}

/// Result of content position checking operations.
///
/// Used with [`BoundsCheck::check_content_position`] to determine the relationship
/// between an index and content boundaries. Essential for text editing and cursor
/// positioning where boundary conditions require different handling.
///
/// See the [module documentation](self) for detailed explanation of content position
/// checking and use cases for each variant.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{BoundsCheck, ContentPositionStatus, idx, len};
///
/// let content_length = len(5);
///
/// assert_eq!(idx(0).check_content_position(content_length), ContentPositionStatus::AtStart);
/// assert_eq!(idx(3).check_content_position(content_length), ContentPositionStatus::Within);
/// assert_eq!(idx(5).check_content_position(content_length), ContentPositionStatus::AtEnd);
/// assert_eq!(idx(7).check_content_position(content_length), ContentPositionStatus::Beyond);
/// ```
///
/// ## Why [`RowContentPositionStatus`](crate::RowContentPositionStatus) exists separate from this enum
///
/// [`crate::RowContentPositionStatus`] has different semantics for handling row positions
/// in a terminal buffer, which is why it does not use this enum.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ContentPositionStatus {
    /// Index is at the start of content (`index == 0`). For empty content, this takes
    /// precedence over `AtEnd`.
    AtStart,

    /// Index points to existing content (`0 < index < length`).
    Within,

    /// Index is at the content end boundary (`index == length && index > 0`), valid for
    /// cursor/insertion.
    AtEnd,

    /// Index exceeds content boundaries (`index > length`), requires correction.
    Beyond,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColIndex, ColWidth, RowHeight, RowIndex, idx};

    #[test]
    fn test_bounds_overflow_status_equality() {
        assert_eq!(BoundsOverflowStatus::Within, BoundsOverflowStatus::Within);
        assert_eq!(
            BoundsOverflowStatus::Overflowed,
            BoundsOverflowStatus::Overflowed
        );
        assert_ne!(
            BoundsOverflowStatus::Within,
            BoundsOverflowStatus::Overflowed
        );
    }

    #[test]
    fn test_bounds_overflow_status_copy() {
        let status1 = BoundsOverflowStatus::Within;
        let status2 = status1;
        assert_eq!(status1, status2);

        let status3 = BoundsOverflowStatus::Overflowed;
        let status4 = status3;
        assert_eq!(status3, status4);
    }

    #[test]
    fn test_bounds_overflow_status_debug() {
        assert_eq!(format!("{:?}", BoundsOverflowStatus::Within), "Within");
        assert_eq!(
            format!("{:?}", BoundsOverflowStatus::Overflowed),
            "Overflowed"
        );
    }

    #[test]
    fn test_position_status_equality() {
        assert_eq!(
            ContentPositionStatus::AtStart,
            ContentPositionStatus::AtStart
        );
        assert_eq!(ContentPositionStatus::Within, ContentPositionStatus::Within);
        assert_eq!(ContentPositionStatus::AtEnd, ContentPositionStatus::AtEnd);
        assert_eq!(ContentPositionStatus::Beyond, ContentPositionStatus::Beyond);
        assert_ne!(
            ContentPositionStatus::AtStart,
            ContentPositionStatus::Within
        );
        assert_ne!(ContentPositionStatus::Within, ContentPositionStatus::AtEnd);
        assert_ne!(ContentPositionStatus::AtEnd, ContentPositionStatus::Beyond);
        assert_ne!(
            ContentPositionStatus::AtStart,
            ContentPositionStatus::Beyond
        );
    }

    #[test]
    fn test_position_status_copy() {
        let status1 = ContentPositionStatus::AtStart;
        let status2 = status1;
        assert_eq!(status1, status2);

        let status3 = ContentPositionStatus::Within;
        let status4 = status3;
        assert_eq!(status3, status4);

        let status5 = ContentPositionStatus::AtEnd;
        let status6 = status5;
        assert_eq!(status5, status6);

        let status7 = ContentPositionStatus::Beyond;
        let status8 = status7;
        assert_eq!(status7, status8);
    }

    #[test]
    fn test_position_status_debug() {
        assert_eq!(format!("{:?}", ContentPositionStatus::AtStart), "AtStart");
        assert_eq!(format!("{:?}", ContentPositionStatus::Within), "Within");
        assert_eq!(format!("{:?}", ContentPositionStatus::AtEnd), "AtEnd");
        assert_eq!(format!("{:?}", ContentPositionStatus::Beyond), "Beyond");
    }

    #[test]
    fn test_is_overflowed_by() {
        // Test basic cases with Index/Length
        assert!(!len(3).is_overflowed_by(idx(1)), "Within bounds");
        assert!(len(3).is_overflowed_by(idx(3)), "At boundary");
        assert!(len(3).is_overflowed_by(idx(5)), "Beyond bounds");
        assert!(
            len(0).is_overflowed_by(idx(0)),
            "Empty collection edge case"
        );

        // Test with typed dimensions
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

        // Verify method matches existing check_overflows behavior
        let test_cases = [(0, 1), (1, 1), (5, 10), (10, 10)];
        for (index_val, length_val) in test_cases {
            let index = idx(index_val);
            let length = len(length_val);
            assert_eq!(
                length.is_overflowed_by(index),
                index.check_overflows(length) == BoundsOverflowStatus::Overflowed,
                "New method should match existing behavior for index {index_val} and length {length_val}"
            );
        }
    }

    #[test]
    fn test_overflows() {
        // Test basic cases with Index/Length - should mirror is_overflowed_by results
        assert!(!idx(1).overflows(len(3)), "Within bounds");
        assert!(idx(3).overflows(len(3)), "At boundary");
        assert!(idx(5).overflows(len(3)), "Beyond bounds");
        assert!(idx(0).overflows(len(0)), "Empty collection edge case");

        // Test with typed dimensions
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

        // Test with specific typed combinations
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
    fn test_check_content_position_basic() {
        let content_length = len(5);

        // At start
        assert_eq!(
            idx(0).check_content_position(content_length),
            ContentPositionStatus::AtStart
        );

        // Within content
        assert_eq!(
            idx(2).check_content_position(content_length),
            ContentPositionStatus::Within
        );
        assert_eq!(
            idx(4).check_content_position(content_length),
            ContentPositionStatus::Within
        );

        // At end boundary
        assert_eq!(
            idx(5).check_content_position(content_length),
            ContentPositionStatus::AtEnd
        );

        // Beyond content
        assert_eq!(
            idx(6).check_content_position(content_length),
            ContentPositionStatus::Beyond
        );
        assert_eq!(
            idx(10).check_content_position(content_length),
            ContentPositionStatus::Beyond
        );
    }

    #[test]
    fn test_check_content_position_edge_cases() {
        // Zero-length content - AtStart takes precedence
        let zero_length = len(0);
        assert_eq!(
            idx(0).check_content_position(zero_length),
            ContentPositionStatus::AtStart
        );
        assert_eq!(
            idx(1).check_content_position(zero_length),
            ContentPositionStatus::Beyond
        );

        // Single element content
        let single_length = len(1);
        assert_eq!(
            idx(0).check_content_position(single_length),
            ContentPositionStatus::AtStart
        );
        assert_eq!(
            idx(1).check_content_position(single_length),
            ContentPositionStatus::AtEnd
        );
        assert_eq!(
            idx(2).check_content_position(single_length),
            ContentPositionStatus::Beyond
        );
    }

    #[test]
    fn test_check_content_position_with_typed_indices() {
        // Test with ColIndex/ColWidth
        let col_width = ColWidth::new(3);
        assert_eq!(
            ColIndex::new(0).check_content_position(col_width),
            ContentPositionStatus::AtStart
        );
        assert_eq!(
            ColIndex::new(2).check_content_position(col_width),
            ContentPositionStatus::Within
        );
        assert_eq!(
            ColIndex::new(3).check_content_position(col_width),
            ContentPositionStatus::AtEnd
        );
        assert_eq!(
            ColIndex::new(4).check_content_position(col_width),
            ContentPositionStatus::Beyond
        );

        // Test with RowIndex/RowHeight
        let row_height = RowHeight::new(2);
        assert_eq!(
            RowIndex::new(0).check_content_position(row_height),
            ContentPositionStatus::AtStart
        );
        assert_eq!(
            RowIndex::new(1).check_content_position(row_height),
            ContentPositionStatus::Within
        );
        assert_eq!(
            RowIndex::new(2).check_content_position(row_height),
            ContentPositionStatus::AtEnd
        );
        assert_eq!(
            RowIndex::new(3).check_content_position(row_height),
            ContentPositionStatus::Beyond
        );

        // Test with Index/Index removed as this is no longer supported.
        // BoundsCheck is now specifically for Index-to-Length comparisons only.
    }

    #[test]
    fn test_position_status_empty_content_precedence() {
        use super::*;

        // Test that AtStart takes precedence over AtEnd for empty content
        let empty_length = len(0);
        assert_eq!(
            idx(0).check_content_position(empty_length),
            ContentPositionStatus::AtStart
        );

        // Test with typed indices too

        let empty_col_width = ColWidth::new(0);
        assert_eq!(
            ColIndex::new(0).check_content_position(empty_col_width),
            ContentPositionStatus::AtStart
        );

        let empty_row_height = RowHeight::new(0);
        assert_eq!(
            RowIndex::new(0).check_content_position(empty_row_height),
            ContentPositionStatus::AtStart
        );
    }

    #[test]
    fn test_position_status_comprehensive() {
        // Test all combinations for a length-3 content
        let content_length = len(3);

        // AtStart: index == 0
        assert_eq!(
            idx(0).check_content_position(content_length),
            ContentPositionStatus::AtStart
        );

        // Within: 0 < index < length
        assert_eq!(
            idx(1).check_content_position(content_length),
            ContentPositionStatus::Within
        );
        assert_eq!(
            idx(2).check_content_position(content_length),
            ContentPositionStatus::Within
        );

        // AtEnd: index == length && index > 0
        assert_eq!(
            idx(3).check_content_position(content_length),
            ContentPositionStatus::AtEnd
        );

        // Beyond: index > length
        assert_eq!(
            idx(4).check_content_position(content_length),
            ContentPositionStatus::Beyond
        );
        assert_eq!(
            idx(10).check_content_position(content_length),
            ContentPositionStatus::Beyond
        );
    }

    #[test]
    fn test_remaining_from() {
        // Test basic cases with Length/Index
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

        // Test edge case: empty length
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

        // Test with typed dimensions - ColWidth/ColIndex
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

        // Test with typed dimensions - RowHeight/RowIndex
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

        // Test single element case
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

        // Test specific examples from documentation
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

    #[test]
    fn test_convert_to_length() {
        // Test basic index to length conversion (0-based to 1-based)
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

        // Test with typed dimensions - ColIndex to ColWidth
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

        // Test with typed dimensions - RowIndex to RowHeight
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

        // Test that the conversion is consistent - converting back should work
        let original_index = idx(42);
        let converted_length = original_index.convert_to_length();
        let back_to_index = converted_length.convert_to_index();
        assert_eq!(
            back_to_index, original_index,
            "Round-trip conversion should be consistent"
        );

        // Test with typed round-trip conversions
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
        // Test basic length to index conversion (1-based to 0-based)
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

        // Test with typed dimensions - ColWidth to ColIndex
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

        // Test with typed dimensions - RowHeight to RowIndex
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

        // Test that the conversion is consistent - converting back should work
        let original_length = len(42);
        let converted_index = original_length.convert_to_index();
        let back_to_length = converted_index.convert_to_length();
        assert_eq!(
            back_to_length, original_length,
            "Round-trip conversion should be consistent"
        );

        // Test with typed round-trip conversions
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
        // implemented but if it is, it should be consistent with the type system
        // Note: Length 0 might be a special case that needs separate handling
    }

    #[test]
    fn test_as_usize() {
        // Test basic index types conversion to usize
        assert_eq!(idx(0).as_usize(), 0, "Index 0 as usize");
        assert_eq!(idx(5).as_usize(), 5, "Index 5 as usize");
        assert_eq!(idx(100).as_usize(), 100, "Index 100 as usize");
        assert_eq!(idx(999).as_usize(), 999, "Index 999 as usize");

        // Test basic length types conversion to usize
        assert_eq!(len(1).as_usize(), 1, "Length 1 as usize");
        assert_eq!(len(6).as_usize(), 6, "Length 6 as usize");
        assert_eq!(len(10).as_usize(), 10, "Length 10 as usize");
        assert_eq!(len(1000).as_usize(), 1000, "Length 1000 as usize");

        // Test typed index conversions - ColIndex
        assert_eq!(ColIndex::new(0).as_usize(), 0, "ColIndex 0 as usize");
        assert_eq!(ColIndex::new(5).as_usize(), 5, "ColIndex 5 as usize");
        assert_eq!(ColIndex::new(80).as_usize(), 80, "ColIndex 80 as usize");
        assert_eq!(
            ColIndex::new(1024).as_usize(),
            1024,
            "ColIndex 1024 as usize"
        );

        // Test typed index conversions - RowIndex
        assert_eq!(RowIndex::new(0).as_usize(), 0, "RowIndex 0 as usize");
        assert_eq!(RowIndex::new(3).as_usize(), 3, "RowIndex 3 as usize");
        assert_eq!(RowIndex::new(25).as_usize(), 25, "RowIndex 25 as usize");
        assert_eq!(RowIndex::new(768).as_usize(), 768, "RowIndex 768 as usize");

        // Test typed length conversions - ColWidth
        assert_eq!(ColWidth::new(1).as_usize(), 1, "ColWidth 1 as usize");
        assert_eq!(ColWidth::new(10).as_usize(), 10, "ColWidth 10 as usize");
        assert_eq!(ColWidth::new(80).as_usize(), 80, "ColWidth 80 as usize");
        assert_eq!(
            ColWidth::new(1920).as_usize(),
            1920,
            "ColWidth 1920 as usize"
        );

        // Test typed length conversions - RowHeight
        assert_eq!(RowHeight::new(1).as_usize(), 1, "RowHeight 1 as usize");
        assert_eq!(RowHeight::new(5).as_usize(), 5, "RowHeight 5 as usize");
        assert_eq!(RowHeight::new(30).as_usize(), 30, "RowHeight 30 as usize");
        assert_eq!(
            RowHeight::new(1080).as_usize(),
            1080,
            "RowHeight 1080 as usize"
        );

        // Test edge cases
        assert_eq!(len(0).as_usize(), 0, "Length 0 as usize");
        assert_eq!(ColWidth::new(0).as_usize(), 0, "ColWidth 0 as usize");
        assert_eq!(RowHeight::new(0).as_usize(), 0, "RowHeight 0 as usize");

        // Test that as_usize preserves the underlying numeric value
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
    fn test_as_u16() {
        // Test basic index types conversion to u16
        assert_eq!(idx(0).as_u16(), 0, "Index 0 as u16");
        assert_eq!(idx(5).as_u16(), 5, "Index 5 as u16");
        assert_eq!(idx(100).as_u16(), 100, "Index 100 as u16");
        assert_eq!(idx(999).as_u16(), 999, "Index 999 as u16");

        // Test basic length types conversion to u16
        assert_eq!(len(1).as_u16(), 1, "Length 1 as u16");
        assert_eq!(len(6).as_u16(), 6, "Length 6 as u16");
        assert_eq!(len(10).as_u16(), 10, "Length 10 as u16");
        assert_eq!(len(1000).as_u16(), 1000, "Length 1000 as u16");

        // Test typed index conversions - ColIndex
        assert_eq!(ColIndex::new(0).as_u16(), 0, "ColIndex 0 as u16");
        assert_eq!(ColIndex::new(5).as_u16(), 5, "ColIndex 5 as u16");
        assert_eq!(ColIndex::new(80).as_u16(), 80, "ColIndex 80 as u16");
        assert_eq!(ColIndex::new(1024).as_u16(), 1024, "ColIndex 1024 as u16");

        // Test typed index conversions - RowIndex
        assert_eq!(RowIndex::new(0).as_u16(), 0, "RowIndex 0 as u16");
        assert_eq!(RowIndex::new(3).as_u16(), 3, "RowIndex 3 as u16");
        assert_eq!(RowIndex::new(25).as_u16(), 25, "RowIndex 25 as u16");
        assert_eq!(RowIndex::new(768).as_u16(), 768, "RowIndex 768 as u16");

        // Test typed length conversions - ColWidth
        assert_eq!(ColWidth::new(1).as_u16(), 1, "ColWidth 1 as u16");
        assert_eq!(ColWidth::new(10).as_u16(), 10, "ColWidth 10 as u16");
        assert_eq!(ColWidth::new(80).as_u16(), 80, "ColWidth 80 as u16");
        assert_eq!(ColWidth::new(1920).as_u16(), 1920, "ColWidth 1920 as u16");

        // Test typed length conversions - RowHeight
        assert_eq!(RowHeight::new(1).as_u16(), 1, "RowHeight 1 as u16");
        assert_eq!(RowHeight::new(5).as_u16(), 5, "RowHeight 5 as u16");
        assert_eq!(RowHeight::new(30).as_u16(), 30, "RowHeight 30 as u16");
        assert_eq!(RowHeight::new(1080).as_u16(), 1080, "RowHeight 1080 as u16");

        // Test edge cases
        assert_eq!(len(0).as_u16(), 0, "Length 0 as u16");
        assert_eq!(ColWidth::new(0).as_u16(), 0, "ColWidth 0 as u16");
        assert_eq!(RowHeight::new(0).as_u16(), 0, "RowHeight 0 as u16");

        // Test terminal-typical values (crossterm compatibility)
        assert_eq!(ColWidth::new(80).as_u16(), 80, "Standard terminal width 80");
        assert_eq!(ColWidth::new(120).as_u16(), 120, "Wide terminal width 120");
        assert_eq!(
            RowHeight::new(24).as_u16(),
            24,
            "Standard terminal height 24"
        );
        assert_eq!(RowHeight::new(50).as_u16(), 50, "Tall terminal height 50");

        // Test u16 max boundary (65535)
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

        // Test that as_u16 preserves the underlying numeric value for typical ranges
        for value in [0, 1, 5, 10, 42, 80, 100, 120, 1024] {
            assert_eq!(
                idx(value).as_u16(),
                value as u16,
                "Index {value} preserves value"
            );
            assert_eq!(
                len(value).as_u16(),
                value as u16,
                "Length {value} preserves value"
            );
            assert_eq!(
                ColIndex::new(value).as_u16(),
                value as u16,
                "ColIndex {value} preserves value"
            );
            assert_eq!(
                ColWidth::new(value).as_u16(),
                value as u16,
                "ColWidth {value} preserves value"
            );
            assert_eq!(
                RowIndex::new(value).as_u16(),
                value as u16,
                "RowIndex {value} preserves value"
            );
            assert_eq!(
                RowHeight::new(value).as_u16(),
                value as u16,
                "RowHeight {value} preserves value"
            );
        }
    }
}
