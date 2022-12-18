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

use std::{cmp::Ordering, mem::replace};

use get_size::GetSize;
use r3bl_rs_utils_core::*;
use serde::{Deserialize, Serialize};

use crate::*;

// ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
// ┃ EditorBuffer manipulation API ┃
// ┛                               ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// Functions that implement the editor engine.
pub struct EditorEngineInternalApi;

impl EditorEngineInternalApi {
    pub fn up(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Option<()> {
        move_caret::up(buffer, engine)
    }

    pub fn left(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Option<()> {
        move_caret::left(buffer, engine)
    }

    pub fn right(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Option<()> {
        move_caret::right(buffer, engine)
    }

    pub fn down(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Option<()> {
        move_caret::down(buffer, engine)
    }

    pub fn page_up(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Option<()> {
        move_caret::page_up(buffer, engine)
    }

    pub fn page_down(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Option<()> {
        move_caret::page_down(buffer, engine)
    }

    pub fn home(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Option<()> {
        move_caret::to_start_of_line(buffer, engine)
    }

    pub fn end(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Option<()> {
        move_caret::to_end_of_line(buffer, engine)
    }

    pub fn validate_scroll(args: EditorArgsMut) { scroll::validate_scroll(args); }

    pub fn string_at_caret(
        buffer: &EditorBuffer,
        engine: &EditorEngine,
    ) -> Option<UnicodeStringSegmentSliceResult> {
        get_content::string_at_caret(buffer, engine)
    }

    pub fn line_at_caret_to_string(
        buffer: &EditorBuffer,
        engine: &EditorEngine,
    ) -> Option<UnicodeString> {
        get_content::line_at_caret_to_string(buffer, engine)
    }

    pub fn line_at_caret_is_empty(buffer: &EditorBuffer, engine: &EditorEngine) -> bool {
        get_content::line_display_width_at_caret(buffer, engine) == ch!(0)
    }

    pub fn insert_str_at_caret(args: EditorArgsMut<'_>, chunk: &str) {
        mut_content::insert_str_at_caret(args, chunk)
    }

    pub fn insert_new_line_at_caret(args: EditorArgsMut<'_>) {
        mut_content::insert_new_line_at_caret(args);
    }

    pub fn delete_at_caret(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Option<()> {
        mut_content::delete_at_caret(buffer, engine)
    }

    pub fn backspace_at_caret(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Option<()> {
        mut_content::backspace_at_caret(buffer, engine)
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
            if let EditorLineMode::SingleLine = $arg_engine.config_options.multiline_mode {
                return None;
            }
        };
        ($arg_engine: expr, @Nothing) => {
            if let EditorLineMode::SingleLine = $arg_engine.config_options.multiline_mode {
                return;
            }
        };
    }
}

// ┏━━━━━━━━━━━┓
// ┃ Caret get ┃
// ┛           ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
mod locate_caret {
    use super::*;

    /// Locate the col.
    pub fn find_col(args: EditorArgs<'_>) -> CaretColLocationInLine {
        let EditorArgs {
            editor_buffer,
            editor_engine,
        } = args;

        if locate_caret::col_is_at_start_of_line(editor_buffer, editor_engine) {
            CaretColLocationInLine::AtStart
        } else if locate_caret::col_is_at_end_of_line(editor_buffer, editor_engine) {
            CaretColLocationInLine::AtEnd
        } else {
            CaretColLocationInLine::InMiddle
        }
    }

    fn col_is_at_start_of_line(buffer: &EditorBuffer, engine: &EditorEngine) -> bool {
        if get_content::line_at_caret_to_string(buffer, engine).is_some() {
            *buffer.get_caret(CaretKind::ScrollAdjusted).col_index == 0
        } else {
            false
        }
    }

    fn col_is_at_end_of_line(buffer: &EditorBuffer, engine: &EditorEngine) -> bool {
        if let Some(line) = get_content::line_at_caret_to_string(buffer, engine) {
            buffer.get_caret(CaretKind::ScrollAdjusted).col_index == line.display_width
        } else {
            false
        }
    }

    /// Locate the row.
    pub fn find_row(args: EditorArgs<'_>) -> CaretRowLocationInBuffer {
        let EditorArgs {
            editor_buffer,
            editor_engine,
        } = args;

        if row_is_at_top_of_buffer(editor_buffer, editor_engine) {
            CaretRowLocationInBuffer::AtTop
        } else if row_is_at_bottom_of_buffer(editor_buffer, editor_engine) {
            CaretRowLocationInBuffer::AtBottom
        } else {
            CaretRowLocationInBuffer::InMiddle
        }
    }

    // R ┌──────────┐
    // 0 ▸          │
    //   └▴─────────┘
    //   C0123456789
    fn row_is_at_top_of_buffer(buffer: &EditorBuffer, _engine: &EditorEngine) -> bool {
        *buffer.get_caret(CaretKind::ScrollAdjusted).row_index == 0
    }

    // R ┌──────────┐
    // 0 │a         │
    // 1 ▸a         │
    //   └▴─────────┘
    //   C0123456789
    fn row_is_at_bottom_of_buffer(buffer: &EditorBuffer, _engine: &EditorEngine) -> bool {
        if buffer.is_empty() || buffer.get_lines().len() == 1 {
            false // If there is only one line, then the caret is not at the bottom, its at the top.
        } else {
            let max_row_count = ch!(buffer.get_lines().len(), @dec);
            buffer.get_caret(CaretKind::ScrollAdjusted).row_index == max_row_count
        }
    }
}

// ┏━━━━━━━━━━━┓
// ┃ Caret mut ┃
// ┛           ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
mod move_caret {
    use super::*;

    pub fn up(editor_buffer: &mut EditorBuffer, editor_engine: &mut EditorEngine) -> Option<()> {
        empty_check_early_return!(editor_buffer, @None);
        multiline_disabled_check_early_return!(editor_engine, @None);

        match locate_caret::find_row(EditorArgs {
            editor_buffer,
            editor_engine,
        }) {
            CaretRowLocationInBuffer::AtTop => {
                // Do nothing.
                if editor_buffer.get_caret(CaretKind::ScrollAdjusted).col_index != ch!(0) {
                    validate::apply_change(
                        editor_buffer,
                        editor_engine,
                        |_, caret, scroll_offset| {
                            scroll::reset_caret_col(caret, scroll_offset);
                        },
                    );
                }
            }
            CaretRowLocationInBuffer::AtBottom | CaretRowLocationInBuffer::InMiddle => {
                // There is a line above the caret.
                validate::apply_change(editor_buffer, editor_engine, |_, caret, scroll_offset| {
                    scroll::dec_caret_row(caret, scroll_offset);
                });
                scroll::clip_caret_to_content_width(EditorArgsMut {
                    editor_buffer,
                    editor_engine,
                });
            }
        }

        None
    }

    pub fn down(editor_buffer: &mut EditorBuffer, editor_engine: &mut EditorEngine) -> Option<()> {
        empty_check_early_return!(editor_buffer, @None);
        multiline_disabled_check_early_return!(editor_engine, @None);

        if get_content::next_line_below_caret_exists(editor_buffer, editor_engine) {
            // There is a line below the caret.
            let viewport_height = editor_engine.viewport_height();
            validate::apply_change(editor_buffer, editor_engine, |_, caret, scroll_offset| {
                scroll::inc_caret_row(caret, scroll_offset, viewport_height);
            });
            scroll::clip_caret_to_content_width(EditorArgsMut {
                editor_buffer,
                editor_engine,
            });
        } else {
            // Move to the end of the line.
            move_caret::to_end_of_line(editor_buffer, editor_engine);
        }

        None
    }

    /// Convenience function for simply calling [reset_caret_col].
    pub fn to_start_of_line(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Option<()> {
        empty_check_early_return!(buffer, @None);

        validate::apply_change(buffer, engine, |_, caret, scroll_offset| {
            scroll::reset_caret_col(caret, scroll_offset);
        });
        None
    }

    pub fn to_end_of_line(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Option<()> {
        empty_check_early_return!(buffer, @None);

        let line_content_display_width = get_content::line_display_width_at_row_index(
            buffer,
            buffer.get_caret(CaretKind::ScrollAdjusted).row_index,
        );
        let viewport_width = engine.viewport_width();
        validate::apply_change(buffer, engine, |_, caret, scroll_offset| {
            scroll::set_caret_col(
                caret,
                scroll_offset,
                viewport_width,
                line_content_display_width,
                line_content_display_width,
            );
        });

        None
    }

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
    pub fn right(editor_buffer: &mut EditorBuffer, editor_engine: &mut EditorEngine) -> Option<()> {
        empty_check_early_return!(editor_buffer, @None);

        let line_is_empty =
            EditorEngineInternalApi::line_at_caret_is_empty(editor_buffer, editor_engine);
        let caret_col_loc_in_line = locate_caret::find_col(EditorArgs {
            editor_buffer,
            editor_engine,
        });

        match caret_col_loc_in_line {
            CaretColLocationInLine::AtEnd => {
                right_at_end(editor_buffer, editor_engine);
            }

            CaretColLocationInLine::AtStart | CaretColLocationInLine::InMiddle => {
                // Special case of empty line w/ caret at start.
                if let CaretColLocationInLine::AtStart = caret_col_loc_in_line {
                    if line_is_empty {
                        right_at_end(editor_buffer, editor_engine);
                        return None;
                    }
                }

                // Normal case.
                right_normal(editor_buffer, editor_engine)?;
            }
        }

        /// 1. Check for wide unicode character to the right of the caret.
        /// 2. [validate::apply_change] checks for wide unicode character at the start of the
        ///    viewport.
        fn right_normal(
            editor_buffer: &mut EditorBuffer,
            editor_engine: &mut EditorEngine,
        ) -> Option<()> {
            let UnicodeStringSegmentSliceResult {
                unicode_width: unicode_width_at_caret,
                ..
            } = get_content::string_at_caret(editor_buffer, editor_engine)?;

            let max_display_width =
                get_content::line_display_width_at_caret(editor_buffer, editor_engine);

            let viewport_width = editor_engine.viewport_width();

            let maybe_char_to_right_of_caret =
                get_content::string_to_right_of_caret(editor_buffer, editor_engine);

            match maybe_char_to_right_of_caret {
                Some(right_of_caret_seg_slice_result) => {
                    let unicode_string_to_right_of_caret =
                        right_of_caret_seg_slice_result.unicode_string_seg;
                    if unicode_string_to_right_of_caret.contains_wide_segments() {
                        let jump_by_col_width =
                            unicode_string_to_right_of_caret.display_width + unicode_width_at_caret;
                        let move_left = unicode_string_to_right_of_caret.display_width;
                        validate::apply_change(
                            editor_buffer,
                            editor_engine,
                            |_, caret, scroll_offset| {
                                scroll::inc_caret_col(
                                    caret,
                                    scroll_offset,
                                    jump_by_col_width,
                                    max_display_width,
                                    viewport_width,
                                );
                            },
                        );
                        if move_left > ch!(0) {
                            validate::apply_change(
                                editor_buffer,
                                editor_engine,
                                |_, caret, scroll_offset| {
                                    scroll::dec_caret_col(caret, scroll_offset, move_left)
                                },
                            );
                        }
                    } else {
                        validate::apply_change(
                            editor_buffer,
                            editor_engine,
                            |_, caret, scroll_offset| {
                                scroll::inc_caret_col(
                                    caret,
                                    scroll_offset,
                                    unicode_width_at_caret,
                                    max_display_width,
                                    viewport_width,
                                );
                            },
                        );
                    }
                }
                None => {
                    validate::apply_change(
                        editor_buffer,
                        editor_engine,
                        |_, caret, scroll_offset| {
                            scroll::inc_caret_col(
                                caret,
                                scroll_offset,
                                unicode_width_at_caret,
                                max_display_width,
                                viewport_width,
                            );
                        },
                    );
                }
            }

            None
        }

        fn right_at_end(editor_buffer: &mut EditorBuffer, editor_engine: &mut EditorEngine) {
            if get_content::next_line_below_caret_exists(editor_buffer, editor_engine) {
                // If there is a line below the caret, move the caret to the start of the next line.
                let viewport_height = editor_engine.viewport_height();
                validate::apply_change(editor_buffer, editor_engine, |_, caret, scroll_offset| {
                    scroll::inc_caret_row(caret, scroll_offset, viewport_height);
                    scroll::reset_caret_col(caret, scroll_offset);
                });
            }
        }

        None
    }

    pub fn left(editor_buffer: &mut EditorBuffer, editor_engine: &mut EditorEngine) -> Option<()> {
        empty_check_early_return!(editor_buffer, @None);

        match locate_caret::find_col(EditorArgs {
            editor_buffer,
            editor_engine,
        }) {
            CaretColLocationInLine::AtStart => {
                if get_content::prev_line_above_caret_exists(editor_buffer, editor_engine) {
                    // If there is a line above the caret, move the caret to the end of the previous line.
                    validate::apply_change(
                        editor_buffer,
                        editor_engine,
                        |_, caret, scroll_offset| {
                            scroll::dec_caret_row(caret, scroll_offset);
                        },
                    );
                    move_caret::to_end_of_line(editor_buffer, editor_engine);
                }
            }
            CaretColLocationInLine::AtEnd => {
                let UnicodeStringSegmentSliceResult { unicode_width, .. } =
                    get_content::string_at_end_of_line_at_caret(editor_buffer, editor_engine)?;
                validate::apply_change(editor_buffer, editor_engine, |_, caret, scroll_offset| {
                    scroll::dec_caret_col(caret, scroll_offset, unicode_width)
                });
            }
            CaretColLocationInLine::InMiddle => {
                let UnicodeStringSegmentSliceResult { unicode_width, .. } =
                    get_content::string_to_left_of_caret(editor_buffer, editor_engine)?;
                validate::apply_change(editor_buffer, editor_engine, |_, caret, scroll_offset| {
                    scroll::dec_caret_col(caret, scroll_offset, unicode_width)
                });
            }
        }
        None
    }

    pub fn page_up(
        editor_buffer: &mut EditorBuffer,
        editor_engine: &mut EditorEngine,
    ) -> Option<()> {
        empty_check_early_return!(editor_buffer, @None);
        multiline_disabled_check_early_return!(editor_engine, @None);

        let viewport_height = editor_engine.viewport_height();
        scroll::change_caret_row_by(
            EditorArgsMut {
                editor_engine,
                editor_buffer,
            },
            viewport_height,
            CaretDirection::Up,
        );
        None
    }

    pub fn page_down(
        editor_buffer: &mut EditorBuffer,
        editor_engine: &mut EditorEngine,
    ) -> Option<()> {
        empty_check_early_return!(editor_buffer, @None);
        multiline_disabled_check_early_return!(editor_engine, @None);

        let viewport_height = editor_engine.viewport_height();
        scroll::change_caret_row_by(
            EditorArgsMut {
                editor_engine,
                editor_buffer,
            },
            viewport_height,
            CaretDirection::Down,
        );
        None
    }
}

// ┏━━━━━━━━━━━━━┓
// ┃ Content get ┃
// ┛             ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
mod get_content {
    use super::*;

    pub fn line_display_width_at_caret(buffer: &EditorBuffer, engine: &EditorEngine) -> ChUnit {
        let line = get_content::line_at_caret_to_string(buffer, engine);
        if let Some(line) = line {
            line.display_width
        } else {
            ch!(0)
        }
    }

    pub fn line_display_width_at_row_index(buffer: &EditorBuffer, row_idx: ChUnit) -> ChUnit {
        let line = buffer.get_lines().get(ch!(@to_usize row_idx));
        if let Some(line) = line {
            line.display_width
        } else {
            ch!(0)
        }
    }

    pub fn next_line_below_caret_exists(buffer: &EditorBuffer, engine: &EditorEngine) -> bool {
        let next_line = get_content::next_line_below_caret_to_string(buffer, engine);
        next_line.is_some()
    }

    pub fn line_at_caret_to_string(
        buffer: &EditorBuffer,
        _engine: &EditorEngine,
    ) -> Option<UnicodeString> {
        empty_check_early_return!(buffer, @None);
        let row_index = buffer.get_caret(CaretKind::ScrollAdjusted).row_index;
        let line = buffer.get_lines().get(ch!(@to_usize row_index))?;
        Some(line.clone())
    }

    pub fn next_line_below_caret_to_string(
        buffer: &EditorBuffer,
        _engine: &EditorEngine,
    ) -> Option<UnicodeString> {
        empty_check_early_return!(buffer, @None);
        let row_index = buffer.get_caret(CaretKind::ScrollAdjusted).row_index;
        let line = buffer.get_lines().get(ch!(@to_usize row_index + 1))?;
        Some(line.clone())
    }

    pub fn prev_line_above_caret_exists(buffer: &EditorBuffer, engine: &EditorEngine) -> bool {
        let prev_line = get_content::prev_line_above_caret_to_string(buffer, engine);
        prev_line.is_some()
    }

    pub fn prev_line_above_caret_to_string(
        buffer: &EditorBuffer,
        _engine: &EditorEngine,
    ) -> Option<UnicodeString> {
        empty_check_early_return!(buffer, @None);
        let row_index = buffer.get_caret(CaretKind::ScrollAdjusted).row_index;
        if row_index == ch!(0) {
            return None;
        }
        let line = buffer.get_lines().get(ch!(@to_usize row_index - 1))?;
        Some(line.clone())
    }

    pub fn string_at_caret(
        buffer: &EditorBuffer,
        _engine: &EditorEngine,
    ) -> Option<UnicodeStringSegmentSliceResult> {
        empty_check_early_return!(buffer, @None);
        let caret_adj = buffer.get_caret(CaretKind::ScrollAdjusted);
        let line = buffer.get_lines().get(ch!(@to_usize caret_adj.row_index))?;
        let result = line.get_string_at_display_col_index(caret_adj.col_index)?;
        Some(result)
    }

    pub fn string_to_right_of_caret(
        buffer: &EditorBuffer,
        _engine: &EditorEngine,
    ) -> Option<UnicodeStringSegmentSliceResult> {
        empty_check_early_return!(buffer, @None);
        let caret_adj = buffer.get_caret(CaretKind::ScrollAdjusted);
        let line = buffer.get_lines().get(ch!(@to_usize caret_adj.row_index))?;

        match locate_caret::find_col(EditorArgs {
            editor_buffer: buffer,
            editor_engine: _engine,
        }) {
            // Caret is at end of line, past the last character.
            CaretColLocationInLine::AtEnd => line.get_string_at_end(),
            // Caret is not at end of line.
            _ => line.get_string_at_right_of_display_col_index(caret_adj.col_index),
        }
    }

    pub fn string_to_left_of_caret(
        editor_buffer: &EditorBuffer,
        editor_engine: &EditorEngine,
    ) -> Option<UnicodeStringSegmentSliceResult> {
        empty_check_early_return!(editor_buffer, @None);
        let caret_adj = editor_buffer.get_caret(CaretKind::ScrollAdjusted);
        let line = editor_buffer
            .get_lines()
            .get(ch!(@to_usize caret_adj.row_index))?;

        match locate_caret::find_col(EditorArgs {
            editor_buffer,
            editor_engine,
        }) {
            // Caret is at end of line, past the last character.
            CaretColLocationInLine::AtEnd => line.get_string_at_end(),
            // Caret is not at end of line.
            _ => line.get_string_at_left_of_display_col_index(caret_adj.col_index),
        }
    }

    pub fn string_at_end_of_line_at_caret(
        editor_buffer: &EditorBuffer,
        editor_engine: &EditorEngine,
    ) -> Option<UnicodeStringSegmentSliceResult> {
        empty_check_early_return!(editor_buffer, @None);
        let line = get_content::line_at_caret_to_string(editor_buffer, editor_engine)?;
        if let CaretColLocationInLine::AtEnd = locate_caret::find_col(EditorArgs {
            editor_buffer,
            editor_engine,
        }) {
            let maybe_last_str_seg = line.get_string_at_end();
            return maybe_last_str_seg;
        }
        None
    }
}

// ┏━━━━━━━━━━━━━┓
// ┃ Content mut ┃
// ┛             ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
mod mut_content {
    use super::*;

    pub fn insert_str_at_caret(args: EditorArgsMut<'_>, chunk: &str) {
        let EditorArgsMut {
            editor_buffer,
            editor_engine,
        } = args;

        let caret_adj = editor_buffer.get_caret(CaretKind::ScrollAdjusted);

        let row: usize = ch!(@to_usize caret_adj.row_index);
        let col: usize = ch!(@to_usize caret_adj.col_index);

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
            validate::apply_change(editor_buffer, editor_engine, |lines, _, _| {
                lines.push(String::new().into());
            });
            return;
        }

        match locate_caret::find_col(EditorArgs {
            editor_buffer,
            editor_engine,
        }) {
            CaretColLocationInLine::AtEnd => {
                insert_new_line_at_end_of_current_line(EditorArgsMut {
                    editor_buffer,
                    editor_engine,
                });
            }
            CaretColLocationInLine::AtStart => {
                insert_new_line_at_start_of_current_line(EditorArgsMut {
                    editor_buffer,
                    editor_engine,
                });
            }
            CaretColLocationInLine::InMiddle => {
                insert_new_line_at_middle_of_current_line(EditorArgsMut {
                    editor_buffer,
                    editor_engine,
                });
            }
        }

        // ┏━━━━━━━━━━━━━━━━━┓
        // ┃ Inner functions ┃
        // ┛                 ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // Handle inserting a new line at the end of the current line.
        fn insert_new_line_at_end_of_current_line(args: EditorArgsMut<'_>) {
            let EditorArgsMut {
                editor_buffer,
                editor_engine,
            } = args;

            let viewport_height = editor_engine.viewport_height();

            validate::apply_change(
                editor_buffer,
                editor_engine,
                |lines, caret, scroll_offset| {
                    let new_row_idx = scroll::inc_caret_row(caret, scroll_offset, viewport_height);
                    scroll::reset_caret_col(caret, scroll_offset);
                    lines.insert(new_row_idx, String::new().into());
                },
            );
        }

        // Handle inserting a new line at the start of the current line.
        fn insert_new_line_at_start_of_current_line(args: EditorArgsMut<'_>) {
            let EditorArgsMut {
                editor_buffer,
                editor_engine,
            } = args;

            let viewport_height = editor_engine.viewport_height();

            validate::apply_change(
                editor_buffer,
                editor_engine,
                |lines, caret, scroll_offset| {
                    let cur_row_idx = EditorBuffer::calc_scroll_adj_caret_row(caret, scroll_offset);
                    lines.insert(cur_row_idx, String::new().into());
                },
            );

            validate::apply_change(editor_buffer, editor_engine, |_, caret, scroll_offset| {
                scroll::inc_caret_row(caret, scroll_offset, viewport_height);
            });
        }

        // Handle inserting a new line at the middle of the current line.
        fn insert_new_line_at_middle_of_current_line(args: EditorArgsMut<'_>) {
            let EditorArgsMut {
                editor_buffer,
                editor_engine,
            } = args;

            if let Some(line_content) =
                get_content::line_at_caret_to_string(editor_buffer, editor_engine)
            {
                let caret_adj = editor_buffer.get_caret(CaretKind::ScrollAdjusted);

                let col_index = caret_adj.col_index;
                let split_result = line_content.split_at_display_col(col_index);
                if let Some((
                    NewUnicodeStringResult {
                        new_unicode_string: left,
                        ..
                    },
                    NewUnicodeStringResult {
                        new_unicode_string: right,
                        ..
                    },
                )) = split_result
                {
                    let row_index = ch!(@to_usize caret_adj.row_index);
                    let viewport_height = editor_engine.viewport_height();

                    validate::apply_change(
                        editor_buffer,
                        editor_engine,
                        |lines, caret, scroll_offset| {
                            let _ = replace(&mut lines[row_index], left);
                            lines.insert(row_index + 1, right);
                            scroll::inc_caret_row(caret, scroll_offset, viewport_height);
                            scroll::reset_caret_col(caret, scroll_offset);
                        },
                    );
                }
            }
        }
    }

    pub fn delete_at_caret(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Option<()> {
        empty_check_early_return!(buffer, @None);
        if get_content::string_at_caret(buffer, engine).is_some() {
            delete_in_middle_of_line(buffer, engine)?;
        } else {
            delete_at_end_of_line(buffer, engine)?;
        }
        return None;

        // ┏━━━━━━━━━━━━━━━━━┓
        // ┃ Inner functions ┃
        // ┛                 ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

        // R ┌──────────┐
        // 0 ▸abc       │
        // 1 │ab        │
        // 2 │a         │
        //   └─▴────────┘
        //   C0123456789
        fn delete_in_middle_of_line(
            buffer: &mut EditorBuffer,
            engine: &mut EditorEngine,
        ) -> Option<()> {
            let cur_line = get_content::line_at_caret_to_string(buffer, engine)?;
            let NewUnicodeStringResult {
                new_unicode_string: new_line,
                ..
            } = cur_line.delete_char_at_display_col(
                buffer.get_caret(CaretKind::ScrollAdjusted).col_index,
            )?;

            validate::apply_change(buffer, engine, |lines, caret, scroll_offset| {
                let row_idx = EditorBuffer::calc_scroll_adj_caret_row(caret, scroll_offset);
                let _ = replace(&mut lines[row_idx], new_line);
            });
            None
        }

        // R ┌──────────┐
        // 0 ▸abc       │
        // 1 │ab        │
        // 2 │a         │
        //   └───▴──────┘
        //   C0123456789
        fn delete_at_end_of_line(
            buffer: &mut EditorBuffer,
            engine: &mut EditorEngine,
        ) -> Option<()> {
            let this_line = get_content::line_at_caret_to_string(buffer, engine)?;
            let next_line = get_content::next_line_below_caret_to_string(buffer, engine)?;

            validate::apply_change(buffer, engine, |lines, caret, scroll_offset| {
                let row_idx = EditorBuffer::calc_scroll_adj_caret_row(caret, scroll_offset);
                let _ = replace(&mut lines[row_idx], this_line + &next_line);
                lines.remove(row_idx + 1);
            });
            None
        }
    }

    pub fn backspace_at_caret(buffer: &mut EditorBuffer, engine: &mut EditorEngine) -> Option<()> {
        empty_check_early_return!(buffer, @None);

        if let Some(UnicodeStringSegmentSliceResult {
            display_col_at_which_seg_starts,
            ..
        }) = get_content::string_to_left_of_caret(buffer, engine)
        {
            backspace_in_middle_of_line(buffer, engine, display_col_at_which_seg_starts)?;
        } else {
            backspace_at_start_of_line(buffer, engine)?;
        }

        return None;

        // ┏━━━━━━━━━━━━━━━━━┓
        // ┃ Inner functions ┃
        // ┛                 ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

        // R ┌──────────┐
        // 0 ▸abc       │
        // 1 │ab        │
        // 2 │a         │
        //   └─▴────────┘
        //   C0123456789
        fn backspace_in_middle_of_line(
            buffer: &mut EditorBuffer,
            engine: &mut EditorEngine,
            delete_at_this_display_col: ChUnit,
        ) -> Option<()> {
            let cur_line = get_content::line_at_caret_to_string(buffer, engine)?;
            let NewUnicodeStringResult {
                new_unicode_string: new_line,
                ..
            } = cur_line.delete_char_at_display_col(delete_at_this_display_col)?;

            let viewport_width = engine.viewport_width();
            validate::apply_change(buffer, engine, |lines, caret, scroll_offset| {
                let cur_row_idx = EditorBuffer::calc_scroll_adj_caret_row(caret, scroll_offset);
                let _ = replace(&mut lines[cur_row_idx], new_line);
                scroll::set_caret_col(
                    caret,
                    scroll_offset,
                    viewport_width,
                    lines[cur_row_idx].display_width,
                    delete_at_this_display_col,
                );
            });

            None
        }

        // R ┌──────────┐
        // 0 │abc       │
        // 1 ▸ab        │
        // 2 │a         │
        //   └▴─────────┘
        //   C0123456789
        fn backspace_at_start_of_line(
            buffer: &mut EditorBuffer,
            engine: &mut EditorEngine,
        ) -> Option<()> {
            let viewport_width = engine.viewport_width();

            let this_line = get_content::line_at_caret_to_string(buffer, engine)?;
            let prev_line = get_content::prev_line_above_caret_to_string(buffer, engine)?;

            let prev_line_eol_col = prev_line.display_width;
            validate::apply_change(buffer, engine, |lines, caret, scroll_offset| {
                let prev_row_idx =
                    EditorBuffer::calc_scroll_adj_caret_row(caret, scroll_offset) - 1;
                let cur_row_idx = EditorBuffer::calc_scroll_adj_caret_row(caret, scroll_offset);
                let _ = replace(&mut lines[prev_row_idx], prev_line + &this_line);
                let new_merged_line_content_display_width = lines[prev_row_idx].display_width;
                lines.remove(cur_row_idx);
                scroll::dec_caret_row(caret, scroll_offset);
                scroll::set_caret_col(
                    caret,
                    scroll_offset,
                    viewport_width,
                    new_merged_line_content_display_width,
                    prev_line_eol_col,
                );
            });

            None
        }
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

        let row_index = ch!(@to_usize caret_adj.row_index);
        let line = editor_buffer.get_lines().get(row_index)?;

        let NewUnicodeStringResult {
            new_unicode_string: new_line,
            unicode_width: char_display_width,
        } = line.insert_char_at_display_col(ch!(caret_adj.col_index), chunk)?;

        let viewport_width = editor_engine.viewport_width();

        validate::apply_change(
            editor_buffer,
            editor_engine,
            |lines, caret, scroll_offset| {
                // Replace existing line w/ new line.
                let _ = replace(&mut lines[row_index], new_line);
                let line_content_display_width = lines[row_index].display_width;

                // Update caret position.
                scroll::inc_caret_col(
                    caret,
                    scroll_offset,
                    char_display_width,
                    line_content_display_width,
                    viewport_width,
                );
            },
        );

        None
    }

    // ┏━━━━━━━━━━━━━━━━━┓
    // ┃ Inner functions ┃
    // ┛                 ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    fn fill_in_missing_lines_up_to_row(args: EditorArgsMut<'_>, caret_row: usize) {
        let EditorArgsMut {
            editor_buffer,
            editor_engine,
        } = args;

        // Fill in any missing lines.
        if editor_buffer.get_lines().get(caret_row).is_none() {
            for row_idx in 0..caret_row + 1 {
                if editor_buffer.get_lines().get(row_idx).is_none() {
                    validate::apply_change(editor_buffer, editor_engine, |lines, _, _| {
                        lines.push(String::new().into());
                    });
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

        validate::apply_change(
            editor_buffer,
            editor_engine,
            |lines, caret, scroll_offset| {
                // Actually add the character to the correct line.
                let _ = replace(
                    &mut lines[ch!(@to_usize caret_adj_row)],
                    UnicodeString::from(chunk),
                );

                let line_content_display_width = lines[caret_adj_row].display_width;
                let col_amt = ch!(UnicodeString::str_display_width(chunk));

                // Update caret position.
                scroll::inc_caret_col(
                    caret,
                    scroll_offset,
                    col_amt,
                    line_content_display_width,
                    viewport_width,
                );
            },
        );

        None
    }
}

// ┏━━━━━━━━━━━━━━━━━━━━━┓
// ┃ Change EditorBuffer ┃
// ┛                     ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// This is marked as `pub` because `apply_change` is needed by `cargo doc`.
pub mod validate {
    use super::*;

    /// In addition to mutating the buffer, this function runs the following validations on the
    /// [EditorBuffer]'s:
    /// 1. `caret`:
    ///    - the caret is in not in the middle of a unicode segment character.
    ///    - if it is then it moves the caret.
    /// 2. `scroll_offset`:
    ///    - make sure that it's not in the middle of a wide unicode segment character.
    ///    - if it is then it moves the scroll_offset and caret.
    pub fn apply_change(
        editor_buffer: &mut EditorBuffer,
        editor_engine: &mut EditorEngine,
        mutator: impl FnOnce(
            /* EditorBuffer::lines */ &mut Vec<UnicodeString>,
            /* EditorBuffer::caret */ &mut Position,
            /* EditorEngine::scroll_offset */ &mut ScrollOffset,
        ),
    ) -> Option<()> {
        let (lines, caret, scroll_offset) = editor_buffer.get_mut();

        // Run the mutator first.
        mutator(lines, caret, scroll_offset);

        // Check caret validity.
        adjust_caret_col_if_not_in_middle_of_grapheme_cluster(EditorArgsMut {
            editor_engine,
            editor_buffer,
        });

        // Check scroll_offset validity.
        if let Some(diff) = is_scroll_offset_in_middle_of_grapheme_cluster(EditorArgs {
            editor_engine,
            editor_buffer,
        }) {
            adjust_scroll_offset_because_in_middle_of_grapheme_cluster(
                EditorArgsMut {
                    editor_engine,
                    editor_buffer,
                },
                diff,
            );
        }

        None
    }

    pub fn is_scroll_offset_in_middle_of_grapheme_cluster(args: EditorArgs<'_>) -> Option<ChUnit> {
        let EditorArgs {
            editor_buffer,
            editor_engine,
        } = args;
        let line = get_content::line_at_caret_to_string(editor_buffer, editor_engine)?;

        let unicode_width_at_caret = {
            match get_content::string_at_caret(editor_buffer, editor_engine) {
                None => ch!(0),
                Some(string_at_caret) => string_at_caret.unicode_width,
            }
        };

        let scroll_offset = editor_buffer.get_scroll_offset();

        if let Some(segment) =
            line.is_display_col_index_in_middle_of_grapheme_cluster(scroll_offset.col_index)
        {
            let diff = segment.unicode_width - unicode_width_at_caret;
            return Some(diff);
        };

        None
    }

    pub fn adjust_scroll_offset_because_in_middle_of_grapheme_cluster(
        args: EditorArgsMut<'_>,
        diff: ChUnit,
    ) -> Option<()> {
        let EditorArgsMut { editor_buffer, .. } = args;
        let (_, caret, scroll_offset) = editor_buffer.get_mut();
        scroll_offset.col_index += diff;
        caret.col_index -= diff;
        None
    }

    /// This function is visible inside the editor_ops.rs module only. It is not meant to be called
    /// directly, but instead is called by [validate::apply_change].
    pub fn adjust_caret_col_if_not_in_middle_of_grapheme_cluster(
        args: EditorArgsMut<'_>,
    ) -> Option<()> {
        let EditorArgsMut {
            editor_buffer,
            editor_engine,
        } = args;

        let viewport_width = editor_engine.viewport_width();

        let (lines, caret, scroll_offset) = editor_buffer.get_mut();
        let row_idx = EditorBuffer::calc_scroll_adj_caret_row(caret, scroll_offset);
        let col_idx = ch!(EditorBuffer::calc_scroll_adj_caret_col(
            caret,
            scroll_offset
        ));

        let line = lines.get(row_idx)?;

        // Caret is in the middle of a grapheme cluster, so jump it.
        let segment = line.is_display_col_index_in_middle_of_grapheme_cluster(col_idx)?;
        scroll::set_caret_col(
            caret,
            scroll_offset,
            viewport_width,
            line.display_width,
            segment.unicode_width + segment.display_col_offset,
        );

        None
    }
}

// ┏━━━━━━━━━━━━━━━━━━━━━┓
// ┃ Scroll EditorBuffer ┃
// ┛                     ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
mod scroll {
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
            get_content::line_display_width_at_caret(editor_buffer, editor_engine);

        let caret_adj_col = ch!(EditorBuffer::calc_scroll_adj_caret_col(
            &caret,
            &scroll_offset
        ));

        let is_caret_col_overflow_content_width = caret_adj_col >= line_content_display_width;

        if is_caret_col_overflow_content_width {
            move_caret::to_end_of_line(editor_buffer, editor_engine);
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
        let caret_adj_col = ch!(EditorBuffer::calc_scroll_adj_caret_col(
            caret,
            scroll_offset
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
            let diff_overflow = ch!(1) + caret.col_index - viewport_width;
            scroll_offset.col_index += diff_overflow; // Activate horiz scroll.
            caret.col_index -= diff_overflow; // Shift caret.
        }
    }

    /// This does not simply decrement the caret.col but mutates scroll_offset if scrolling is active.
    ///
    /// This is meant to be called inside [validate::apply_change].
    pub fn dec_caret_col(caret: &mut Position, scroll_offset: &mut ScrollOffset, col_amt: ChUnit) {
        let horiz_scroll_is_active = scroll_offset.col_index > ch!(0);
        let not_at_start_of_viewport = caret.col_index > ch!(0);

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
        scroll_offset.col_index = ch!(0);
        caret.col_index = ch!(0);
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
    pub fn dec_caret_row(caret: &mut Position, scroll_offset: &mut ScrollOffset) -> usize {
        let vert_scroll_is_active = scroll_offset.row_index > ch!(0);
        let not_at_top_of_viewport = caret.row_index > ch!(0);

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
                let viewport_height = editor_engine.viewport_height();
                let current_caret_adj_row =
                    editor_buffer.get_caret(CaretKind::ScrollAdjusted).row_index;
                let mut desired_caret_adj_row = current_caret_adj_row + row_amt;
                scroll::clip_caret_row_to_content_height(editor_buffer, &mut desired_caret_adj_row);

                // Calculate how many rows we need to increment caret row by.
                let mut diff = desired_caret_adj_row - current_caret_adj_row;

                validate::apply_change(editor_buffer, editor_engine, |_, caret, scroll_offset| {
                    while diff > ch!(0) {
                        scroll::inc_caret_row(caret, scroll_offset, viewport_height);
                        diff -= 1;
                    }
                });
            }
            CaretDirection::Up => {
                let mut diff = row_amt;
                validate::apply_change(editor_buffer, editor_engine, |_, caret, scroll_offset| {
                    while diff > ch!(0) {
                        scroll::dec_caret_row(caret, scroll_offset);
                        diff -= 1;
                        if EditorBuffer::calc_scroll_adj_caret_row(caret, scroll_offset) == 0 {
                            break;
                        }
                    }
                });
            }
            _ => {}
        }
    }

    /// Clip desired_caret_adj_row (to the max buffer length) if it overflows past the bottom of the
    /// buffer.
    pub fn clip_caret_row_to_content_height(
        buffer: &mut EditorBuffer,
        desired_caret_adj_row: &mut ChUnit,
    ) {
        // Clip desired_caret_adj_row if it overflows past the bottom of the buffer.
        let max_row_count = ch!(buffer.get_lines().len(), @dec);
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
                let caret_row_adj = editor_buffer.get_caret(CaretKind::ScrollAdjusted).row_index;
                let is_caret_row_adj_overflows_buffer = caret_row_adj > editor_buffer.len();
                if is_caret_row_adj_overflows_buffer {
                    let diff = editor_buffer.len() - caret_row_adj;
                    let (_, caret, _) = editor_buffer.get_mut();
                    caret.row_index -= diff;
                }
            }

            // Make sure that scroll_offset can't go past the bottom of the buffer.
            {
                let scroll_offset_row = editor_buffer.get_scroll_offset().row_index;
                let is_scroll_offset_row_overflows_buffer = scroll_offset_row > editor_buffer.len();
                if is_scroll_offset_row_overflows_buffer {
                    let diff = editor_buffer.len() - scroll_offset_row;
                    let (_, _, scroll_offset) = editor_buffer.get_mut();
                    scroll_offset.row_index -= diff;
                }
            }

            let caret_row_adj = editor_buffer.get_caret(CaretKind::ScrollAdjusted).row_index;
            let scroll_offset_row = editor_buffer.get_scroll_offset().row_index;

            let is_caret_row_adj_within_viewport = caret_row_adj >= scroll_offset_row
                && caret_row_adj <= (scroll_offset_row + viewport_height);

            match is_caret_row_adj_within_viewport {
                true => {
                    // Caret is within viewport, do nothing.
                }
                false => {
                    // Caret is outside viewport.
                    let is_caret_row_adj_above_viewport = caret_row_adj < scroll_offset_row;
                    match is_caret_row_adj_above_viewport {
                        false => {
                            // Caret is below viewport.
                            let row_diff = caret_row_adj - (scroll_offset_row + viewport_height);
                            let (_, caret, scroll_offset) = editor_buffer.get_mut();
                            scroll_offset.row_index += row_diff;
                            caret.row_index -= row_diff;
                        }
                        true => {
                            // Caret is above viewport.
                            let row_diff = scroll_offset_row - caret_row_adj;
                            let (_, caret, scroll_offset) = editor_buffer.get_mut();
                            scroll_offset.row_index -= row_diff;
                            caret.row_index += row_diff;
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

            let caret_col_adj = editor_buffer.get_caret(CaretKind::ScrollAdjusted).col_index;
            let scroll_offset_col = editor_buffer.get_scroll_offset().col_index;

            let is_caret_col_abs_within_viewport = caret_col_adj >= scroll_offset_col
                && caret_col_adj < scroll_offset_col + viewport_width;

            match is_caret_col_abs_within_viewport {
                true => {
                    // Caret is within viewport, nothing to do.
                }
                false => {
                    // Caret is outside viewport.
                    let (_, caret, scroll_offset) = editor_buffer.get_mut();

                    if caret_col_adj < scroll_offset_col {
                        // Caret is to the left of viewport.
                        scroll_offset.col_index = caret_col_adj;
                        caret.col_index = ch!(0);
                    } else {
                        // Caret is to the right of viewport.
                        scroll_offset.col_index = caret_col_adj - viewport_width + ch!(1);
                        caret.col_index = viewport_width - ch!(1);
                    }
                }
            }
        }
    }
}

// ┏━━━━━━━━━━━━━━━━━━━━━━┓
// ┃ Caret location enums ┃
// ┛                      ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
mod enums {
    use super::*;

    #[derive(Clone, Eq, PartialEq, Serialize, Deserialize, GetSize)]
    pub enum CaretColLocationInLine {
        /// Also covers state where there is no col, or only 1 col.
        AtStart,
        AtEnd,
        InMiddle,
    }

    #[derive(Clone, Eq, PartialEq, Serialize, Deserialize, GetSize)]
    pub enum CaretRowLocationInBuffer {
        /// Also covers state where there is no row, or only 1 row.
        AtTop,
        AtBottom,
        InMiddle,
    }
}
use enums::*;
