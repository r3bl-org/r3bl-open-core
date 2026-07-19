// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words Ohello

//! The functions in this module need information from both [`EditorBuffer`] and
//! [`EditorEngine`] in order to work.
//! - [`EditorBuffer`] provides [`EditorContent`].
//! - [`EditorEngine`] provides [`EditorEngine::viewport()`].
//!
//! # Scrolling not active
//!
//! Note that a caret is allowed to "go past" the end of its max index, so max index + 1
//! is a valid position. This is without taking scrolling into account. The max index must
//! still be within the viewport (max index) bounds.
//!
//! - Let's assume the caret is represented by "░".
//! - Think about typing "hello", and you expected the caret "░" to go past the end of the
//!   string "hello░".
//! - So the caret's col index is 5 in this case. Still within viewport bounds (max
//!   index). But greater than the line content max index (4).
//!
//! ```text
//! R ┌──────────┐
//! 0 ▸hello░    │
//!   └─────▴────┘
//!   C0123456789
//! ```
//!
//! # Scrolling active
//!
//! When scrolling is introduced (or activated), this behavior changes a bit. The caret
//! can't be allowed to go past the viewport bounds. So the caret must be adjusted to the
//! end of the line. In this case if the text is "helloHELLOhello" then the following will
//! be displayed (the caret is at the end of the line on top of the "o"). You can see this
//! in action in the test
//! [`editor_move_caret_home_end_overflow_viewport()`].
//! ```text
//! R ┌──────────┐
//! 0 ▸ELLOhello░│
//!   └─────────▴┘
//!   C0123456789
//! ```
//!
//! And viewport origin will be adjusted to show the end of the line. So the numbers will
//! be as follows:
//! - `vp_caret`: `vp_col(9)` + `vp_row(0)`
//! - `vp_origin`: `vp_col(6)` + `vp_row(0)`
//!
//! # Validation checks
//!
//! Once scrolling functions run, it is necessary to run the [Drop] impl for
//! [`EditorBufferMut`], which runs this function:
//! [`perform_validation_checks_after_mutation`]. Due to the nature of [`UTF-8`] and its
//! variable width characters, where the memory size is not the same as display size.
//!
//! Eg:
//! - `a` is 1 byte and 1 display width (unicode segment width display).
//! - `😄` is 3 bytes but it's display width is 2!
//!
//! To ensure that caret position and viewport origin positions are not in the middle of a
//! unicode segment character, we need to run the validation checks.
//!
//! [`editor_move_caret_home_end_overflow_viewport()`]: crate::tui::editor::editor_engine::caret_mut::tests::editor_move_caret_home_end_overflow_viewport
//! [`EditorBuffer`]: crate::EditorBuffer
//! [`EditorBufferMut`]: crate::validate_buffer_mut::EditorBufferMut
//! [`EditorContent`]: crate::EditorContent
//! [`EditorEngine::viewport()`]: crate::EditorEngine::viewport
//! [`EditorEngine`]: crate::EditorEngine
//! [`perform_validation_checks_after_mutation`]:
//!     crate::validate_buffer_mut::perform_validation_checks_after_mutation
//! [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8

use super::{SelectMode, caret_mut};
use crate::{CCaret, CCol, CRow, CWidth, CanvasCameraExt, CaretDirection,
            CursorBoundsCheck, CursorPositionBoundsStatus, EditorArgsMut, EditorBuffer,
            VPHeight, Viewport, c_col, c_row};
use std::cmp::Ordering;

pub mod horiz_caret_movement {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Increments the caret's column index by `col_amt`.
    ///
    /// See the [module-level documentation] for details on how scrolling and validation
    /// work.
    ///
    /// [module-level documentation]: super
    pub fn inc_c_caret_col_by(
        c_caret: &mut CCaret,
        viewport: &mut Viewport,
        col_amt: CWidth,
        line_display_width: CWidth,
    ) {
        // Get valid desired col index after incrementing by `col_amt`.
        let current_c_caret_col = c_caret.col_index;
        let new_c_caret_col = current_c_caret_col + col_amt;
        let valid_new_c_caret_col =
            line_display_width.clamp_cursor_position(new_c_caret_col);

        // Update the caret's col index.
        c_caret.col_index = valid_new_c_caret_col;

        // Pan the viewport origin horizontally so that the caret remains visible.
        viewport.pan_to_keep_coord_in_view(valid_new_c_caret_col);
    }

    /// Sets the caret's column index to `desired_col_index`.
    ///
    /// See the [module-level documentation] for details on how scrolling and validation
    /// work.
    ///
    /// [module-level documentation]: super
    pub fn set_c_caret_col_to(
        desired_col_index: CCol,
        c_caret: &mut CCaret,
        viewport: &mut Viewport,
        line_content_display_width: CWidth,
    ) {
        let c_caret_col = c_caret.col_index;
        match c_caret_col.cmp(&desired_col_index) {
            Ordering::Less => {
                let diff = desired_col_index - c_caret_col;
                inc_c_caret_col_by(c_caret, viewport, diff, line_content_display_width);
            }
            Ordering::Greater => {
                let diff = c_caret_col - desired_col_index;
                dec_c_caret_col_by(c_caret, viewport, diff);
            }
            Ordering::Equal => {}
        }
    }

    /// Decrements the caret's column index by `col_amt`.
    ///
    /// See the [module-level documentation] for details on how scrolling and validation
    /// work.
    ///
    /// [module-level documentation]: super
    pub fn dec_c_caret_col_by(
        c_caret: &mut CCaret,
        viewport: &mut Viewport,
        col_amt: CWidth,
    ) {
        c_caret.col_index -= col_amt;

        viewport.pan_to_keep_coord_in_view(c_caret.col_index);
    }

    /// Resets both the caret's column index and the viewport origin's column index to
    /// `0`.
    ///
    /// See the [module-level documentation] for details on how scrolling and validation
    /// work.
    ///
    /// [module-level documentation]: super
    pub fn reset_c_caret_col(c_caret: &mut CCaret, viewport: &mut Viewport) {
        c_caret.col_index.set(c_col(0));
        viewport.set_origin_pos(|pos| pos.col_index.set(c_col(0)));
    }
}

pub mod vert_caret_movement {
    use super::clip_caret_to_bounds::{clip_c_caret_row_to_content_height,
                                      clip_c_caret_to_content_width};
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Decrements the caret's row index.
    ///
    /// If this causes the caret to move above the top edge of the viewport,
    /// the viewport origin is automatically panned upwards to keep the caret visible.
    ///
    /// See the [module-level documentation] for details on how scrolling and validation
    /// work.
    ///
    /// [module-level documentation]: super
    pub fn dec_c_caret_row(c_caret: &mut CCaret, viewport: &mut Viewport) -> CRow {
        if c_caret.row_index > c_row(0) {
            c_caret.row_index -= 1;
            viewport.pan_to_keep_coord_in_view(c_caret.row_index);
        }
        c_caret.row_index
    }

    /// Increments the caret's row index.
    ///
    /// If this causes the caret to move below the bottom edge of the viewport, the
    /// viewport origin is automatically panned downwards
    /// ([`pan_to_keep_coord_in_view()`]) to keep the caret visible.
    ///
    /// See the [module-level documentation] for details on how scrolling and validation
    /// work.
    ///
    /// [`pan_to_keep_coord_in_view()`]: crate::CanvasCameraExt::pan_to_keep_coord_in_view
    /// [module-level documentation]: super
    pub fn inc_c_caret_row(c_caret: &mut CCaret, viewport: &mut Viewport) -> CRow {
        c_caret.row_index += 1;
        viewport.pan_to_keep_coord_in_view(c_caret.row_index);
        c_caret.row_index
    }

    /// Changes the caret's row index by `row_amt` in the given `direction`.
    ///
    /// See the [module-level documentation] for details on how scrolling and validation
    /// work.
    ///
    /// [module-level documentation]: super
    pub fn change_c_caret_row_by(
        args: EditorArgsMut<'_>,
        row_amt: VPHeight,
        direction: CaretDirection,
    ) {
        let EditorArgsMut { buffer, engine } = args;

        let c_caret_row = buffer.get_c_caret().row_index;

        let target_row = match direction {
            CaretDirection::Down => {
                let mut desired = c_caret_row + row_amt;
                clip_c_caret_row_to_content_height(buffer, &mut desired);
                Some(desired)
            }
            CaretDirection::Up => Some(c_caret_row - row_amt),
            _ => None,
        };

        if let Some(desired_c_caret_row) = target_row {
            let buffer_mut = buffer.get_mut(engine.viewport());
            // Move caret to the desired row.
            buffer_mut.inner.c_caret.row_index = desired_c_caret_row;
            // Pan the viewport origin vertically so that the desired caret row remains
            // visible.
            buffer_mut
                .inner
                .viewport
                .pan_to_keep_coord_in_view(desired_c_caret_row);
        }

        clip_c_caret_to_content_width(EditorArgsMut::new(buffer, engine));
    }
}

pub mod clip_caret_to_bounds {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Clips the caret's column index so it does not exceed the line's display width.
    ///
    /// If the caret position is beyond the bounds of the line content, it moves the caret
    /// to the end of the line.
    ///
    /// See the [module-level documentation] for details on how scrolling and validation
    /// work.
    ///
    /// [module-level documentation]: super
    pub fn clip_c_caret_to_content_width(args: EditorArgsMut<'_>) {
        let EditorArgsMut { buffer, engine } = args;

        let c_caret_col = buffer.get_c_caret().col_index;
        let line_display_width = buffer.get_line_display_width_at_c_caret();

        if line_display_width.check_cursor_position_bounds(c_caret_col)
            == CursorPositionBoundsStatus::Beyond
        {
            caret_mut::to_end_of_line(buffer, engine, SelectMode::Disabled);
        }
    }

    /// Clips `desired_c_caret_row_index` so it does not exceed the buffer's max row
    /// index.
    ///
    /// See the [module-level documentation] for details on how scrolling and validation
    /// work.
    ///
    /// [module-level documentation]: super
    pub fn clip_c_caret_row_to_content_height(
        buffer: &EditorBuffer,
        desired_c_caret_row_index: &mut CRow,
    ) {
        let max_row_index = buffer.get_max_row_index();
        if *desired_c_caret_row_index > max_row_index {
            *desired_c_caret_row_index = max_row_index;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{clip_caret_to_bounds::*, horiz_caret_movement::*, vert_caret_movement::*};
    use crate::{CaretDirection, DEFAULT_SYN_HI_FILE_EXT, EditorBuffer, EditorEvent,
                FileExtensionToken, GCStringOwned, assert_eq2, c_caret, c_col, c_height,
                c_pos, c_row, c_width, clipboard_test_fixtures::TestClipboard,
                editor::test_fixtures_editor::mock_real_objects_for_editor, vp_caret,
                vp_col, vp_height, vp_row, vp_width};

    #[test]
    fn editor_scroll_vertical() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "hello" many times.
        let max_lines = 20_usize;
        for count in 1..=max_lines {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![
                    EditorEvent::InsertString(format!("{count}: {}", "hello")),
                    EditorEvent::InsertNewLine,
                ],
                &mut TestClipboard::default(),
            );
        }
        assert_eq2!(buffer.get_lines().get_line_count(), c_height(max_lines + 1)); /* One empty line after content */

        // Press up 12 times.
        for _ in 1..12 {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Up)],
                &mut TestClipboard::default(),
            );
        }
        assert_eq2!(buffer.get_vp_caret(), vp_caret(vp_col(0) + vp_row(0)));
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(0) + c_row(9)));
        assert_eq2!(buffer.get_vp_origin(), c_pos(0, 9));

        // Press down 9 times.
        for _ in 1..9 {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Down)],
                &mut TestClipboard::default(),
            );
        }
        assert_eq2!(buffer.get_vp_caret(), vp_caret(vp_col(0) + vp_row(8)));
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(0) + c_row(17)));
        assert_eq2!(buffer.get_vp_origin(), c_pos(0, 9));
    }

    #[test]
    fn editor_scroll_horizontal() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert a long line of text.
        let max_cols = 15;
        for count in 1..=max_cols {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::InsertString(format!("{count}"))],
                &mut TestClipboard::default(),
            );
        }
        assert_eq2!(buffer.get_lines().get_line_count(), c_height(1));
        assert_eq2!(buffer.get_vp_caret(), vp_caret(vp_col(9) + vp_row(0)));
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(21) + c_row(0)));
        assert_eq2!(buffer.get_vp_origin(), c_pos(12, 0));

        // Press left 5 times.
        for _ in 1..5 {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Left)],
                &mut TestClipboard::default(),
            );
        }
        assert_eq2!(buffer.get_vp_caret(), vp_caret(vp_col(5) + vp_row(0)));
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(17) + c_row(0)));
        assert_eq2!(buffer.get_vp_origin(), c_pos(12, 0));

        // Press right 3 times.
        for _ in 1..3 {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                &mut TestClipboard::default(),
            );
        }
        assert_eq2!(buffer.get_vp_caret(), vp_caret(vp_col(7) + vp_row(0)));
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(19) + c_row(0)));
        assert_eq2!(buffer.get_vp_origin(), c_pos(12, 0));
    }

    /// A jumbo emoji is a combination of 2 emoji (each one of which has > 1 display
    /// width, or unicode width).
    ///
    /// 🙏🏽 = U+1F64F + U+1F3FD
    /// 1. <https://unicodeplus.com/U+1F64F>
    /// 2. <https://unicodeplus.com/U+1F3FD>
    #[allow(clippy::too_many_lines)]
    #[test]
    fn editor_scroll_right_horizontal_long_line_with_jumbo_emoji() {
        // Setup.
        let test_vp_width = vp_width(65);
        let vp_height = vp_height(2);
        let window_size = test_vp_width + vp_height;
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine =
            mock_real_objects_for_editor::make_editor_engine_with_bounds(window_size);

        let long_line = "# Did he take those two new droids with him? They hit accelerator.🙏🏽😀░ We will deal with your Rebel friends. Commence primary ignition.🙏🏽😀░";
        let _long_line_gcs = GCStringOwned::from(long_line);
        buffer.init_with([long_line]);

        // Setup assertions.
        {
            assert_eq2!(vp_width(2), GCStringOwned::from("🙏🏽").width());
            assert_eq2!(buffer.get_lines().get_line_count(), c_height(1));
            assert_eq2!(
                buffer
                    .get_lines()
                    .get_line_content(c_row(0))
                    .expect("conversion error"),
                long_line
            );
            let us = buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error");
            assert_eq2!(us, long_line);
            assert_eq2!(buffer.get_vp_caret(), vp_caret(vp_col(0) + vp_row(0)));
            assert_eq2!(buffer.get_c_caret(), c_caret(c_col(0) + c_row(0)));
            assert_eq2!(buffer.get_vp_origin(), c_pos(0, 0));
        }

        // Press right 67 times. The caret should correctly jump the width of the jumbo
        // emoji (🙏🏽) on the **RIGHT** of viewport and select it.
        {
            let num_of_right = 67;
            for _ in 1..num_of_right {
                EditorEvent::apply_editor_events::<(), ()>(
                    &mut engine,
                    &mut buffer,
                    vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                    &mut TestClipboard::default(),
                );
            }
            assert_eq2!(buffer.get_vp_origin(), c_pos(4, 0));
            assert_eq2!(buffer.get_c_caret(), c_caret(c_col(66) + c_row(0)));
            // Right of viewport.
            let display_col_index = buffer.get_c_caret().col_index;
            let result = buffer
                .get_lines()
                .get_string_at_col(c_row(0), display_col_index);
            assert_eq2!(result.expect("conversion error").string.string, "🙏🏽");

            // Press right 1 more time. The caret should correctly jump the width of "😀".
            // from 68 to 70.
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                &mut TestClipboard::default(),
            );
            assert_eq2!(buffer.get_c_caret(), c_caret(c_col(68) + c_row(0)));
            // Right of viewport.
            let display_col_index = buffer.get_c_caret().col_index;
            let result = buffer
                .get_lines()
                .get_string_at_col(c_row(0), display_col_index);
            assert_eq2!(result.expect("conversion error").string.string, "😀");
        }

        // Press right 60 more times. The **LEFT** side of the viewport should be at the
        // jumbo emoji.
        {
            for _ in 1..60 {
                EditorEvent::apply_editor_events::<(), ()>(
                    &mut engine,
                    &mut buffer,
                    vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                    &mut TestClipboard::default(),
                );
            }
            assert_eq2!(buffer.get_vp_caret(), vp_caret(vp_col(64) + vp_row(0)));
            assert_eq2!(buffer.get_c_caret(), c_caret(c_col(128) + c_row(0)));
            assert_eq2!(buffer.get_vp_origin(), c_pos(64, 0));
            // Start of viewport.
            let display_col_index = buffer.get_vp_origin().col_index;
            let result = buffer
                .get_lines()
                .get_string_at_col(c_row(0), display_col_index);
            assert_eq2!(result.expect("conversion error").string.string, "r");
        }

        // Press right 1 more time. It should jump the jumbo emoji at the start of the.
        // line (and not just 1 character width). This moves the caret and the scroll
        // offset to make sure that the emoji at the start of the line can be displayed
        // properly.
        {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                &mut TestClipboard::default(),
            );
            assert_eq2!(buffer.get_vp_caret(), vp_caret(vp_col(64) + vp_row(0)));
            assert_eq2!(buffer.get_c_caret(), c_caret(c_col(129) + c_row(0)));
            assert_eq2!(buffer.get_vp_origin(), c_pos(65, 0));
            // Start of viewport.
            let display_col_index = buffer.get_vp_origin().col_index;
            let result = buffer
                .get_lines()
                .get_string_at_col(c_row(0), display_col_index);
            assert_eq2!(result.expect("conversion error").string.string, ".");
        }

        // Press right 4 times. It should jump the emoji at the start of the line (and not
        // just 1 character width); this moves the viewport origin to make sure that the
        // emoji can be properly displayed & it moves the caret too.
        {
            for _ in 1..4 {
                EditorEvent::apply_editor_events::<(), ()>(
                    &mut engine,
                    &mut buffer,
                    vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                    &mut TestClipboard::default(),
                );
            }
            // Start of viewport.
            let display_col_index = buffer.get_vp_origin().col_index;
            let result = buffer
                .get_lines()
                .get_string_at_col(c_row(0), display_col_index);
            assert_eq2!(result.expect("conversion error").string.string, "😀");
        }

        // Press right 2 more times to move caret off the right edge of the viewport. It
        // should scroll past the emoji.
        {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![
                    EditorEvent::MoveCaret(CaretDirection::Right),
                    EditorEvent::MoveCaret(CaretDirection::Right),
                ],
                &mut TestClipboard::default(),
            );
            // Start of viewport.
            let display_col_index = buffer.get_vp_origin().col_index;
            let result = buffer
                .get_lines()
                .get_string_at_col(c_row(0), display_col_index);
            assert_eq2!(result.expect("conversion error").string.string, "░");
        }
    }

    #[test]
    fn test_dec_caret_col_by_saturates_and_pans() {
        use super::*;

        let mut c_caret = c_caret(c_col(5) + c_row(0));
        let mut viewport = Viewport::new(c_pos(5, 0), vp_width(20) + vp_height(1));

        // Decrement by 10 columns (exceeds current col 5).
        dec_c_caret_col_by(&mut c_caret, &mut viewport, c_width(10));

        assert_eq2!(c_caret.col_index, c_col(0));
        assert_eq2!(viewport.get_origin_pos().col_index, c_col(0));
    }

    #[test]
    fn test_dec_caret_row_at_zero_is_noop() {
        use super::*;

        let mut c_caret = c_caret(c_col(0) + c_row(0));
        let mut viewport = Viewport::new(c_pos(0, 0), vp_width(1) + vp_height(10));

        let row_res = dec_c_caret_row(&mut c_caret, &mut viewport);

        assert_eq2!(row_res, c_row(0));
        assert_eq2!(c_caret.row_index, c_row(0));
        assert_eq2!(viewport.get_origin_pos().row_index, c_row(0));
    }

    #[test]
    fn test_reset_caret_col() {
        use super::*;

        let mut c_caret = c_caret(c_col(15) + c_row(2));
        let mut viewport = Viewport::new(c_pos(10, 2), vp_width(1) + vp_height(1));

        reset_c_caret_col(&mut c_caret, &mut viewport);

        assert_eq2!(c_caret.col_index, c_col(0));
        assert_eq2!(viewport.get_origin_pos().col_index, c_col(0));
    }

    #[test]
    fn test_clip_caret_row_to_content_height() {
        use super::*;

        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        buffer.init_with(["line 1", "line 2", "line 3"]);

        let mut desired_row = c_row(100);
        clip_c_caret_row_to_content_height(&buffer, &mut desired_row);

        assert_eq2!(desired_row, c_row(2));
    }

    #[test]
    fn test_inc_caret_col_by_clips_and_pans() {
        use super::*;

        let mut c_caret = c_caret(c_col(0) + c_row(0));
        let mut viewport = Viewport::new(c_pos(0, 0), vp_width(5) + vp_height(1));

        // Move right by 15 on a line of max width 10, with viewport width 5.
        inc_c_caret_col_by(&mut c_caret, &mut viewport, c_width(15), c_width(10));

        // Caret clipped to max_col (10).
        assert_eq2!(c_caret.col_index, c_col(10));
        // Viewport origin panned right so index 10 is visible (origin = 10 - 4 = 6).
        assert_eq2!(viewport.get_origin_pos().col_index, c_col(6));
    }

    #[test]
    fn test_inc_caret_row_pans_when_overflowed() {
        use super::*;

        let mut c_caret = c_caret(c_col(0) + c_row(4));
        let mut viewport = Viewport::new(c_pos(0, 0), vp_width(1) + vp_height(5));

        // Increment row when height is 5 (visible rows 0..=4). Moving to row 5 causes
        // overflow.
        let new_row = inc_c_caret_row(&mut c_caret, &mut viewport);

        assert_eq2!(new_row, c_row(5));
        assert_eq2!(c_caret.row_index, c_row(5));
        assert_eq2!(viewport.get_origin_pos().row_index, c_row(1));
    }

    #[test]
    fn test_dec_caret_row_decrements_and_pans() {
        use super::*;

        let mut c_caret1 = c_caret(c_col(0) + c_row(5));
        // Viewport origin is at row 2, so visible rows are 2.. (vp_height = 5, rows 2, 3,
        // 4, 5, 6)
        let mut viewport = Viewport::new(c_pos(0, 2), vp_width(1) + vp_height(5));

        // Decrement row when height is 5
        let new_row = dec_c_caret_row(&mut c_caret1, &mut viewport);

        assert_eq2!(new_row, c_row(4));
        assert_eq2!(c_caret1.row_index, c_row(4));
        // Viewport origin should not change as row 4 is still visible (2 <= 4 < 2 + 5)
        assert_eq2!(viewport.get_origin_pos().row_index, c_row(2));

        // Now move caret out of bounds at the top
        let mut c_caret2 = c_caret(c_col(0) + c_row(2));
        let new_row2 = dec_c_caret_row(&mut c_caret2, &mut viewport);

        assert_eq2!(new_row2, c_row(1));
        assert_eq2!(c_caret2.row_index, c_row(1));
        // Viewport origin pans up because 1 < 2
        assert_eq2!(viewport.get_origin_pos().row_index, c_row(1));
    }

    #[test]
    fn test_set_caret_col_to() {
        use super::*;

        let mut c_caret = c_caret(c_col(5) + c_row(0));
        let mut viewport = Viewport::new(c_pos(2, 0), vp_width(10) + vp_height(1));
        let line_content_display_width = c_width(20);

        // Move right (Greater)
        set_c_caret_col_to(
            c_col(8),
            &mut c_caret,
            &mut viewport,
            line_content_display_width,
        );
        assert_eq2!(c_caret.col_index, c_col(8));
        assert_eq2!(viewport.get_origin_pos().col_index, c_col(2));

        // Move right and pan
        set_c_caret_col_to(
            c_col(15),
            &mut c_caret,
            &mut viewport,
            line_content_display_width,
        );
        assert_eq2!(c_caret.col_index, c_col(15));
        assert_eq2!(viewport.get_origin_pos().col_index, c_col(6)); // 15 - 10 + 1 = 6

        // Move left (Less)
        set_c_caret_col_to(
            c_col(3),
            &mut c_caret,
            &mut viewport,
            line_content_display_width,
        );
        assert_eq2!(c_caret.col_index, c_col(3));
        assert_eq2!(viewport.get_origin_pos().col_index, c_col(3)); // Panned left to include 3
    }

    #[test]
    fn test_clip_caret_to_content_width() {
        use super::*;

        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        buffer.init_with(["hello", "world"]);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Move caret past the end of the line (width is 5).
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            *buffer_mut.inner.c_caret = c_caret(c_col(10) + c_row(0));
        }

        let args = EditorArgsMut::new(&mut buffer, &mut engine);
        clip_c_caret_to_content_width(args);

        // Caret should be clipped to the end of the line (width 5).
        assert_eq2!(buffer.get_c_caret().col_index, c_col(5));
    }

    #[test]
    fn test_change_caret_row_by() {
        use super::*;

        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        buffer.init_with(["line 1", "line 2", "line 3", "line 4", "line 5"]);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Down
        let args = EditorArgsMut::new(&mut buffer, &mut engine);
        change_c_caret_row_by(args, vp_height(2), CaretDirection::Down);
        assert_eq2!(buffer.get_c_caret().row_index, c_row(2));

        // Down overflow (clips to max row 4)
        let args = EditorArgsMut::new(&mut buffer, &mut engine);
        change_c_caret_row_by(args, vp_height(10), CaretDirection::Down);
        assert_eq2!(buffer.get_c_caret().row_index, c_row(4));

        // Up
        let args = EditorArgsMut::new(&mut buffer, &mut engine);
        change_c_caret_row_by(args, vp_height(1), CaretDirection::Up);
        assert_eq2!(buffer.get_c_caret().row_index, c_row(3));
    }
}
