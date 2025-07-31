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

use crate::{col, row, ColIndex, ColWidth, RowHeight, RowIndex};
use super::buffer_struct::EditorBuffer;

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum CaretColLocationInLine {
    /// Also covers state where there is no col, or only 1 col.
    AtStart,
    AtEnd,
    InMiddle,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum CaretRowLocationInBuffer {
    /// Also covers state where there is no row, or only 1 row.
    AtTop,
    AtBottom,
    InMiddle,
}

/// Locate the col.
#[must_use]
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
#[must_use]
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
    if buffer.is_empty() || buffer.get_lines().len().as_usize() == 1 {
        // If there is only one line, then the caret is not at the bottom, its at the top.
        false
    } else {
        /* lines.len() - 1 is the last row index */
        let max_row_index = buffer.get_max_row_index();
        buffer.get_caret_scr_adj().row_index == max_row_index
    }
}

pub mod caret_scroll_index {
    #[allow(clippy::wildcard_imports)]
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
    #[must_use]
    pub fn col_index_for_width(col_amt: ColWidth) -> ColIndex {
        col_amt.convert_to_col_index() /* -1 */ + col(1) /* +1 */
    }

    /// This is the same number as the given height, just in different "unit". The caret
    /// max index which is the scroll index goes 1 past the end of the given height's
    /// index.
    #[must_use]
    pub fn row_index_for_height(row_amt: RowHeight) -> RowIndex {
        row_amt.convert_to_row_index() /* -1 */ + row(1) /* +1 */
    }

    #[test]
    fn test_scroll_col_index_for_width() {
        use crate::width;

        let width = width(5);
        let scroll_col_index = col_index_for_width(width);
        assert_eq!(*scroll_col_index, *width);
    }

    #[test]
    fn test_scroll_row_index_for_height() {
        use crate::height;

        let height = height(5);
        let scroll_row_index = row_index_for_height(height);
        assert_eq!(*scroll_row_index, *height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assert_eq2, EditorEngine, EditorEngineConfig};

    #[test]
    fn test_locate_col_at_start() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Hello World"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Set caret at start of line
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }

        let location = locate_col(&buffer);
        assert_eq2!(location, CaretColLocationInLine::AtStart);
    }

    #[test]
    fn test_locate_col_at_end() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Hello World"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Set caret at end of line (display width 11, so caret index is also 11)
        let line_width = buffer.get_lines().get_line_display_width(row(0)).unwrap();
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = caret_scroll_index::col_index_for_width(line_width);
        }

        let location = locate_col(&buffer);
        assert_eq2!(location, CaretColLocationInLine::AtEnd);
    }

    #[test]
    fn test_locate_col_in_middle() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Hello World"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Set caret in middle of line
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(5);
        }

        let location = locate_col(&buffer);
        assert_eq2!(location, CaretColLocationInLine::InMiddle);
    }

    #[test]
    fn test_locate_col_empty_line() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec![""]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // On empty line, caret is both at start and end
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }

        let location = locate_col(&buffer);
        // Empty line: col 0 is both start and end, implementation treats this as AtStart
        assert_eq2!(location, CaretColLocationInLine::AtStart);
    }

    #[test]
    fn test_locate_col_with_unicode() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Hello 😄 World"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Test at emoji position
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(6); // Right before emoji
        }
        let location = locate_col(&buffer);
        assert_eq2!(location, CaretColLocationInLine::InMiddle);

        // Test at end with Unicode
        let line_width = buffer.get_lines().get_line_display_width(row(0)).unwrap();
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = caret_scroll_index::col_index_for_width(line_width);
        }
        let location = locate_col(&buffer);
        assert_eq2!(location, CaretColLocationInLine::AtEnd);
    }

    #[test]
    fn test_locate_row_at_top() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Line 1", "Line 2", "Line 3"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Set caret at first row
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }

        let location = locate_row(&buffer);
        assert_eq2!(location, CaretRowLocationInBuffer::AtTop);
    }

    #[test]
    fn test_locate_row_at_bottom() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Line 1", "Line 2", "Line 3"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Set caret at last row
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(2);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }

        let location = locate_row(&buffer);
        assert_eq2!(location, CaretRowLocationInBuffer::AtBottom);
    }

    #[test]
    fn test_locate_row_in_middle() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Line 1", "Line 2", "Line 3"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Set caret at middle row
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(1);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }

        let location = locate_row(&buffer);
        assert_eq2!(location, CaretRowLocationInBuffer::InMiddle);
    }

    #[test]
    fn test_locate_row_single_line() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Only line"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // With only one line, caret is at top (not bottom)
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }

        let location = locate_row(&buffer);
        assert_eq2!(location, CaretRowLocationInBuffer::AtTop);
    }

    #[test]
    fn test_locate_row_empty_buffer() {
        let buffer = EditorBuffer::new_empty(None, None);
        // Empty buffer

        let location = locate_row(&buffer);
        assert_eq2!(location, CaretRowLocationInBuffer::AtTop);
    }

    #[test]
    fn test_col_is_at_start_of_line() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Test line"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Test at start
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        let location = locate_col(&buffer);
        assert_eq2!(location, CaretColLocationInLine::AtStart);

        // Test not at start
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(5);
        }
        let location = locate_col(&buffer);
        assert_eq2!(location, CaretColLocationInLine::InMiddle);
    }

    #[test]
    fn test_col_is_at_end_of_line() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Test"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Test at end (display width 4, caret index is also 4)
        let line_width = buffer.get_lines().get_line_display_width(row(0)).unwrap();
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = caret_scroll_index::col_index_for_width(line_width);
        }
        let location = locate_col(&buffer);
        assert_eq2!(location, CaretColLocationInLine::AtEnd);

        // Test not at end
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(2);
        }
        let location = locate_col(&buffer);
        assert_eq2!(location, CaretColLocationInLine::InMiddle);
    }

    #[test]
    fn test_row_is_at_top_of_buffer() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Line 1", "Line 2", "Line 3"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Test at top
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        let location = locate_row(&buffer);
        assert_eq2!(location, CaretRowLocationInBuffer::AtTop);

        // Test not at top (with 2 lines, row 1 is the bottom)
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(1);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        let location = locate_row(&buffer);
        assert_eq2!(location, CaretRowLocationInBuffer::InMiddle);
    }

    #[test]
    fn test_row_is_at_bottom_of_buffer() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Line 1", "Line 2", "Line 3"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Test at bottom
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(2);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        let location = locate_row(&buffer);
        assert_eq2!(location, CaretRowLocationInBuffer::AtBottom);

        // Test not at bottom
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(1);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        let location = locate_row(&buffer);
        assert_eq2!(location, CaretRowLocationInBuffer::InMiddle);

        // Test single line (should return false)
        let mut single_line_buffer = EditorBuffer::new_empty(None, None);
        single_line_buffer.init_with(vec!["Only line"]);
        {
            let buffer_mut = single_line_buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        let location = locate_row(&single_line_buffer);
        assert_eq2!(location, CaretRowLocationInBuffer::AtTop); // Single line is at top, not bottom
    }

    #[test]
    fn test_locate_functions_with_scroll_offset() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Very long line with many characters"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Set scroll offset and caret
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.scr_ofs.row_index = row(0);
            buffer_mut.inner.scr_ofs.col_index = col(5);
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }

        // The caret is at the start of the visible area, but not the start of the line
        let location = locate_col(&buffer);
        // Scroll adjusted position is col 5, which is in the middle of the line
        assert_eq2!(location, CaretColLocationInLine::InMiddle);
    }
}
