// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CursorBoundsCheck, EditorArgsMut, Viewport, core::CanvasCameraExt};

/// Ensures that caret is vertically and horizontally within the viewport when the
/// "camera" is panned (or moved vertically and horizontally).
///
/// It checks whether the caret is vertically and horizontally within the viewport:
/// - If it isn't then scroll by mutating:
///   1. [`EditorContent::c_caret`]'s `vp_row`, so it is within the viewport. This is
///      where the "camera" is looking.
///   2. [`Viewport::set_origin_pos()`]'s `vp_row`, to actually apply scrolling. This is
///      where the cursor is, relative to the "camera".
/// - Otherwise, no changes are made.
///
/// This function exists to enforce two primary invariants for the editor's 2D scroll
/// state:
/// 1. Clamping (Bounds Enforcement): Ensures that neither the caret nor the viewport's
///    origin position ([`Viewport::get_origin_pos()`]) are placed past the actual end of
///    the document (`EOF` or `EOL`/max line width). If the user shrinks the terminal or
///    deletes text, this logic pulls the view and caret back into the valid document
///    area.
/// 2. Alignment (Auto-Scrolling): Ensures that the canvas caret is always visible. If the
///    caret moves outside the `[origin, origin + vp_size)` window, this logic shifts
///    [`Viewport::set_origin_pos()`] just enough to bring the caret back into view, and
///    updates the [`EditorContent::c_caret`] offset to match.
///
/// Here's the main math for the "camera" movement:
///
/// ```text
/// CPos    - CPos      = VPPos
/// c_caret - vp_origin = vp_caret
/// ```
///
/// - The two [`CPos`] values are:
///   1. `c_caret`: The canvas coordinate of the cursor in the entire document (e.g., line
///      515).
///   2. `vp_origin`: The canvas coordinate of the top-left corner of the camera/screen in
///      the document (e.g., scrolled down to line 500).
/// - When you subtract the origin from the caret, you get the [`VPPos`]:
///   - `vp_caret`: The viewport coordinate where the cursor is drawn on the physical
///     screen (e.g., row 15).
///
/// # Vertical Scrolling (Caret within Viewport Height)
///
/// Checks whether caret is in the vertical viewport:
/// - If to top of viewport, adjust `vp_origin` & set it.
/// - If to bottom of viewport, adjust `vp_origin` & set it.
/// - If in viewport, do nothing.
///
/// ```text
///                         0
///                       0 ┌───────────────────┐
///                         │                   │
///                         │  above viewport   │ ← c_caret.row_index
///                         │                   │   (< vp_origin)
/// vp_origin             → ├───────────────────┤ ┬
///                         │         ▲         │ │
/// c_caret.row             │         │         │ │
/// (viewport coordinate) → │    within vp      │ │ vp height (row_height)
///                         │         │         │ │
///                         │         ▼         │ │
/// vp_origin             → ├───────────────────┤ ┴
/// + vp height             │                   │
///                         │  below viewport   │ ← c_caret.row_index
///                         │                   │   (>= vp_origin + vp height)
///                         └───────────────────┘
/// ```
///
/// # Horizontal Scrolling (Caret within Viewport Width)
///
/// Checks whether caret is in the horizontal viewport:
/// - If to left of viewport, adjust `vp_origin` & set it.
/// - If to right of viewport, adjust `vp_origin` & set it.
/// - If in viewport, do nothing.
///
/// ```text
///           ╭─── vp width ───╮
/// ╭0────────┼────────────────┼─────────→
/// 0         │                │
/// │ left of │←  within vp   →│ right of
/// │         │                │
/// ╰─────────┴────────────────┴─────────→
///           ↑                ↑
///        vp_origin     vp_origin + vp width
/// ```
///
/// [`CPos`]: crate::CPos
/// [`EditorContent::c_caret`]: crate::EditorBuffer::get_c_caret
/// [`Viewport::get_origin_pos()`]: crate::Viewport::get_origin_pos
/// [`Viewport::set_origin_pos()`]: crate::Viewport::set_origin_pos
/// [`VPPos`]: crate::VPPos
pub fn validate_scroll_on_resize(args: EditorArgsMut<'_>) {
    let EditorArgsMut { buffer, engine } = args;
    let vp_size = engine.viewport();

    let mut c_caret = buffer.get_c_caret();
    let mut origin = buffer.get_vp_origin();

    // Vertical clamping: clamp to EOF (max row).
    let max_row = buffer.get_lines().get_line_count().eol_cursor_position();
    c_caret.row_index = c_caret.row_index.min(max_row);
    origin.row_index = origin.row_index.min(max_row);

    // Horizontal clamping: clamp to EOL (max col on caret's row).
    let max_col = buffer
        .get_lines()
        .get_line_display_width_at_row_index(c_caret.row_index)
        .eol_cursor_position();
    c_caret.col_index = c_caret.col_index.min(max_col);
    origin.col_index = origin.col_index.min(max_col);

    // 2D Camera Panning.
    let mut viewport = Viewport::new(origin, vp_size);
    viewport.pan_to_keep_coord_in_view(*c_caret);

    // Update the buffer.
    let editor_buffer_mut = &mut buffer.get_mut_no_drop(vp_size).inner;
    editor_buffer_mut
        .viewport
        .set_origin_pos(|pos| *pos = viewport.get_origin_pos());
    *editor_buffer_mut.c_caret = c_caret;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DEFAULT_SYN_HI_FILE_EXT, EditorBuffer, EditorEngine, EditorEngineConfig,
                FileExtensionToken, LineMode, c_caret, c_col, c_pos, c_row,
                editor::test_fixtures_editor::mock_real_objects_for_editor, vp_caret,
                vp_height, vp_pos, vp_row, vp_width};

    #[test]
    fn test_validate_vertical_scroll_caret_overflows_max_c_row() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        let viewport = vp_height(10) + vp_width(10);
        // Buffer has 10 lines (max_row = row 9).
        buffer.init_with(vec!["line"; 10]);

        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            // Caret at row 15 (past max_row 9).
            *buffer_mut.inner.c_caret = c_caret(c_row(15) + c_col(0));
            buffer_mut
                .inner
                .viewport
                .set_origin_pos(|pos| *pos = c_pos(0, 0));
        }

        validate_scroll_on_resize(EditorArgsMut::new(&mut buffer, &mut engine));

        // Caret should be clamped back to max_row (row 10).
        assert_eq!(buffer.get_c_caret().row_index, c_row(10));
    }

    #[test]
    fn test_validate_vertical_scroll_caret_underflow_safety() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        let viewport = vp_height(10) + vp_width(10);
        // Buffer has 8 lines (max_row = row 8).
        buffer.init_with(vec!["line"; 8]);

        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            // vp_origin = 10, c_caret = 5 => c_caret = 15.
            // diff = 15 - 8 = 7.
            // Previously, c_caret (5) - diff (7) underflowed!
            buffer_mut
                .inner
                .viewport
                .set_origin_pos(|pos| *pos = c_pos(0, 10));
            *buffer_mut.inner.c_caret = c_caret(c_row(15) + c_col(0));
        }

        // Must not panic on underflow!
        validate_scroll_on_resize(EditorArgsMut::new(&mut buffer, &mut engine));

        // Caret should be safely clamped to max_row (row 8).
        assert_eq!(buffer.get_c_caret().row_index, c_row(8));
    }

    #[test]
    fn test_validate_vertical_scroll_within_viewport() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        let viewport = vp_height(10) + vp_width(10);
        buffer.init_with(vec!["line"; 30]);

        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            *buffer_mut.inner.c_caret = c_caret(c_row(5) + c_col(0));
            buffer_mut
                .inner
                .viewport
                .set_origin_pos(|pos| *pos = c_pos(0, 0));
        }

        validate_scroll_on_resize(EditorArgsMut::new(&mut buffer, &mut engine));

        assert_eq!(buffer.get_vp_origin().row_index, c_row(0));
        assert_eq!(buffer.get_c_caret().row_index, c_row(5));
        assert_eq!(buffer.get_vp_caret().row_index, vp_row(5));
    }

    #[test]
    fn test_validate_vertical_scroll_above_viewport() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        let viewport = vp_height(10) + vp_width(10);
        buffer.init_with(vec!["line"; 30]);

        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            *buffer_mut.inner.c_caret = c_caret(c_row(5) + c_col(0));
            buffer_mut
                .inner
                .viewport
                .set_origin_pos(|pos| *pos = c_pos(0, 5));
        }

        assert_eq!(buffer.get_c_caret().row_index, c_row(5));

        validate_scroll_on_resize(EditorArgsMut::new(&mut buffer, &mut engine));

        // c_caret = 0 + 5 = 5, which equals vp_origin (at top edge of
        // viewport). This is within viewport bounds [5, 15), so no adjustment.
        assert_eq!(buffer.get_vp_origin().row_index, c_row(5));
        assert_eq!(buffer.get_vp_caret().row_index, vp_row(0));
        assert_eq!(buffer.get_c_caret().row_index, c_row(5));
    }

    #[test]
    fn test_validate_vertical_scroll_below_viewport() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        let viewport = vp_height(10) + vp_width(10);
        buffer.init_with(vec!["line"; 30]);

        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            // Viewport origin at row 5, c_caret at row 15 => c_caret = row
            // 20. Viewport of height 10 at origin 5 covers rows [5, 15).
            // Caret at 20 is below viewport.
            buffer_mut
                .inner
                .viewport
                .set_origin_pos(|pos| *pos = c_pos(0, 5));
            *buffer_mut.inner.c_caret = c_caret(c_row(20) + c_col(0));
        }

        assert_eq!(buffer.get_c_caret().row_index, c_row(20));

        validate_scroll_on_resize(EditorArgsMut::new(&mut buffer, &mut engine));

        // After validation, vp_origin should adjust to 20 - 10 + 1 = 11.
        // c_caret should adjust to 10 - 1 = 9.
        // c_caret remains row 20.
        assert_eq!(buffer.get_vp_origin().row_index, c_row(11));
        assert_eq!(buffer.get_vp_caret().row_index, vp_row(9));
        assert_eq!(buffer.get_c_caret().row_index, c_row(20));
    }

    #[test]
    fn test_validate_vertical_scroll_at_bottom_edge() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        let viewport = vp_height(10) + vp_width(10);
        buffer.init_with(vec!["line"; 30]);

        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            // Viewport origin at 5, c_caret at row 9 => c_caret = row 14.
            // In viewport height 10 (rows 5..15), row 14 is the last visible row
            // (Within).
            buffer_mut
                .inner
                .viewport
                .set_origin_pos(|pos| *pos = c_pos(0, 5));
            *buffer_mut.inner.c_caret = c_caret(c_row(14) + c_col(0));
        }

        validate_scroll_on_resize(EditorArgsMut::new(&mut buffer, &mut engine));

        assert_eq!(buffer.get_vp_origin().row_index, c_row(5));
        assert_eq!(buffer.get_vp_caret().row_index, vp_row(9));
        assert_eq!(buffer.get_c_caret().row_index, c_row(14));
    }

    #[test]
    fn test_validate_vertical_scroll_one_past_bottom_edge() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        let viewport = vp_height(10) + vp_width(10);
        buffer.init_with(vec!["line"; 30]);

        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            // Viewport origin at 5, c_caret at row 10 => c_caret = row 15.
            // In viewport height 10 at origin 5 (rows 5..15), row 15 is 1 row past bottom
            // (Overflowed).
            buffer_mut
                .inner
                .viewport
                .set_origin_pos(|pos| *pos = c_pos(0, 5));
            *buffer_mut.inner.c_caret = c_caret(c_row(15) + c_col(0));
        }

        validate_scroll_on_resize(EditorArgsMut::new(&mut buffer, &mut engine));

        // Viewport origin should adjust to 15 - 10 + 1 = 6.
        // Viewport caret should adjust to 9.
        // Canvas caret remains 15.
        assert_eq!(buffer.get_vp_origin().row_index, c_row(6));
        assert_eq!(buffer.get_vp_caret().row_index, vp_row(9));
        assert_eq!(buffer.get_c_caret().row_index, c_row(15));
    }

    // ┌───────────────────────────────────────────────────────────────────────────────┐
    // │ Horizontal scroll tests                                                       │
    // └───────────────────────────────────────────────────────────────────────────────┘

    /// Test: Caret column overflows `max_col` (line width).
    ///
    /// Setup: Line has 10 chars, caret at col 15 (past end of line).
    /// Expected: Caret col should be adjusted back to `max_col` (10).
    #[test]
    fn test_validate_horizontal_scroll_caret_col_overflows_max_c_col() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        // Create a line with 10 characters: "0123456789"
        buffer.init_with(["0123456789"]);

        let viewport = vp_height(10) + vp_width(20);

        // Set caret col to 15 (past line width of 10).
        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            *buffer_mut.inner.c_caret = c_caret(c_row(0) + c_col(15));
            buffer_mut
                .inner
                .viewport
                .set_origin_pos(|pos| *pos = c_pos(0, 0));
        }

        validate_scroll_on_resize(EditorArgsMut::new(&mut buffer, &mut engine));

        // Line width = 10, max_col.convert_to_index() = 9 (last char index).
        // Caret at 15 overflows, adjusted by: 15 - 9 = 6.
        // New c_caret = 15 - 6 = 9, c_caret = 9 + 0 = 9.
        assert_eq!(buffer.get_c_caret().col_index, c_col(10));
    }

    /// Test: Viewport origin column overflows `max_col` (line width).
    ///
    /// Setup: Line has 10 chars, `vp_origin` at col 15 (past end of line).
    /// Expected: `vp_origin` col should be adjusted back.
    #[test]
    fn test_validate_horizontal_scroll_viewport_origin_col_overflows_max_c_col() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        // Create a line with 10 characters.
        buffer.init_with(["0123456789"]);

        let viewport = vp_height(10) + vp_width(5);

        // Set vp_origin col to 15 (past line width of 10).
        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            *buffer_mut.inner.c_caret = c_caret(c_row(0) + c_col(15));
            buffer_mut
                .inner
                .viewport
                .set_origin_pos(|pos| *pos = c_pos(15, 0));
        }

        validate_scroll_on_resize(EditorArgsMut::new(&mut buffer, &mut engine));

        // Line width = 10, max_col.convert_to_index() = 9.
        // vp_origin at 15 overflows, adjusted by: 15 - 9 = 6.
        // New vp_origin = 15 - 6 = 9.
        assert_eq!(buffer.get_vp_origin().col_index, c_col(10));
    }

    /// Test: Caret within viewport horizontally.
    ///
    /// Setup: Line has 20 chars, caret at col 5, `vp_origin` at col 0, viewport
    /// width 10. Expected: No change needed (caret is within viewport).
    #[test]
    fn test_validate_horizontal_scroll_within_viewport() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        // Create a line with 20 characters.
        buffer.init_with(["01234567890123456789"]);

        let viewport = vp_height(10) + vp_width(10);

        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            *buffer_mut.inner.c_caret = c_caret(c_row(0) + c_col(5));
            buffer_mut
                .inner
                .viewport
                .set_origin_pos(|pos| *pos = c_pos(0, 0));
        }

        validate_scroll_on_resize(EditorArgsMut::new(&mut buffer, &mut engine));

        // Caret should remain at col 5, vp_origin at col 0.
        assert_eq!(buffer.get_vp_origin().col_index, c_col(0));
        assert_eq!(buffer.get_c_caret().col_index, c_col(5));
    }

    /// Test: Caret at edge of viewport (boundary condition).
    ///
    /// Setup: Line has 20 chars, `c_caret` at col 0, `vp_origin` at col 5.
    /// Result: `c_caret` = 5, which is exactly at `vp_origin` (left edge of
    /// viewport). Expected: No adjustment needed (caret is within viewport at left
    /// edge).
    ///
    /// Note: The "left of viewport" case (`c_caret` < `vp_origin`) cannot
    /// occur with non-negative `c_caret` values since `c_caret` =
    /// `c_caret` + `vp_origin`.
    #[test]
    fn test_validate_horizontal_scroll_at_left_edge() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        // Create a line with 20 characters.
        buffer.init_with(["01234567890123456789"]);

        let viewport = vp_height(10) + vp_width(10);

        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            *buffer_mut.inner.c_caret = c_caret(c_row(0) + c_col(5));
            buffer_mut
                .inner
                .viewport
                .set_origin_pos(|pos| *pos = c_pos(5, 0));
        }

        validate_scroll_on_resize(EditorArgsMut::new(&mut buffer, &mut engine));

        // c_caret = 0 + 5 = 5, which equals vp_origin (at left edge of
        // viewport). This is within viewport bounds [5, 15), so no adjustment.
        assert_eq!(buffer.get_vp_origin().col_index, c_col(5));
        assert_eq!(buffer.get_c_caret().col_index, c_col(5));
    }

    /// Test: Caret to right of viewport.
    ///
    /// Setup: Line has 30 chars, caret at col 25, `vp_origin` at col 5, viewport
    /// width 10. Expected: `vp_origin` adjusted to bring caret into view.
    #[test]
    fn test_validate_horizontal_scroll_right_of_viewport() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        // Create a line with 30 characters.
        buffer.init_with(["012345678901234567890123456789"]);

        let viewport = vp_height(10) + vp_width(10);

        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            // c_caret = 20 means caret is at viewport coordinate 20 (within
            // vp_height = 5) vp_origin. vp_origin = 5.
            // c_caret = 20 + 5 = 25.
            // Viewport shows cols 5-14 (vp_origin to vp_origin + width - 1).
            // Caret at 25 is to the right of viewport (25 >= 5 + 10 = 15).
            *buffer_mut.inner.c_caret = c_caret(c_row(0) + c_col(25));
            buffer_mut
                .inner
                .viewport
                .set_origin_pos(|pos| *pos = c_pos(5, 0));
        }

        validate_scroll_on_resize(EditorArgsMut::new(&mut buffer, &mut engine));

        // After adjustment, caret should be at right edge of viewport.
        // vp_origin should be adjusted: 25 - 10 + 1 = 16.
        // c_caret should be adjusted to viewport width - 1 = 9.
        // c_caret = 9 + 16 = 25 (unchanged canvas coordinate).
        assert_eq!(buffer.get_vp_origin().col_index, c_col(16));
        assert_eq!(buffer.get_c_caret().col_index, c_col(25));
    }

    /// Test: Horizontal caret underflow safety when adjusting `c_caret` relative
    /// offset.
    ///
    /// Setup: Line has 8 chars (`max_col` = 8).
    /// `vp_origin` = 10, `c_caret` = 5 => `c_caret` = 15.
    /// In this case both caret (15) and origin (10) overflow `max_col` (8).
    /// Origin is pulled back to `max_col` (8) and caret relative offset is reset to 0
    /// without underflowing.
    #[test]
    fn test_validate_horizontal_scroll_caret_underflow_safety() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        // Create a line with 8 characters: "01234567"
        buffer.init_with(["01234567"]);

        let viewport = vp_height(10) + vp_width(10);

        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            buffer_mut
                .inner
                .viewport
                .set_origin_pos(|pos| *pos = c_pos(10, 0));
            *buffer_mut.inner.c_caret = c_caret(c_row(0) + c_col(15));
        }

        // Must not panic on underflow!
        validate_scroll_on_resize(EditorArgsMut::new(&mut buffer, &mut engine));

        // Caret should be safely clamped to max_col (col 8).
        assert_eq!(buffer.get_c_caret().col_index, c_col(8));
    }

    #[test]
    fn test_validate_scroll_on_resize_simultaneous_2d() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        // 30 lines, each with 30 characters.
        buffer.init_with(vec!["012345678901234567890123456789"; 30]);

        let viewport = vp_height(10) + vp_width(10);

        {
            let buffer_mut = buffer.get_mut_no_drop(viewport);
            // Caret at (row 25, col 25) with origin at (row 5, col 5).
            // Both row 25 and col 25 are beyond the 10x10 viewport starting at (5, 5).
            *buffer_mut.inner.c_caret = c_caret(c_row(25) + c_col(25));
            buffer_mut
                .inner
                .viewport
                .set_origin_pos(|pos| *pos = c_pos(5, 5));
        }

        validate_scroll_on_resize(EditorArgsMut::new(&mut buffer, &mut engine));

        // Both vertical and horizontal origins should pan:
        // row: 25 - 10 + 1 = 16
        // col: 25 - 10 + 1 = 16
        assert_eq!(buffer.get_vp_origin(), c_pos(16, 16));
        assert_eq!(buffer.get_c_caret(), c_caret(c_row(25) + c_col(25)));
        assert_eq!(buffer.get_vp_caret(), vp_caret(vp_pos(9, 9)));
    }
}
