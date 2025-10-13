// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ArrayBoundsCheck, ArrayOverflowResult, CursorBoundsCheck, EditorArgsMut,
            RangeBoundsResult, ch, core::units::bounds_check::ViewportBoundsCheck};

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
///           ├    vp width    ┤
/// ╭0────────┬────────────────┬─────────→
/// 0         │                │
/// │ left of │←  within vp   →│ right of
/// │         │                │
/// ╰─────────┴────────────────┴─────────→
///           ↑                ↑
///        scr_ofs     scr_ofs + vp width
/// ```
fn validate_horizontal_scroll(args: EditorArgsMut<'_>) {
    let EditorArgsMut { buffer, engine } = args;
    let viewport_width = engine.viewport().col_width;

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
            let buffer_mut = buffer.get_mut_no_drop(engine.viewport());
            *buffer_mut.inner.scr_ofs.col_index = *caret_scr_adj_col_index;
            *buffer_mut.inner.caret_raw.col_index = ch(0);
        }
        (CaretColWithinVp::No, CaretAtSideOfVp::Right) => {
            // Caret is to the right of viewport.
            let buffer_mut = buffer.get_mut_no_drop(engine.viewport());
            *buffer_mut.inner.scr_ofs.col_index =
                *caret_scr_adj_col_index - *viewport_width + ch(1);
            *buffer_mut.inner.caret_raw.col_index = *viewport_width - ch(1);
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
}
