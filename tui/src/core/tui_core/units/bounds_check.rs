/*
 *   Copyright (c) 2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

/// Represents the result of a bounds check operation.
///
/// This enum is used to indicate whether an index is within the bounds of a length
/// or another index, or if it has overflowed those bounds.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{BoundsCheck, BoundsStatus, Index, Length, idx, len};
///
/// let index = idx(5);
/// let length = len(10);
///
/// // Check if the index is within the bounds of the length
/// let status = index.check_overflows(length);
/// assert_eq!(status, BoundsStatus::Within);
///
/// // Check if an index that exceeds the length is overflowed
/// let large_index = idx(10);
/// let status = large_index.check_overflows(length);
/// assert_eq!(status, BoundsStatus::Overflowed);
/// ```
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BoundsStatus {
    /// Indicates that an index is within the bounds of a length or another index.
    Within,
    /// Indicates that an index has overflowed the bounds of a length or another index.
    Overflowed,
}

/// A macro for performing bounds checks with a concise syntax.
///
/// This macro automatically calls `len()` on the `$length` parameter before passing it to
/// `check_overflows`. If the check determines that the index has overflowed, the provided
/// block of code is executed.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{BoundsCheck, BoundsStatus, bounds_check, idx, len};
///
/// let index = idx(15);
/// let array = [1, 2, 3, 4, 5];
///
/// bounds_check!(index, array.len(), {
///     println!("Index {:?} overflows array length {:?}", index, array.len());
/// });
/// ```
#[macro_export]
macro_rules! bounds_check {
    ($index:expr, $length:expr, $overflow_handler:block) => {
        if $index.check_overflows($crate::len($length))
            == $crate::BoundsStatus::Overflowed
        {
            $overflow_handler
        }
    };
}

/// This trait "formalizes" the concept of checking for overflow. More specifically, when
/// an index (row or col index) overflows a length (width or height).
///
/// When `a` and `b` are both unsigned integers, the following are equivalent:
/// - `a >= b`
/// - `a > b-1`
///
/// So, the following expressions are equivalent:
/// - `row_index >= height`
/// - `row_index > height - 1`
///
/// # Examples
///
/// ```
/// use r3bl_tui::{
///     BoundsCheck, BoundsStatus,
///     RowHeight, RowIndex, ColIndex, ColWidth
/// };
///
/// let row_index = RowIndex::new(5);
/// let height = RowHeight::new(5);
/// assert_eq!(
///     row_index.check_overflows(height),
///     BoundsStatus::Overflowed
/// );
///
/// let col_index = ColIndex::new(3);
/// let width = ColWidth::new(5);
/// assert_eq!(
///     col_index.check_overflows(width),
///     BoundsStatus::Within
/// );
/// ```
pub trait BoundsCheck<OtherType> {
    /// Checks if this index overflows the given bounds.
    ///
    /// This method cleans up the expression doing the following manual comparison.
    /// Before this method, code like this was used: `col_index >= width`.
    /// - And: `a >= b` === `a > b-1`.
    /// - So: `col_index > width - 1`.
    ///
    /// Returns:
    /// - `BoundsStatus::Within` if the index is within the bounds
    /// - `BoundsStatus::Overflowed` if the index exceeds the bounds
    ///
    /// This method performs array-style bounds checking where an index is considered
    /// to overflow if it is greater than the maximum valid index (length - 1).
    ///
    /// # Examples
    ///
    /// ```
    /// use r3bl_tui::{BoundsCheck, BoundsStatus, idx, len};
    ///
    /// let index = idx(5);
    /// let length = len(10);
    /// assert_eq!(index.check_overflows(length), BoundsStatus::Within);
    ///
    /// let index = idx(10);
    /// let length = len(10);
    /// assert_eq!(index.check_overflows(length), BoundsStatus::Overflowed);
    /// ```
    fn check_overflows(&self, max: OtherType) -> BoundsStatus;

    /// Checks the position of this index relative to content length.
    ///
    /// This is different from array bounds checking, as it treats the length as content
    /// size rather than array capacity, where index == length is considered a
    /// boundary position rather than an overflow.
    ///
    /// Returns:
    /// - `PositionStatus::Within` if the index is less than the content length (valid for
    ///   content access)
    /// - `PositionStatus::Boundary` if the index equals the content length (at the end of
    ///   content)
    /// - `PositionStatus::Beyond` if the index is greater than the content length (past
    ///   the end of content)
    fn check_content_position(&self, content_length: OtherType) -> PositionStatus;
}

/// Represents the position status of an index relative to content boundaries.
///
/// This enum provides detailed information about where an index falls in relation
/// to content length, which is particularly useful for text editing, cursor positioning,
/// and content navigation scenarios where different boundary conditions require
/// different handling.
///
/// # Variants
///
/// - [`Within`](PositionStatus::Within) - Index points to a valid position inside the
///   content
/// - [`Boundary`](PositionStatus::Boundary) - Index is at the exact boundary (end) of
///   content
/// - [`Beyond`](PositionStatus::Beyond) - Index exceeds the content boundaries
///
/// # Examples
///
/// ```
/// use r3bl_tui::{BoundsCheck, PositionStatus, Index, Length};
///
/// let content_length = Length::new(5); // Content with 5 elements (indices 0-4)
///
/// // Within content - valid content positions
/// let index = Index::new(0);
/// assert_eq!(index.check_content_position(content_length), PositionStatus::Within);
///
/// let index = Index::new(3);
/// assert_eq!(index.check_content_position(content_length), PositionStatus::Within);
///
/// // At boundary - often valid for cursor positioning
/// let index = Index::new(5);
/// assert_eq!(index.check_content_position(content_length), PositionStatus::Boundary);
///
/// // Beyond content - invalid position
/// let index = Index::new(7);
/// assert_eq!(index.check_content_position(content_length), PositionStatus::Beyond);
/// ```
///
/// # Use Cases
///
/// ## Text Editor Cursor Positioning
/// - `Within`: Cursor is positioned on an existing character
/// - `Boundary`: Cursor is at the end of the line/content (valid for insertion)
/// - `Beyond`: Invalid cursor position that needs correction
///
/// ## Content Validation
/// - `Within`: Safe to access content at this index
/// - `Boundary`: Safe for append operations, but not for content access
/// - `Beyond`: Requires bounds checking or error handling
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PositionStatus {
    /// The index is within valid content boundaries.
    ///
    /// This indicates that the index points to an actual element within the content.
    /// For example, in a string of length 5, indices 0 through 4 would be `Within`.
    ///
    /// # Use Cases
    /// - Safe to access content at this position
    /// - Valid cursor position on existing content
    /// - Can perform read/write operations at this index
    Within,

    /// The index is exactly at the content boundary.
    ///
    /// This indicates that the index is equal to the content length, positioning
    /// it at the exact end of the content. While this is not a valid index for
    /// accessing existing content, it's often a valid position for operations
    /// like cursor placement or content insertion.
    ///
    /// # Use Cases
    /// - Valid cursor position at the end of content
    /// - Safe position for append/insert operations
    /// - Boundary condition that may need special handling
    ///
    /// # Examples
    /// For content of length 5, index 5 would be `Boundary`.
    Boundary,

    /// The index exceeds the content boundaries.
    ///
    /// This indicates that the index is greater than the content length,
    /// positioning it well beyond any valid content or boundary position.
    /// This typically represents an error condition or requires bounds
    /// correction.
    ///
    /// # Use Cases
    /// - Invalid position requiring error handling
    /// - Needs bounds checking or index clamping
    /// - May indicate a programming error or user input validation issue
    ///
    /// # Examples
    /// For content of length 5, any index greater than 5 would be `Beyond`.
    Beyond,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bounds_check, idx};

    #[test]
    fn test_bounds_status_equality() {
        assert_eq!(BoundsStatus::Within, BoundsStatus::Within);
        assert_eq!(BoundsStatus::Overflowed, BoundsStatus::Overflowed);
        assert_ne!(BoundsStatus::Within, BoundsStatus::Overflowed);
    }

    #[test]
    fn test_bounds_status_copy() {
        let status1 = BoundsStatus::Within;
        let status2 = status1;
        assert_eq!(status1, status2);

        let status3 = BoundsStatus::Overflowed;
        let status4 = status3;
        assert_eq!(status3, status4);
    }

    #[test]
    fn test_bounds_status_debug() {
        assert_eq!(format!("{:?}", BoundsStatus::Within), "Within");
        assert_eq!(format!("{:?}", BoundsStatus::Overflowed), "Overflowed");
    }

    #[test]
    fn test_bounds_check_macro() {
        let lines = ["line1", "line2", "line3"];

        // Test case: index within bounds
        {
            let line_index = idx(1);
            let mut executed = false;

            bounds_check!(line_index, lines.len(), {
                executed = true;
            });

            // Block should not execute when index is within bounds
            assert!(
                !executed,
                "Handler block should not execute when index is within bounds"
            );
        }

        // Test case: index at boundary (equal to length)
        {
            let line_index = idx(3); // lines.len() == 3
            let mut executed = false;

            bounds_check!(line_index, lines.len(), {
                executed = true;
            });

            // Block should execute when index equals length
            assert!(
                executed,
                "Handler block should execute when index equals length"
            );
        }

        // Test case: index beyond bounds
        {
            let line_index = idx(5);
            let mut executed = false;

            bounds_check!(line_index, lines.len(), {
                executed = true;
            });

            // Block should execute when index exceeds length
            assert!(
                executed,
                "Handler block should execute when index exceeds length"
            );
        }

        // Test case: return value from handler block
        {
            let line_index = idx(10);

            let result = (|| {
                bounds_check!(line_index, lines.len(), {
                    return "Overflow detected";
                });

                "No overflow"
            })();

            assert_eq!(
                result, "Overflow detected",
                "Handler block's return value should be propagated"
            );
        }

        // Test case: empty collection
        {
            let empty_vec: Vec<String> = vec![];
            let line_index = idx(0);
            let mut executed = false;

            bounds_check!(line_index, empty_vec.len(), {
                executed = true;
            });

            // Block should execute when index equals length of empty collection
            assert!(
                !executed,
                "Handler block should not execute when index equals length of empty collection"
            );
        }
    }
}
