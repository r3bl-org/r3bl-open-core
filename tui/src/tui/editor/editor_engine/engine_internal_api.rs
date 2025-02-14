/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

//! Functions that implement the internal & functional API of the editor engine. See
//! [mod@super::engine_public_api] for the event based API.

use std::collections::HashMap;

use r3bl_core::{caret_scr_adj,
                col,
                row,
                string_storage,
                width,
                CaretScrAdj,
                ColIndex,
                RowIndex,
                StringStorage,
                UnicodeString,
                UnicodeStringExt,
                UnicodeStringSegmentSliceResult,
                VecArray};

use super::{scroll_editor_content, DeleteSelectionWith, SelectMode};
use crate::{buffer_clipboard_support,
            buffer_clipboard_support::ClipboardService,
            caret_locate,
            caret_locate::{locate_col,
                           CaretColLocationInLine,
                           CaretRowLocationInBuffer},
            caret_scroll_index,
            CaretDirection,
            EditorArgsMut,
            EditorBuffer,
            EditorEngine};

pub fn up(buffer: &mut EditorBuffer, engine: &mut EditorEngine, sel_mod: SelectMode) {
    caret_mut::up(buffer, engine, sel_mod);
}

pub fn left(buffer: &mut EditorBuffer, engine: &mut EditorEngine, sel_mod: SelectMode) {
    caret_mut::left(buffer, engine, sel_mod);
}

pub fn right(buffer: &mut EditorBuffer, engine: &mut EditorEngine, sel_mod: SelectMode) {
    caret_mut::right(buffer, engine, sel_mod);
}

pub fn down(buffer: &mut EditorBuffer, engine: &mut EditorEngine, sel_mod: SelectMode) {
    caret_mut::down(buffer, engine, sel_mod);
}

pub fn page_up(
    buffer: &mut EditorBuffer,
    engine: &mut EditorEngine,
    sel_mod: SelectMode,
) {
    caret_mut::page_up(buffer, engine, sel_mod);
}

pub fn page_down(
    buffer: &mut EditorBuffer,
    engine: &mut EditorEngine,
    sel_mod: SelectMode,
) {
    caret_mut::page_down(buffer, engine, sel_mod);
}

pub fn home(buffer: &mut EditorBuffer, engine: &mut EditorEngine, sel_mod: SelectMode) {
    caret_mut::to_start_of_line(buffer, engine, sel_mod);
}

pub fn end(buffer: &mut EditorBuffer, engine: &mut EditorEngine, sel_mod: SelectMode) {
    caret_mut::to_end_of_line(buffer, engine, sel_mod);
}

pub fn select_all(buffer: &mut EditorBuffer, sel_mod: SelectMode) {
    caret_mut::select_all(buffer, sel_mod);
}

pub fn clear_selection(buffer: &mut EditorBuffer) { buffer.clear_selection(); }

pub fn line_at_caret_to_string(buffer: &EditorBuffer) -> Option<&UnicodeString> {
    buffer.line_at_caret_scr_adj()
}

pub fn line_at_caret_is_empty(buffer: &EditorBuffer) -> Option<bool> {
    Some(buffer.get_line_display_width_at_caret_scr_adj() == width(0))
}

pub fn insert_str_at_caret(args: EditorArgsMut<'_>, chunk: &str) {
    content_mut::insert_chunk_at_caret(args, chunk);
}

pub fn insert_new_line_at_caret(args: EditorArgsMut<'_>) {
    content_mut::insert_new_line_at_caret(args);
}

pub fn delete_at_caret(buffer: &mut EditorBuffer, engine: &mut EditorEngine) {
    content_mut::delete_at_caret(buffer, engine);
}

pub fn delete_selected(
    buffer: &mut EditorBuffer,
    engine: &mut EditorEngine,
    with: DeleteSelectionWith,
) {
    content_mut::delete_selected(buffer, engine, with);
}

pub fn backspace_at_caret(buffer: &mut EditorBuffer, engine: &mut EditorEngine) {
    content_mut::backspace_at_caret(buffer, engine);
}

pub fn copy_editor_selection_to_clipboard(
    buffer: &EditorBuffer,
    clipboard: &mut impl ClipboardService,
) {
    buffer_clipboard_support::copy_to_clipboard(buffer, clipboard);
}

pub fn paste_clipboard_content_into_editor(
    args: EditorArgsMut<'_>,
    clipboard: &mut impl ClipboardService,
) {
    buffer_clipboard_support::paste_from_clipboard(args, clipboard);
}

/// Helper macros just for this module.
#[macro_use]
mod macros {
    /// Check to see if buffer is empty and return early if it is.
    macro_rules! empty_check_early_return {
        ($arg_buffer: expr, @None) => {
            if $arg_buffer.is_empty() {
                return None;
            }
        };

        ($arg_buffer: expr, @Nothing) => {
            if $arg_buffer.is_empty() {
                return;
            }
        };
    }

    /// Check to see if multiline mode is disabled and return early if it is.
    macro_rules! multiline_disabled_check_early_return {
        ($arg_engine: expr, @None) => {
            if let $crate::LineMode::SingleLine =
                $arg_engine.config_options.multiline_mode
            {
                return None;
            }
        };

        ($arg_engine: expr, @Nothing) => {
            if let $crate::LineMode::SingleLine =
                $arg_engine.config_options.multiline_mode
            {
                return;
            }
        };
    }
}

// REVIEW: [ ] move this out into its own module
// REFACTOR: [ ] replace the use of position and scroll offset with Caret!
pub mod caret_mut {
    use super::*;

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

    pub fn down(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        sel_mod: SelectMode,
    ) {
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
                        caret_scroll_index::scroll_col_index_for_width(
                            line_display_width,
                        ),
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
    pub fn right(
        editor_buffer: &mut EditorBuffer,
        editor_engine: &mut EditorEngine,
        sel_mod: SelectMode,
    ) -> Option<()> {
        empty_check_early_return!(editor_buffer, @None);

        let line_is_empty = line_at_caret_is_empty(editor_buffer)?;

        let caret_col_loc_in_line = locate_col(editor_buffer);

        // This is only set if sel_mod is enabled.
        let maybe_previous_caret_display_position =
            sel_mod.get_caret_display_position_scroll_adjusted(editor_buffer);

        match caret_col_loc_in_line {
            // Special case of empty line w/ caret at start.
            CaretColLocationInLine::AtStart if line_is_empty => {
                inner::right_at_end(editor_buffer, editor_engine)
            }
            CaretColLocationInLine::AtStart | CaretColLocationInLine::InMiddle => {
                inner::right_normal(editor_buffer, editor_engine)
            }
            CaretColLocationInLine::AtEnd => {
                inner::right_at_end(editor_buffer, editor_engine)
            }
        };

        // This is only set if sel_mod is enabled.
        let maybe_current_caret_display_position =
            sel_mod.get_caret_display_position_scroll_adjusted(editor_buffer);

        // This is only runs if sel_mod is enabled.
        sel_mod.handle_selection_single_line_caret_movement(
            editor_buffer,
            maybe_previous_caret_display_position,
            maybe_current_caret_display_position,
        );

        return None;

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
                            let jump_by_col_width = chunk_to_right_of_caret_us
                                .display_width
                                + unicode_width_at_caret;
                            let move_left_by_amt =
                                chunk_to_right_of_caret_us.display_width;

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

            pub fn right_at_end(
                buffer: &mut EditorBuffer,
                engine: &mut EditorEngine,
            ) -> Option<()> {
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

                None
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
}

// REVIEW: [ ] move this out into its own module
pub mod content_mut {
    use super::*;

    pub fn insert_chunk_at_caret(args: EditorArgsMut<'_>, chunk: &str) {
        let EditorArgsMut { buffer, engine } = args;

        let caret_scr_adj = buffer.get_caret_scr_adj();
        let row_index_scr_adj = caret_scr_adj.row_index;

        if buffer.line_at_row_index(row_index_scr_adj).is_some() {
            insert_into_existing_line(
                EditorArgsMut { buffer, engine },
                caret_scr_adj,
                chunk,
            );
        } else {
            fill_in_missing_lines_up_to_row(
                EditorArgsMut { buffer, engine },
                row_index_scr_adj,
            );
            insert_chunk_into_new_line(
                EditorArgsMut { buffer, engine },
                caret_scr_adj,
                chunk,
            );
        }
    }

    pub fn insert_new_line_at_caret(args: EditorArgsMut<'_>) {
        let EditorArgsMut { buffer, engine } = args;

        multiline_disabled_check_early_return!(engine, @Nothing);

        if buffer.is_empty() {
            // When buffer_mut goes out of scope, it will be dropped &
            // validation performed.
            {
                let buffer_mut = buffer.get_mut(engine.viewport());
                buffer_mut.lines.push("".unicode_string());
            }
            return;
        }

        match locate_col(buffer) {
            CaretColLocationInLine::AtEnd => {
                inner::insert_new_line_at_end_of_current_line(EditorArgsMut {
                    buffer,
                    engine,
                });
            }
            CaretColLocationInLine::AtStart => {
                inner::insert_new_line_at_start_of_current_line(EditorArgsMut {
                    buffer,
                    engine,
                });
            }
            CaretColLocationInLine::InMiddle => {
                inner::insert_new_line_at_middle_of_current_line(EditorArgsMut {
                    buffer,
                    engine,
                });
            }
        }

        mod inner {
            use super::*;

            // Handle inserting a new line at the end of the current line.
            pub fn insert_new_line_at_end_of_current_line(args: EditorArgsMut<'_>) {
                let EditorArgsMut { buffer, engine } = args;

                // When buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let buffer_mut = buffer.get_mut(engine.viewport());

                    let new_row_index = scroll_editor_content::inc_caret_row(
                        buffer_mut.caret_raw,
                        buffer_mut.scr_ofs,
                        buffer_mut.vp.row_height,
                    );

                    scroll_editor_content::reset_caret_col(
                        buffer_mut.caret_raw,
                        buffer_mut.scr_ofs,
                    );

                    buffer_mut
                        .lines
                        .insert(new_row_index.as_usize(), "".unicode_string());
                }
            }

            // Handle inserting a new line at the start of the current line.
            pub fn insert_new_line_at_start_of_current_line(args: EditorArgsMut<'_>) {
                let EditorArgsMut { buffer, engine } = args;

                // When buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let buffer_mut = buffer.get_mut(engine.viewport());
                    let cur_row_index =
                        (*buffer_mut.caret_raw + *buffer_mut.scr_ofs).row_index;
                    buffer_mut
                        .lines
                        .insert(cur_row_index.as_usize(), "".unicode_string());
                }

                // When buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let buffer_mut = buffer.get_mut(engine.viewport());
                    scroll_editor_content::inc_caret_row(
                        buffer_mut.caret_raw,
                        buffer_mut.scr_ofs,
                        buffer_mut.vp.row_height,
                    );
                }
            }

            // Handle inserting a new line at the middle of the current line.
            pub fn insert_new_line_at_middle_of_current_line(args: EditorArgsMut<'_>) {
                let EditorArgsMut { buffer, engine } = args;

                if let Some(line) = buffer.line_at_caret_scr_adj() {
                    let caret_adj = buffer.get_caret_scr_adj();
                    let col_index = caret_adj.col_index;
                    let split_result = line.split_at_display_col(col_index);
                    if let Some((left_string, right_string)) = split_result {
                        let row_index = caret_adj.row_index.as_usize();

                        // When buffer_mut goes out of scope, it will be dropped &
                        // validation performed.
                        {
                            let buffer_mut = buffer.get_mut(engine.viewport());

                            let _ = std::mem::replace(
                                &mut buffer_mut.lines[row_index],
                                left_string.unicode_string(),
                            );

                            buffer_mut
                                .lines
                                .insert(row_index + 1, right_string.unicode_string());

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
    }

    pub fn delete_at_caret(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
    ) -> Option<()> {
        empty_check_early_return!(buffer, @None);
        if buffer.string_at_caret().is_some() {
            inner::delete_in_middle_of_line(buffer, engine)?;
        } else {
            inner::delete_at_end_of_line(buffer, engine)?;
        }
        return None;

        mod inner {
            use r3bl_core::string_storage;

            use super::*;

            /// ```text
            /// R ┌──────────┐
            /// 0 ❱abc       │
            /// 1 │ab        │
            /// 2 │a         │
            ///   └─⮬────────┘
            ///   C0123456789
            /// ```
            pub fn delete_in_middle_of_line(
                buffer: &mut EditorBuffer,
                engine: &mut EditorEngine,
            ) -> Option<()> {
                let line = buffer.line_at_caret_scr_adj()?;

                let new_line_content = line
                    .delete_char_at_display_col(buffer.get_caret_scr_adj().col_index)?;

                // When buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let buffer_mut = buffer.get_mut(engine.viewport());
                    let row_index =
                        (*buffer_mut.caret_raw + *buffer_mut.scr_ofs).row_index;
                    let _ = std::mem::replace(
                        &mut buffer_mut.lines[row_index.as_usize()],
                        new_line_content.unicode_string(),
                    );
                }

                None
            }

            /// ```text
            /// R ┌──────────┐
            /// 0 ❱abc       │
            /// 1 │ab        │
            /// 2 │a         │
            ///   └───⮬──────┘
            ///   C0123456789
            /// ```
            pub fn delete_at_end_of_line(
                buffer: &mut EditorBuffer,
                engine: &mut EditorEngine,
            ) -> Option<()> {
                let this_line = buffer.line_at_caret_scr_adj()?;
                let next_line = buffer.next_line_below_caret_to_string()?;
                let new_line_content =
                    string_storage!("{a}{b}", a = this_line.string, b = next_line.string);

                // When buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let buffer_mut = buffer.get_mut(engine.viewport());
                    let row_index =
                        (*buffer_mut.caret_raw + *buffer_mut.scr_ofs).row_index;
                    let _ = std::mem::replace(
                        &mut buffer_mut.lines[row_index.as_usize()],
                        new_line_content.unicode_string(),
                    );
                    buffer_mut.lines.remove(row_index.as_usize() + 1);
                }

                None
            }
        }
    }

    pub fn backspace_at_caret(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
    ) -> Option<()> {
        empty_check_early_return!(buffer, @None);

        match buffer.string_to_left_of_caret() {
            Some(seg_result) => {
                inner::backspace_in_middle_of_line(
                    buffer,
                    engine,
                    seg_result.display_col_at_which_seg_starts,
                )?;
            }
            None => {
                inner::backspace_at_start_of_line(buffer, engine)?;
            }
        }

        return None;

        mod inner {
            use super::*;

            /// ```text
            /// R ┌──────────┐
            /// 0 ❱abc       │
            /// 1 │ab        │
            /// 2 │a         │
            ///   └─⮬────────┘
            ///   C0123456789
            /// ```
            pub fn backspace_in_middle_of_line(
                buffer: &mut EditorBuffer,
                engine: &mut EditorEngine,
                delete_at_this_display_col: ColIndex,
            ) -> Option<()> {
                let cur_line = buffer.line_at_caret_scr_adj()?;
                let new_line_content =
                    cur_line.delete_char_at_display_col(delete_at_this_display_col)?;

                // When buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let buffer_mut = buffer.get_mut(engine.viewport());
                    let cur_row_index =
                        (*buffer_mut.caret_raw + *buffer_mut.scr_ofs).row_index;
                    let _ = std::mem::replace(
                        &mut buffer_mut.lines[cur_row_index.as_usize()],
                        new_line_content.unicode_string(),
                    );

                    let new_line_content_display_width =
                        buffer_mut.lines[cur_row_index.as_usize()].display_width;

                    scroll_editor_content::set_caret_col_to(
                        delete_at_this_display_col,
                        buffer_mut.caret_raw,
                        buffer_mut.scr_ofs,
                        buffer_mut.vp.col_width,
                        new_line_content_display_width,
                    );
                }

                None
            }

            /// ```text
            /// R ┌──────────┐
            /// 0 │abc       │
            /// 1 ❱ab        │
            /// 2 │a         │
            ///   └⮬─────────┘
            ///   C0123456789
            /// ```
            pub fn backspace_at_start_of_line(
                buffer: &mut EditorBuffer,
                engine: &mut EditorEngine,
            ) -> Option<()> {
                let this_line = buffer.line_at_caret_scr_adj()?;
                let prev_line = buffer.prev_line_above_caret()?;

                // A line above the caret exists.
                let prev_line_display_width = {
                    let prev_row_index = buffer.get_caret_scr_adj().row_index - row(1);
                    buffer.get_line_display_width_at_row_index(prev_row_index)
                };

                let new_line_content =
                    string_storage!("{a}{b}", a = prev_line.string, b = this_line.string);

                // When buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let buffer_mut = buffer.get_mut(engine.viewport());

                    let prev_row_index =
                        (*buffer_mut.caret_raw + *buffer_mut.scr_ofs).row_index - row(1);

                    let cur_row_index =
                        (*buffer_mut.caret_raw + *buffer_mut.scr_ofs).row_index;

                    let _ = std::mem::replace(
                        &mut buffer_mut.lines[prev_row_index.as_usize()],
                        new_line_content.unicode_string(),
                    );

                    let new_line_content_display_width =
                        buffer_mut.lines[prev_row_index.as_usize()].display_width;

                    buffer_mut.lines.remove(cur_row_index.as_usize());

                    scroll_editor_content::dec_caret_row(
                        buffer_mut.caret_raw,
                        buffer_mut.scr_ofs,
                    );

                    scroll_editor_content::set_caret_col_to(
                        // This caret col index goes 1 past the end of the line width, ie:
                        // - `prev_line_display_width` which is the same as:
                        // - `prev_line_display_width.convert_to_col_index() /*-1*/ + 1`
                        caret_scroll_index::scroll_col_index_for_width(
                            prev_line_display_width,
                        ),
                        buffer_mut.caret_raw,
                        buffer_mut.scr_ofs,
                        buffer_mut.vp.col_width,
                        new_line_content_display_width,
                    );
                }

                None
            }
        }
    }

    pub fn delete_selected(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        with: DeleteSelectionWith,
    ) -> Option<()> {
        // Early return if any of the following are met.
        empty_check_early_return!(buffer, @None);
        if buffer.get_selection_list().is_empty() {
            return None;
        }

        let my_selection_map = buffer.get_selection_list().clone();
        let lines = buffer.get_lines();
        let selected_row_indices = my_selection_map.get_ordered_indices();
        let mut vec_row_indices_to_remove = VecArray::<RowIndex>::new();
        let mut map_lines_to_replace = HashMap::new();

        for selected_row_index in selected_row_indices {
            if let Some(selection_range) = my_selection_map.get(selected_row_index) {
                let line_width =
                    buffer.get_line_display_width_at_row_index(selected_row_index);

                // Remove entire line.
                if selection_range.start_disp_col_idx_scr_adj == col(0)
                    && selection_range.end_disp_col_idx_scr_adj
                        == caret_scroll_index::scroll_col_index_for_width(line_width)
                {
                    vec_row_indices_to_remove.push(selected_row_index);
                    continue;
                }

                // Skip if selection range is empty.
                if selection_range.start_disp_col_idx_scr_adj
                    == selection_range.end_disp_col_idx_scr_adj
                {
                    continue;
                }

                // Remove selection range (part of the line).
                let line_us = lines.get(selected_row_index.as_usize())?.clone();

                let keep_before_selected = line_us.clip_to_width(
                    col(0),
                    selection_range.get_start_display_col_index_as_width(),
                );

                let keep_after_selected = line_us
                    .clip_to_width(selection_range.end_disp_col_idx_scr_adj, line_width);

                let mut remaining_text = StringStorage::with_capacity(
                    keep_before_selected.len() + keep_after_selected.len(),
                );

                remaining_text.push_str(keep_before_selected);
                remaining_text.push_str(keep_after_selected);
                map_lines_to_replace.insert(selected_row_index, remaining_text);
            }
        }

        // When buffer_mut goes out of scope, it will be dropped &
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());

            // Replace lines, before removing them (to prevent indices from being invalidated).
            for row_index in map_lines_to_replace.keys() {
                let new_line_content = map_lines_to_replace[row_index].clone();
                let _ = std::mem::replace(
                    &mut buffer_mut.lines[row_index.as_usize()],
                    new_line_content.unicode_string(),
                );
            }

            // Remove lines in inverse order, in order to preserve the validity of indices.
            vec_row_indices_to_remove.reverse();
            for row_index in vec_row_indices_to_remove {
                buffer_mut.lines.remove(row_index.as_usize());
            }

            // Restore caret position to start of selection range.
            let maybe_new_caret =
                my_selection_map.get_caret_at_start_of_range_scroll_adjusted(with);

            if let Some(new_caret_scr_adj) = maybe_new_caret {
                // Convert scroll adjusted caret to raw caret by applying scroll offset.
                // Equivalent to: `let caret_raw = *new_caret_scr_adj - *buffer_mut.scr_ofs;`
                let caret_raw = new_caret_scr_adj + *buffer_mut.scr_ofs;
                *buffer_mut.caret_raw = caret_raw;
            }
        }

        buffer.clear_selection();

        None
    }

    fn insert_into_existing_line(
        args: EditorArgsMut<'_>,
        caret_scr_adj: CaretScrAdj,
        chunk: &str,
    ) -> Option<()> {
        let EditorArgsMut { buffer, engine } = args;

        let row_index = caret_scr_adj.row_index;
        let line = buffer.line_at_row_index(row_index)?;

        let (new_line_content, chunk_display_width) =
            line.insert_chunk_at_display_col(caret_scr_adj.col_index, chunk);

        // When buffer_mut goes out of scope, it will be dropped & validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());

            // Replace existing line w/ new line.
            let _ = std::mem::replace(
                &mut buffer_mut.lines[row_index.as_usize()],
                new_line_content.unicode_string(),
            );

            let new_line_content_display_width =
                buffer_mut.lines[row_index.as_usize()].display_width;

            // Update caret position.
            scroll_editor_content::inc_caret_col_by(
                buffer_mut.caret_raw,
                buffer_mut.scr_ofs,
                chunk_display_width,
                new_line_content_display_width,
                buffer_mut.vp.col_width,
            );
        }

        None
    }

    /// Insert empty lines up to the row index.
    fn fill_in_missing_lines_up_to_row(args: EditorArgsMut<'_>, row_index: RowIndex) {
        let EditorArgsMut { buffer, engine } = args;

        let max_row_index = row_index.as_usize();

        // Fill in any missing lines.
        if buffer.get_lines().get(max_row_index).is_none() {
            for row_index in 0..max_row_index + 1 {
                if buffer.get_lines().get(row_index).is_none() {
                    // When buffer_mut goes out of scope, it will be dropped & validation
                    // performed.
                    {
                        let buffer_mut = buffer.get_mut(engine.viewport());
                        buffer_mut.lines.push("".unicode_string());
                    }
                }
            }
        }
    }

    fn insert_chunk_into_new_line(
        args: EditorArgsMut<'_>,
        caret_scr_adj: CaretScrAdj,
        chunk: &str,
    ) -> Option<()> {
        let EditorArgsMut { buffer, engine } = args;
        let row_index_scr_adj = caret_scr_adj.row_index.as_usize();

        // Make sure there's a line at caret_adj_row.
        let _ = buffer.get_lines().get(row_index_scr_adj)?;

        // When buffer_mut goes out of scope, it will be dropped & validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());

            // Actually add the character to the correct line.
            let new_content = chunk.unicode_string();
            let _ =
                std::mem::replace(&mut buffer_mut.lines[row_index_scr_adj], new_content);

            let line_content = &buffer_mut.lines[row_index_scr_adj];
            let line_content_display_width = line_content.display_width;
            let col_amt = UnicodeString::str_display_width(chunk);

            // Update caret position.
            scroll_editor_content::inc_caret_col_by(
                buffer_mut.caret_raw,
                buffer_mut.scr_ofs,
                col_amt,
                line_content_display_width,
                buffer_mut.vp.col_width,
            );
        }

        None
    }
}
