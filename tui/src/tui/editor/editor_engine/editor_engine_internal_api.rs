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
use std::{cmp::Ordering, collections::HashMap};

use r3bl_core::{caret_scr_adj,
                ch,
                col,
                height,
                row,
                usize,
                width,
                Caret,
                CaretRaw,
                CaretScrAdj,
                ColIndex,
                ColWidth,
                Dim,
                RowHeight,
                RowIndex,
                ScrOfs,
                StringStorage,
                UnicodeString,
                UnicodeStringExt,
                UnicodeStringSegmentSliceResult,
                VecArray};

use crate::{caret_locate,
            caret_locate::{locate_col, CaretColLocationInLine},
            editor::sizing::VecEditorContentLines,
            editor_buffer_clipboard_support,
            editor_buffer_clipboard_support::ClipboardService,
            editor_engine_internal_api::caret_locate::CaretRowLocationInBuffer,
            CaretDirection,
            EditorArgsMut,
            EditorBuffer,
            EditorBufferApi,
            EditorEngine,
            SelectionList};

/// Functions that implement the editor engine.
pub struct EditorEngineInternalApi;

impl EditorEngineInternalApi {
    pub fn up(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        sel_mod: SelectMode,
    ) -> Option<()> {
        caret_mut::up(buffer, engine, sel_mod)
    }

    pub fn left(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        sel_mod: SelectMode,
    ) -> Option<()> {
        caret_mut::left(buffer, engine, sel_mod)
    }

    pub fn right(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        sel_mod: SelectMode,
    ) -> Option<()> {
        caret_mut::right(buffer, engine, sel_mod)
    }

    pub fn down(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        sel_mod: SelectMode,
    ) -> Option<()> {
        caret_mut::down(buffer, engine, sel_mod)
    }

    pub fn page_up(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        sel_mod: SelectMode,
    ) -> Option<()> {
        caret_mut::page_up(buffer, engine, sel_mod)
    }

    pub fn page_down(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        sel_mod: SelectMode,
    ) -> Option<()> {
        caret_mut::page_down(buffer, engine, sel_mod)
    }

    pub fn home(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        sel_mod: SelectMode,
    ) -> Option<()> {
        caret_mut::to_start_of_line(buffer, engine, sel_mod)
    }

    pub fn end(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        sel_mod: SelectMode,
    ) -> Option<()> {
        caret_mut::to_end_of_line(buffer, engine, sel_mod)
    }

    pub fn select_all(buffer: &mut EditorBuffer, sel_mod: SelectMode) -> Option<()> {
        caret_mut::select_all(buffer, sel_mod)
    }

    pub fn clear_selection(buffer: &mut EditorBuffer) -> Option<()> {
        caret_mut::clear_selection(buffer)
    }

    pub fn validate_scroll(args: EditorArgsMut<'_>) -> Option<()> {
        scroll_editor_buffer::validate_scroll(args)
    }

    pub fn line_at_caret_to_string(buffer: &EditorBuffer) -> Option<&UnicodeString> {
        buffer.line_at_caret_scr_adj()
    }

    pub fn line_at_caret_is_empty(buffer: &EditorBuffer) -> Option<bool> {
        Some(buffer.get_line_display_width_at_caret_scr_adj() == width(0))
    }

    pub fn insert_str_at_caret(args: EditorArgsMut<'_>, chunk: &str) -> Option<()> {
        content_mut::insert_chunk_at_caret(args, chunk)
    }

    pub fn insert_new_line_at_caret(args: EditorArgsMut<'_>) -> Option<()> {
        content_mut::insert_new_line_at_caret(args)
    }

    pub fn delete_at_caret(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
    ) -> Option<()> {
        content_mut::delete_at_caret(buffer, engine)
    }

    pub fn delete_selected(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        with: DeleteSelectionWith,
    ) -> Option<()> {
        content_mut::delete_selected(buffer, engine, with)
    }

    pub fn backspace_at_caret(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
    ) -> Option<()> {
        content_mut::backspace_at_caret(buffer, engine)
    }

    pub fn copy_editor_selection_to_clipboard(
        buffer: &EditorBuffer,
        clipboard: &mut impl ClipboardService,
    ) -> Option<()> {
        editor_buffer_clipboard_support::copy_to_clipboard(buffer, clipboard)
    }

    pub fn paste_clipboard_content_into_editor(
        args: EditorArgsMut<'_>,
        clipboard: &mut impl ClipboardService,
    ) -> Option<()> {
        editor_buffer_clipboard_support::paste_from_clipboard(args, clipboard)
    }
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

#[derive(Clone, Copy, Debug)]
pub enum SelectMode {
    Enabled,
    Disabled,
}

impl SelectMode {
    // BUG: [ ] introduce scroll adjusted type
    pub fn get_caret_display_position_scroll_adjusted(
        &self,
        buffer: &EditorBuffer,
    ) -> Option<CaretScrAdj> {
        match self {
            SelectMode::Enabled => Some(buffer.get_caret_scr_adj()),
            _ => None,
        }
    }

    /// Manage the selection based on the movement of the caret:
    /// - <kbd>Shift + Left</kbd>
    /// - <kbd>Shift + Right</kbd>
    /// - <kbd>Shift + Home</kbd>
    /// - <kbd>Shift + End</kbd>
    ///
    /// # Arguments
    /// - `editor_buffer` - The buffer to update. This holds the selection map.
    /// - `maybe_previous_caret_display_pos` - The previous position of the caret.
    ///    - This maybe [None] if [SelectMode] is [SelectMode::Disabled].
    ///    - This has to be [Some] if [SelectMode::Enabled].
    /// - `maybe_current_caret_display_pos` - The current position of the caret.
    ///    - This maybe [None] if [SelectMode] is [SelectMode::Disabled].
    ///    - This has to be [Some] if [SelectMode::Enabled].
    pub fn handle_selection_single_line_caret_movement(
        &self,
        editor_buffer: &mut EditorBuffer,
        maybe_previous_caret_display_position: Option<CaretScrAdj>,
        maybe_current_caret_display_position: Option<CaretScrAdj>,
    ) -> Option<()> {
        match self {
            // Cancel the selection. We don't care about the caret positions (they maybe
            // None or not).
            SelectMode::Disabled => editor_buffer.clear_selection(),
            // Create or update the selection w/ the caret positions (which can't be
            // None).
            SelectMode::Enabled => {
                let previous = maybe_previous_caret_display_position?;
                let current = maybe_current_caret_display_position?;

                if previous.row_index != current.row_index {
                    return None;
                }

                EditorBufferApi::handle_selection_single_line_caret_movement(
                    editor_buffer,
                    previous.row_index, // Same as `current.row_index`.
                    previous.col_index,
                    current.col_index,
                )
            }
        };

        None
    }

    pub fn update_selection_based_on_caret_movement_in_multiple_lines(
        &self,
        buffer: &mut EditorBuffer,
        maybe_prev_caret_scr_adj: Option<CaretScrAdj>,
        maybe_curr_caret_scr_adj: Option<CaretScrAdj>,
    ) -> Option<()> {
        match self {
            // Cancel the selection. We don't care about the caret positions (they maybe
            // None or not).
            SelectMode::Disabled => buffer.clear_selection(),
            // Create or update the selection w/ the caret positions (which can't be
            // None).
            SelectMode::Enabled => {
                let prev = maybe_prev_caret_scr_adj?;
                let curr = maybe_curr_caret_scr_adj?;

                match prev.row_index.cmp(&curr.row_index) {
                    Ordering::Equal => EditorBufferApi::handle_selection_multiline_caret_movement_hit_top_or_bottom_of_document(
                        buffer,
                        prev,
                        curr,
                    ),
                    _ => EditorBufferApi::handle_selection_multiline_caret_movement(
                        buffer,
                        prev,
                        curr,
                    ),
                }
            }
        };

        None
    }
}

// REFACTOR: [ ] replace the use of position and scroll offset with Caret!
mod caret_mut {
    use r3bl_core::Caret;

    use super::*;

    pub fn up(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        sel_mod: SelectMode,
    ) -> Option<()> {
        empty_check_early_return!(buffer, @None);
        multiline_disabled_check_early_return!(engine, @None);

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

                        scroll_editor_buffer::reset_caret_col(
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

                        scroll_editor_buffer::dec_caret_row(
                            buffer_mut.caret_raw,
                            buffer_mut.scr_ofs,
                        );
                    }

                    scroll_editor_buffer::clip_caret_to_content_width(EditorArgsMut {
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

        None
    }

    pub fn page_up(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        sel_mod: SelectMode,
    ) -> Option<()> {
        empty_check_early_return!(buffer, @None);
        multiline_disabled_check_early_return!(engine, @None);

        // This is only set if sel_mod is enabled.
        let maybe_previous_caret_display_position =
            sel_mod.get_caret_display_position_scroll_adjusted(buffer);

        let viewport_height = engine.viewport().row_height;
        scroll_editor_buffer::change_caret_row_by(
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

        None
    }

    pub fn down(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        sel_mod: SelectMode,
    ) -> Option<()> {
        empty_check_early_return!(buffer, @None);
        multiline_disabled_check_early_return!(engine, @None);

        // This is only set if sel_mod is enabled.
        let maybe_previous_caret_display_position =
            sel_mod.get_caret_display_position_scroll_adjusted(buffer);

        if buffer.next_line_below_caret_to_string().is_some() {
            // When buffer_mut goes out of scope, it will be dropped &
            // validation performed.
            {
                // There is a line below the caret.
                let buffer_mut = buffer.get_mut(engine.viewport());

                scroll_editor_buffer::inc_caret_row(
                    buffer_mut.caret_raw,
                    buffer_mut.scr_ofs,
                    buffer_mut.vp.row_height,
                );
            }

            scroll_editor_buffer::clip_caret_to_content_width(EditorArgsMut {
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

        None
    }

    pub fn page_down(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        sel_mod: SelectMode,
    ) -> Option<()> {
        empty_check_early_return!(buffer, @None);
        multiline_disabled_check_early_return!(engine, @None);

        // This is only set if sel_mod is enabled.
        let maybe_previous_caret_display_position =
            sel_mod.get_caret_display_position_scroll_adjusted(buffer);

        let viewport_height = engine.viewport().row_height;
        scroll_editor_buffer::change_caret_row_by(
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

        None
    }

    /// Depending on [SelectMode], this acts as a:
    /// - Convenience function for simply calling [left] repeatedly.
    /// - Convenience function for simply calling [scroll_editor_buffer::reset_caret_col].
    pub fn to_start_of_line(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        sel_mod: SelectMode,
    ) -> Option<()> {
        empty_check_early_return!(buffer, @None);

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

                    scroll_editor_buffer::reset_caret_col(
                        buffer_mut.caret_raw,
                        buffer_mut.scr_ofs,
                    );
                }
            }
        }

        None
    }

    /// Depending on [SelectMode], this acts as a:
    /// - Convenience function for simply calling [right] repeatedly.
    /// - Convenience function for simply calling [scroll_editor_buffer::set_caret_col].
    pub fn to_end_of_line(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        sel_mod: SelectMode,
    ) -> Option<()> {
        empty_check_early_return!(buffer, @None);

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

                    scroll_editor_buffer::set_caret_col_to(
                        buffer_mut.caret_raw,
                        buffer_mut.scr_ofs,
                        buffer_mut.vp.col_width,
                        line_display_width,
                        // REVIEW: [x] make sure this comment is correct (logic unchanged)
                        // This caret col index goes 1 past the end of the line width, ie:
                        // `line_display_width.convert_to_col_index() /*-1*/ + 1 == line_display_width`
                        Caret::scroll_col_index_for_width(line_display_width),
                    );
                }
            }
        }

        None
    }

    pub fn clear_selection(editor_buffer: &mut EditorBuffer) -> Option<()> {
        editor_buffer.clear_selection();
        None
    }

    pub fn select_all(buffer: &mut EditorBuffer, sel_mod: SelectMode) -> Option<()> {
        empty_check_early_return!(buffer, @None);

        let max_row_index = buffer.get_max_row_index();

        // REVIEW: [x] make sure this comment is correct (logic unchanged)
        // This caret col index goes 1 past the end of the line width, ie:
        // `last_line_display_width.convert_to_col_index() /*-1*/ + 1 == last_line_display_width`
        let max_col_index = {
            let last_line_display_width =
                buffer.get_line_display_width_at_row_index(max_row_index);
            Caret::scroll_col_index_for_width(last_line_display_width)
        };

        buffer.clear_selection();
        sel_mod.update_selection_based_on_caret_movement_in_multiple_lines(
            buffer,
            Some(caret_scr_adj(col(0) + row(0))),
            Some(caret_scr_adj(max_col_index + max_row_index)),
        );

        None
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

        let line_is_empty =
            EditorEngineInternalApi::line_at_caret_is_empty(editor_buffer)?;

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

                                scroll_editor_buffer::inc_caret_col_by(
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

                                    scroll_editor_buffer::dec_caret_col_by(
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

                                scroll_editor_buffer::inc_caret_col_by(
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

                            scroll_editor_buffer::inc_caret_col_by(
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

                        scroll_editor_buffer::inc_caret_row(
                            buffer_mut.caret_raw,
                            buffer_mut.scr_ofs,
                            buffer_mut.vp.row_height,
                        );

                        scroll_editor_buffer::reset_caret_col(
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

                        scroll_editor_buffer::dec_caret_row(
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

                    scroll_editor_buffer::dec_caret_col_by(
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

                    scroll_editor_buffer::dec_caret_col_by(
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

mod content_mut {
    use r3bl_core::Caret;

    use super::*;

    pub fn insert_chunk_at_caret(args: EditorArgsMut<'_>, chunk: &str) -> Option<()> {
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

        None
    }

    pub fn insert_new_line_at_caret(args: EditorArgsMut<'_>) -> Option<()> {
        let EditorArgsMut { buffer, engine } = args;

        multiline_disabled_check_early_return!(engine, @None);

        if buffer.is_empty() {
            // When buffer_mut goes out of scope, it will be dropped &
            // validation performed.
            {
                let buffer_mut = buffer.get_mut(engine.viewport());
                buffer_mut.lines.push("".unicode_string());
            }
            return None;
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

                    let new_row_index = scroll_editor_buffer::inc_caret_row(
                        buffer_mut.caret_raw,
                        buffer_mut.scr_ofs,
                        buffer_mut.vp.row_height,
                    );

                    scroll_editor_buffer::reset_caret_col(
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
                    scroll_editor_buffer::inc_caret_row(
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

                            scroll_editor_buffer::inc_caret_row(
                                buffer_mut.caret_raw,
                                buffer_mut.scr_ofs,
                                buffer_mut.vp.row_height,
                            );

                            scroll_editor_buffer::reset_caret_col(
                                buffer_mut.caret_raw,
                                buffer_mut.scr_ofs,
                            );
                        }
                    }
                }
            }
        }

        None
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
            use r3bl_core::{row, string_storage, Caret};

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

                    scroll_editor_buffer::set_caret_col_to(
                        buffer_mut.caret_raw,
                        buffer_mut.scr_ofs,
                        buffer_mut.vp.col_width,
                        new_line_content_display_width,
                        delete_at_this_display_col,
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

                    scroll_editor_buffer::dec_caret_row(
                        buffer_mut.caret_raw,
                        buffer_mut.scr_ofs,
                    );

                    scroll_editor_buffer::set_caret_col_to(
                        buffer_mut.caret_raw,
                        buffer_mut.scr_ofs,
                        buffer_mut.vp.col_width,
                        new_line_content_display_width,
                        // REVIEW: [x] make sure this comment is correct (logic unchanged)
                        // This caret col index goes 1 past the end of the line width, ie:
                        // `prev_line_display_width.convert_to_col_index() /*-1*/ + 1 == prev_line_display_width`
                        Caret::scroll_col_index_for_width(prev_line_display_width),
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
                        == Caret::scroll_col_index_for_width(line_width)
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

            // BUG: [ ] introduce scroll adjusted type
            // BUG: [x] fix the cut / copy bug!
            if let Some(new_caret_scr_adj) = maybe_new_caret {
                // REVIEW: [x] make sure this works (equivalent logic, not tested)
                // Equivalent to: `let caret_raw = *new_caret_scr_adj - *buffer_mut.scr_ofs;`
                // Convert scroll adjusted caret to raw caret by applying scroll offset.
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
            scroll_editor_buffer::inc_caret_col_by(
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
            scroll_editor_buffer::inc_caret_col_by(
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

/// This is marked as `pub` because `apply_change` is needed by `cargo doc`.
pub mod validate_editor_buffer_change {
    use super::*;

    pub struct EditorBufferMut<'a> {
        pub lines: &'a mut VecEditorContentLines,
        pub caret_raw: &'a mut CaretRaw,
        pub scr_ofs: &'a mut ScrOfs,
        pub sel_list: &'a mut SelectionList,
        /// - Viewport width is optional because it's only needed for caret validation.
        ///   And you can get it from [EditorEngine]. You can pass `0` if you don't have
        ///   it.
        /// - Viewport height is optional because it's only needed for caret validation.
        ///   And you can get it from [EditorEngine]. You can pass `0` if you don't have
        ///   it.
        pub vp: Dim,
    }

    // XMARK: Clever Rust, use of Drop to perform transaction close / end.

    impl Drop for EditorBufferMut<'_> {
        /// In addition to mutating the buffer, this function runs the following validations on the
        /// [EditorBuffer]'s:
        /// 1. `caret`:
        ///    - the caret is in not in the middle of a unicode segment character.
        ///    - if it is then it moves the caret.
        /// 2. `scroll_offset`:
        ///    - make sure that it's not in the middle of a wide unicode segment character.
        ///    - if it is then it moves the scroll_offset and caret.
        fn drop(&mut self) {
            // Check caret validity.
            adjust_caret_col_if_not_in_middle_of_grapheme_cluster(self);
            adjust_caret_col_if_not_in_bounds_of_line(self);
            // Check scroll_offset validity.
            if let Some(diff) = is_scroll_offset_in_middle_of_grapheme_cluster(self) {
                adjust_scroll_offset_because_in_middle_of_grapheme_cluster(self, diff);
            }
        }
    }

    impl<'a> EditorBufferMut<'a> {
        pub fn new(
            lines: &'a mut VecEditorContentLines,
            caret_raw: &'a mut CaretRaw,
            scr_ofs: &'a mut ScrOfs,
            sel_list: &'a mut SelectionList,
            vp: Dim,
        ) -> Self {
            Self {
                lines,
                caret_raw,
                scr_ofs,
                sel_list,
                vp,
            }
        }

        /// Returns the display width of the line at the caret (at it's scroll adjusted
        /// row index).
        pub fn get_line_display_width_at_caret_scr_adj_row_index(&self) -> ColWidth {
            EditorBuffer::impl_get_line_display_width_at_caret_scr_adj(
                *self.caret_raw,
                *self.scr_ofs,
                self.lines,
            )
        }
    }

    /// ```text
    ///     0   4    9    1    2    2
    ///                   4    0    5
    ///    ┌────┴────┴────┴────┴────┴⮮─┤ col
    ///  0 ┤     ├─      line     ─┤
    ///  1 ❱     TEXT-TEXT-TEXT-TEXT ░❬───┐Caret is out of
    ///  2 ┤         ▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲  ⎝bounds of line.
    ///    │         ├─    viewport   ─┤
    ///    ┴
    ///   row
    /// ```
    fn adjust_caret_col_if_not_in_bounds_of_line(
        editor_buffer_mut: &mut EditorBufferMut<'_>,
    ) {
        // Check right side of line. Clip scroll adjusted caret to max line width.
        let row_width = {
            let line_display_width_at_caret_row =
                editor_buffer_mut.get_line_display_width_at_caret_scr_adj_row_index();
            let scr_ofs_col_index = editor_buffer_mut.scr_ofs.col_index;
            width(*line_display_width_at_caret_row - *scr_ofs_col_index)
        };

        // Make sure that the col_index is within the bounds of the given line width.
        let new_caret_col_index = col(std::cmp::min(
            *editor_buffer_mut.caret_raw.col_index,
            *row_width,
        ));

        editor_buffer_mut.caret_raw.col_index = new_caret_col_index;
    }

    pub fn is_scroll_offset_in_middle_of_grapheme_cluster(
        editor_buffer_mut: &mut EditorBufferMut<'_>,
    ) -> Option<ColWidth> {
        let scroll_adjusted_caret =
            *editor_buffer_mut.caret_raw + *editor_buffer_mut.scr_ofs;

        let line_at_caret = editor_buffer_mut
            .lines
            .get(usize(*scroll_adjusted_caret.row_index))?;

        let display_width_of_str_at_caret = {
            let str_at_caret = line_at_caret
                .get_string_at_display_col_index(scroll_adjusted_caret.col_index);
            match str_at_caret {
                None => width(0),
                Some(string_at_caret) => string_at_caret.seg_display_width,
            }
        };

        if let Some(segment) = line_at_caret
            .is_display_col_index_in_middle_of_grapheme_cluster(
                editor_buffer_mut.scr_ofs.col_index,
            )
        {
            let diff = segment.unicode_width - display_width_of_str_at_caret;
            return Some(diff);
        };

        None
    }

    pub fn adjust_scroll_offset_because_in_middle_of_grapheme_cluster(
        editor_buffer_mut: &mut EditorBufferMut<'_>,
        diff: ColWidth,
    ) -> Option<()> {
        editor_buffer_mut.scr_ofs.col_index += diff;
        editor_buffer_mut.caret_raw.col_index -= diff;
        None
    }

    /// This function is visible inside the editor_ops.rs module only. It is not meant to
    /// be called directly, but instead is called by the [Drop] impl of [EditorBufferMut].
    pub fn adjust_caret_col_if_not_in_middle_of_grapheme_cluster(
        editor_buffer_mut: &mut EditorBufferMut<'_>,
    ) -> Option<()> {
        let caret_scr_adj = *editor_buffer_mut.caret_raw + *editor_buffer_mut.scr_ofs;
        let row_index = caret_scr_adj.row_index;
        let col_index = caret_scr_adj.col_index;
        let line = editor_buffer_mut.lines.get(row_index.as_usize())?;

        // Caret is in the middle of a grapheme cluster, so jump it.
        let segment =
            line.is_display_col_index_in_middle_of_grapheme_cluster(col_index)?;
        scroll_editor_buffer::set_caret_col_to(
            editor_buffer_mut.caret_raw,
            editor_buffer_mut.scr_ofs,
            editor_buffer_mut.vp.col_width,
            line.display_width,
            segment.start_display_col_index + segment.unicode_width,
        );

        None
    }
}

/// For more information on scrolling, take a look at the
/// [scroll_editor_buffer::inc_caret_col_by] docs.
// REVIEW: [x] try remove add +1 or -1 from this file
pub mod scroll_editor_buffer {
    use super::*;

    /// # Scrolling not active
    ///
    /// Note that a caret is allowed to "go past" the end of its max index, so max index +
    /// 1 is a valid position. This is without taking scrolling into account. The max
    /// index must still be within the viewport (max index) bounds.
    ///
    /// - Let's assume the caret is represented by "░".
    /// - Think about typing "hello", and you expected the caret "░" to go past the end of
    ///   the string "hello░".
    /// - So the caret's col index is 5 in this case. Still within viewport bounds (max
    ///   index). But greater than the line content max index (4).
    ///
    /// ```text
    /// R ┌──────────┐
    /// 0 ▸hello░    │
    ///   └─────▴────┘
    ///   C0123456789
    /// ```
    ///
    /// # Scrolling active
    ///
    /// When scrolling is introduced (or activated), this behavior changes a bit. The
    /// caret can't be allowed to go past the viewport bounds. So the caret must be
    /// adjusted to the end of the line. In this case if the text is "helloHELLOhello"
    /// then the following will be displayed (the caret is at the end of the line on top
    /// of the "o"). You can see this in action in the test
    /// `test_editor_ops::editor_move_caret_home_end_overflow_viewport()`.
    ///
    // <!-- cspell:disable -->
    /// ```text
    /// R ┌──────────┐
    /// 0 ▸ELLOhello░│
    ///   └─────────▴┘
    ///   C0123456789
    /// ```
    // <!-- cspell:enable -->
    ///
    /// And scroll offset will be adjusted to show the end of the line. So the numbers
    /// will be as follows:
    /// - caret_raw: col(9) + row(0)
    /// - scr_ofs:   col(6) + row(0)
    ///
    /// Once this function runs, it is necessary to run the [Drop] impl for
    /// [validate_editor_buffer_change::EditorBufferMut] in
    /// [mod@validate_editor_buffer_change].
    pub fn inc_caret_col_by(
        caret_raw: &mut CaretRaw,
        scr_ofs: &mut ScrOfs,
        col_amt: ColWidth,
        line_display_width: ColWidth,
        vp_width: ColWidth,
    ) {
        // Just move the caret right.
        caret_raw.add_col_with_bounds(col_amt, line_display_width);

        // Check to see if viewport needs to be scrolled.
        // The following are equivalent:
        // - `a >= b`
        // - `a >  b-1`
        // The following are equivalent:
        // - caret_raw.col_index >= vp_width
        // - caret_raw.col_index > vp_width - 1 (aka vp_width.convert_to_col_index())
        // REVIEW: [x] make sure this works (equivalent changed logic, not tested)
        let overflow_viewport_width =
            caret_raw.col_index > vp_width.convert_to_col_index();

        if overflow_viewport_width {
            // REVIEW: [x] EXPERIMENT!!! remove dangling +1 using ColIndex::convert_to_width()
            // The following is equivalent to:
            // `let diff_overflow = caret_raw.col_index + col(1) + vp_width;`
            let diff_overflow = caret_raw.col_index.convert_to_width() - vp_width;
            scr_ofs.col_index += diff_overflow; // Activate horiz scroll.
            caret_raw.col_index -= diff_overflow; // Shift caret.
        }
    }

    /// Try and leave the caret where it is, however, if the caret is out of the viewport,
    /// then scroll.
    ///
    /// Once this function runs, it is necessary to run the [Drop] impl for
    /// [validate_editor_buffer_change::EditorBufferMut] in
    /// [mod@validate_editor_buffer_change].
    pub fn clip_caret_to_content_width(args: EditorArgsMut<'_>) {
        let EditorArgsMut { buffer, engine } = args;

        let caret_scr_adj = buffer.get_caret_scr_adj();
        let line_display_width = buffer.get_line_display_width_at_caret_scr_adj();

        // line_content_display_width - 1 is the last col index
        // The following are equivalent:
        // - `a >= b`
        // - `a >  b-1`
        // The following are equivalent:
        // - col_index >= line_content_display_width
        // - col_index > line_content_display_width - 1
        // REVIEW: [x] make sure this works (equivalent changed logic, not tested)
        let overflow_content_width =
            caret_scr_adj.col_index > line_display_width.convert_to_col_index();

        if overflow_content_width {
            caret_mut::to_end_of_line(buffer, engine, SelectMode::Disabled);
        }
    }

    /// Once this function runs, it is necessary to run the [Drop] impl for
    /// [validate_editor_buffer_change::EditorBufferMut] in
    /// [mod@validate_editor_buffer_change].
    pub fn set_caret_col_to(
        caret_raw: &mut CaretRaw,
        scr_ofs: &mut ScrOfs,
        vp_width: ColWidth,
        line_content_display_width: ColWidth,
        desired_col_index: ColIndex,
    ) {
        let curr_caret_scr_adj_col = (*caret_raw + *scr_ofs).col_index;

        match curr_caret_scr_adj_col.cmp(&desired_col_index) {
            Ordering::Less => {
                // Move caret right.
                let diff = desired_col_index - curr_caret_scr_adj_col;
                inc_caret_col_by(
                    caret_raw,
                    scr_ofs,
                    width(*diff),
                    line_content_display_width,
                    vp_width,
                );
            }
            Ordering::Greater => {
                // Move caret left.
                let diff = curr_caret_scr_adj_col - desired_col_index;
                dec_caret_col_by(caret_raw, scr_ofs, width(*diff));
            }
            Ordering::Equal => {
                // Do nothing.
            }
        }
    }

    /// This does not simply decrement the caret.col_index but mutates scroll_offset if
    /// scrolling is active.
    ///
    /// Once this function runs, it is necessary to run the [Drop] impl for
    /// [validate_editor_buffer_change::EditorBufferMut] in
    /// [mod@validate_editor_buffer_change].
    pub fn dec_caret_col_by(
        caret_raw: &mut CaretRaw,
        scr_ofs: &mut ScrOfs,
        col_amt: ColWidth,
    ) {
        let horiz_scroll_active = scr_ofs.col_index > col(0);
        let not_at_start_of_viewport = caret_raw.col_index > col(0);

        match (horiz_scroll_active, not_at_start_of_viewport) {
            // Scroll inactive. Simply move caret left by col_amt.
            (false, _) => {
                caret_raw.col_index -= col_amt;
            }
            // Scroll active & At start of viewport.
            (true, false) => {
                // Safe to sub, since scroll_offset.col_index can never be negative.
                scr_ofs.col_index -= col_amt;
            }
            // Scroll active & Not at start of viewport.
            (true, true) => {
                // REVIEW: [x] make sure this works
                // - Used to be: `col_amt > caret_raw.col_index`
                // - And `a > b` === `a >= b+1`
                let need_to_scroll_left =
                    col_amt >= caret_raw.col_index.convert_to_width() /*+1*/;

                match need_to_scroll_left {
                    // Just move caret left by col_amt.
                    false => {
                        caret_raw.col_index -= col_amt;
                    }
                    // Adjust caret and scroll_offset.
                    true => {
                        // Move caret left by col_amt.
                        caret_raw.col_index -= col_amt;

                        // Move scroll left by diff.
                        // REVIEW: [x] make sure this works (equivalent changed logic, not tested)
                        scr_ofs.col_index -= {
                            let lhs = Caret::scroll_col_index_for_width(col_amt);
                            let rhs = caret_raw.col_index;
                            lhs - rhs
                        };
                    }
                }
            }
        }
    }

    /// Once this function runs, it is necessary to run the [Drop] impl for
    /// [validate_editor_buffer_change::EditorBufferMut] in
    /// [mod@validate_editor_buffer_change].
    pub fn reset_caret_col(caret_raw: &mut CaretRaw, scr_ofs: &mut ScrOfs) {
        *scr_ofs.col_index = ch(0);
        *caret_raw.col_index = ch(0);
    }

    /// Decrement caret.row_index by 1, and adjust scrolling if active. This won't check
    /// whether it is inside or outside the buffer content boundary. You should check that
    /// before calling this function.
    ///
    /// This does not simply decrement the caret.row_index but mutates scroll_offset if
    /// scrolling is active. This can end up deactivating vertical scrolling as well.
    ///
    /// > Since caret.row_index can never be negative, this function must handle changes
    /// > to scroll_offset itself, and can't rely on the validations in
    /// > [mod@validate_editor_buffer_change].
    ///
    /// Once this function runs, it is necessary to run the [Drop] impl for
    /// [validate_editor_buffer_change::EditorBufferMut] in
    /// [mod@validate_editor_buffer_change].
    pub fn dec_caret_row(caret_raw: &mut CaretRaw, scr_ofs: &mut ScrOfs) -> RowIndex {
        let vert_scroll_active = scr_ofs.row_index > row(0);
        let not_at_top_of_viewport = caret_raw.row_index > row(0);

        // REVIEW: [x] make sure this works

        match (vert_scroll_active, not_at_top_of_viewport) {
            // Vertical scroll inactive.
            (false, _) => {
                // Scroll inactive.
                // Safe to minus 1, since caret.row_index can never be negative.
                caret_raw.row_index -= row(1);
            }
            // Scroll active & Not at top of viewport.
            (true, true) => {
                caret_raw.row_index -= height(1);
            }
            // Scroll active & At top of viewport.
            (true, false) => {
                // Safe to minus 1, since scroll_offset.row_index can never be negative.
                scr_ofs.row_index -= height(1);
            }
        };

        (*caret_raw + *scr_ofs).row_index
    }

    /// Try to increment caret.row_index by row_amt. This will not scroll past the bottom of the buffer. It
    /// will also activate scrolling if needed.
    ///
    /// ```text
    /// +---------------------+
    /// 0                     |
    /// |        above        | <- caret_row_adj
    /// |                     |
    /// +--- scroll_offset ---+
    /// |         ↑           |
    /// |                     |
    /// |      within vp      |
    /// |                     |
    /// |         ↓           |
    /// +--- scroll_offset ---+
    /// |    + vp height      |
    /// |                     |
    /// |        below        | <- caret_row_adj
    /// |                     |
    /// +---------------------+
    /// ```
    pub fn change_caret_row_by(
        args: EditorArgsMut<'_>,
        row_amt: RowHeight,
        direction: CaretDirection,
    ) {
        let EditorArgsMut { buffer, engine } = args;

        match direction {
            CaretDirection::Down => {
                let current_caret_adj_row = buffer.get_caret_scr_adj().row_index;
                let mut desired_caret_adj_row = current_caret_adj_row + row_amt;
                scroll_editor_buffer::clip_caret_row_to_content_height(
                    buffer,
                    &mut desired_caret_adj_row,
                );

                // Calculate how many rows we need to increment caret row by.
                let mut diff = desired_caret_adj_row - current_caret_adj_row;

                // When buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let buffer_mut = buffer.get_mut(engine.viewport());

                    while diff > row(0) {
                        scroll_editor_buffer::inc_caret_row(
                            buffer_mut.caret_raw,
                            buffer_mut.scr_ofs,
                            buffer_mut.vp.row_height,
                        );
                        diff -= row(1);
                    }
                }
            }
            CaretDirection::Up => {
                let mut diff = row_amt;

                // When buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let buffer_mut = buffer.get_mut(engine.viewport());

                    while diff > height(0) {
                        scroll_editor_buffer::dec_caret_row(
                            buffer_mut.caret_raw,
                            buffer_mut.scr_ofs,
                        );
                        diff -= height(1);
                        let row_index =
                            (*buffer_mut.caret_raw + *buffer_mut.scr_ofs).row_index;
                        if row_index == row(0) {
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Clip desired_caret_adj_row (to the max buffer length) if it overflows past the
    /// bottom of the buffer.
    pub fn clip_caret_row_to_content_height(
        buffer: &EditorBuffer,
        desired_caret_scr_adj_row_index: &mut RowIndex,
    ) {
        // Clip desired_caret_adj_row if it overflows past the bottom of the buffer.
        let max_row_index = buffer.get_max_row_index();
        let is_past_end_of_buffer = *desired_caret_scr_adj_row_index > max_row_index;
        if is_past_end_of_buffer {
            *desired_caret_scr_adj_row_index = max_row_index;
        }
    }

    /// Increment caret.row_index by 1, and adjust scrolling if active. This won't check whether it is
    /// inside or outside the buffer content boundary. You should check that before calling this
    /// function.
    ///
    /// Returns the new scroll adjusted caret row.
    ///
    /// This increments the caret.row_index and can activate vertical scrolling if the caret.row_index goes past
    /// the viewport height.
    ///
    /// Once this function runs, it is necessary to run the [Drop] impl for
    /// [validate_editor_buffer_change::EditorBufferMut] in
    /// [mod@validate_editor_buffer_change].
    pub fn inc_caret_row(
        caret: &mut CaretRaw,
        scroll_offset: &mut ScrOfs,
        viewport_height: RowHeight,
    ) -> RowIndex {
        // The following are equivalent:
        // - `a >= b`
        // - `a >  b-1`
        // The following are equivalent:
        // - caret_raw.row_index >= vp_height
        // - caret_raw.row_index > vp_height - 1 (aka vp_height.convert_to_row_index())
        let at_bottom_of_viewport =
            // REVIEW: [x] make sure this works (equivalent changed logic, not tested)
            //
            // Used to be `caret.row_index >= viewport_height`.
            // And: `a >= b` === `a > b-1`.
            // So: `caret.row_index > viewport_height - 1`.
            caret.row_index > viewport_height.convert_to_row_index() /*-1*/;

        // Fun fact: The following logic is the same whether scroll is active or not.
        if at_bottom_of_viewport {
            scroll_offset.row_index += row(1); // Activate scroll since at bottom of viewport.
        } else {
            caret.row_index += row(1); // Scroll inactive & Not at bottom of viewport.
        }

        (*caret + *scroll_offset).row_index
    }

    /// Check whether caret is vertically within the viewport. This is meant to be used
    /// after resize events and for [inc_caret_col_by], [inc_caret_row] operations. Note
    /// that [dec_caret_col_by] and [dec_caret_row] are handled differently (and not by
    /// this function) since they can never be negative.
    ///
    /// - If it isn't then scroll by mutating:
    ///    1. [crate::EditorContent::caret_raw]'s row , so it is within the viewport.
    ///    2. [crate::EditorContent::scr_ofs]'s row, to actually apply scrolling.
    /// - Otherwise, no changes are made.
    ///
    /// Once this function runs, it is necessary to run the [Drop] impl for
    /// [validate_editor_buffer_change::EditorBufferMut] in
    /// [mod@validate_editor_buffer_change].
    pub fn validate_scroll(args: EditorArgsMut<'_>) -> Option<()> {
        let EditorArgsMut { buffer, engine } = args;

        validate_vertical_scroll(EditorArgsMut { buffer, engine });
        validate_horizontal_scroll(EditorArgsMut { buffer, engine });

        /// Handle vertical scrolling (make sure caret is within viewport).
        ///
        /// Check whether caret is in the viewport.
        /// - If to top of viewport, then adjust scroll_offset & set it.
        /// - If to bottom of viewport, then adjust scroll_offset & set it.
        /// - If in viewport, then do nothing.
        ///
        /// ```text
        ///                    +0--------------------+
        ///                    0                     |
        ///                    |        above        | <- caret_row_adj
        ///                    |                     |
        ///                    +--- scroll_offset ---+
        ///              ->    |         ↑           |      ↑
        ///              |     |                     |      |
        ///   caret.row_index  |     |      within vp      |  vp height
        ///              |     |                     |      |
        ///              ->    |         ↓           |      ↓
        ///                    +--- scroll_offset ---+
        ///                    |    + vp height      |
        ///                    |                     |
        ///                    |        below        | <- caret_row_adj
        ///                    |                     |
        ///                    +---------------------+
        /// ```
        fn validate_vertical_scroll(args: EditorArgsMut<'_>) {
            let EditorArgsMut { buffer, engine } = args;

            let viewport = engine.viewport();

            // Make sure that caret can't go past the bottom of the buffer.
            {
                let caret_row = buffer.get_caret_scr_adj().row_index;
                let spilled_row_index = Caret::scroll_row_index_for_height(buffer.len());
                let overflows_buffer = caret_row > spilled_row_index;
                if overflows_buffer {
                    let diff = spilled_row_index - caret_row;
                    let buffer_mut = buffer.get_mut(viewport);
                    buffer_mut.caret_raw.row_index -= diff;
                }
            }

            // Make sure that scroll_offset can't go past the bottom of the buffer.
            {
                let scr_ofs_row = buffer.get_scr_ofs().row_index;
                let spilled_row_index = Caret::scroll_row_index_for_height(buffer.len());
                let overflows_buffer = scr_ofs_row > spilled_row_index;
                if overflows_buffer {
                    let diff = spilled_row_index - scr_ofs_row;
                    let buffer_mut = buffer.get_mut(viewport);
                    buffer_mut.scr_ofs.row_index -= diff;
                }
            }

            let caret_row = buffer.get_caret_scr_adj().row_index;
            let scr_ofs_row = buffer.get_scr_ofs().row_index;

            let is_caret_row_within_viewport = caret_row >= scr_ofs_row
                && caret_row <= (scr_ofs_row + viewport.row_height);
            let is_caret_row_above_viewport = caret_row < scr_ofs_row;

            match (is_caret_row_within_viewport, is_caret_row_above_viewport) {
                (true, _) => {
                    // Caret is within viewport, do nothing.
                }
                (false, true) => {
                    // Caret is above viewport.
                    let row_diff = scr_ofs_row - caret_row;
                    let buffer_mut = buffer.get_mut(viewport);
                    buffer_mut.scr_ofs.row_index -= row_diff;
                    buffer_mut.caret_raw.row_index += row_diff;
                }
                (false, false) => {
                    // Caret is below viewport.
                    let row_diff = caret_row - (scr_ofs_row + viewport.row_height);
                    let buffer_mut = buffer.get_mut(viewport);
                    buffer_mut.scr_ofs.row_index += row_diff;
                    buffer_mut.caret_raw.row_index -= row_diff;
                }
            }
        }

        /// Handle horizontal scrolling (make sure caret is within viewport).
        ///
        /// Check whether caret is in the viewport.
        /// - If to left of viewport, then adjust scroll_offset & set it.
        /// - If to right of viewport, then adjust scroll_offset & set it.
        /// - If in viewport, then do nothing.
        ///
        /// ```text
        ///           <-   vp width   ->
        /// +0--------+----------------+---------->
        /// 0         |                |
        /// | left of |<-  within vp ->| right of
        /// |         |                |
        /// +---------+----------------+---------->
        ///       scroll_offset    scroll_offset
        ///                        + vp width
        /// ```
        fn validate_horizontal_scroll(args: EditorArgsMut<'_>) {
            let EditorArgsMut { buffer, engine } = args;

            let viewport = engine.viewport();

            let caret_col = buffer.get_caret_scr_adj().col_index;
            let scr_ofs_col = buffer.get_scr_ofs().col_index;

            let is_caret_col_abs_within_viewport =
                caret_col >= scr_ofs_col && caret_col < scr_ofs_col + viewport.col_width;

            match is_caret_col_abs_within_viewport {
                true => {
                    // Caret is within viewport, nothing to do.
                }
                false => {
                    // Caret is outside viewport.
                    let buffer_mut = buffer.get_mut(viewport);

                    if caret_col < scr_ofs_col {
                        // Caret is to the left of viewport.
                        buffer_mut.scr_ofs.col_index = caret_col;
                        buffer_mut.caret_raw.col_index = col(0);
                    } else {
                        // Caret is to the right of viewport.
                        let viewport_width = buffer_mut.vp.col_width;
                        buffer_mut.scr_ofs.col_index =
                            caret_col - viewport_width + col(1);
                        buffer_mut.caret_raw.col_index =
                            viewport_width.convert_to_col_index();
                    }
                }
            }
        }
        None
    }
}

#[derive(Clone, PartialEq, Copy)]
pub enum DeleteSelectionWith {
    Backspace,
    Delete,
    AnyKey,
}
