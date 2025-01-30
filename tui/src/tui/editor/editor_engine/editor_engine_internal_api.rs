/*
 *   Copyright (c) 2022 R3BL LLC
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
use std::{cmp::Ordering, collections::HashMap, mem::replace};

use r3bl_core::{ch,
                position,
                string_storage,
                usize,
                ChUnit,
                Position,
                ScrollOffset,
                StringStorage,
                UnicodeString,
                UnicodeStringExt,
                UnicodeStringSegmentSliceResult,
                VecArray};

use crate::{editor::sizing::VecEditorContentLines,
            editor_buffer_clipboard_support,
            editor_buffer_clipboard_support::ClipboardService,
            CaretDirection,
            CaretKind,
            EditorArgsMut,
            EditorBuffer,
            EditorBufferApi,
            EditorEngine,
            LineMode,
            SelectionList};

/// Functions that implement the editor engine.
pub struct EditorEngineInternalApi;

impl EditorEngineInternalApi {
    pub fn up(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        select_mode: SelectMode,
    ) -> Option<()> {
        caret_mut::up(buffer, engine, select_mode)
    }

    pub fn left(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        select_mode: SelectMode,
    ) -> Option<()> {
        caret_mut::left(buffer, engine, select_mode)
    }

    pub fn right(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        select_mode: SelectMode,
    ) -> Option<()> {
        caret_mut::right(buffer, engine, select_mode)
    }

    pub fn down(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        select_mode: SelectMode,
    ) -> Option<()> {
        caret_mut::down(buffer, engine, select_mode)
    }

    pub fn page_up(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        select_mode: SelectMode,
    ) -> Option<()> {
        caret_mut::page_up(buffer, engine, select_mode)
    }

    pub fn page_down(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        select_mode: SelectMode,
    ) -> Option<()> {
        caret_mut::page_down(buffer, engine, select_mode)
    }

    pub fn home(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        select_mode: SelectMode,
    ) -> Option<()> {
        caret_mut::to_start_of_line(buffer, engine, select_mode)
    }

    pub fn end(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        select_mode: SelectMode,
    ) -> Option<()> {
        caret_mut::to_end_of_line(buffer, engine, select_mode)
    }

    pub fn select_all(buffer: &mut EditorBuffer, select_mode: SelectMode) -> Option<()> {
        caret_mut::select_all(buffer, select_mode)
    }

    pub fn clear_selection(buffer: &mut EditorBuffer) -> Option<()> {
        caret_mut::clear_selection(buffer)
    }

    pub fn validate_scroll(args: EditorArgsMut<'_>) {
        scroll_editor_buffer::validate_scroll(args);
    }

    pub fn string_at_caret(
        buffer: &EditorBuffer,
    ) -> Option<UnicodeStringSegmentSliceResult> {
        content_get::string_at_caret(buffer)
    }

    pub fn line_at_caret_to_string(buffer: &EditorBuffer) -> Option<&UnicodeString> {
        content_get::line_at_caret_to_string(buffer)
    }

    pub fn line_at_caret_is_empty(buffer: &EditorBuffer) -> bool {
        content_get::line_display_width_at_caret(buffer) == ch(0)
    }

    pub fn insert_str_at_caret(args: EditorArgsMut<'_>, chunk: &str) {
        content_mut::insert_str_at_caret(args, chunk)
    }

    pub fn insert_new_line_at_caret(args: EditorArgsMut<'_>) {
        content_mut::insert_new_line_at_caret(args);
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
    ) {
        editor_buffer_clipboard_support::copy_to_clipboard(buffer, clipboard)
    }

    pub fn paste_clipboard_content_into_editor(
        args: EditorArgsMut<'_>,
        clipboard: &mut impl ClipboardService,
    ) {
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
            if let LineMode::SingleLine = $arg_engine.config_options.multiline_mode {
                return None;
            }
        };

        ($arg_engine: expr, @Nothing) => {
            if let LineMode::SingleLine = $arg_engine.config_options.multiline_mode {
                return;
            }
        };
    }
}

mod caret_get {
    use super::*;

    /// Locate the col.
    pub fn find_col(editor_buffer: &EditorBuffer) -> CaretColLocationInLine {
        if caret_get::col_is_at_start_of_line(editor_buffer) {
            CaretColLocationInLine::AtStart
        } else if caret_get::col_is_at_end_of_line(editor_buffer) {
            CaretColLocationInLine::AtEnd
        } else {
            CaretColLocationInLine::InMiddle
        }
    }

    fn col_is_at_start_of_line(buffer: &EditorBuffer) -> bool {
        if content_get::line_at_caret_to_string(buffer).is_some() {
            *buffer.get_caret(CaretKind::ScrollAdjusted).col_index == 0
        } else {
            false
        }
    }

    fn col_is_at_end_of_line(buffer: &EditorBuffer) -> bool {
        if content_get::line_at_caret_to_string(buffer).is_some() {
            let line_display_width = content_get::line_display_width_at_caret(buffer);
            buffer.get_caret(CaretKind::ScrollAdjusted).col_index == line_display_width
        } else {
            false
        }
    }

    /// Locate the row.
    pub fn find_row(buffer: &EditorBuffer) -> CaretRowLocationInBuffer {
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
    /// 0 ▸          │
    ///   └▴─────────┘
    ///   C0123456789
    /// ```
    fn row_is_at_top_of_buffer(buffer: &EditorBuffer) -> bool {
        *buffer.get_caret(CaretKind::ScrollAdjusted).row_index == 0
    }

    /// ```text
    /// R ┌──────────┐
    /// 0 │a         │
    /// 1 ▸a         │
    ///   └▴─────────┘
    ///   C0123456789
    /// ```
    fn row_is_at_bottom_of_buffer(buffer: &EditorBuffer) -> bool {
        if buffer.is_empty() || buffer.get_lines().len() == 1 {
            false // If there is only one line, then the caret is not at the bottom, its at the top.
        } else {
            let max_row_count = ch(buffer.get_lines().len()) - ch(1);
            buffer.get_caret(CaretKind::ScrollAdjusted).row_index == max_row_count
        }
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
        editor_buffer: &EditorBuffer,
    ) -> Option<Position> {
        match self {
            SelectMode::Enabled => {
                Some(editor_buffer.get_caret(CaretKind::ScrollAdjusted))
            }
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
        maybe_previous_caret_display_position: Option<Position>,
        maybe_current_caret_display_position: Option<Position>,
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
        editor_buffer: &mut EditorBuffer,
        maybe_previous_caret_display_position: Option<Position>,
        maybe_current_caret_display_position: Option<Position>,
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

                match previous.row_index.cmp(&current.row_index) {
                    Ordering::Equal => EditorBufferApi::handle_selection_multiline_caret_movement_hit_top_or_bottom_of_document(
                        editor_buffer,
                        previous,
                        current,
                    ),
                    _ => EditorBufferApi::handle_selection_multiline_caret_movement(
                        editor_buffer,
                        previous,
                        current,
                    ),
                }
            }
        };

        None
    }
}

mod caret_mut {
    use super::*;

    pub fn up(
        editor_buffer: &mut EditorBuffer,
        editor_engine: &mut EditorEngine,
        select_mode: SelectMode,
    ) -> Option<()> {
        empty_check_early_return!(editor_buffer, @None);
        multiline_disabled_check_early_return!(editor_engine, @None);

        // This is only set if select_mode is enabled.
        // BUG: [ ] introduce scroll adjusted type
        let maybe_previous_caret_display_position =
            select_mode.get_caret_display_position_scroll_adjusted(editor_buffer);

        match caret_get::find_row(editor_buffer) {
            CaretRowLocationInBuffer::AtTop => {
                // Do nothing if the caret (scroll adjusted) is at the top.
                if editor_buffer.get_caret(CaretKind::ScrollAdjusted).col_index != ch(0) {
                    // When editor_buffer_mut goes out of scope, it will be dropped &
                    // validation performed.
                    {
                        let editor_buffer_mut = editor_buffer.get_mut(
                            editor_engine.viewport_width(),
                            editor_engine.viewport_height(),
                        );

                        scroll_editor_buffer::reset_caret_col(
                            editor_buffer_mut.caret,
                            editor_buffer_mut.scroll_offset,
                        );
                    }
                }
            }
            CaretRowLocationInBuffer::AtBottom | CaretRowLocationInBuffer::InMiddle => {
                {
                    // When editor_buffer_mut goes out of scope, it will be dropped &
                    // validation performed.
                    {
                        // There is a line above the caret.
                        let editor_buffer_mut = editor_buffer.get_mut(
                            editor_engine.viewport_width(),
                            editor_engine.viewport_height(),
                        );

                        scroll_editor_buffer::dec_caret_row(
                            editor_buffer_mut.caret,
                            editor_buffer_mut.scroll_offset,
                        );
                    }

                    scroll_editor_buffer::clip_caret_to_content_width(EditorArgsMut {
                        editor_buffer,
                        editor_engine,
                    });
                }
            }
        }

        // This is only set if select_mode is enabled.
        // BUG: [ ] introduce scroll adjusted type
        let maybe_current_caret_display_position =
            select_mode.get_caret_display_position_scroll_adjusted(editor_buffer);

        // This is only runs if select_mode is enabled.
        select_mode.update_selection_based_on_caret_movement_in_multiple_lines(
            editor_buffer,
            maybe_previous_caret_display_position,
            maybe_current_caret_display_position,
        );

        None
    }

    pub fn page_up(
        editor_buffer: &mut EditorBuffer,
        editor_engine: &mut EditorEngine,
        select_mode: SelectMode,
    ) -> Option<()> {
        empty_check_early_return!(editor_buffer, @None);
        multiline_disabled_check_early_return!(editor_engine, @None);

        // This is only set if select_mode is enabled.
        let maybe_previous_caret_display_position =
            select_mode.get_caret_display_position_scroll_adjusted(editor_buffer);

        let viewport_height = editor_engine.viewport_height();
        scroll_editor_buffer::change_caret_row_by(
            EditorArgsMut {
                editor_engine,
                editor_buffer,
            },
            viewport_height,
            CaretDirection::Up,
        );

        // This is only set if select_mode is enabled.
        let maybe_current_caret_display_position =
            select_mode.get_caret_display_position_scroll_adjusted(editor_buffer);

        // This is only runs if select_mode is enabled.
        select_mode.update_selection_based_on_caret_movement_in_multiple_lines(
            editor_buffer,
            maybe_previous_caret_display_position,
            maybe_current_caret_display_position,
        );

        None
    }

    pub fn down(
        editor_buffer: &mut EditorBuffer,
        editor_engine: &mut EditorEngine,
        select_mode: SelectMode,
    ) -> Option<()> {
        empty_check_early_return!(editor_buffer, @None);
        multiline_disabled_check_early_return!(editor_engine, @None);

        // This is only set if select_mode is enabled.
        let maybe_previous_caret_display_position =
            select_mode.get_caret_display_position_scroll_adjusted(editor_buffer);

        if content_get::next_line_below_caret_exists(editor_buffer) {
            // When editor_buffer_mut goes out of scope, it will be dropped &
            // validation performed.
            {
                // There is a line below the caret.
                let editor_buffer_mut = editor_buffer.get_mut(
                    editor_engine.viewport_width(),
                    editor_engine.viewport_height(),
                );

                scroll_editor_buffer::inc_caret_row(
                    editor_buffer_mut.caret,
                    editor_buffer_mut.scroll_offset,
                    editor_buffer_mut.viewport_height,
                );
            }

            scroll_editor_buffer::clip_caret_to_content_width(EditorArgsMut {
                editor_buffer,
                editor_engine,
            });
        } else {
            // Move to the end of the line.
            caret_mut::to_end_of_line(editor_buffer, editor_engine, select_mode);
        }

        // This is only set if select_mode is enabled.
        let maybe_current_caret_display_position =
            select_mode.get_caret_display_position_scroll_adjusted(editor_buffer);

        // This is only runs if select_mode is enabled.
        select_mode.update_selection_based_on_caret_movement_in_multiple_lines(
            editor_buffer,
            maybe_previous_caret_display_position,
            maybe_current_caret_display_position,
        );

        None
    }

    pub fn page_down(
        editor_buffer: &mut EditorBuffer,
        editor_engine: &mut EditorEngine,
        select_mode: SelectMode,
    ) -> Option<()> {
        empty_check_early_return!(editor_buffer, @None);
        multiline_disabled_check_early_return!(editor_engine, @None);

        // This is only set if select_mode is enabled.
        let maybe_previous_caret_display_position =
            select_mode.get_caret_display_position_scroll_adjusted(editor_buffer);

        let viewport_height = editor_engine.viewport_height();
        scroll_editor_buffer::change_caret_row_by(
            EditorArgsMut {
                editor_engine,
                editor_buffer,
            },
            viewport_height,
            CaretDirection::Down,
        );

        // This is only set if select_mode is enabled.
        let maybe_current_caret_display_position =
            select_mode.get_caret_display_position_scroll_adjusted(editor_buffer);

        // This is only runs if select_mode is enabled.
        select_mode.update_selection_based_on_caret_movement_in_multiple_lines(
            editor_buffer,
            maybe_previous_caret_display_position,
            maybe_current_caret_display_position,
        );

        None
    }

    /// Depending on [SelectMode], this acts as a:
    /// - Convenience function for simply calling [left] repeatedly.
    /// - Convenience function for simply calling [scroll_editor_buffer::reset_caret_col].
    pub fn to_start_of_line(
        editor_buffer: &mut EditorBuffer,
        editor_engine: &mut EditorEngine,
        select_mode: SelectMode,
    ) -> Option<()> {
        empty_check_early_return!(editor_buffer, @None);

        match select_mode {
            SelectMode::Enabled => {
                let caret = editor_buffer.get_caret(CaretKind::ScrollAdjusted);
                for _ in 0..caret.col_index.value {
                    left(editor_buffer, editor_engine, select_mode);
                }
            }
            SelectMode::Disabled => {
                // When editor_buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let editor_buffer_mut = editor_buffer.get_mut(
                        editor_engine.viewport_width(),
                        editor_engine.viewport_height(),
                    );

                    scroll_editor_buffer::reset_caret_col(
                        editor_buffer_mut.caret,
                        editor_buffer_mut.scroll_offset,
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
        editor_buffer: &mut EditorBuffer,
        editor_engine: &mut EditorEngine,
        select_mode: SelectMode,
    ) -> Option<()> {
        empty_check_early_return!(editor_buffer, @None);

        match select_mode {
            SelectMode::Enabled => {
                let caret = editor_buffer.get_caret(CaretKind::ScrollAdjusted);
                let line_display_width =
                    content_get::line_display_width_at_caret(editor_buffer);
                for _ in caret.col_index.value..line_display_width.value {
                    right(editor_buffer, editor_engine, select_mode);
                }
            }
            SelectMode::Disabled => {
                let line_content_display_width =
                    content_get::line_display_width_at_row_index(
                        editor_buffer.get_lines(),
                        editor_buffer.get_caret(CaretKind::ScrollAdjusted).row_index,
                    );

                // When editor_buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let editor_buffer_mut = editor_buffer.get_mut(
                        editor_engine.viewport_width(),
                        editor_engine.viewport_height(),
                    );

                    scroll_editor_buffer::set_caret_col(
                        editor_buffer_mut.caret,
                        editor_buffer_mut.scroll_offset,
                        editor_buffer_mut.viewport_width,
                        line_content_display_width,
                        line_content_display_width,
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

    pub fn select_all(
        editor_buffer: &mut EditorBuffer,
        select_mode: SelectMode,
    ) -> Option<()> {
        empty_check_early_return!(editor_buffer, @None);

        let number_of_lines = editor_buffer.get_lines().len();
        let max_row_index = ch(number_of_lines) - ch(1);
        let last_line_width = editor_buffer.get_line_display_width(max_row_index);

        editor_buffer.clear_selection();
        select_mode.update_selection_based_on_caret_movement_in_multiple_lines(
            editor_buffer,
            Some(position!(col_index: 0, row_index: 0)),
            Some(position!(col_index: last_line_width, row_index: max_row_index)),
        );

        None
    }

    /// ```text
    /// Caret : ▴, ▸
    ///
    /// Start of line:
    /// R ┌──────────┐
    /// 0 ▸abcab     │
    ///   └▴─────────┘
    ///   C0123456789
    ///
    /// Middle of line:
    /// R ┌──────────┐
    /// 0 ▸abcab     │
    ///   └───▴──────┘
    ///   C0123456789
    ///
    /// End of line:
    /// R ┌──────────┐
    /// 0 ▸abcab     │
    ///   └─────▴────┘
    ///   C0123456789
    /// ```
    pub fn right(
        editor_buffer: &mut EditorBuffer,
        editor_engine: &mut EditorEngine,
        select_mode: SelectMode,
    ) -> Option<()> {
        empty_check_early_return!(editor_buffer, @None);

        let line_is_empty =
            EditorEngineInternalApi::line_at_caret_is_empty(editor_buffer);

        let caret_col_loc_in_line = caret_get::find_col(editor_buffer);

        // This is only set if select_mode is enabled.
        let maybe_previous_caret_display_position =
            select_mode.get_caret_display_position_scroll_adjusted(editor_buffer);

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

        // This is only set if select_mode is enabled.
        let maybe_current_caret_display_position =
            select_mode.get_caret_display_position_scroll_adjusted(editor_buffer);

        // This is only runs if select_mode is enabled.
        select_mode.handle_selection_single_line_caret_movement(
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
                editor_buffer: &mut EditorBuffer,
                editor_engine: &mut EditorEngine,
            ) -> Option<()> {
                let UnicodeStringSegmentSliceResult {
                    display_width: unicode_width_at_caret,
                    ..
                } = content_get::string_at_caret(editor_buffer)?;

                let max_display_width =
                    content_get::line_display_width_at_caret(editor_buffer);

                let viewport_width = editor_engine.viewport_width();

                let maybe_char_to_right_of_caret =
                    content_get::string_to_right_of_caret(editor_buffer);

                match maybe_char_to_right_of_caret {
                    Some(right_of_caret_seg_slice_result) => {
                        let chunk_to_right_of_caret_us =
                            right_of_caret_seg_slice_result.unicode_string;

                        if chunk_to_right_of_caret_us.contains_wide_segments() {
                            let jump_by_col_width = chunk_to_right_of_caret_us
                                .display_width
                                + unicode_width_at_caret;
                            let move_left = chunk_to_right_of_caret_us.display_width;

                            // When editor_buffer_mut goes out of scope, it will be dropped &
                            // validation performed.
                            {
                                let editor_buffer_mut = editor_buffer.get_mut(
                                    editor_engine.viewport_width(),
                                    editor_engine.viewport_height(),
                                );

                                scroll_editor_buffer::inc_caret_col(
                                    editor_buffer_mut.caret,
                                    editor_buffer_mut.scroll_offset,
                                    jump_by_col_width,
                                    max_display_width,
                                    viewport_width,
                                );
                            }

                            if move_left > ch(0) {
                                // When editor_buffer_mut goes out of scope, it will be dropped &
                                // validation performed.
                                {
                                    let editor_buffer_mut = editor_buffer.get_mut(
                                        editor_engine.viewport_width(),
                                        editor_engine.viewport_height(),
                                    );

                                    scroll_editor_buffer::dec_caret_col(
                                        editor_buffer_mut.caret,
                                        editor_buffer_mut.scroll_offset,
                                        move_left,
                                    );
                                }
                            }
                        } else {
                            // When editor_buffer_mut goes out of scope, it will be dropped &
                            // validation performed.
                            {
                                let editor_buffer_mut = editor_buffer.get_mut(
                                    editor_engine.viewport_width(),
                                    editor_engine.viewport_height(),
                                );

                                scroll_editor_buffer::inc_caret_col(
                                    editor_buffer_mut.caret,
                                    editor_buffer_mut.scroll_offset,
                                    unicode_width_at_caret,
                                    max_display_width,
                                    viewport_width,
                                );
                            }
                        }
                    }

                    None => {
                        // When editor_buffer_mut goes out of scope, it will be dropped &
                        // validation performed.
                        {
                            let editor_buffer_mut = editor_buffer.get_mut(
                                editor_engine.viewport_width(),
                                editor_engine.viewport_height(),
                            );

                            scroll_editor_buffer::inc_caret_col(
                                editor_buffer_mut.caret,
                                editor_buffer_mut.scroll_offset,
                                unicode_width_at_caret,
                                max_display_width,
                                viewport_width,
                            );
                        }
                    }
                }

                None
            }

            pub fn right_at_end(
                editor_buffer: &mut EditorBuffer,
                editor_engine: &mut EditorEngine,
            ) -> Option<()> {
                if content_get::next_line_below_caret_exists(editor_buffer) {
                    // If there is a line below the caret, move the caret to the start of
                    // the next line.

                    // When editor_buffer_mut goes out of scope, it will be dropped &
                    // validation performed.
                    {
                        let editor_buffer_mut = editor_buffer.get_mut(
                            editor_engine.viewport_width(),
                            editor_engine.viewport_height(),
                        );

                        scroll_editor_buffer::inc_caret_row(
                            editor_buffer_mut.caret,
                            editor_buffer_mut.scroll_offset,
                            editor_buffer_mut.viewport_height,
                        );

                        scroll_editor_buffer::reset_caret_col(
                            editor_buffer_mut.caret,
                            editor_buffer_mut.scroll_offset,
                        );
                    }
                }

                None
            }
        }
    }

    pub fn left(
        editor_buffer: &mut EditorBuffer,
        editor_engine: &mut EditorEngine,
        select_mode: SelectMode,
    ) -> Option<()> {
        empty_check_early_return!(editor_buffer, @None);

        // This is only set if select_mode is enabled.
        let maybe_previous_caret_display_position =
            select_mode.get_caret_display_position_scroll_adjusted(editor_buffer);

        match caret_get::find_col(editor_buffer) {
            CaretColLocationInLine::AtStart => {
                if content_get::prev_line_above_caret_exists(editor_buffer) {
                    // If there is a line above the caret, move the caret to the end of
                    // the previous line.

                    // When editor_buffer_mut goes out of scope, it will be dropped &
                    // validation performed.
                    {
                        let editor_buffer_mut = editor_buffer.get_mut(
                            editor_engine.viewport_width(),
                            editor_engine.viewport_height(),
                        );

                        scroll_editor_buffer::dec_caret_row(
                            editor_buffer_mut.caret,
                            editor_buffer_mut.scroll_offset,
                        );
                    }

                    caret_mut::to_end_of_line(
                        editor_buffer,
                        editor_engine,
                        SelectMode::Disabled,
                    );
                }
            }
            CaretColLocationInLine::AtEnd => {
                let seg_slice =
                    content_get::string_at_end_of_line_at_caret(editor_buffer)?;
                // When editor_buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let editor_buffer_mut = editor_buffer.get_mut(
                        editor_engine.viewport_width(),
                        editor_engine.viewport_height(),
                    );

                    scroll_editor_buffer::dec_caret_col(
                        editor_buffer_mut.caret,
                        editor_buffer_mut.scroll_offset,
                        seg_slice.display_width,
                    );
                }
            }
            CaretColLocationInLine::InMiddle => {
                let seg_slice = content_get::string_to_left_of_caret(editor_buffer)?;
                // When editor_buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let editor_buffer_mut = editor_buffer.get_mut(
                        editor_engine.viewport_width(),
                        editor_engine.viewport_height(),
                    );

                    scroll_editor_buffer::dec_caret_col(
                        editor_buffer_mut.caret,
                        editor_buffer_mut.scroll_offset,
                        seg_slice.display_width,
                    );
                }
            }
        }

        // This is only set if select_mode is enabled.
        let maybe_current_caret_display_position =
            select_mode.get_caret_display_position_scroll_adjusted(editor_buffer);

        // This is only runs if select_mode is enabled.
        select_mode.handle_selection_single_line_caret_movement(
            editor_buffer,
            maybe_previous_caret_display_position,
            maybe_current_caret_display_position,
        );

        None
    }
}

mod content_get {
    use super::*;

    pub fn line_display_width_at_caret(buffer: &EditorBuffer) -> ChUnit {
        let row_index = buffer.get_caret(CaretKind::ScrollAdjusted).row_index;
        line_display_width_at_row_index(buffer.get_lines(), row_index)
    }

    pub fn line_display_width_at_row_index(
        lines: &VecEditorContentLines,
        row_idx: ChUnit,
    ) -> ChUnit {
        let maybe_line_us = lines.get(usize(row_idx));
        if let Some(line_us) = maybe_line_us {
            line_us.display_width
        } else {
            ch(0)
        }
    }

    pub fn next_line_below_caret_exists(buffer: &EditorBuffer) -> bool {
        let next_line = content_get::next_line_below_caret_to_string(buffer);
        next_line.is_some()
    }

    pub fn line_at_caret_to_string(buffer: &EditorBuffer) -> Option<&UnicodeString> {
        empty_check_early_return!(buffer, @None);
        let row_index = buffer.get_caret(CaretKind::ScrollAdjusted).row_index;
        let line = buffer.get_lines().get(usize(row_index))?;
        Some(line)
    }

    pub fn next_line_below_caret_to_string(
        buffer: &EditorBuffer,
    ) -> Option<&UnicodeString> {
        empty_check_early_return!(buffer, @None);
        let row_index = buffer.get_caret(CaretKind::ScrollAdjusted).row_index;
        let line = buffer.get_lines().get(usize(row_index + ch(1)))?;
        Some(line)
    }

    pub fn prev_line_above_caret_exists(buffer: &EditorBuffer) -> bool {
        let prev_line = content_get::prev_line_above_caret_to_string(buffer);
        prev_line.is_some()
    }

    pub fn prev_line_above_caret_to_string(
        buffer: &EditorBuffer,
    ) -> Option<&UnicodeString> {
        empty_check_early_return!(buffer, @None);
        let row_index = buffer.get_caret(CaretKind::ScrollAdjusted).row_index;
        if row_index == ch(0) {
            return None;
        }
        let line = buffer.get_lines().get(usize(row_index - ch(1)))?;
        Some(line)
    }

    pub fn string_at_caret(
        buffer: &EditorBuffer,
    ) -> Option<UnicodeStringSegmentSliceResult> {
        empty_check_early_return!(buffer, @None);
        let caret_adj = buffer.get_caret(CaretKind::ScrollAdjusted);
        let line = buffer.get_lines().get(usize(caret_adj.row_index))?;
        let result = line.get_string_at_display_col_index(caret_adj.col_index)?;
        Some(result)
    }

    pub fn string_to_right_of_caret(
        buffer: &EditorBuffer,
    ) -> Option<UnicodeStringSegmentSliceResult> {
        empty_check_early_return!(buffer, @None);
        let caret_adj = buffer.get_caret(CaretKind::ScrollAdjusted);
        let line = buffer.get_lines().get(usize(caret_adj.row_index))?;

        match caret_get::find_col(buffer) {
            // Caret is at end of line, past the last character.
            CaretColLocationInLine::AtEnd => line.get_string_at_end(),
            // Caret is not at end of line.
            _ => line.get_string_at_right_of_display_col_index(caret_adj.col_index),
        }
    }

    pub fn string_to_left_of_caret(
        buffer: &EditorBuffer,
    ) -> Option<UnicodeStringSegmentSliceResult> {
        empty_check_early_return!(buffer, @None);
        let caret_adj = buffer.get_caret(CaretKind::ScrollAdjusted);
        let line = buffer.get_lines().get(usize(caret_adj.row_index))?;

        match caret_get::find_col(buffer) {
            // Caret is at end of line, past the last character.
            CaretColLocationInLine::AtEnd => line.get_string_at_end(),
            // Caret is not at end of line.
            _ => line.get_string_at_left_of_display_col_index(caret_adj.col_index),
        }
    }

    pub fn string_at_end_of_line_at_caret(
        buffer: &EditorBuffer,
    ) -> Option<UnicodeStringSegmentSliceResult> {
        empty_check_early_return!(buffer, @None);
        let line = content_get::line_at_caret_to_string(buffer)?;

        if let CaretColLocationInLine::AtEnd = caret_get::find_col(buffer) {
            let maybe_last_str_seg = line.get_string_at_end();
            return maybe_last_str_seg;
        }
        None
    }
}

mod content_mut {
    use super::*;

    pub fn insert_str_at_caret(args: EditorArgsMut<'_>, chunk: &str) {
        let EditorArgsMut {
            editor_buffer,
            editor_engine,
        } = args;

        let caret_adj = editor_buffer.get_caret(CaretKind::ScrollAdjusted);

        let row: usize = usize(caret_adj.row_index);
        let col: usize = usize(caret_adj.col_index);

        if editor_buffer.get_lines().get(row).is_some() {
            insert_into_existing_line(
                EditorArgsMut {
                    editor_buffer,
                    editor_engine,
                },
                position!(col_index: col, row_index: row),
                chunk,
            );
        } else {
            fill_in_missing_lines_up_to_row(
                EditorArgsMut {
                    editor_buffer,
                    editor_engine,
                },
                row,
            );
            insert_into_new_line(
                EditorArgsMut {
                    editor_buffer,
                    editor_engine,
                },
                row,
                chunk,
            );
        }
    }

    pub fn insert_new_line_at_caret(args: EditorArgsMut<'_>) {
        let EditorArgsMut {
            editor_buffer,
            editor_engine,
        } = args;

        multiline_disabled_check_early_return!(editor_engine, @Nothing);

        if editor_buffer.is_empty() {
            // When editor_buffer_mut goes out of scope, it will be dropped &
            // validation performed.
            {
                let editor_buffer_mut = editor_buffer.get_mut(
                    editor_engine.viewport_width(),
                    editor_engine.viewport_height(),
                );

                editor_buffer_mut.lines.push("".unicode_string());
            }
            return;
        }

        match caret_get::find_col(editor_buffer) {
            CaretColLocationInLine::AtEnd => {
                inner::insert_new_line_at_end_of_current_line(EditorArgsMut {
                    editor_buffer,
                    editor_engine,
                });
            }
            CaretColLocationInLine::AtStart => {
                inner::insert_new_line_at_start_of_current_line(EditorArgsMut {
                    editor_buffer,
                    editor_engine,
                });
            }
            CaretColLocationInLine::InMiddle => {
                inner::insert_new_line_at_middle_of_current_line(EditorArgsMut {
                    editor_buffer,
                    editor_engine,
                });
            }
        }

        mod inner {
            use super::*;

            // Handle inserting a new line at the end of the current line.
            pub fn insert_new_line_at_end_of_current_line(args: EditorArgsMut<'_>) {
                let EditorArgsMut {
                    editor_buffer,
                    editor_engine,
                } = args;

                // When editor_buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let editor_buffer_mut = editor_buffer.get_mut(
                        editor_engine.viewport_width(),
                        editor_engine.viewport_height(),
                    );

                    let new_row_idx = scroll_editor_buffer::inc_caret_row(
                        editor_buffer_mut.caret,
                        editor_buffer_mut.scroll_offset,
                        editor_buffer_mut.viewport_height,
                    );

                    scroll_editor_buffer::reset_caret_col(
                        editor_buffer_mut.caret,
                        editor_buffer_mut.scroll_offset,
                    );

                    editor_buffer_mut
                        .lines
                        .insert(new_row_idx, "".unicode_string());
                }
            }

            // Handle inserting a new line at the start of the current line.
            pub fn insert_new_line_at_start_of_current_line(args: EditorArgsMut<'_>) {
                let EditorArgsMut {
                    editor_buffer,
                    editor_engine,
                } = args;

                // When editor_buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let editor_buffer_mut = editor_buffer.get_mut(
                        editor_engine.viewport_width(),
                        editor_engine.viewport_height(),
                    );

                    let cur_row_idx = EditorBuffer::calc_scroll_adj_caret_row(
                        editor_buffer_mut.caret,
                        editor_buffer_mut.scroll_offset,
                    );

                    editor_buffer_mut
                        .lines
                        .insert(cur_row_idx, "".unicode_string());
                }

                // When editor_buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let editor_buffer_mut = editor_buffer.get_mut(
                        editor_engine.viewport_width(),
                        editor_engine.viewport_height(),
                    );

                    scroll_editor_buffer::inc_caret_row(
                        editor_buffer_mut.caret,
                        editor_buffer_mut.scroll_offset,
                        editor_buffer_mut.viewport_height,
                    );
                }
            }

            // Handle inserting a new line at the middle of the current line.
            pub fn insert_new_line_at_middle_of_current_line(args: EditorArgsMut<'_>) {
                let EditorArgsMut {
                    editor_buffer,
                    editor_engine,
                } = args;

                if let Some(line) = content_get::line_at_caret_to_string(editor_buffer) {
                    let caret_adj = editor_buffer.get_caret(CaretKind::ScrollAdjusted);
                    let col_index = caret_adj.col_index;
                    let split_result = line.split_at_display_col(col_index);
                    if let Some((left_string, right_string)) = split_result {
                        let row_index = usize(caret_adj.row_index);

                        // When editor_buffer_mut goes out of scope, it will be dropped &
                        // validation performed.
                        {
                            let editor_buffer_mut = editor_buffer.get_mut(
                                editor_engine.viewport_width(),
                                editor_engine.viewport_height(),
                            );

                            let _ = replace(
                                &mut editor_buffer_mut.lines[row_index],
                                left_string.unicode_string(),
                            );

                            editor_buffer_mut
                                .lines
                                .insert(row_index + 1, right_string.unicode_string());

                            scroll_editor_buffer::inc_caret_row(
                                editor_buffer_mut.caret,
                                editor_buffer_mut.scroll_offset,
                                editor_buffer_mut.viewport_height,
                            );

                            scroll_editor_buffer::reset_caret_col(
                                editor_buffer_mut.caret,
                                editor_buffer_mut.scroll_offset,
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
        if content_get::string_at_caret(buffer).is_some() {
            inner::delete_in_middle_of_line(buffer, engine)?;
        } else {
            inner::delete_at_end_of_line(buffer, engine)?;
        }
        return None;

        mod inner {
            use super::*;

            /// ```text
            /// R ┌──────────┐
            /// 0 ▸abc       │
            /// 1 │ab        │
            /// 2 │a         │
            ///   └─▴────────┘
            ///   C0123456789
            /// ```
            pub fn delete_in_middle_of_line(
                buffer: &mut EditorBuffer,
                engine: &mut EditorEngine,
            ) -> Option<()> {
                let line = content_get::line_at_caret_to_string(buffer)?;

                let new_line_content = line.delete_char_at_display_col(
                    buffer.get_caret(CaretKind::ScrollAdjusted).col_index,
                )?;

                // When editor_buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let editor_buffer_mut =
                        buffer.get_mut(engine.viewport_width(), engine.viewport_height());

                    let row_idx = EditorBuffer::calc_scroll_adj_caret_row(
                        editor_buffer_mut.caret,
                        editor_buffer_mut.scroll_offset,
                    );

                    let _ = replace(
                        &mut editor_buffer_mut.lines[row_idx],
                        new_line_content.unicode_string(),
                    );
                }

                None
            }

            /// ```text
            /// R ┌──────────┐
            /// 0 ▸abc       │
            /// 1 │ab        │
            /// 2 │a         │
            ///   └───▴──────┘
            ///   C0123456789
            /// ```
            pub fn delete_at_end_of_line(
                buffer: &mut EditorBuffer,
                engine: &mut EditorEngine,
            ) -> Option<()> {
                let this_line = content_get::line_at_caret_to_string(buffer)?;
                let next_line = content_get::next_line_below_caret_to_string(buffer)?;
                let new_line_content =
                    string_storage!("{a}{b}", a = this_line.string, b = next_line.string);

                // When editor_buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let editor_buffer_mut =
                        buffer.get_mut(engine.viewport_width(), engine.viewport_height());

                    let row_idx = EditorBuffer::calc_scroll_adj_caret_row(
                        editor_buffer_mut.caret,
                        editor_buffer_mut.scroll_offset,
                    );

                    let _ = replace(
                        &mut editor_buffer_mut.lines[row_idx],
                        new_line_content.unicode_string(),
                    );

                    editor_buffer_mut.lines.remove(row_idx + 1);
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

        if let Some(UnicodeStringSegmentSliceResult {
            display_col_at_which_this_seg_starts: display_col_at_which_seg_starts,
            ..
        }) = content_get::string_to_left_of_caret(buffer)
        {
            inner::backspace_in_middle_of_line(
                buffer,
                engine,
                display_col_at_which_seg_starts,
            )?;
        } else {
            inner::backspace_at_start_of_line(buffer, engine)?;
        }

        return None;

        mod inner {
            use super::*;

            /// ```text
            /// R ┌──────────┐
            /// 0 ▸abc       │
            /// 1 │ab        │
            /// 2 │a         │
            ///   └─▴────────┘
            ///   C0123456789
            /// ```
            pub fn backspace_in_middle_of_line(
                buffer: &mut EditorBuffer,
                engine: &mut EditorEngine,
                delete_at_this_display_col: ChUnit,
            ) -> Option<()> {
                let cur_line = content_get::line_at_caret_to_string(buffer)?;
                let new_line_content =
                    cur_line.delete_char_at_display_col(delete_at_this_display_col)?;

                // When editor_buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let editor_buffer_mut =
                        buffer.get_mut(engine.viewport_width(), engine.viewport_height());

                    let cur_row_idx = EditorBuffer::calc_scroll_adj_caret_row(
                        editor_buffer_mut.caret,
                        editor_buffer_mut.scroll_offset,
                    );

                    let _ = replace(
                        &mut editor_buffer_mut.lines[cur_row_idx],
                        new_line_content.unicode_string(),
                    );

                    let new_line_content_display_width =
                        editor_buffer_mut.lines[cur_row_idx].display_width;

                    scroll_editor_buffer::set_caret_col(
                        editor_buffer_mut.caret,
                        editor_buffer_mut.scroll_offset,
                        editor_buffer_mut.viewport_width,
                        new_line_content_display_width,
                        delete_at_this_display_col,
                    );
                }

                None
            }

            /// ```text
            /// R ┌──────────┐
            /// 0 │abc       │
            /// 1 ▸ab        │
            /// 2 │a         │
            ///   └▴─────────┘
            ///   C0123456789
            /// ```
            pub fn backspace_at_start_of_line(
                buffer: &mut EditorBuffer,
                engine: &mut EditorEngine,
            ) -> Option<()> {
                let this_line = content_get::line_at_caret_to_string(buffer)?;
                let prev_line = content_get::prev_line_above_caret_to_string(buffer)?;

                // A line above the caret exists.
                let prev_line_display_width = {
                    let prev_row_idx =
                        buffer.get_caret(CaretKind::ScrollAdjusted).row_index - ch(1);
                    content_get::line_display_width_at_row_index(
                        buffer.get_lines(),
                        prev_row_idx,
                    )
                };

                let prev_line_eol_col = prev_line_display_width;

                let new_line_content =
                    string_storage!("{a}{b}", a = prev_line.string, b = this_line.string);

                // When editor_buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let editor_buffer_mut =
                        buffer.get_mut(engine.viewport_width(), engine.viewport_height());

                    let prev_row_idx = EditorBuffer::calc_scroll_adj_caret_row(
                        editor_buffer_mut.caret,
                        editor_buffer_mut.scroll_offset,
                    ) - 1;

                    let cur_row_idx = EditorBuffer::calc_scroll_adj_caret_row(
                        editor_buffer_mut.caret,
                        editor_buffer_mut.scroll_offset,
                    );

                    let _ = replace(
                        &mut editor_buffer_mut.lines[prev_row_idx],
                        new_line_content.unicode_string(),
                    );

                    let new_line_content_display_width =
                        editor_buffer_mut.lines[prev_row_idx].display_width;

                    editor_buffer_mut.lines.remove(cur_row_idx);

                    scroll_editor_buffer::dec_caret_row(
                        editor_buffer_mut.caret,
                        editor_buffer_mut.scroll_offset,
                    );

                    scroll_editor_buffer::set_caret_col(
                        editor_buffer_mut.caret,
                        editor_buffer_mut.scroll_offset,
                        editor_buffer_mut.viewport_width,
                        new_line_content_display_width,
                        prev_line_eol_col,
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
        if buffer.get_selection_map().is_empty() {
            return None;
        }

        let my_selection_map = buffer.get_selection_map().clone();
        let lines = buffer.get_lines();
        let selected_row_indices = my_selection_map.get_ordered_indices();
        let mut vec_row_indices_to_remove = VecArray::<ChUnit>::new();
        let mut map_lines_to_replace = HashMap::new();

        for selected_row_index in selected_row_indices {
            if let Some(selection_range) = my_selection_map.get(selected_row_index) {
                let line_width = buffer.get_line_display_width(selected_row_index);

                // Remove entire line.
                if selection_range.start_display_col_index_scroll_adjusted == ch(0)
                    && selection_range.end_display_col_index_scroll_adjusted == line_width
                {
                    vec_row_indices_to_remove.push(selected_row_index);
                    continue;
                }

                // Skip if selection range is empty.
                if selection_range.start_display_col_index_scroll_adjusted
                    == selection_range.end_display_col_index_scroll_adjusted
                {
                    continue;
                }

                // Remove selection range (part of the line).
                let start_col_index =
                    selection_range.start_display_col_index_scroll_adjusted;
                let end_col_index = selection_range.end_display_col_index_scroll_adjusted;
                let line_us = lines.get(usize(selected_row_index))?.clone();

                let keep_before_selected = line_us.clip_to_width(ch(0), start_col_index);

                let keep_after_selected =
                    line_us.clip_to_width(ch(end_col_index), line_width);

                let mut remaining_text = StringStorage::with_capacity(
                    keep_before_selected.len() + keep_after_selected.len(),
                );

                remaining_text.push_str(keep_before_selected);
                remaining_text.push_str(keep_after_selected);
                map_lines_to_replace.insert(selected_row_index, remaining_text);
            }
        }

        // When editor_buffer_mut goes out of scope, it will be dropped &
        // validation performed.
        {
            let editor_buffer_mut =
                buffer.get_mut(engine.viewport_width(), engine.viewport_height());

            // Replace lines, before removing them (to prevent indices from being invalidated).
            for row_index in map_lines_to_replace.keys() {
                let new_line_content = map_lines_to_replace[row_index].clone();
                let _ = replace(
                    &mut editor_buffer_mut.lines[usize(*row_index)],
                    new_line_content.unicode_string(),
                );
            }

            // Remove lines in inverse order, in order to preserve the validity of indices.
            vec_row_indices_to_remove.reverse();
            for row_index in vec_row_indices_to_remove {
                editor_buffer_mut.lines.remove(usize(row_index));
            }

            // Restore caret position to start of selection range.
            let maybe_new_position =
                my_selection_map.get_caret_at_start_of_range_scroll_adjusted(with);

            // BUG: [ ] introduce scroll adjusted type
            // BUG: [x] fix the cut / copy bug!
            if let Some(new_caret_position) = maybe_new_position {
                let it = EditorBuffer::convert_from_scroll_adjusted_to_raw_caret(
                    new_caret_position,
                    *editor_buffer_mut.scroll_offset,
                );
                editor_buffer_mut.caret.row_index = it.row_index;
                editor_buffer_mut.caret.col_index = it.col_index;
            }
        }

        buffer.clear_selection();

        None
    }

    fn insert_into_existing_line(
        args: EditorArgsMut<'_>,
        caret_adj: Position,
        chunk: &str,
    ) -> Option<()> {
        let EditorArgsMut {
            editor_buffer,
            editor_engine,
        } = args;

        let row_index = usize(caret_adj.row_index);
        let line = editor_buffer.get_lines().get(row_index)?;
        let (new_line_content, chunk_display_width) =
            line.insert_chunk_at_display_col(ch(caret_adj.col_index), chunk);

        let viewport_width = editor_engine.viewport_width();

        // When editor_buffer_mut goes out of scope, it will be dropped &
        // validation performed.
        {
            let editor_buffer_mut = editor_buffer.get_mut(
                editor_engine.viewport_width(),
                editor_engine.viewport_height(),
            );

            // Replace existing line w/ new line.
            let _ = replace(
                &mut editor_buffer_mut.lines[row_index],
                new_line_content.unicode_string(),
            );

            let new_line_content_display_width =
                editor_buffer_mut.lines[row_index].display_width;

            // Update caret position.
            scroll_editor_buffer::inc_caret_col(
                editor_buffer_mut.caret,
                editor_buffer_mut.scroll_offset,
                chunk_display_width,
                new_line_content_display_width,
                viewport_width,
            );
        }

        None
    }

    fn fill_in_missing_lines_up_to_row(args: EditorArgsMut<'_>, caret_row: usize) {
        let EditorArgsMut {
            editor_buffer,
            editor_engine,
        } = args;

        // Fill in any missing lines.
        if editor_buffer.get_lines().get(caret_row).is_none() {
            for row_idx in 0..caret_row + 1 {
                if editor_buffer.get_lines().get(row_idx).is_none() {
                    // When editor_buffer_mut goes out of scope, it will be dropped &
                    // validation performed.
                    {
                        let editor_buffer_mut = editor_buffer.get_mut(
                            editor_engine.viewport_width(),
                            editor_engine.viewport_height(),
                        );

                        editor_buffer_mut.lines.push("".unicode_string());
                    }
                }
            }
        }
    }

    fn insert_into_new_line(
        args: EditorArgsMut<'_>,
        caret_adj_row: usize,
        chunk: &str,
    ) -> Option<()> {
        let EditorArgsMut {
            editor_buffer,
            editor_engine,
        } = args;

        // Make sure there's a line at caret_adj_row.
        let _ = editor_buffer.get_lines().get(caret_adj_row)?;

        let viewport_width = editor_engine.viewport_width();

        // When editor_buffer_mut goes out of scope, it will be dropped &
        // validation performed.
        {
            let editor_buffer_mut = editor_buffer.get_mut(
                editor_engine.viewport_width(),
                editor_engine.viewport_height(),
            );

            // Actually add the character to the correct line.
            let new_content = chunk.unicode_string();
            let _ = replace(
                &mut editor_buffer_mut.lines[usize(caret_adj_row)],
                new_content,
            );

            let line_content = &editor_buffer_mut.lines[caret_adj_row];
            let line_content_display_width = line_content.display_width;
            let col_amt = UnicodeString::str_display_width(chunk);

            // Update caret position.
            scroll_editor_buffer::inc_caret_col(
                editor_buffer_mut.caret,
                editor_buffer_mut.scroll_offset,
                col_amt,
                line_content_display_width,
                viewport_width,
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
        pub caret: &'a mut Position,
        pub scroll_offset: &'a mut ScrollOffset,
        pub selection_map: &'a mut SelectionList,
        /// This is optional because it's only needed for caret validation. And you can
        /// get it from [EditorEngine].
        pub viewport_width: ChUnit,
        /// This is optional because it's only needed for caret validation. And you can
        /// get it from [EditorEngine].
        pub viewport_height: ChUnit,
    }

    // BOOKM: Clever Rust, use of Drop to perform transaction close / end.

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
            caret: &'a mut Position,
            scroll_offset: &'a mut ScrollOffset,
            selection_map: &'a mut SelectionList,
            viewport_width: ChUnit,
            viewport_height: ChUnit,
        ) -> Self {
            Self {
                lines,
                caret,
                scroll_offset,
                selection_map,
                viewport_width,
                viewport_height,
            }
        }
    }

    /// ```text
    ///     0   4    9    1    2    2
    ///                   4    0    5
    ///    ┌────┴────┴────┴────┴────┴▾─→ col
    ///  0 ┤
    ///  1 ▸     TEXT-TEXT-TEXT-TEXT ░←── Caret is out of bounds of line.
    ///  2 ┤         ▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲
    ///    ↓         │←    viewport   →│
    ///    row
    /// ```
    fn adjust_caret_col_if_not_in_bounds_of_line(
        editor_buffer_mut: &mut EditorBufferMut<'_>,
    ) -> Option<()> {
        // Check right side of line. Clip scroll adjusted caret to max line width.
        let scroll_adjusted_caret = EditorBuffer::get_caret_adjusted(
            *editor_buffer_mut.caret,
            *editor_buffer_mut.scroll_offset,
            CaretKind::ScrollAdjusted,
        );
        let row_content_width = content_get::line_display_width_at_row_index(
            editor_buffer_mut.lines,
            scroll_adjusted_caret.row_index,
        ) - editor_buffer_mut.scroll_offset.col_index;

        let new_caret_col_index = validate_col_index_for_row_width(
            editor_buffer_mut.caret.col_index,
            row_content_width,
        );
        editor_buffer_mut.caret.col_index = new_caret_col_index;

        None
    }

    /// Make sure that the col_index is within the bounds of the given line width.
    fn validate_col_index_for_row_width(col_index: ChUnit, row_width: ChUnit) -> ChUnit {
        if row_width == ch(0) {
            return ch(0);
        }
        if col_index > row_width {
            row_width
        } else {
            col_index
        }
    }

    pub fn is_scroll_offset_in_middle_of_grapheme_cluster(
        editor_buffer_mut: &mut EditorBufferMut<'_>,
    ) -> Option<ChUnit> {
        let scroll_adjusted_caret = EditorBuffer::get_caret_adjusted(
            *editor_buffer_mut.caret,
            *editor_buffer_mut.scroll_offset,
            CaretKind::ScrollAdjusted,
        );
        let line = editor_buffer_mut
            .lines
            .get(usize(scroll_adjusted_caret.row_index))?;

        let unicode_width_at_caret = {
            let it =
                line.get_string_at_display_col_index(scroll_adjusted_caret.col_index);
            match it {
                None => ch(0),
                Some(string_at_caret) => string_at_caret.display_width,
            }
        };

        if let Some(segment) = line.is_display_col_index_in_middle_of_grapheme_cluster(
            editor_buffer_mut.scroll_offset.col_index,
        ) {
            let diff = segment.unicode_width - unicode_width_at_caret;
            return Some(diff);
        };

        None
    }

    pub fn adjust_scroll_offset_because_in_middle_of_grapheme_cluster(
        editor_buffer_mut: &mut EditorBufferMut<'_>,
        diff: ChUnit,
    ) -> Option<()> {
        editor_buffer_mut.scroll_offset.col_index += diff;
        editor_buffer_mut.caret.col_index -= diff;
        None
    }

    /// This function is visible inside the editor_ops.rs module only. It is not meant to
    /// be called directly, but instead is called by the [Drop] impl of [EditorBufferMut].
    pub fn adjust_caret_col_if_not_in_middle_of_grapheme_cluster(
        editor_buffer_mut: &mut EditorBufferMut<'_>,
    ) -> Option<()> {
        let row_idx = EditorBuffer::calc_scroll_adj_caret_row(
            editor_buffer_mut.caret,
            editor_buffer_mut.scroll_offset,
        );
        let col_idx = ch(EditorBuffer::calc_scroll_adj_caret_col(
            editor_buffer_mut.caret,
            editor_buffer_mut.scroll_offset,
        ));

        let line = editor_buffer_mut.lines.get(row_idx)?;

        // Caret is in the middle of a grapheme cluster, so jump it.
        let segment = line.is_display_col_index_in_middle_of_grapheme_cluster(col_idx)?;
        scroll_editor_buffer::set_caret_col(
            editor_buffer_mut.caret,
            editor_buffer_mut.scroll_offset,
            editor_buffer_mut.viewport_width,
            line.display_width,
            segment.unicode_width + segment.display_col_offset,
        );

        None
    }
}

mod scroll_editor_buffer {
    use super::*;

    /// Try and leave the caret where it is, however, if the caret is out of the viewport, then
    /// scroll. This is meant to be called inside [validate::apply_change].
    pub fn clip_caret_to_content_width(args: EditorArgsMut<'_>) {
        let EditorArgsMut {
            editor_buffer,
            editor_engine,
        } = args;

        let caret = editor_buffer.get_caret(CaretKind::ScrollAdjusted);
        let scroll_offset = editor_buffer.get_scroll_offset();
        let line_content_display_width =
            content_get::line_display_width_at_caret(editor_buffer);

        let caret_adj_col = ch(EditorBuffer::calc_scroll_adj_caret_col(
            &caret,
            &scroll_offset,
        ));

        let is_caret_col_overflow_content_width =
            caret_adj_col >= line_content_display_width;

        if is_caret_col_overflow_content_width {
            caret_mut::to_end_of_line(editor_buffer, editor_engine, SelectMode::Disabled);
        }
    }

    /// This is meant to be called inside [validate::apply_change] or
    /// [validate::validate_caret_col_position_not_in_middle_of_grapheme_cluster].
    pub fn set_caret_col(
        caret: &mut Position,
        scroll_offset: &mut ScrollOffset,
        viewport_width: ChUnit,
        line_content_display_width: ChUnit,
        desired_col: ChUnit,
    ) {
        let caret_adj_col = ch(EditorBuffer::calc_scroll_adj_caret_col(
            caret,
            scroll_offset,
        ));

        match caret_adj_col.cmp(&desired_col) {
            Ordering::Less => {
                // Move caret right.
                let diff = desired_col - caret_adj_col;
                inc_caret_col(
                    caret,
                    scroll_offset,
                    diff,
                    line_content_display_width,
                    viewport_width,
                );
            }
            Ordering::Greater => {
                // Move caret left.
                let diff = caret_adj_col - desired_col;
                dec_caret_col(caret, scroll_offset, diff);
            }
            Ordering::Equal => {
                // Do nothing.
            }
        }
    }

    /// This is meant to be called inside [validate::apply_change].
    pub fn inc_caret_col(
        caret: &mut Position,
        scroll_offset: &mut ScrollOffset,
        col_amt: ChUnit,
        line_content_display_width: ChUnit,
        viewport_width: ChUnit,
    ) {
        // Just move the caret right.
        caret.add_col_with_bounds(col_amt, line_content_display_width);

        // Check to see if viewport needs to be scrolled.
        let is_caret_col_overflow_viewport_width = caret.col_index >= viewport_width;

        if is_caret_col_overflow_viewport_width {
            let diff_overflow = ch(1) + caret.col_index - viewport_width;
            scroll_offset.col_index += diff_overflow; // Activate horiz scroll.
            caret.col_index -= diff_overflow; // Shift caret.
        }
    }

    /// This does not simply decrement the caret.col but mutates scroll_offset if scrolling is
    /// active.
    ///
    /// This is meant to be called inside [validate::apply_change].
    pub fn dec_caret_col(
        caret: &mut Position,
        scroll_offset: &mut ScrollOffset,
        col_amt: ChUnit,
    ) {
        let horiz_scroll_is_active = scroll_offset.col_index > ch(0);
        let not_at_start_of_viewport = caret.col_index > ch(0);

        match horiz_scroll_is_active {
            // HORIZONTAL SCROLL INACTIVE
            false => {
                caret.col_index -= col_amt; // Scroll inactive.
            }
            true => {
                // HORIZONTAL SCROLL ACTIVE
                if not_at_start_of_viewport {
                    let need_to_scroll_left = col_amt > caret.col_index;
                    match need_to_scroll_left {
                        false => {
                            caret.col_index -= col_amt; // Just move caret left by col_amt.
                        }
                        true => {
                            let diff = col_amt - caret.col_index;
                            caret.col_index -= col_amt; // Move caret left by col_amt.
                            scroll_offset.col_index -= diff; // Move scroll left by diff.
                        }
                    }
                } else {
                    scroll_offset.col_index -= col_amt; // Scroll active & At start of viewport.
                                                        // Safe to sub, since scroll_offset.col can never be negative.
                }
            }
        }
    }

    /// This is meant to be called inside [validate::apply_change].
    pub fn reset_caret_col(caret: &mut Position, scroll_offset: &mut ScrollOffset) {
        scroll_offset.col_index = ch(0);
        caret.col_index = ch(0);
    }

    /// Decrement caret.row by 1, and adjust scrolling if active. This won't check whether it is
    /// inside or outside the buffer content boundary. You should check that before calling this
    /// function.
    ///
    /// This does not simply decrement the caret.row but mutates scroll_offset if scrolling is active.
    /// This can end up deactivating vertical scrolling as well.
    ///
    /// > Since caret.row can never be negative, this function must handle changes to scroll_offset
    /// > itself, and can't rely on [validate::apply_change] scroll validations
    /// > [scroll::validate_scroll].
    ///
    /// This is meant to be called inside [validate::apply_change].
    pub fn dec_caret_row(
        caret: &mut Position,
        scroll_offset: &mut ScrollOffset,
    ) -> usize {
        let vert_scroll_is_active = scroll_offset.row_index > ch(0);
        let not_at_top_of_viewport = caret.row_index > ch(0);

        match vert_scroll_is_active {
            // VERTICAL SCROLL INACTIVE
            false => {
                caret.row_index -= 1; // Scroll inactive.
                                      // Safe to minus 1, since caret.row can never be negative.
            }
            // VERTICAL SCROLL ACTIVE
            true => {
                if not_at_top_of_viewport {
                    caret.row_index -= 1; // Scroll active & Not at top of viewport.
                } else {
                    scroll_offset.row_index -= 1; // Scroll active & At top of viewport.
                                                  // Safe to minus 1, since scroll_offset.row can never be negative.
                }
            }
        }

        EditorBuffer::calc_scroll_adj_caret_row(caret, scroll_offset)
    }

    /// Try to increment caret.row by row_amt. This will not scroll past the bottom of the buffer. It
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
        row_amt: ChUnit,
        direction: CaretDirection,
    ) {
        let EditorArgsMut {
            editor_buffer,
            editor_engine,
        } = args;

        match direction {
            CaretDirection::Down => {
                let current_caret_adj_row =
                    editor_buffer.get_caret(CaretKind::ScrollAdjusted).row_index;
                let mut desired_caret_adj_row = current_caret_adj_row + row_amt;
                scroll_editor_buffer::clip_caret_row_to_content_height(
                    editor_buffer,
                    &mut desired_caret_adj_row,
                );

                // Calculate how many rows we need to increment caret row by.
                let mut diff = desired_caret_adj_row - current_caret_adj_row;

                // When editor_buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let editor_buffer_mut = editor_buffer.get_mut(
                        editor_engine.viewport_width(),
                        editor_engine.viewport_height(),
                    );

                    while diff > ch(0) {
                        scroll_editor_buffer::inc_caret_row(
                            editor_buffer_mut.caret,
                            editor_buffer_mut.scroll_offset,
                            editor_buffer_mut.viewport_height,
                        );
                        diff -= 1;
                    }
                }
            }
            CaretDirection::Up => {
                let mut diff = row_amt;

                // When editor_buffer_mut goes out of scope, it will be dropped &
                // validation performed.
                {
                    let editor_buffer_mut = editor_buffer.get_mut(
                        editor_engine.viewport_width(),
                        editor_engine.viewport_height(),
                    );

                    while diff > ch(0) {
                        scroll_editor_buffer::dec_caret_row(
                            editor_buffer_mut.caret,
                            editor_buffer_mut.scroll_offset,
                        );
                        diff -= 1;
                        if EditorBuffer::calc_scroll_adj_caret_row(
                            editor_buffer_mut.caret,
                            editor_buffer_mut.scroll_offset,
                        ) == 0
                        {
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Clip desired_caret_adj_row (to the max buffer length) if it overflows past the bottom of the
    /// buffer.
    pub fn clip_caret_row_to_content_height(
        buffer: &EditorBuffer,
        desired_caret_adj_row: &mut ChUnit,
    ) {
        // Clip desired_caret_adj_row if it overflows past the bottom of the buffer.
        let max_row_count = ch(buffer.get_lines().len()) - ch(1);
        let is_past_end_of_buffer = *desired_caret_adj_row > max_row_count;
        if is_past_end_of_buffer {
            *desired_caret_adj_row = max_row_count;
        }
    }

    /// Increment caret.row by 1, and adjust scrolling if active. This won't check whether it is
    /// inside or outside the buffer content boundary. You should check that before calling this
    /// function.
    ///
    /// Returns the new scroll adjusted caret row.
    ///
    /// This increments the caret.row and can activate vertical scrolling if the caret.row goes past
    /// the viewport height.
    ///
    /// This is meant to be called inside [validate::apply_change].
    pub fn inc_caret_row(
        caret: &mut Position,
        scroll_offset: &mut ScrollOffset,
        viewport_height: ChUnit,
    ) -> usize {
        let at_bottom_of_viewport = caret.row_index >= viewport_height;

        // Fun fact: The following logic is the same whether scroll is active or not.
        if at_bottom_of_viewport {
            scroll_offset.row_index += 1; // Activate scroll since at bottom of viewport.
        } else {
            caret.row_index += 1; // Scroll inactive & Not at bottom of viewport.
        }

        EditorBuffer::calc_scroll_adj_caret_row(caret, scroll_offset)
    }

    /// Check whether caret is vertically within the viewport. This is meant to be used after resize
    /// events and for [inc_caret_col], [inc_caret_row] operations. Note that [dec_caret_col] and
    /// [dec_caret_row] are handled differently (and not by this function) since they can never be
    /// negative.
    ///
    /// - If it isn't then scroll by mutating:
    ///    1. [caret](EditorBuffer::get_caret())'s row , so it is within the viewport.
    ///    2. [scroll_offset](EditorBuffer::get_scroll_offset())'s row, to actually apply scrolling.
    /// - Otherwise, no changes are made.
    ///
    /// This function is not meant to be called directly, but instead is called by
    /// [validate::apply_change].
    pub fn validate_scroll(args: EditorArgsMut<'_>) {
        let EditorArgsMut {
            editor_buffer,
            editor_engine,
        } = args;

        validate_vertical_scroll(EditorArgsMut {
            editor_buffer,
            editor_engine,
        });
        validate_horizontal_scroll(EditorArgsMut {
            editor_buffer,
            editor_engine,
        });

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
        ///   caret.row  |     |      within vp      |  vp height
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
            let EditorArgsMut {
                editor_buffer,
                editor_engine,
            } = args;

            let viewport_height = editor_engine.viewport_height();

            // Make sure that caret can't go past the bottom of the buffer.
            {
                let caret_row_adj =
                    editor_buffer.get_caret(CaretKind::ScrollAdjusted).row_index;
                let is_caret_row_adj_overflows_buffer =
                    caret_row_adj > editor_buffer.len();
                if is_caret_row_adj_overflows_buffer {
                    let diff = editor_buffer.len() - caret_row_adj;
                    let editor_buffer_mut = editor_buffer.get_mut(
                        editor_engine.viewport_width(),
                        editor_engine.viewport_height(),
                    );
                    editor_buffer_mut.caret.row_index -= diff;
                }
            }

            // Make sure that scroll_offset can't go past the bottom of the buffer.
            {
                let scroll_offset_row = editor_buffer.get_scroll_offset().row_index;
                let is_scroll_offset_row_overflows_buffer =
                    scroll_offset_row > editor_buffer.len();
                if is_scroll_offset_row_overflows_buffer {
                    let diff = editor_buffer.len() - scroll_offset_row;

                    let editor_buffer_mut = editor_buffer.get_mut(
                        editor_engine.viewport_width(),
                        editor_engine.viewport_height(),
                    );
                    editor_buffer_mut.scroll_offset.row_index -= diff;
                }
            }

            let caret_row_adj =
                editor_buffer.get_caret(CaretKind::ScrollAdjusted).row_index;
            let scroll_offset_row = editor_buffer.get_scroll_offset().row_index;

            let is_caret_row_adj_within_viewport = caret_row_adj >= scroll_offset_row
                && caret_row_adj <= (scroll_offset_row + viewport_height);

            match is_caret_row_adj_within_viewport {
                true => {
                    // Caret is within viewport, do nothing.
                }
                false => {
                    // Caret is outside viewport.
                    let is_caret_row_adj_above_viewport =
                        caret_row_adj < scroll_offset_row;
                    match is_caret_row_adj_above_viewport {
                        false => {
                            // Caret is below viewport.
                            let row_diff =
                                caret_row_adj - (scroll_offset_row + viewport_height);

                            let editor_buffer_mut = editor_buffer.get_mut(
                                editor_engine.viewport_width(),
                                editor_engine.viewport_height(),
                            );
                            editor_buffer_mut.scroll_offset.row_index += row_diff;

                            editor_buffer_mut.caret.row_index -= row_diff;
                        }
                        true => {
                            // Caret is above viewport.
                            let row_diff = scroll_offset_row - caret_row_adj;

                            let editor_buffer_mut = editor_buffer.get_mut(
                                editor_engine.viewport_width(),
                                editor_engine.viewport_height(),
                            );
                            editor_buffer_mut.scroll_offset.row_index -= row_diff;
                            editor_buffer_mut.caret.row_index += row_diff;
                        }
                    }
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
            let EditorArgsMut {
                editor_buffer,
                editor_engine,
            } = args;

            let viewport_width = editor_engine.viewport_width();

            let caret_col_adj =
                editor_buffer.get_caret(CaretKind::ScrollAdjusted).col_index;
            let scroll_offset_col = editor_buffer.get_scroll_offset().col_index;

            let is_caret_col_abs_within_viewport = caret_col_adj >= scroll_offset_col
                && caret_col_adj < scroll_offset_col + viewport_width;

            match is_caret_col_abs_within_viewport {
                true => {
                    // Caret is within viewport, nothing to do.
                }
                false => {
                    // Caret is outside viewport.
                    let editor_buffer_mut = editor_buffer.get_mut(
                        editor_engine.viewport_width(),
                        editor_engine.viewport_height(),
                    );
                    if caret_col_adj < scroll_offset_col {
                        // Caret is to the left of viewport.
                        editor_buffer_mut.scroll_offset.col_index = caret_col_adj;
                        editor_buffer_mut.caret.col_index = ch(0);
                    } else {
                        // Caret is to the right of viewport.
                        editor_buffer_mut.scroll_offset.col_index =
                            caret_col_adj - viewport_width + ch(1);
                        editor_buffer_mut.caret.col_index = viewport_width - ch(1);
                    }
                }
            }
        }
    }
}

mod caret_location_enums {
    #[derive(Clone, Eq, PartialEq)]
    pub enum CaretColLocationInLine {
        /// Also covers state where there is no col, or only 1 col.
        AtStart,
        AtEnd,
        InMiddle,
    }

    #[derive(Clone, Eq, PartialEq)]
    pub enum CaretRowLocationInBuffer {
        /// Also covers state where there is no row, or only 1 row.
        AtTop,
        AtBottom,
        InMiddle,
    }
}
use caret_location_enums::*;

#[derive(Clone, PartialEq, Copy)]
pub enum DeleteSelectionWith {
    Backspace,
    Delete,
    AnyKey,
}
