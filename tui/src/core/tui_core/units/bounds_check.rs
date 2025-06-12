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
///     BoundsStatus::Overflowed);
///
/// let col_index = ColIndex::new(3);
/// let width = ColWidth::new(5);
/// assert_eq!(
///     col_index.check_overflows(width),
///     BoundsStatus::Within);
/// ```

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
///     println!("Index {} overflows array length {}", index, array.len());
///     return None;
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

pub trait BoundsCheck<OtherType> {
    fn check_overflows(&self, max: OtherType) -> BoundsStatus;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bounds_check, idx, len};

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
        let lines = vec!["line1", "line2", "line3"];

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
