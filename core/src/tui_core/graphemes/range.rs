/*
 *   Copyright (c) 2023 R3BL LLC
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

use std::{cmp::{self, Ord},
          fmt::{Debug, Display}};

use get_size::GetSize;
use serde::{Deserialize, Serialize};

use crate::*;

/// Represents a range of characters in a line. The range is not inclusive of the item at
/// the end index, which means that when you call
/// [clip_to_range](UnicodeString::clip_to_range) the item at the end index will not be
/// part of the result (this is shown in the example below). The indices are all display
/// column indices, not logical ones.
///
/// ```text
/// 0 1 2 3 4 5 6 7 8 9
/// h e ▓ ▓ o   w o r l
///   ↑     ↑
///   │     │
///   │     end_display_col_index
/// start_display_col_index
/// ```
/// - `▓▓` = `😃`
/// - [clip_to_range](UnicodeString::clip_to_range): "e😃"
#[derive(Default, Clone, PartialEq, Serialize, Deserialize, GetSize, Copy)]
pub struct SelectionRange {
    pub start_display_col_index: ChUnit,
    pub end_display_col_index: ChUnit,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, GetSize, Copy, Debug)]
pub enum CaretLocationInRange {
    Overflow,
    Underflow,
    Contained,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, GetSize, Copy, Debug)]
pub enum CaretMovementDirection {
    Left,
    Right,
    Overlap,
}

#[cfg(test)]
mod tests_range {
    use super::*;

    /// ```text
    /// 0 1 2 3 4 5 6 7 8 9
    /// h e l l o   w o r l
    ///   ↑     ↑
    ///   │     │
    ///   │     4 = end_display_col_index
    ///   1 = start_display_col_index
    /// ```
    /// - [UnicodeString::clip_to_range](UnicodeString::clip_to_range): "ell"
    #[test]
    fn test_locate() {
        let range = {
            let start = ch!(1);
            let end = ch!(4);
            SelectionRange::new(start, end)
        };
        assert_eq2!(range.locate(ch!(0)), CaretLocationInRange::Underflow);
        assert_eq2!(range.locate(ch!(1)), CaretLocationInRange::Contained);
        assert_eq2!(range.locate(ch!(2)), CaretLocationInRange::Contained);
        assert_eq2!(range.locate(ch!(3)), CaretLocationInRange::Contained);
        assert_eq2!(range.locate(ch!(4)), CaretLocationInRange::Overflow);
        assert_eq2!(range.locate(ch!(5)), CaretLocationInRange::Overflow);
    }
}

// TODO: add RangeApi to allow Range changes (add to end, remove from end, etc.)
impl SelectionRange {
    pub fn caret_movement_direction(
        previous_caret_display_col_index: ChUnit,
        current_caret_display_col_index: ChUnit,
    ) -> CaretMovementDirection {
        match current_caret_display_col_index.cmp(&previous_caret_display_col_index) {
            cmp::Ordering::Greater => CaretMovementDirection::Right,
            cmp::Ordering::Less => CaretMovementDirection::Left,
            cmp::Ordering::Equal => CaretMovementDirection::Overlap,
        }
    }

    /// ```text
    /// 0 1 2 3 4 5 6 7 8 9
    /// h e l l o   w o r l
    ///   ↑     ↑
    ///   │     │
    ///   │     4 = end_display_col_index
    ///   1 = start_display_col_index
    /// ```
    /// - [UnicodeString::clip_to_range](UnicodeString::clip_to_range): "ell"
    pub fn locate(&self, caret_display_col_index: ChUnit) -> CaretLocationInRange {
        if caret_display_col_index < self.start_display_col_index {
            CaretLocationInRange::Underflow
        } else if caret_display_col_index >= self.end_display_col_index {
            CaretLocationInRange::Overflow
        } else {
            CaretLocationInRange::Contained
        }
    }

    pub fn new(start_display_col_index: ChUnit, end_display_col_index: ChUnit) -> Self {
        Self {
            start_display_col_index,
            end_display_col_index,
        }
    }

    /// ```text
    /// 0 1 2 3 4 5 6 7 8 9
    /// h e l l o   w o r l
    ///   ↑     ↑
    ///   │     │
    ///   │     → end_display_col_index + amount
    /// start_display_col_index
    /// ```
    pub fn grow_end_by(&self, amount: ChUnit) -> Self {
        let mut copy = *self;
        copy.end_display_col_index += amount;
        copy
    }

    /// ```text
    /// 0 1 2 3 4 5 6 7 8 9
    /// h e l l o   w o r l
    ///   ↑     ↑
    ///   │     │
    ///   │     ← end_display_col_index - amount
    /// start_display_col_index
    /// ```
    pub fn shrink_end_by(&self, amount: ChUnit) -> Self {
        let mut copy = *self;
        copy.end_display_col_index -= amount;
        copy
    }

    /// ```text
    /// 0 1 2 3 4 5 6 7 8 9
    /// h e l l o   w o r l
    ///   ↑     ↑
    ///   │     │
    ///   │     end_display_col_index
    ///   ← start_display_col_index - amount
    /// ```
    pub fn grow_start_by(&self, amount: ChUnit) -> Self {
        let mut copy = *self;
        copy.start_display_col_index -= amount;
        copy
    }

    /// ```text
    /// 0 1 2 3 4 5 6 7 8 9
    /// h e l l o   w o r l
    ///   ↑     ↑
    ///   │     │
    ///   │     end_display_col_index
    ///   → start_display_col_index + amount
    /// ```
    pub fn shrink_start_by(&self, amount: ChUnit) -> Self {
        let mut copy = *self;
        copy.start_display_col_index += amount;
        copy
    }
}

mod range_impl_debug_format {
    use super::*;

    pub fn debug_format(
        range: &SelectionRange,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(
            f,
            "[start_display_col_index: {0}, end_display_col_index: {1}]",
            /* 0 */ range.start_display_col_index,
            /* 1 */ range.end_display_col_index
        )
    }

    impl Display for SelectionRange {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            debug_format(self, f)
        }
    }

    impl Debug for SelectionRange {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            debug_format(self, f)
        }
    }
}
