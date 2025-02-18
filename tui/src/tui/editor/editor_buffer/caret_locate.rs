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

use r3bl_core::{col, row, ColIndex, ColWidth, RowHeight, RowIndex};

use crate::EditorBuffer;

#[derive(Clone, Eq, PartialEq)]
pub enum CaretColLocationInLine {
    /// Also covers state where there is no col, or only 1 col.
    AtStart,
    AtEnd,
    InMiddle,
}

#[derive(Clone, Eq, PartialEq)]
pub enum CaretRowLocationInBuffer {
    /// Also covers state where there is no row, or only 1 row.
    AtTop,
    AtBottom,
    InMiddle,
}

/// Locate the col.
pub fn locate_col(editor_buffer: &EditorBuffer) -> CaretColLocationInLine {
    if col_is_at_start_of_line(editor_buffer) {
        CaretColLocationInLine::AtStart
    } else if col_is_at_end_of_line(editor_buffer) {
        CaretColLocationInLine::AtEnd
    } else {
        CaretColLocationInLine::InMiddle
    }
}

fn col_is_at_start_of_line(buffer: &EditorBuffer) -> bool {
    if buffer.line_at_caret_scr_adj().is_some() {
        buffer.get_caret_scr_adj().col_index == col(0)
    } else {
        false
    }
}

fn col_is_at_end_of_line(buffer: &EditorBuffer) -> bool {
    if buffer.line_at_caret_scr_adj().is_some() {
        let line_display_width = buffer.get_line_display_width_at_caret_scr_adj();
        buffer.get_caret_scr_adj().col_index
            == caret_scroll_index::col_index_for_width(line_display_width)
    } else {
        false
    }
}

/// Locate the row.
pub fn locate_row(buffer: &EditorBuffer) -> CaretRowLocationInBuffer {
    if row_is_at_top_of_buffer(buffer) {
        CaretRowLocationInBuffer::AtTop
    } else if row_is_at_bottom_of_buffer(buffer) {
        CaretRowLocationInBuffer::AtBottom
    } else {
        CaretRowLocationInBuffer::InMiddle
    }
}

/// ```text
/// R ┌──────────┐
/// 0 ❱          │
///   └⮬─────────┘
///   C0123456789
/// ```
fn row_is_at_top_of_buffer(buffer: &EditorBuffer) -> bool {
    buffer.get_caret_scr_adj().row_index == row(0)
}

/// ```text
/// R ┌──────────┐
/// 0 │a         │
/// 1 ❱a         │
///   └⮬─────────┘
///   C0123456789
/// ```
fn row_is_at_bottom_of_buffer(buffer: &EditorBuffer) -> bool {
    if buffer.is_empty() || buffer.get_lines().len() == 1 {
        // If there is only one line, then the caret is not at the bottom, its at the top.
        false
    } else {
        /* lines.len() - 1 is the last row index */
        let max_row_index = buffer.get_max_row_index();
        buffer.get_caret_scr_adj().row_index == max_row_index
    }
}

pub mod caret_scroll_index {
    use super::*;

    /// This is the same number as the given width, just in different "unit". The caret
    /// max index which is the scroll index goes 1 past the end of the given width's
    /// index.
    ///
    /// Equivalent to:
    /// ```text
    /// col_amt_index = col_amt - 1;
    /// scroll_past_col_amt_index = col_amt_index + 1;
    /// ```
    ///
    /// Here's an example:
    /// ```text
    /// R ┌──────────┐
    /// 0 ▸hello░    │
    ///   └─────▴────┘
    ///   C0123456789
    /// ```
    pub fn col_index_for_width(col_amt: ColWidth) -> ColIndex {
        col_amt.convert_to_col_index() /* -1 */ + col(1) /* +1 */
    }

    /// This is the same number as the given height, just in different "unit". The caret
    /// max index which is the scroll index goes 1 past the end of the given height's
    /// index.
    pub fn row_index_for_height(row_amt: RowHeight) -> RowIndex {
        row_amt.convert_to_row_index() /* -1 */ + row(1) /* +1 */
    }

    #[test]
    fn test_scroll_col_index_for_width() {
        use r3bl_core::width;

        let width = width(5);
        let scroll_col_index = col_index_for_width(width);
        assert_eq!(*scroll_col_index, *width);
    }

    #[test]
    fn test_scroll_row_index_for_height() {
        use r3bl_core::height;

        let height = height(5);
        let scroll_row_index = row_index_for_height(height);
        assert_eq!(*scroll_row_index, *height);
    }
}
