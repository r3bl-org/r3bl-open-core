// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ArrayBoundsCheck, ArrayOverflowResult, CursorBoundsCheck, EditorArgsMut,
            LengthOps, RangeBoundsResult, ch,
            core::coordinates::bounds_check::ViewportBoundsCheck};

// Unicode glyphs links (for the ASCII diagrams):
// - https://symbl.cc/en/unicode/blocks/box-drawing/
// - https://symbl.cc/en/unicode/blocks/arrows/
// - https://symbl.cc/en/collections/brackets/

#[derive(Debug, Copy, Clone)]
pub enum CaretColWithinVp {
    Yes,
    No,
}

#[derive(Debug, Copy, Clone)]
pub enum CaretAtSideOfVp {
    Left,
    Right,
}

#[derive(Debug, Copy, Clone)]
enum CaretLocRelativeToVp {
    Within,
    Above,
    Below,
}

/// Check whether caret is vertically within the viewport.
/// - If it isn't then scroll by mutating:
///   1. [`crate::EditorContent::caret_raw`]'s row , so it is within the viewport.
///   2. [`crate::EditorContent::scr_ofs`]'s row, to actually apply scrolling.
/// - Otherwise, no changes are made.
pub fn validate_scroll_on_resize(args: EditorArgsMut<'_>) {
    let EditorArgsMut { buffer, engine } = args;
    validate_vertical_scroll(EditorArgsMut { engine, buffer });
    validate_horizontal_scroll(EditorArgsMut { engine, buffer });
}

/// Handle vertical scrolling (make sure caret is within viewport).
///
/// Check whether caret is in the viewport.
/// - If to top of viewport, then adjust `scr_ofs` & set it.
/// - If to bottom of viewport, then adjust `scr_ofs` & set it.
/// - If in viewport, then do nothing.
///
/// ```text
///                    ╭0───────────────────╮
///                    0                    │
///                    │       above        │ ← caret_row_scr_adj
///                    │                    │
///                    ├───    scr_ofs    ──┤
///              ╭→    │         ↑          │      ┬
///              │     │                    │      │
/// caret_raw.row_index│     within vp      │  vp height
///              │     │                    │      │
///              ╰→    │         ↓          │      ┴
///                    ├───    scr_ofs    ──┤
///                    │    + vp height     │
///                    │                    │
///                    │       below        │ ← caret_row_scr_adj
///                    │                    │
///                    ╰────────────────────╯
fn validate_vertical_scroll(args: EditorArgsMut<'_>) {
    let EditorArgsMut { buffer, engine } = args;
    let vp = engine.viewport();
    let vp_height = vp.row_height;
    let max_row = buffer.len().eol_cursor_position();

    // Make sure that caret row can't go past the bottom of the buffer.
    {
        let caret_scr_adj_row_index = buffer.get_caret_scr_adj().row_index;
        if caret_scr_adj_row_index.overflows(max_row.convert_to_length())
            == ArrayOverflowResult::Overflowed
        {
            let diff = max_row - buffer.get_caret_scr_adj().row_index;
            let buffer_mut = buffer.get_mut_no_drop(vp);
            buffer_mut.inner.caret_raw.row_index -= diff;
        }
    }

    // Make sure that scr_ofs row can't go past the bottom of the buffer.
    {
        let scr_ofs_row_index = buffer.get_scr_ofs().row_index;
        if scr_ofs_row_index.overflows(max_row.convert_to_length())
            == ArrayOverflowResult::Overflowed
        {
            let diff = max_row - scr_ofs_row_index;
            let buffer_mut = buffer.get_mut_no_drop(vp);
            buffer_mut.inner.scr_ofs.row_index -= diff;
        }
    }

    // Check whether caret is within viewport.
    {
        let caret_scr_adj_row_index = buffer.get_caret_scr_adj().row_index;
        let scr_ofs_row_index = buffer.get_scr_ofs().row_index;

        let location = {
            let is_within = caret_scr_adj_row_index
                .check_viewport_bounds(scr_ofs_row_index, vp_height)
                == RangeBoundsResult::Within;
            let is_above_or_below = caret_scr_adj_row_index < scr_ofs_row_index;
            match (is_within, is_above_or_below) {
                (true, _) => CaretLocRelativeToVp::Within,
                (false, true) => CaretLocRelativeToVp::Above,
                (false, false) => CaretLocRelativeToVp::Below,
            }
        };

        match location {
            CaretLocRelativeToVp::Within => {
                // Caret is within viewport, do nothing.
            }
            CaretLocRelativeToVp::Above => {
                // Caret is above viewport.
                let row_diff = scr_ofs_row_index - caret_scr_adj_row_index;
                let buffer_mut = buffer.get_mut_no_drop(vp);
                buffer_mut.inner.scr_ofs.row_index -= row_diff;
                buffer_mut.inner.caret_raw.row_index += row_diff;
            }
            CaretLocRelativeToVp::Below => {
                // Caret is below viewport.
                let row_diff =
                    caret_scr_adj_row_index - (scr_ofs_row_index + vp.row_height);
                let buffer_mut = buffer.get_mut_no_drop(vp);
                buffer_mut.inner.scr_ofs.row_index += row_diff;
                buffer_mut.inner.caret_raw.row_index -= row_diff;
            }
        }
    }
}

/// Handle horizontal scrolling (make sure caret is within viewport).
///
/// Check whether caret is in the viewport.
/// - If to left of viewport, then adjust `scr_ofs` & set it.
/// - If to right of viewport, then adjust `scr_ofs` & set it.
/// - If in viewport, then do nothing.
///
/// ```text
///           ╭─── vp width ───╮
/// ╭0────────┼────────────────┼─────────→
/// 0         │                │
/// │ left of │←  within vp   →│ right of
/// │         │                │
/// ╰─────────┴────────────────┴─────────→
///           ↑                ↑
///        scr_ofs     scr_ofs + vp width
/// ```
fn validate_horizontal_scroll(args: EditorArgsMut<'_>) {
    let EditorArgsMut { buffer, engine } = args;
    let vp = engine.viewport();
    let viewport_width = vp.col_width;

    // Get the maximum valid column position (line width) for bounds checking.
    // This is analogous to `max_row` in validate_vertical_scroll().
    let max_col = buffer.get_line_display_width_at_caret_scr_adj();

    // Make sure that caret col can't go past the end of the line.
    {
        let caret_scr_adj_col_index = buffer.get_caret_scr_adj().col_index;
        if caret_scr_adj_col_index.overflows(max_col) == ArrayOverflowResult::Overflowed {
            let diff = caret_scr_adj_col_index - max_col.convert_to_index();
            let buffer_mut = buffer.get_mut_no_drop(vp);
            buffer_mut.inner.caret_raw.col_index -= diff;
        }
    }

    // Make sure that scr_ofs col can't go past the end of the line.
    {
        let scr_ofs_col_index = buffer.get_scr_ofs().col_index;
        if scr_ofs_col_index.overflows(max_col) == ArrayOverflowResult::Overflowed {
            let diff = scr_ofs_col_index - max_col.convert_to_index();
            let buffer_mut = buffer.get_mut_no_drop(vp);
            buffer_mut.inner.scr_ofs.col_index -= diff;
        }
    }

    // Check whether caret is within viewport.
    {
        let caret_scr_adj_col_index = buffer.get_caret_scr_adj().col_index;
        let scr_ofs_col_index = buffer.get_scr_ofs().col_index;

        let is_within = if caret_scr_adj_col_index
            .check_viewport_bounds(scr_ofs_col_index, viewport_width)
            == RangeBoundsResult::Within
        {
            CaretColWithinVp::Yes
        } else {
            CaretColWithinVp::No
        };

        let is_outside = if caret_scr_adj_col_index < scr_ofs_col_index {
            CaretAtSideOfVp::Left
        } else {
            CaretAtSideOfVp::Right
        };

        match (is_within, is_outside) {
            (CaretColWithinVp::Yes, _) => {
                // Caret is within viewport, do nothing.
            }
            (CaretColWithinVp::No, CaretAtSideOfVp::Left) => {
                // Caret is to the left of viewport.
                let buffer_mut = buffer.get_mut_no_drop(vp);
                *buffer_mut.inner.scr_ofs.col_index = *caret_scr_adj_col_index;
                *buffer_mut.inner.caret_raw.col_index = ch(0);
            }
            (CaretColWithinVp::No, CaretAtSideOfVp::Right) => {
                // Caret is to the right of viewport.
                let buffer_mut = buffer.get_mut_no_drop(vp);
                *buffer_mut.inner.scr_ofs.col_index =
                    *caret_scr_adj_col_index - *viewport_width + ch(1);
                *buffer_mut.inner.caret_raw.col_index = *viewport_width - ch(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DEFAULT_SYN_HI_FILE_EXT, EditorBuffer, EditorEngine, EditorEngineConfig,
                LineMode, caret_raw, col,
                editor::test_fixtures_editor::mock_real_objects_for_editor, height, row,
                scr_ofs, width};

    // ┌───────────────────────────────────────────────────────────────────────────────┐
    // │ Vertical scroll tests                                                         │
    // └───────────────────────────────────────────────────────────────────────────────┘

    #[test]
    fn test_validate_vertical_scroll_within_viewport() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        let viewport = height(10) + width(10);

        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            *buffer_mut.inner.caret_raw = caret_raw(row(5) + col(0));
            *buffer_mut.inner.scr_ofs = scr_ofs(row(0) + col(0));
        }

        let editor_args_mut = EditorArgsMut {
            engine: &mut engine,
            buffer: &mut buffer,
        };

        validate_vertical_scroll(editor_args_mut);

        assert_eq!(buffer.get_scr_ofs().row_index, row(0));
        assert_eq!(buffer.get_caret_scr_adj().row_index, row(5));
    }

    #[test]
    fn test_validate_vertical_scroll_above_viewport() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        let viewport = height(10) + width(10);

        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            *buffer_mut.inner.caret_raw = caret_raw(row(0) + col(0));
            *buffer_mut.inner.scr_ofs = scr_ofs(row(5) + col(0));
        }

        let editor_args_mut = EditorArgsMut {
            engine: &mut engine,
            buffer: &mut buffer,
        };

        validate_vertical_scroll(editor_args_mut);

        assert_eq!(buffer.get_scr_ofs().row_index, row(5));
        assert_eq!(buffer.get_caret_scr_adj().row_index, row(5));
    }

    #[test]
    fn test_validate_vertical_scroll_below_viewport() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        let viewport = height(10) + width(10);

        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            *buffer_mut.inner.caret_raw = caret_raw(row(10) + col(0));
            *buffer_mut.inner.scr_ofs = scr_ofs(row(5) + col(0));
        }

        let editor_args_mut = EditorArgsMut {
            engine: &mut engine,
            buffer: &mut buffer,
        };

        validate_vertical_scroll(editor_args_mut);

        assert_eq!(buffer.get_scr_ofs().row_index, row(5));
        assert_eq!(buffer.get_caret_scr_adj().row_index, row(15));
    }

    // ┌───────────────────────────────────────────────────────────────────────────────┐
    // │ Horizontal scroll tests                                                       │
    // └───────────────────────────────────────────────────────────────────────────────┘

    /// Test: Caret column overflows `max_col` (line width).
    ///
    /// Setup: Line has 10 chars, caret at col 15 (past end of line).
    /// Expected: Caret col should be adjusted back to `max_col` (10).
    #[test]
    fn test_validate_horizontal_scroll_caret_col_overflows_max_col() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        // Create a line with 10 characters: "0123456789"
        buffer.init_with(["0123456789"]);

        let viewport = height(10) + width(20);

        // Set caret col to 15 (past line width of 10).
        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            *buffer_mut.inner.caret_raw = caret_raw(row(0) + col(15));
            *buffer_mut.inner.scr_ofs = scr_ofs(row(0) + col(0));
        }

        let editor_args_mut = EditorArgsMut {
            engine: &mut engine,
            buffer: &mut buffer,
        };

        validate_horizontal_scroll(editor_args_mut);

        // Line width = 10, max_col.convert_to_index() = 9 (last char index).
        // Caret at 15 overflows, adjusted by: 15 - 9 = 6.
        // New caret_raw = 15 - 6 = 9, caret_scr_adj = 9 + 0 = 9.
        assert_eq!(buffer.get_caret_scr_adj().col_index, col(9));
    }

    /// Test: Scroll offset column overflows `max_col` (line width).
    ///
    /// Setup: Line has 10 chars, `scr_ofs` at col 15 (past end of line).
    /// Expected: `scr_ofs` col should be adjusted back.
    #[test]
    fn test_validate_horizontal_scroll_scr_ofs_col_overflows_max_col() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        // Create a line with 10 characters.
        buffer.init_with(["0123456789"]);

        let viewport = height(10) + width(5);

        // Set scr_ofs col to 15 (past line width of 10).
        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            *buffer_mut.inner.caret_raw = caret_raw(row(0) + col(0));
            *buffer_mut.inner.scr_ofs = scr_ofs(row(0) + col(15));
        }

        let editor_args_mut = EditorArgsMut {
            engine: &mut engine,
            buffer: &mut buffer,
        };

        validate_horizontal_scroll(editor_args_mut);

        // Line width = 10, max_col.convert_to_index() = 9.
        // scr_ofs at 15 overflows, adjusted by: 15 - 9 = 6.
        // New scr_ofs = 15 - 6 = 9.
        assert_eq!(buffer.get_scr_ofs().col_index, col(9));
    }

    /// Test: Caret within viewport horizontally.
    ///
    /// Setup: Line has 20 chars, caret at col 5, `scr_ofs` at col 0, viewport width 10.
    /// Expected: No change needed (caret is within viewport).
    #[test]
    fn test_validate_horizontal_scroll_within_viewport() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        // Create a line with 20 characters.
        buffer.init_with(["01234567890123456789"]);

        let viewport = height(10) + width(10);

        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            *buffer_mut.inner.caret_raw = caret_raw(row(0) + col(5));
            *buffer_mut.inner.scr_ofs = scr_ofs(row(0) + col(0));
        }

        let editor_args_mut = EditorArgsMut {
            engine: &mut engine,
            buffer: &mut buffer,
        };

        validate_horizontal_scroll(editor_args_mut);

        // Caret should remain at col 5, scr_ofs at col 0.
        assert_eq!(buffer.get_scr_ofs().col_index, col(0));
        assert_eq!(buffer.get_caret_scr_adj().col_index, col(5));
    }

    /// Test: Caret at edge of viewport (boundary condition).
    ///
    /// Setup: Line has 20 chars, `caret_raw` at col 0, `scr_ofs` at col 5.
    /// Result: `caret_scr_adj` = 5, which is exactly at `scr_ofs` (left edge of
    /// viewport). Expected: No adjustment needed (caret is within viewport at left
    /// edge).
    ///
    /// Note: The "left of viewport" case (`caret_scr_adj` < `scr_ofs`) cannot occur with
    /// non-negative `caret_raw` values since `caret_scr_adj` = `caret_raw` + `scr_ofs`.
    #[test]
    fn test_validate_horizontal_scroll_at_left_edge() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        // Create a line with 20 characters.
        buffer.init_with(["01234567890123456789"]);

        let viewport = height(10) + width(10);

        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            *buffer_mut.inner.caret_raw = caret_raw(row(0) + col(0));
            *buffer_mut.inner.scr_ofs = scr_ofs(row(0) + col(5));
        }

        let editor_args_mut = EditorArgsMut {
            engine: &mut engine,
            buffer: &mut buffer,
        };

        validate_horizontal_scroll(editor_args_mut);

        // caret_scr_adj = 0 + 5 = 5, which equals scr_ofs (at left edge of viewport).
        // This is within viewport bounds [5, 15), so no adjustment.
        assert_eq!(buffer.get_scr_ofs().col_index, col(5));
        assert_eq!(buffer.get_caret_scr_adj().col_index, col(5));
    }

    /// Test: Caret to right of viewport.
    ///
    /// Setup: Line has 30 chars, caret at col 25, `scr_ofs` at col 5, viewport width 10.
    /// Expected: `scr_ofs` adjusted to bring caret into view.
    #[test]
    fn test_validate_horizontal_scroll_right_of_viewport() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        // Create a line with 30 characters.
        buffer.init_with(["012345678901234567890123456789"]);

        let viewport = height(10) + width(10);

        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            // caret_raw = 20 means caret is at position 20 relative to scr_ofs.
            // scr_ofs = 5.
            // caret_scr_adj = 20 + 5 = 25.
            // Viewport shows cols 5-14 (scr_ofs to scr_ofs + width - 1).
            // Caret at 25 is to the right of viewport (25 >= 5 + 10 = 15).
            *buffer_mut.inner.caret_raw = caret_raw(row(0) + col(20));
            *buffer_mut.inner.scr_ofs = scr_ofs(row(0) + col(5));
        }

        let editor_args_mut = EditorArgsMut {
            engine: &mut engine,
            buffer: &mut buffer,
        };

        validate_horizontal_scroll(editor_args_mut);

        // After adjustment, caret should be at right edge of viewport.
        // scr_ofs should be adjusted: 25 - 10 + 1 = 16.
        // caret_raw should be adjusted to viewport width - 1 = 9.
        // caret_scr_adj = 9 + 16 = 25 (unchanged absolute position).
        assert_eq!(buffer.get_scr_ofs().col_index, col(16));
        assert_eq!(buffer.get_caret_scr_adj().col_index, col(25));
    }
}
