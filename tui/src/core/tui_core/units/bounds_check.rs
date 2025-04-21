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

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BoundsStatus {
    Within,
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
/// ```rust
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
pub trait BoundsCheck<OtherType> {
    fn check_overflows(&self, max: OtherType) -> BoundsStatus;
}
