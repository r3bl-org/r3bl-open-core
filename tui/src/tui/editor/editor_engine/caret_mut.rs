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
use super::{SelectMode, scroll_editor_content};
use crate::{CaretDirection, EditorArgsMut, EditorBuffer, EditorEngine,
            SegStringOwned,
            caret_locate::{self, CaretColLocationInLine, CaretRowLocationInBuffer,
                           locate_col},
            caret_mut, caret_scr_adj, caret_scroll_index, col, empty_check_early_return,
            multiline_disabled_check_early_return, row,
            ContainsWideSegments,
            width};

pub fn up(buffer: &mut EditorBuffer, engine: &mut EditorEngine, sel_mod: SelectMode) {
    empty_check_early_return!(buffer, @Nothing);
    multiline_disabled_check_early_return!(engine, @Nothing);

    // This is only set if sel_mod is enabled.
    let maybe_prev_caret = sel_mod.get_caret_scr_adj(buffer);

    match caret_locate::locate_row(buffer) {
        CaretRowLocationInBuffer::AtTop => {
            // Do nothing if the caret (scroll adjusted) is at the top.
            if buffer.get_caret_scr_adj().col_index != col(0) {
                // When buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let buffer_mut = buffer.get_mut(engine.viewport());

                    scroll_editor_content::reset_caret_col(
                        buffer_mut.inner.caret_raw,
                        buffer_mut.inner.scr_ofs,
                    );
                }
            }
        }
        CaretRowLocationInBuffer::AtBottom | CaretRowLocationInBuffer::InMiddle => {
            {
                // When buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    // There is a line above the caret.
                    let buffer_mut = buffer.get_mut(engine.viewport());

                    scroll_editor_content::dec_caret_row(
                        buffer_mut.inner.caret_raw,
                        buffer_mut.inner.scr_ofs,
                    );
                }

                scroll_editor_content::clip_caret_to_content_width(EditorArgsMut {
                    engine,
                    buffer,
                });
            }
        }
    }

    // This is only set if sel_mod is enabled.
    let maybe_curr_caret = sel_mod.get_caret_scr_adj(buffer);

    // This is only runs if sel_mod is enabled.
    sel_mod.update_selection_based_on_caret_movement_in_multiple_lines(
        buffer,
        maybe_prev_caret,
        maybe_curr_caret,
    );
}

pub fn page_up(
    buffer: &mut EditorBuffer,
    engine: &mut EditorEngine,
    sel_mod: SelectMode,
) {
    empty_check_early_return!(buffer, @Nothing);
    multiline_disabled_check_early_return!(engine, @Nothing);

    // This is only set if sel_mod is enabled.
    let maybe_prev_caret = sel_mod.get_caret_scr_adj(buffer);

    let viewport_height = engine.viewport().row_height;
    scroll_editor_content::change_caret_row_by(
        EditorArgsMut { engine, buffer },
        viewport_height,
        CaretDirection::Up,
    );

    // This is only set if sel_mod is enabled.
    let maybe_curr_caret = sel_mod.get_caret_scr_adj(buffer);

    // This is only runs if sel_mod is enabled.
    sel_mod.update_selection_based_on_caret_movement_in_multiple_lines(
        buffer,
        maybe_prev_caret,
        maybe_curr_caret,
    );
}

pub fn down(
    buffer: &mut EditorBuffer,
    engine: &mut EditorEngine,
    sel_mod: SelectMode,
) {
    empty_check_early_return!(buffer, @Nothing);
    multiline_disabled_check_early_return!(engine, @Nothing);

    // This is only set if sel_mod is enabled.
    let maybe_prev_caret = sel_mod.get_caret_scr_adj(buffer);

    if buffer.next_line_below_caret_to_string().is_some() {
        // When buffer_mut goes out of scope, it will be dropped &
        // validation performed.
        {
            // There is a line below the caret.
            let buffer_mut = buffer.get_mut(engine.viewport());

            scroll_editor_content::inc_caret_row(
                buffer_mut.inner.caret_raw,
                buffer_mut.inner.scr_ofs,
                buffer_mut.inner.vp.row_height,
            );
        }

        scroll_editor_content::clip_caret_to_content_width(EditorArgsMut {
            engine,
            buffer,
        });
    } else {
        // Move to the end of the line.
        caret_mut::to_end_of_line(buffer, engine, sel_mod);
    }

    // This is only set if sel_mod is enabled.
    let maybe_curr_caret = sel_mod.get_caret_scr_adj(buffer);

    // This is only runs if sel_mod is enabled.
    sel_mod.update_selection_based_on_caret_movement_in_multiple_lines(
        buffer,
        maybe_prev_caret,
        maybe_curr_caret,
    );
}

pub fn page_down(
    buffer: &mut EditorBuffer,
    engine: &mut EditorEngine,
    sel_mod: SelectMode,
) {
    empty_check_early_return!(buffer, @Nothing);
    multiline_disabled_check_early_return!(engine, @Nothing);

    // This is only set if sel_mod is enabled.
    let maybe_prev_caret = sel_mod.get_caret_scr_adj(buffer);

    let viewport_height = engine.viewport().row_height;
    scroll_editor_content::change_caret_row_by(
        EditorArgsMut { engine, buffer },
        viewport_height,
        CaretDirection::Down,
    );

    // This is only set if sel_mod is enabled.
    let maybe_curr_caret = sel_mod.get_caret_scr_adj(buffer);

    // This is only runs if sel_mod is enabled.
    sel_mod.update_selection_based_on_caret_movement_in_multiple_lines(
        buffer,
        maybe_prev_caret,
        maybe_curr_caret,
    );
}

/// Depending on [`SelectMode`], this acts as a:
/// - Convenience function for simply calling [left] repeatedly.
/// - Convenience function for simply calling [`scroll_editor_content::reset_caret_col`].
pub fn to_start_of_line(
    buffer: &mut EditorBuffer,
    engine: &mut EditorEngine,
    sel_mod: SelectMode,
) {
    empty_check_early_return!(buffer, @Nothing);

    match sel_mod {
        SelectMode::Enabled => {
            let caret = buffer.get_caret_scr_adj();
            for _ in 0..caret.col_index.value {
                left(buffer, engine, sel_mod);
            }
        }
        SelectMode::Disabled => {
            // When buffer_mut goes out of scope, it will be dropped &
            // validation performed.
            {
                let buffer_mut = buffer.get_mut(engine.viewport());

                scroll_editor_content::reset_caret_col(
                    buffer_mut.inner.caret_raw,
                    buffer_mut.inner.scr_ofs,
                );
            }
        }
    }
}

/// Depending on [`SelectMode`], this acts as a:
/// - Convenience function for simply calling [right] repeatedly.
/// - Convenience function for simply calling [`scroll_editor_content::set_caret_col_to`].
pub fn to_end_of_line(
    buffer: &mut EditorBuffer,
    engine: &mut EditorEngine,
    sel_mod: SelectMode,
) {
    empty_check_early_return!(buffer, @Nothing);

    match sel_mod {
        SelectMode::Enabled => {
            let caret = buffer.get_caret_scr_adj();
            let line_display_width = buffer.get_line_display_width_at_caret_scr_adj();
            for _ in caret.col_index.value..line_display_width.value {
                right(buffer, engine, sel_mod);
            }
        }
        SelectMode::Disabled => {
            let line_display_width = buffer.get_line_display_width_at_row_index(
                buffer.get_caret_scr_adj().row_index,
            );

            // When buffer_mut goes out of scope, it will be dropped &
            // validation performed.
            {
                let buffer_mut = buffer.get_mut(engine.viewport());

                scroll_editor_content::set_caret_col_to(
                    // This caret col index goes 1 past the end of the line width, ie:
                    // - `line_display_width` which is the same as:
                    // - `line_display_width.convert_to_col_index() /*-1*/ + 1`
                    caret_scroll_index::col_index_for_width(line_display_width),
                    buffer_mut.inner.caret_raw,
                    buffer_mut.inner.scr_ofs,
                    buffer_mut.inner.vp.col_width,
                    line_display_width,
                );
            }
        }
    }
}

pub fn select_all(buffer: &mut EditorBuffer, sel_mod: SelectMode) {
    empty_check_early_return!(buffer, @Nothing);

    let max_row_index = buffer.get_max_row_index();

    let max_col_index = {
        let last_line_display_width =
            buffer.get_line_display_width_at_row_index(max_row_index);

        // This caret col index goes 1 past the end of the line width, ie:
        // - `last_line_display_width` which is the same as:
        // - `last_line_display_width.convert_to_col_index() /*-1*/ + 1`
        caret_scroll_index::col_index_for_width(last_line_display_width)
    };

    buffer.clear_selection();

    sel_mod.update_selection_based_on_caret_movement_in_multiple_lines(
        buffer,
        Some(caret_scr_adj(col(0) + row(0))),
        Some(caret_scr_adj(max_col_index + max_row_index)),
    );
}

/// ```text
/// Caret : ⮬, ❱
///
/// Start of line:
/// R ┌──────────┐
/// 0 ❱abcab     │
///   └⮬─────────┘
///   C0123456789
///
/// Middle of line:
/// R ┌──────────┐
/// 0 ❱abcab     │
///   └───⮬──────┘
///   C0123456789
///
/// End of line:
/// R ┌──────────┐
/// 0 ❱abcab     │
///   └─────⮬────┘
///   C0123456789
/// ```
pub fn right(
    buffer: &mut EditorBuffer,
    engine: &mut EditorEngine,
    sel_mod: SelectMode,
) {
    empty_check_early_return!(buffer, @Nothing);

    let line_is_empty = buffer.line_at_caret_is_empty();

    let caret_col_loc_in_line = locate_col(buffer);

    // This is only set if sel_mod is enabled.
    let maybe_prev_caret = sel_mod.get_caret_scr_adj(buffer);

    match caret_col_loc_in_line {
        // Special case of empty line w/ caret at start.
        CaretColLocationInLine::AtStart if line_is_empty => {
            right_helper::right_at_end(buffer, engine);
        }
        CaretColLocationInLine::AtStart | CaretColLocationInLine::InMiddle => {
            right_helper::right_normal(buffer, engine);
        }
        CaretColLocationInLine::AtEnd => {
            right_helper::right_at_end(buffer, engine);
        }
    }

    // This is only set if sel_mod is enabled.
    let maybe_curr_caret = sel_mod.get_caret_scr_adj(buffer);

    // This is only runs if sel_mod is enabled.
    sel_mod.handle_selection_single_line_caret_movement(
        buffer,
        maybe_prev_caret,
        maybe_curr_caret,
    );
}

mod right_helper {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// 1. Check for wide unicode character to the right of the caret.
    /// 2. [`validate::apply_change`] checks for wide unicode character at the start of
    ///    the viewport.
    pub fn right_normal(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
    ) -> Option<()> {
        let SegStringOwned {
            width: unicode_width_at_caret,
            ..
        } = buffer.string_at_caret()?;

        let max_display_width = buffer.get_line_display_width_at_caret_scr_adj();

        let maybe_char_to_right_of_caret = buffer.string_to_right_of_caret();

        match maybe_char_to_right_of_caret {
            Some(right_of_caret_seg_string) => {
                let chunk_to_right_of_caret_gcs = right_of_caret_seg_string.string;

                match chunk_to_right_of_caret_gcs.contains_wide_segments() {
                    ContainsWideSegments::Yes => {
                        let jump_by_col_width = chunk_to_right_of_caret_gcs.display_width
                            + unicode_width_at_caret;
                        let move_left_by_amt = chunk_to_right_of_caret_gcs.display_width;
                        {
                            let buffer_mut = buffer.get_mut(engine.viewport());

                            scroll_editor_content::inc_caret_col_by(
                                buffer_mut.inner.caret_raw,
                                buffer_mut.inner.scr_ofs,
                                jump_by_col_width,
                                max_display_width,
                                buffer_mut.inner.vp.col_width,
                            );
                        }
                        if move_left_by_amt > width(0) {
                            // When buffer_mut goes out of scope, it will be dropped &
                            // validation performed.
                            {
                                let buffer_mut = buffer.get_mut(engine.viewport());

                                scroll_editor_content::dec_caret_col_by(
                                    buffer_mut.inner.caret_raw,
                                    buffer_mut.inner.scr_ofs,
                                    move_left_by_amt,
                                );
                            }
                        }
                    }
                    ContainsWideSegments::No => {
                        // When buffer_mut goes out of scope, it will be dropped &
                        // validation performed.
                        {
                            let buffer_mut = buffer.get_mut(engine.viewport());

                            scroll_editor_content::inc_caret_col_by(
                                buffer_mut.inner.caret_raw,
                                buffer_mut.inner.scr_ofs,
                                unicode_width_at_caret,
                                max_display_width,
                                buffer_mut.inner.vp.col_width,
                            );
                        }
                    }
                }
            }

            None => {
                // When buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let buffer_mut = buffer.get_mut(engine.viewport());

                    scroll_editor_content::inc_caret_col_by(
                        buffer_mut.inner.caret_raw,
                        buffer_mut.inner.scr_ofs,
                        unicode_width_at_caret,
                        max_display_width,
                        buffer_mut.inner.vp.col_width,
                    );
                }
            }
        }

        None
    }

    pub fn right_at_end(buffer: &mut EditorBuffer, engine: &mut EditorEngine) {
        if buffer.next_line_below_caret_to_string().is_some() {
            // If there is a line below the caret, move the caret to the start of
            // the next line.

            // When buffer_mut goes out of scope, it will be dropped &
            // validation performed.
            {
                let buffer_mut = buffer.get_mut(engine.viewport());

                scroll_editor_content::inc_caret_row(
                    buffer_mut.inner.caret_raw,
                    buffer_mut.inner.scr_ofs,
                    buffer_mut.inner.vp.row_height,
                );

                scroll_editor_content::reset_caret_col(
                    buffer_mut.inner.caret_raw,
                    buffer_mut.inner.scr_ofs,
                );
            }
        }
    }
}

/// ```text
/// Caret : ⮬, ❱
///
/// Start of line (move to end of previous line if available):
/// R ┌──────────┐
/// 0 ❱line1     │
/// 1 │line2     │
///   └⮬─────────┘
///   C0123456789
///
/// Middle of line (move left by one character):
/// R ┌──────────┐
/// 0 ❱line1     │
///   └──⮬───────┘
///   C0123456789
///
/// End of line (move left by one character):
/// R ┌──────────┐
/// 0 ❱line1     │
///   └────⮬─────┘
///   C0123456789
/// ```
pub fn left(
    buffer: &mut EditorBuffer,
    editor: &mut EditorEngine,
    sel_mod: SelectMode,
) -> Option<()> {
    empty_check_early_return!(buffer, @None);

    // This is only set if sel_mod is enabled.
    let maybe_prev_caret = sel_mod.get_caret_scr_adj(buffer);

    match locate_col(buffer) {
        CaretColLocationInLine::AtStart => {
            left_helper::left_at_start(buffer, editor);
        }
        CaretColLocationInLine::AtEnd => {
            left_helper::left_at_end(buffer, editor);
        }
        CaretColLocationInLine::InMiddle => {
            left_helper::left_in_middle(buffer, editor);
        }
    }

    // This is only set if sel_mod is enabled.
    let maybe_curr_caret = sel_mod.get_caret_scr_adj(buffer);

    // This is only runs if sel_mod is enabled.
    sel_mod.handle_selection_single_line_caret_movement(
        buffer,
        maybe_prev_caret,
        maybe_curr_caret,
    );

    None
}

mod left_helper {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub fn left_at_start(buffer: &mut EditorBuffer, editor: &mut EditorEngine) {
        if buffer.prev_line_above_caret().is_some() {
            // If there is a line above the caret, move the caret to the end of
            // the previous line.

            // When buffer_mut goes out of scope, it will be dropped &
            // validation performed.
            {
                let buffer_mut = buffer.get_mut(editor.viewport());

                scroll_editor_content::dec_caret_row(
                    buffer_mut.inner.caret_raw,
                    buffer_mut.inner.scr_ofs,
                );
            }

            caret_mut::to_end_of_line(buffer, editor, SelectMode::Disabled);
        }
    }

    pub fn left_at_end(buffer: &mut EditorBuffer, editor: &mut EditorEngine) {
        if let Some(seg_slice) = buffer.string_at_end_of_line_at_caret_scr_adj() {
            // When buffer_mut goes out of scope, it will be dropped &
            // validation performed.
            {
                let buffer_mut = buffer.get_mut(editor.viewport());

                scroll_editor_content::dec_caret_col_by(
                    buffer_mut.inner.caret_raw,
                    buffer_mut.inner.scr_ofs,
                    seg_slice.width,
                );
            }
        }
    }

    pub fn left_in_middle(buffer: &mut EditorBuffer, editor: &mut EditorEngine) {
        if let Some(left_seg_string) = buffer.string_to_left_of_caret() {
            // When buffer_mut goes out of scope, it will be dropped & validation
            // performed.
            {
                let buffer_mut = buffer.get_mut(editor.viewport());

                scroll_editor_content::dec_caret_col_by(
                    buffer_mut.inner.caret_raw,
                    buffer_mut.inner.scr_ofs,
                    left_seg_string.width,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{CaretDirection, DEFAULT_SYN_HI_FILE_EXT, EditorBuffer, EditorEvent, assert_eq2, caret_raw, caret_scr_adj,
                clipboard_service::clipboard_test_fixtures::TestClipboard,
                col,
                editor::editor_test_fixtures::{assert, mock_real_objects_for_editor},
                editor_engine::engine_internal_api,
                height, row};

    #[test]
    fn editor_validate_caret_pos_on_up() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "😀\n1".
        // R ┌──────────┐
        // 0 │😀        │
        // 1 ❱1         │
        //   └─⮬────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("😀".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertChar('1'),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(1)));

        // Move caret up. It should not be in the middle of the smiley face.
        // R ┌──────────┐
        // 0 ❱😀        │
        // 1 │1         │
        //   └──⮬───────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Up)],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(2) + row(0)));
    }

    #[test]
    fn editor_validate_caret_pos_on_down() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "😀\n1".
        // R ┌──────────┐
        // 0 ❱1         │
        // 1 │😀        │
        //   └──⮬───────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertChar('1'),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("😀".into()),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(2) + row(1)));

        // Move caret up, and right. It should wrap around to the start of the next line
        // and be to the left of the smiley face.
        // R ┌──────────┐
        // 0 ❱1         │
        // 1 │😀        │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Right),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(1)));

        // Move caret down. It should move to the end of the last line.
        // R ┌──────────┐
        // 0 │1         │
        // 1 ❱😀        │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Down)],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(2) + row(1)));
    }

    #[test]
    fn editor_move_caret_up_down() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "abc\nab\na".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ❱a         │
        //   └─⮬────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("abc".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("ab".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("a".into()),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(2)));

        // Move caret down. Goes to end of line 2 and stops.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ❱a         │
        //   └─⮬────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Down),
                EditorEvent::MoveCaret(CaretDirection::Down),
                EditorEvent::MoveCaret(CaretDirection::Down),
                EditorEvent::MoveCaret(CaretDirection::Down),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(2)));

        // Move caret up.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Up)],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(1)));

        // Move caret up.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Up)],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(0)));

        // Move caret up a few times. Caret moves to position 0.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Up),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(0)));

        // Move right to end of line. Then down.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 ❱ab        │
        // 2 │a         │
        //   └──⮬───────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::MoveCaret(CaretDirection::Down),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(2) + row(1)));

        // Move caret down.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ❱a         │
        //   └─⮬────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Down)],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(2)));
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn editor_move_caret_left_right() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "a".
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱a         │
        //   └─⮬────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('a')],
            &mut TestClipboard::default(),
        );
        assert::none_is_at_caret(&buffer);

        // Move caret left.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱a         │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::MoveCaret(CaretDirection::Left), // No-op.
            ],
            &mut TestClipboard::default(),
        );
        assert::str_is_at_caret(&buffer, "a");

        // Insert "1".
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱1a        │
        //   └─⮬────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('1')],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            engine_internal_api::line_at_caret_to_string(&buffer,).unwrap().content(),
            "1a"
        );
        assert::str_is_at_caret(&buffer, "a");

        // Move caret left.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱1a        │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Left)],
            &mut TestClipboard::default(),
        );
        assert::str_is_at_caret(&buffer, "1");

        // Move caret right.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱1a        │
        //   └─⮬────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Right)],
            &mut TestClipboard::default(),
        );
        assert::str_is_at_caret(&buffer, "a");

        // Insert "2".
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱12a       │
        //   └──⮬───────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('2')],
            &mut TestClipboard::default(),
        );
        assert::str_is_at_caret(&buffer, "a");
        assert_eq2!(
            engine_internal_api::line_at_caret_to_string(&buffer,).unwrap().content(),
            "12a"
        );

        // Move caret right. It should do nothing.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱12a       │
        //   └───⮬──────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::MoveCaret(CaretDirection::Right), // No-op.
            ],
            &mut TestClipboard::default(),
        );
        assert::none_is_at_caret(&buffer);
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(3) + row(0)));

        // Move caret left.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱12a       │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::MoveCaret(CaretDirection::Left),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(0)));

        // Move caret to end of line, press enter, then move caret left (should be at end
        // of prev line). `this` should look like:
        // R ┌──────────┐
        // 0 ❱12a       │
        // 1 │          │
        //   └───⮬──────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::InsertNewLine,
                EditorEvent::MoveCaret(CaretDirection::Left),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(3) + row(0)));

        // Move caret right (should be at start of next line).
        // `this` should look like:
        // R ┌──────────┐
        // 0 │12a       │
        // 1 ❱          │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Right)],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(1)));

        // Press enter. Press up. Press right (should be at start of next line).
        // `this` should look like:
        // R ┌──────────┐
        // 0 │12a       │
        // 1 │          │
        // 2 ❱          │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertNewLine,
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Right),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(2)));
    }

    #[test]
    fn editor_move_caret_home_end() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "hello". Then press home.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱hello     │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("hello".to_string()),
                EditorEvent::Home,
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(0)));

        // Press end.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::End],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(5) + row(0)));
    }

    #[test]
    fn editor_move_caret_home_end_overflow_viewport() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // console_log!(OK_RAW "press hello");

        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("hello".to_string())],
            &mut TestClipboard::default(),
        );

        // console_log!(OK_RAW "press helloHello + END");

        // Insert "hello". Then press home.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸helloHELLO│
        //   └▴─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("HELLOhello".to_string()),
                EditorEvent::Home,
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(0)));

        // console_log!(OK_RAW "press end");

        // Press end.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::End],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_raw(), caret_raw(col(9) + row(0)));
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(15) + row(0)));
    }

    #[test]
    fn editor_move_caret_page_up_page_down() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "hello" many times.
        let max_lines = 20;
        let mut count = max_lines;
        while count > 0 {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![
                    EditorEvent::InsertString(format!("{count}: {}", "hello")),
                    EditorEvent::InsertNewLine,
                ],
                &mut TestClipboard::default(),
            );
            count -= 1;
        }
        assert_eq2!(buffer.len(), height(max_lines + 1)); /* One empty line after content */

        // Press page up.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageUp],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(10)));

        // Press page up.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageUp],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(0)));

        // Press page up.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageUp],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(0)));

        // Press page down.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageDown],
            &mut TestClipboard::default(),
        );

        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(10)));

        // Press page down.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageDown],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(20)));

        // Press page down.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageDown],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(20)));
    }
}
