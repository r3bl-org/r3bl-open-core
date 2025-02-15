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

use r3bl_core::{caret_scr_adj, col, row, width, UnicodeStringSegmentSliceResult};

use super::{scroll_editor_content, SelectMode};
use crate::{caret_locate::{self,
                           locate_col,
                           CaretColLocationInLine,
                           CaretRowLocationInBuffer},
            caret_mut,
            caret_scroll_index,
            empty_check_early_return,
            multiline_disabled_check_early_return,
            CaretDirection,
            EditorArgsMut,
            EditorBuffer,
            EditorEngine};

pub fn up(buffer: &mut EditorBuffer, engine: &mut EditorEngine, sel_mod: SelectMode) {
    empty_check_early_return!(buffer, @Nothing);
    multiline_disabled_check_early_return!(engine, @Nothing);

    // This is only set if sel_mod is enabled.
    // BUG: [ ] introduce scroll adjusted type
    let maybe_previous_caret_display_position =
        sel_mod.get_caret_display_position_scroll_adjusted(buffer);

    match caret_locate::locate_row(buffer) {
        CaretRowLocationInBuffer::AtTop => {
            // Do nothing if the caret (scroll adjusted) is at the top.
            if buffer.get_caret_scr_adj().col_index != col(0) {
                // When buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let buffer_mut = buffer.get_mut(engine.viewport());

                    scroll_editor_content::reset_caret_col(
                        buffer_mut.caret_raw,
                        buffer_mut.scr_ofs,
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
                        buffer_mut.caret_raw,
                        buffer_mut.scr_ofs,
                    );
                }

                scroll_editor_content::clip_caret_to_content_width(EditorArgsMut {
                    buffer,
                    engine,
                });
            }
        }
    }

    // This is only set if sel_mod is enabled.
    // BUG: [ ] introduce scroll adjusted type
    let maybe_current_caret_display_position =
        sel_mod.get_caret_display_position_scroll_adjusted(buffer);

    // This is only runs if sel_mod is enabled.
    sel_mod.update_selection_based_on_caret_movement_in_multiple_lines(
        buffer,
        maybe_previous_caret_display_position,
        maybe_current_caret_display_position,
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
    let maybe_previous_caret_display_position =
        sel_mod.get_caret_display_position_scroll_adjusted(buffer);

    let viewport_height = engine.viewport().row_height;
    scroll_editor_content::change_caret_row_by(
        EditorArgsMut { engine, buffer },
        viewport_height,
        CaretDirection::Up,
    );

    // This is only set if sel_mod is enabled.
    let maybe_current_caret_display_position =
        sel_mod.get_caret_display_position_scroll_adjusted(buffer);

    // This is only runs if sel_mod is enabled.
    sel_mod.update_selection_based_on_caret_movement_in_multiple_lines(
        buffer,
        maybe_previous_caret_display_position,
        maybe_current_caret_display_position,
    );
}

pub fn down(buffer: &mut EditorBuffer, engine: &mut EditorEngine, sel_mod: SelectMode) {
    empty_check_early_return!(buffer, @Nothing);
    multiline_disabled_check_early_return!(engine, @Nothing);

    // This is only set if sel_mod is enabled.
    let maybe_previous_caret_display_position =
        sel_mod.get_caret_display_position_scroll_adjusted(buffer);

    if buffer.next_line_below_caret_to_string().is_some() {
        // When buffer_mut goes out of scope, it will be dropped &
        // validation performed.
        {
            // There is a line below the caret.
            let buffer_mut = buffer.get_mut(engine.viewport());

            scroll_editor_content::inc_caret_row(
                buffer_mut.caret_raw,
                buffer_mut.scr_ofs,
                buffer_mut.vp.row_height,
            );
        }

        scroll_editor_content::clip_caret_to_content_width(EditorArgsMut {
            buffer,
            engine,
        });
    } else {
        // Move to the end of the line.
        caret_mut::to_end_of_line(buffer, engine, sel_mod);
    }

    // This is only set if sel_mod is enabled.
    let maybe_current_caret_display_position =
        sel_mod.get_caret_display_position_scroll_adjusted(buffer);

    // This is only runs if sel_mod is enabled.
    sel_mod.update_selection_based_on_caret_movement_in_multiple_lines(
        buffer,
        maybe_previous_caret_display_position,
        maybe_current_caret_display_position,
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
    let maybe_previous_caret_display_position =
        sel_mod.get_caret_display_position_scroll_adjusted(buffer);

    let viewport_height = engine.viewport().row_height;
    scroll_editor_content::change_caret_row_by(
        EditorArgsMut { engine, buffer },
        viewport_height,
        CaretDirection::Down,
    );

    // This is only set if sel_mod is enabled.
    let maybe_current_caret_display_position =
        sel_mod.get_caret_display_position_scroll_adjusted(buffer);

    // This is only runs if sel_mod is enabled.
    sel_mod.update_selection_based_on_caret_movement_in_multiple_lines(
        buffer,
        maybe_previous_caret_display_position,
        maybe_current_caret_display_position,
    );
}

/// Depending on [SelectMode], this acts as a:
/// - Convenience function for simply calling [left] repeatedly.
/// - Convenience function for simply calling
///   [scroll_editor_content::reset_caret_col].
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
                    buffer_mut.caret_raw,
                    buffer_mut.scr_ofs,
                );
            }
        }
    }
}

/// Depending on [SelectMode], this acts as a:
/// - Convenience function for simply calling [right] repeatedly.
/// - Convenience function for simply calling
///   [scroll_editor_content::set_caret_col_to].
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
                    caret_scroll_index::scroll_col_index_for_width(line_display_width),
                    buffer_mut.caret_raw,
                    buffer_mut.scr_ofs,
                    buffer_mut.vp.col_width,
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
        caret_scroll_index::scroll_col_index_for_width(last_line_display_width)
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
pub fn right(buffer: &mut EditorBuffer, engine: &mut EditorEngine, sel_mod: SelectMode) {
    empty_check_early_return!(buffer, @Nothing);

    let line_is_empty = buffer.line_at_caret_is_empty();

    let caret_col_loc_in_line = locate_col(buffer);

    // This is only set if sel_mod is enabled.
    let maybe_previous_caret_display_position =
        sel_mod.get_caret_display_position_scroll_adjusted(buffer);

    match caret_col_loc_in_line {
        // Special case of empty line w/ caret at start.
        CaretColLocationInLine::AtStart if line_is_empty => {
            inner::right_at_end(buffer, engine);
        }
        CaretColLocationInLine::AtStart | CaretColLocationInLine::InMiddle => {
            inner::right_normal(buffer, engine);
        }
        CaretColLocationInLine::AtEnd => {
            inner::right_at_end(buffer, engine);
        }
    };

    // This is only set if sel_mod is enabled.
    let maybe_current_caret_display_position =
        sel_mod.get_caret_display_position_scroll_adjusted(buffer);

    // This is only runs if sel_mod is enabled.
    sel_mod.handle_selection_single_line_caret_movement(
        buffer,
        maybe_previous_caret_display_position,
        maybe_current_caret_display_position,
    );

    return;

    mod inner {
        use super::*;

        /// 1. Check for wide unicode character to the right of the caret.
        /// 2. [validate::apply_change] checks for wide unicode character at the start of the
        ///    viewport.
        pub fn right_normal(
            buffer: &mut EditorBuffer,
            engine: &mut EditorEngine,
        ) -> Option<()> {
            let UnicodeStringSegmentSliceResult {
                seg_display_width: unicode_width_at_caret,
                ..
            } = buffer.string_at_caret()?;

            let max_display_width = buffer.get_line_display_width_at_caret_scr_adj();

            let maybe_char_to_right_of_caret = buffer.string_to_right_of_caret();

            match maybe_char_to_right_of_caret {
                Some(right_of_caret_seg_slice_result) => {
                    let chunk_to_right_of_caret_us =
                        right_of_caret_seg_slice_result.seg_text;

                    if chunk_to_right_of_caret_us.contains_wide_segments() {
                        let jump_by_col_width = chunk_to_right_of_caret_us.display_width
                            + unicode_width_at_caret;
                        let move_left_by_amt = chunk_to_right_of_caret_us.display_width;

                        // When buffer_mut goes out of scope, it will be dropped &
                        // validation performed.
                        {
                            let buffer_mut = buffer.get_mut(engine.viewport());

                            scroll_editor_content::inc_caret_col_by(
                                buffer_mut.caret_raw,
                                buffer_mut.scr_ofs,
                                jump_by_col_width,
                                max_display_width,
                                buffer_mut.vp.col_width,
                            );
                        }

                        if move_left_by_amt > width(0) {
                            // When buffer_mut goes out of scope, it will be dropped &
                            // validation performed.
                            {
                                let buffer_mut = buffer.get_mut(engine.viewport());

                                scroll_editor_content::dec_caret_col_by(
                                    buffer_mut.caret_raw,
                                    buffer_mut.scr_ofs,
                                    move_left_by_amt,
                                );
                            }
                        }
                    } else {
                        // When buffer_mut goes out of scope, it will be dropped &
                        // validation performed.
                        {
                            let buffer_mut = buffer.get_mut(engine.viewport());

                            scroll_editor_content::inc_caret_col_by(
                                buffer_mut.caret_raw,
                                buffer_mut.scr_ofs,
                                unicode_width_at_caret,
                                max_display_width,
                                buffer_mut.vp.col_width,
                            );
                        }
                    }
                }

                None => {
                    // When buffer_mut goes out of scope, it will be dropped &
                    // validation performed.
                    {
                        let buffer_mut = buffer.get_mut(engine.viewport());

                        scroll_editor_content::inc_caret_col_by(
                            buffer_mut.caret_raw,
                            buffer_mut.scr_ofs,
                            unicode_width_at_caret,
                            max_display_width,
                            buffer_mut.vp.col_width,
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
                        buffer_mut.caret_raw,
                        buffer_mut.scr_ofs,
                        buffer_mut.vp.row_height,
                    );

                    scroll_editor_content::reset_caret_col(
                        buffer_mut.caret_raw,
                        buffer_mut.scr_ofs,
                    );
                }
            }
        }
    }
}

pub fn left(
    buffer: &mut EditorBuffer,
    editor: &mut EditorEngine,
    sel_mod: SelectMode,
) -> Option<()> {
    empty_check_early_return!(buffer, @None);

    // This is only set if sel_mod is enabled.
    let maybe_previous_caret_display_position =
        sel_mod.get_caret_display_position_scroll_adjusted(buffer);

    match locate_col(buffer) {
        CaretColLocationInLine::AtStart => {
            if buffer.prev_line_above_caret().is_some() {
                // If there is a line above the caret, move the caret to the end of
                // the previous line.

                // When buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let buffer_mut = buffer.get_mut(editor.viewport());

                    scroll_editor_content::dec_caret_row(
                        buffer_mut.caret_raw,
                        buffer_mut.scr_ofs,
                    );
                }

                caret_mut::to_end_of_line(buffer, editor, SelectMode::Disabled);
            }
        }
        CaretColLocationInLine::AtEnd => {
            let seg_slice = buffer.string_at_end_of_line_at_caret_scr_adj()?;
            // When buffer_mut goes out of scope, it will be dropped &
            // validation performed.
            {
                let buffer_mut = buffer.get_mut(editor.viewport());

                scroll_editor_content::dec_caret_col_by(
                    buffer_mut.caret_raw,
                    buffer_mut.scr_ofs,
                    seg_slice.seg_display_width,
                );
            }
        }
        CaretColLocationInLine::InMiddle => {
            let seg_slice = buffer.string_to_left_of_caret()?;
            // When buffer_mut goes out of scope, it will be dropped &
            // validation performed.
            {
                let buffer_mut = buffer.get_mut(editor.viewport());

                scroll_editor_content::dec_caret_col_by(
                    buffer_mut.caret_raw,
                    buffer_mut.scr_ofs,
                    seg_slice.seg_display_width,
                );
            }
        }
    }

    // This is only set if sel_mod is enabled.
    let maybe_current_caret_display_position =
        sel_mod.get_caret_display_position_scroll_adjusted(buffer);

    // This is only runs if sel_mod is enabled.
    sel_mod.handle_selection_single_line_caret_movement(
        buffer,
        maybe_previous_caret_display_position,
        maybe_current_caret_display_position,
    );

    None
}
