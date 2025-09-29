// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::buffer_struct::EditorBuffer;
use crate::{ArrayBoundsCheck, ArrayOverflowResult, CursorBoundsCheck,
            CursorPositionBoundsStatus, NumericValue};

/// Represents the position of a row within a buffer.
///
/// ## Why this exists separately from [`CursorPositionBoundsStatus`]
///
/// [`CursorPositionBoundsStatus`] is designed for cursor/content positions where the
/// index can be one position *after* the last element (for insertion). For example, in a
/// string of length 5, position 5 is valid and means "after the last character".
///
/// Row positions have different semantics: you are ON a specific row, not between rows.
/// Being on row 2 in a 3-row buffer means you're ON the last row, not after it.
///
/// ### Key differences:
/// - [`AtEnd`]: Position is at `index == length` (cursor after last element)
/// - [`OnLastRow`]: Row index is `index == length - 1` (ON the last row)
///
/// ### Precedence rule for ambiguous cases:
/// - Single line buffer (row 0 is both first and last): Returns [`OnFirstRow`]
/// - Empty buffer: Returns [`OnFirstRow`]
/// - This maintains consistency: "first" takes precedence over "last" in ambiguous cases
///
/// [`OnFirstRow`]: RowContentPositionStatus::OnFirstRow
/// [`OnLastRow`]: RowContentPositionStatus::OnLastRow
/// [`AtEnd`]: CursorPositionBoundsStatus::AtEnd
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RowContentPositionStatus {
    /// On the first row (row index 0).
    /// For single-line buffers, this takes precedence over `OnLastRow`.
    OnFirstRow,

    /// On a middle row (neither first nor last).
    /// Only possible when buffer has 3+ lines.
    OnMiddleRow,

    /// On the last row of the buffer.
    /// Only returned when buffer has 2+ lines.
    OnLastRow,

    /// Row index is beyond the buffer bounds (row >= `line_count`).
    BeyondBuffer,
}

/// Locate the col position using [`CursorBoundsCheck::check_cursor_position_bounds`]
/// method on column widths.
///
/// ```text
/// R ┌──────────┐
/// 0 ❱hello     │  <- AtStart (col 0)
///   └⮬─────────┘
///   C0123456789
/// ```
///
/// ```text
/// R ┌──────────┐
/// 0 ❱hello     │  <- Within (col 3)
///   └───⮬──────┘
///   C0123456789
/// ```
///
/// ```text
/// R ┌──────────┐
/// 0 ❱hello░    │  <- AtEnd (col 5, after last char)
///   └─────⮬────┘
///   C0123456789
/// ```
#[must_use]
pub fn locate_col(editor_buffer: &EditorBuffer) -> CursorPositionBoundsStatus {
    if let Some(_line) = editor_buffer.line_at_caret_scr_adj() {
        let col_index = editor_buffer.get_caret_scr_adj().col_index;
        let line_display_width = editor_buffer.get_line_display_width_at_caret_scr_adj();
        line_display_width.check_cursor_position_bounds(col_index)
    } else {
        // No line available - treat as at start.
        CursorPositionBoundsStatus::AtStart
    }
}

/// Locate the row position in the buffer.
///
/// Returns [`RowContentPositionStatus`] instead of [`CursorPositionBoundsStatus`] because
/// row positions have different semantics than cursor positions (see
/// [`RowContentPositionStatus`] documentation).
///
/// ```text
/// R ┌──────────┐
/// 0 ❱░         │  <- OnFirstRow (single line or empty buffer)
///   └⮬─────────┘
///   C0123456789
/// ```
///
/// ```text
/// R ┌──────────┐
/// 0 │a         │
/// 1 ❱a         │  <- OnMiddleRow
/// 2 │b         │
///   └⮬─────────┘
///   C0123456789
/// ```
///
/// ```text
/// R ┌──────────┐
/// 0 │a         │
/// 1 │b         │
/// 2 ❱c         │  <- OnLastRow (only when buffer has >1 line)
///   └⮬─────────┘
///   C0123456789
/// ```
#[must_use]
pub fn locate_row(buffer: &EditorBuffer) -> RowContentPositionStatus {
    let row_index = buffer.get_caret_scr_adj().row_index;
    let buffer_line_count = buffer.get_lines().len();

    if buffer_line_count.is_zero() {
        // Empty buffer: treat as on first row.
        RowContentPositionStatus::OnFirstRow
    } else if row_index.overflows(buffer_line_count) == ArrayOverflowResult::Overflowed {
        // Beyond buffer bounds.
        RowContentPositionStatus::BeyondBuffer
    } else if buffer_line_count == 1.into() {
        // Single line: always on first row (precedence rule)
        RowContentPositionStatus::OnFirstRow
    } else {
        // Multiple lines (2+)
        if row_index.is_zero() {
            RowContentPositionStatus::OnFirstRow
        } else if row_index == buffer_line_count.convert_to_index().into() {
            RowContentPositionStatus::OnLastRow
        } else {
            RowContentPositionStatus::OnMiddleRow
        }
    }
}

/// Helper function to check if column is at the start of line.
///
/// ```text
/// R ┌──────────┐
/// 0 ❱hello     │  <- returns true
///   └⮬─────────┘
///   C0123456789
/// ```
#[must_use]
pub fn col_is_at_start(buffer: &EditorBuffer) -> bool {
    locate_col(buffer) == CursorPositionBoundsStatus::AtStart
}

/// Helper function to check if column is at the end of line.
///
/// ```text
/// R ┌──────────┐
/// 0 ❱hello░    │  <- returns true
///   └─────⮬────┘
///   C0123456789
/// ```
#[must_use]
pub fn col_is_at_end(buffer: &EditorBuffer) -> bool {
    locate_col(buffer) == CursorPositionBoundsStatus::AtEnd
}

/// Helper function to check if row is at the top of buffer.
///
/// ```text
/// R ┌──────────┐
/// 0 ❱first     │  <- returns true
/// 1 │second    │
///   └⮬─────────┘
///   C0123456789
/// ```
#[must_use]
pub fn row_is_at_top(buffer: &EditorBuffer) -> bool {
    locate_row(buffer) == RowContentPositionStatus::OnFirstRow
}

/// Helper function to check if row is at the bottom of buffer.
///
/// ```text
/// R ┌──────────┐
/// 0 │first     │
/// 1 │second    │
/// 2 ❱last      │  <- returns true
///   └⮬─────────┘
///   C0123456789
/// ```
#[must_use]
pub fn row_is_at_bottom(buffer: &EditorBuffer) -> bool {
    locate_row(buffer) == RowContentPositionStatus::OnLastRow
}

#[cfg(test)]
mod locate_col_tests {
    use super::*;
    use crate::{EditorEngine, EditorEngineConfig, assert_eq2, col, row};

    #[test]
    fn test_locate_col_at_start() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Hello World"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Set caret at start of line.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }

        let location = locate_col(&buffer);
        assert_eq2!(location, CursorPositionBoundsStatus::AtStart);
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
            buffer_mut.inner.caret_raw.col_index = line_width.eol_cursor_position();
        }

        let location = locate_col(&buffer);
        assert_eq2!(location, CursorPositionBoundsStatus::AtEnd);
    }

    #[test]
    fn test_locate_col_in_middle() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Hello World"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Set caret in middle of line.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(5);
        }

        let location = locate_col(&buffer);
        assert_eq2!(location, CursorPositionBoundsStatus::Within);
    }

    #[test]
    fn test_locate_col_empty_line() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec![""]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // On empty line, caret is both at start and end.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }

        let location = locate_col(&buffer);
        // Empty line: col 0 is both start and end, implementation treats this as AtStart
        assert_eq2!(location, CursorPositionBoundsStatus::AtStart);
    }

    #[test]
    fn test_locate_col_with_unicode() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Hello 😄 World"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Test at emoji position.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(6); // Right before emoji
        }
        let location = locate_col(&buffer);
        assert_eq2!(location, CursorPositionBoundsStatus::Within);

        // Test at end with Unicode.
        let line_width = buffer.get_lines().get_line_display_width(row(0)).unwrap();
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = line_width.eol_cursor_position();
        }
        let location = locate_col(&buffer);
        assert_eq2!(location, CursorPositionBoundsStatus::AtEnd);
    }

    #[test]
    fn test_locate_col_with_scroll_offset() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Very long line with many characters"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Set scroll offset and caret.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.scr_ofs.row_index = row(0);
            buffer_mut.inner.scr_ofs.col_index = col(5);
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }

        // The caret is at the start of the visible area, but not the start of the line.
        let location = locate_col(&buffer);
        // Scroll adjusted position is col 5, which is in the middle of the line.
        assert_eq2!(location, CursorPositionBoundsStatus::Within);
    }

    #[test]
    fn test_locate_col_beyond_line_width() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Short"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Get the actual line width to ensure our test is correct.
        let line_width = buffer.get_lines().get_line_display_width(row(0)).unwrap();
        assert_eq2!(line_width.as_usize(), 5); // "Short" has 5 characters

        // Attempt to set caret beyond line width (line width is 5, try to set caret to
        // col 10). Note: The editor buffer system validates and clamps positions,
        // so this tests the actual behavior rather than an impossible state.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(10);
        }

        // The editor system clamps invalid positions, so col 10 becomes col 5 (end of
        // line). This tests that when attempting to go beyond bounds, the system
        // handles it gracefully.
        let location = locate_col(&buffer);
        // Given the validation/clamping, we expect AtEnd rather than Beyond.
        assert_eq2!(location, CursorPositionBoundsStatus::AtEnd);
    }

    #[test]
    fn test_locate_col_no_line_available() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        // Empty buffer - no lines exist
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Set caret position even though no line exists.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(5);
        }

        let location = locate_col(&buffer);
        // When no line is available, function should return AtStart (fallback behavior)
        assert_eq2!(location, CursorPositionBoundsStatus::AtStart);
    }

    #[test]
    fn test_locate_col_multiple_rows() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Line 1", "Different Length Line", "Short"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Test column position on second row (longer line).
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(1);
            buffer_mut.inner.caret_raw.col_index = col(15); // Within "Different Length Line"
        }
        let location = locate_col(&buffer);
        assert_eq2!(location, CursorPositionBoundsStatus::Within);

        // Test column position at end of third row (shorter line).
        let line_width = buffer.get_lines().get_line_display_width(row(2)).unwrap();
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(2);
            buffer_mut.inner.caret_raw.col_index = line_width.eol_cursor_position();
        }
        let location = locate_col(&buffer);
        assert_eq2!(location, CursorPositionBoundsStatus::AtEnd);
    }

    #[test]
    fn test_locate_col_empty_line_at_end_position() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec![""]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // For empty line, test what happens when trying to position at "end".
        // Empty line has width 0, so end position would be col 0.
        let line_width = buffer.get_lines().get_line_display_width(row(0)).unwrap();
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = line_width.eol_cursor_position();
        }

        let location = locate_col(&buffer);
        // For empty line, end position is same as start position (col 0).
        // Implementation should return AtStart due to precedence rule.
        assert_eq2!(location, CursorPositionBoundsStatus::AtStart);
    }
}

#[cfg(test)]
mod locate_row_tests {
    use super::*;
    use crate::{EditorEngine, EditorEngineConfig, assert_eq2, col, row};

    #[test]
    fn test_locate_row_at_top() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Line 1", "Line 2", "Line 3"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Set caret at first row.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }

        let location = locate_row(&buffer);
        assert_eq2!(location, RowContentPositionStatus::OnFirstRow);
    }

    #[test]
    fn test_locate_row_at_bottom() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Line 1", "Line 2", "Line 3"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Set caret at last row.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(2);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }

        let location = locate_row(&buffer);
        assert_eq2!(location, RowContentPositionStatus::OnLastRow);
    }

    #[test]
    fn test_locate_row_in_middle() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Line 1", "Line 2", "Line 3"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Set caret at middle row.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(1);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }

        let location = locate_row(&buffer);
        assert_eq2!(location, RowContentPositionStatus::OnMiddleRow);
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
        assert_eq2!(location, RowContentPositionStatus::OnFirstRow);
    }

    #[test]
    fn test_locate_row_empty_buffer() {
        let buffer = EditorBuffer::new_empty(None, None);
        // Empty buffer

        let location = locate_row(&buffer);
        assert_eq2!(location, RowContentPositionStatus::OnFirstRow);
    }

    #[test]
    fn test_locate_row_beyond_buffer() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Line 1", "Line 2"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Set caret beyond buffer bounds (buffer has 2 lines, set row to 5).
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(5);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }

        let location = locate_row(&buffer);
        assert_eq2!(location, RowContentPositionStatus::BeyondBuffer);
    }

    #[test]
    fn test_locate_row_two_line_buffer() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["First line", "Second line"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Test first row of two-line buffer.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        let location = locate_row(&buffer);
        assert_eq2!(location, RowContentPositionStatus::OnFirstRow);

        // Test second row of two-line buffer (should be OnLastRow, not OnMiddleRow).
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(1);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        let location = locate_row(&buffer);
        assert_eq2!(location, RowContentPositionStatus::OnLastRow);
    }

    #[test]
    fn test_locate_row_column_independent() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Line 1", "Line 2", "Line 3"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Test that row detection is independent of column position.
        // Test middle row with different column positions.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(1);
            buffer_mut.inner.caret_raw.col_index = col(0); // Start of line
        }
        let location = locate_row(&buffer);
        assert_eq2!(location, RowContentPositionStatus::OnMiddleRow);

        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(1);
            buffer_mut.inner.caret_raw.col_index = col(5); // Middle of line
        }
        let location = locate_row(&buffer);
        assert_eq2!(location, RowContentPositionStatus::OnMiddleRow);

        // Test last row with different column positions.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(2);
            buffer_mut.inner.caret_raw.col_index = col(0); // Start of line
        }
        let location = locate_row(&buffer);
        assert_eq2!(location, RowContentPositionStatus::OnLastRow);

        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(2);
            buffer_mut.inner.caret_raw.col_index = col(3); // Middle of line
        }
        let location = locate_row(&buffer);
        assert_eq2!(location, RowContentPositionStatus::OnLastRow);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EditorEngine, EditorEngineConfig, col, row};

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
        assert!(col_is_at_start(&buffer));

        // Test not at start.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(5);
        }
        assert!(!col_is_at_start(&buffer));
    }

    #[test]
    fn test_col_is_at_start_empty_buffer() {
        let buffer = EditorBuffer::new_empty(None, None);
        // Empty buffer - should return true (cursor is at start of empty content)
        assert!(col_is_at_start(&buffer));
    }

    #[test]
    fn test_col_is_at_start_multiple_rows() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["First line", "Second line is longer", "Short"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Test col position is independent of row - start of row 0
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        assert!(col_is_at_start(&buffer));

        // Test start of row 1 (longer line)
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(1);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        assert!(col_is_at_start(&buffer));

        // Test start of row 2 (shorter line)
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(2);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        assert!(col_is_at_start(&buffer));

        // Test not at start on row 1
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(1);
            buffer_mut.inner.caret_raw.col_index = col(5);
        }
        assert!(!col_is_at_start(&buffer));
    }

    #[test]
    fn test_col_is_at_start_unicode_text() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Hello 😄 World"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Test at start with Unicode content
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        assert!(col_is_at_start(&buffer));

        // Test not at start (before emoji)
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(6);
        }
        assert!(!col_is_at_start(&buffer));
    }

    #[test]
    fn test_col_is_at_start_empty_line() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec![""]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Test empty line - col 0 is both start and end, should return true for start
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        assert!(col_is_at_start(&buffer));
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
            buffer_mut.inner.caret_raw.col_index = line_width.eol_cursor_position();
        }
        assert!(col_is_at_end(&buffer));

        // Test not at end.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(2);
        }
        assert!(!col_is_at_end(&buffer));
    }

    #[test]
    fn test_col_is_at_end_empty_buffer() {
        let buffer = EditorBuffer::new_empty(None, None);
        // Empty buffer - should return false (locate_col returns AtStart for empty, not
        // AtEnd)
        assert!(!col_is_at_end(&buffer));
    }

    #[test]
    fn test_col_is_at_end_multiple_rows() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["First", "Second line is longer", "Short"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Test end of row 0 (shorter line)
        let line_width = buffer.get_lines().get_line_display_width(row(0)).unwrap();
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = line_width.eol_cursor_position();
        }
        assert!(col_is_at_end(&buffer));

        // Test end of row 1 (longer line)
        let line_width = buffer.get_lines().get_line_display_width(row(1)).unwrap();
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(1);
            buffer_mut.inner.caret_raw.col_index = line_width.eol_cursor_position();
        }
        assert!(col_is_at_end(&buffer));

        // Test end of row 2 (shorter line)
        let line_width = buffer.get_lines().get_line_display_width(row(2)).unwrap();
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(2);
            buffer_mut.inner.caret_raw.col_index = line_width.eol_cursor_position();
        }
        assert!(col_is_at_end(&buffer));

        // Test not at end on row 1
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(1);
            buffer_mut.inner.caret_raw.col_index = col(5);
        }
        assert!(!col_is_at_end(&buffer));
    }

    #[test]
    fn test_col_is_at_end_unicode_text() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Hello 😄 World"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Test at end with Unicode content
        let line_width = buffer.get_lines().get_line_display_width(row(0)).unwrap();
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = line_width.eol_cursor_position();
        }
        assert!(col_is_at_end(&buffer));

        // Test not at end (before emoji)
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(6);
        }
        assert!(!col_is_at_end(&buffer));
    }

    #[test]
    fn test_col_is_at_end_empty_line() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec![""]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Test empty line - col 0 is both start and end, but should return false for end
        // due to precedence rule (AtStart takes precedence over AtEnd for empty content)
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        assert!(!col_is_at_end(&buffer));
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
        assert!(row_is_at_top(&buffer));

        // Test not at top
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(1);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        assert!(!row_is_at_top(&buffer));
    }

    #[test]
    fn test_row_is_at_top_empty_buffer() {
        let buffer = EditorBuffer::new_empty(None, None);
        // Empty buffer - should return true (locate_row returns OnFirstRow for empty)
        assert!(row_is_at_top(&buffer));
    }

    #[test]
    fn test_row_is_at_top_single_line() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["Only line"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Single line buffer - should return true (OnFirstRow takes precedence)
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        assert!(row_is_at_top(&buffer));
    }

    #[test]
    fn test_row_is_at_top_two_line() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["First line", "Second line"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Test first row of two-line buffer
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        assert!(row_is_at_top(&buffer));

        // Test second row of two-line buffer (should not be top)
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(1);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        assert!(!row_is_at_top(&buffer));
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
        assert!(row_is_at_bottom(&buffer));

        // Test not at bottom.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(1);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        assert!(!row_is_at_bottom(&buffer));

        // Test single line buffer (should return false for bottom)
        let mut single_line_buffer = EditorBuffer::new_empty(None, None);
        single_line_buffer.init_with(vec!["Only line"]);
        {
            let buffer_mut = single_line_buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        assert!(!row_is_at_bottom(&single_line_buffer)); // Single line is at top, not bottom
    }

    #[test]
    fn test_row_is_at_bottom_empty_buffer() {
        let buffer = EditorBuffer::new_empty(None, None);
        // Empty buffer - should return false (locate_row returns OnFirstRow for empty,
        // not OnLastRow)
        assert!(!row_is_at_bottom(&buffer));
    }

    #[test]
    fn test_row_is_at_bottom_two_line() {
        let mut buffer = EditorBuffer::new_empty(None, None);
        buffer.init_with(vec!["First line", "Second line"]);
        let engine = EditorEngine::new(EditorEngineConfig::default());

        // Test first row of two-line buffer (should not be bottom)
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(0);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        assert!(!row_is_at_bottom(&buffer));

        // Test second row of two-line buffer (should be bottom)
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.caret_raw.row_index = row(1);
            buffer_mut.inner.caret_raw.col_index = col(0);
        }
        assert!(row_is_at_bottom(&buffer));
    }
}
