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

use std::collections::HashMap;

use super::{scroll_editor_content, DeleteSelectionWith};
use crate::{caret_locate::{locate_col, CaretColLocationInLine},
            caret_scroll_index, col, empty_check_early_return, inline_string,
            multiline_disabled_check_early_return, row, CaretScrAdj, ColIndex,
            EditorArgsMut, EditorBuffer, EditorEngine, GCString, GCStringExt,
            InlineString, InlineVec, RowIndex, SelectionRange, Width};

pub fn insert_chunk_at_caret(args: EditorArgsMut<'_>, chunk: &str) {
    let EditorArgsMut { buffer, engine } = args;

    let caret_scr_adj = buffer.get_caret_scr_adj();
    let row_index_scr_adj = caret_scr_adj.row_index;

    if buffer.line_at_row_index(row_index_scr_adj).is_some() {
        insert_into_existing_line(EditorArgsMut { engine, buffer }, caret_scr_adj, chunk);
    } else {
        fill_in_missing_lines_up_to_row(
            EditorArgsMut { engine, buffer },
            row_index_scr_adj,
        );
        insert_chunk_into_new_line(
            EditorArgsMut { engine, buffer },
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
            buffer_mut.inner.lines.push("".grapheme_string());
        }
        return;
    }

    match locate_col(buffer) {
        CaretColLocationInLine::AtEnd => {
            insert_new_line_at_caret_helper::insert_new_line_at_end_of_current_line(
                EditorArgsMut { engine, buffer },
            );
        }
        CaretColLocationInLine::AtStart => {
            insert_new_line_at_caret_helper::insert_new_line_at_start_of_current_line(
                EditorArgsMut { engine, buffer },
            );
        }
        CaretColLocationInLine::InMiddle => {
            insert_new_line_at_caret_helper::insert_new_line_at_middle_of_current_line(
                EditorArgsMut { engine, buffer },
            );
        }
    }
}

mod insert_new_line_at_caret_helper {
    use super::{scroll_editor_content, EditorArgsMut, GCString, GCStringExt};

    // Handle inserting a new line at the end of the current line.
    pub fn insert_new_line_at_end_of_current_line(args: EditorArgsMut<'_>) {
        let EditorArgsMut { buffer, engine } = args;

        // When buffer_mut goes out of scope, it will be dropped &
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());

            let new_row_index = scroll_editor_content::inc_caret_row(
                buffer_mut.inner.caret_raw,
                buffer_mut.inner.scr_ofs,
                buffer_mut.inner.vp.row_height,
            );

            scroll_editor_content::reset_caret_col(
                buffer_mut.inner.caret_raw,
                buffer_mut.inner.scr_ofs,
            );

            buffer_mut
                .inner
                .lines
                .insert(new_row_index.as_usize(), "".grapheme_string());
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
                (*buffer_mut.inner.caret_raw + *buffer_mut.inner.scr_ofs).row_index;
            buffer_mut
                .inner
                .lines
                .insert(cur_row_index.as_usize(), "".grapheme_string());
        }

        // When buffer_mut goes out of scope, it will be dropped &
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            scroll_editor_content::inc_caret_row(
                buffer_mut.inner.caret_raw,
                buffer_mut.inner.scr_ofs,
                buffer_mut.inner.vp.row_height,
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

                // When buffer_mut goes out of scope, it will be dropped & validation
                // performed.
                {
                    let buffer_mut = buffer.get_mut(engine.viewport());

                    let _unused: GCString = std::mem::replace(
                        &mut buffer_mut.inner.lines[row_index],
                        left_string.grapheme_string(),
                    );

                    buffer_mut
                        .inner
                        .lines
                        .insert(row_index + 1, right_string.grapheme_string());

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
}

pub fn delete_at_caret(
    buffer: &mut EditorBuffer,
    engine: &mut EditorEngine,
) -> Option<()> {
    empty_check_early_return!(buffer, @None);
    if buffer.string_at_caret().is_some() {
        delete_at_caret_helper::delete_in_middle_of_line(buffer, engine)?;
    } else {
        delete_at_caret_helper::delete_at_end_of_line(buffer, engine)?;
    }
    None
}

mod delete_at_caret_helper {
    use super::{inline_string, EditorBuffer, EditorEngine, GCString, GCStringExt};

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

        let new_line_content =
            line.delete_char_at_col(buffer.get_caret_scr_adj().col_index)?;

        // When buffer_mut goes out of scope, it will be dropped &
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            let row_index =
                (*buffer_mut.inner.caret_raw + *buffer_mut.inner.scr_ofs).row_index;
            let _unused: GCString = std::mem::replace(
                &mut buffer_mut.inner.lines[row_index.as_usize()],
                new_line_content.grapheme_string(),
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
            inline_string!("{a}{b}", a = this_line.string, b = next_line.string);

        // When buffer_mut goes out of scope, it will be dropped &
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            let row_index =
                (*buffer_mut.inner.caret_raw + *buffer_mut.inner.scr_ofs).row_index;
            let _unused: GCString = std::mem::replace(
                &mut buffer_mut.inner.lines[row_index.as_usize()],
                new_line_content.grapheme_string(),
            );
            buffer_mut.inner.lines.remove(row_index.as_usize() + 1);
        }

        None
    }
}

pub fn backspace_at_caret(
    buffer: &mut EditorBuffer,
    engine: &mut EditorEngine,
) -> Option<()> {
    empty_check_early_return!(buffer, @None);

    match buffer.string_to_left_of_caret() {
        Some(seg_result) => {
            backspace_at_caret_helper::backspace_in_middle_of_line(
                buffer,
                engine,
                seg_result.start_at,
            )?;
        }
        None => {
            backspace_at_caret_helper::backspace_at_start_of_line(buffer, engine)?;
        }
    }

    None
}

mod backspace_at_caret_helper {
    use super::{caret_scroll_index, inline_string, row, scroll_editor_content, ColIndex,
                EditorBuffer, EditorEngine, GCString, GCStringExt};

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
        let new_line_content = cur_line.delete_char_at_col(delete_at_this_display_col)?;

        // When buffer_mut goes out of scope, it will be dropped &
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            let cur_row_index =
                (*buffer_mut.inner.caret_raw + *buffer_mut.inner.scr_ofs).row_index;
            let _unused: GCString = std::mem::replace(
                &mut buffer_mut.inner.lines[cur_row_index.as_usize()],
                new_line_content.grapheme_string(),
            );

            let new_line_content_display_width =
                buffer_mut.inner.lines[cur_row_index.as_usize()].display_width;

            scroll_editor_content::set_caret_col_to(
                delete_at_this_display_col,
                buffer_mut.inner.caret_raw,
                buffer_mut.inner.scr_ofs,
                buffer_mut.inner.vp.col_width,
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
            inline_string!("{a}{b}", a = prev_line.string, b = this_line.string);

        // When buffer_mut goes out of scope, it will be dropped &
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());

            let prev_row_index =
                (*buffer_mut.inner.caret_raw + *buffer_mut.inner.scr_ofs).row_index
                    - row(1);

            let cur_row_index =
                (*buffer_mut.inner.caret_raw + *buffer_mut.inner.scr_ofs).row_index;

            let _unused: GCString = std::mem::replace(
                &mut buffer_mut.inner.lines[prev_row_index.as_usize()],
                new_line_content.grapheme_string(),
            );

            let new_line_content_display_width =
                buffer_mut.inner.lines[prev_row_index.as_usize()].display_width;

            buffer_mut.inner.lines.remove(cur_row_index.as_usize());

            scroll_editor_content::dec_caret_row(
                buffer_mut.inner.caret_raw,
                buffer_mut.inner.scr_ofs,
            );

            scroll_editor_content::set_caret_col_to(
                // This caret col index goes 1 past the end of the line width, ie:
                // - `prev_line_display_width` which is the same as:
                // - `prev_line_display_width.convert_to_col_index() /*-1*/ + 1`
                caret_scroll_index::col_index_for_width(prev_line_display_width),
                buffer_mut.inner.caret_raw,
                buffer_mut.inner.scr_ofs,
                buffer_mut.inner.vp.col_width,
                new_line_content_display_width,
            );
        }

        None
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

    let selection_map = buffer.get_selection_list().clone();

    // Analyze selections and prepare deletion operations
    let (lines_to_remove, lines_to_replace) =
        delete_selected_helper::analyze_selections(buffer, &selection_map);

    // Apply the deletions
    delete_selected_helper::apply_deletions(
        buffer,
        engine,
        lines_to_remove,
        lines_to_replace,
    );

    // Restore caret position and clear selection
    delete_selected_helper::restore_caret_and_clear_selection(
        buffer,
        engine,
        &selection_map,
        with,
    );

    None
}

mod delete_selected_helper {
    use super::{caret_scroll_index, col, ColIndex, DeleteSelectionWith, EditorBuffer,
                EditorEngine, GCString, GCStringExt, HashMap, InlineString, InlineVec,
                RowIndex, SelectionRange, Width};

    pub fn analyze_selections(
        buffer: &EditorBuffer,
        selection_map: &crate::SelectionList,
    ) -> (InlineVec<RowIndex>, HashMap<RowIndex, InlineString>) {
        let lines = buffer.get_lines();
        let selected_row_indices = selection_map.get_ordered_indices();
        let mut vec_row_indices_to_remove = InlineVec::<RowIndex>::new();
        let mut map_lines_to_replace = HashMap::new();

        for selected_row_index in selected_row_indices {
            if let Some(selection_range) = selection_map.get(selected_row_index) {
                let line_width =
                    buffer.get_line_display_width_at_row_index(selected_row_index);

                let (start_col_index, end_col_index) = selection_range.as_tuple();

                // Check if entire line should be removed
                if should_remove_entire_line(start_col_index, end_col_index, line_width) {
                    vec_row_indices_to_remove.push(selected_row_index);
                    continue;
                }

                // Skip if selection range is empty
                if selection_range.start() == selection_range.end() {
                    continue;
                }

                // Prepare partial line replacement
                if let Some(remaining_text) = prepare_partial_line_replacement(
                    lines,
                    selected_row_index,
                    selection_range,
                    end_col_index,
                    line_width,
                ) {
                    map_lines_to_replace.insert(selected_row_index, remaining_text);
                }
            }
        }

        (vec_row_indices_to_remove, map_lines_to_replace)
    }

    fn should_remove_entire_line(
        start_col_index: ColIndex,
        end_col_index: ColIndex,
        line_width: Width,
    ) -> bool {
        start_col_index == col(0)
            && end_col_index == caret_scroll_index::col_index_for_width(line_width)
    }

    fn prepare_partial_line_replacement(
        lines: &[GCString],
        selected_row_index: RowIndex,
        selection_range: SelectionRange,
        end_col_index: ColIndex,
        line_width: Width,
    ) -> Option<InlineString> {
        let line_gcs = lines.get(selected_row_index.as_usize())?.clone();

        let keep_before_selected_str = line_gcs.clip(
            col(0),
            selection_range.get_start_display_col_index_as_width(),
        );

        let keep_after_selected_str = line_gcs.clip(end_col_index, line_width);

        let mut remaining_text = InlineString::with_capacity(
            keep_before_selected_str.len() + keep_after_selected_str.len(),
        );

        remaining_text.push_str(keep_before_selected_str);
        remaining_text.push_str(keep_after_selected_str);

        Some(remaining_text)
    }

    pub fn apply_deletions(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        mut lines_to_remove: InlineVec<RowIndex>,
        lines_to_replace: HashMap<RowIndex, InlineString>,
    ) {
        // When buffer_mut goes out of scope, it will be dropped &
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());

            // Replace lines, before removing them (to prevent indices from being
            // invalidated)
            for row_index in lines_to_replace.keys() {
                let new_line_content = lines_to_replace[row_index].clone();
                let _unused: GCString = std::mem::replace(
                    &mut buffer_mut.inner.lines[row_index.as_usize()],
                    new_line_content.grapheme_string(),
                );
            }

            // Remove lines in inverse order, in order to preserve the validity of indices
            lines_to_remove.reverse();
            for row_index in lines_to_remove {
                buffer_mut.inner.lines.remove(row_index.as_usize());
            }
        }
    }

    pub fn restore_caret_and_clear_selection(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        selection_map: &crate::SelectionList,
        with: DeleteSelectionWith,
    ) {
        // Restore caret position to start of selection range
        let maybe_new_caret =
            selection_map.get_caret_at_start_of_range_scroll_adjusted(with);

        if let Some(new_caret_scr_adj) = maybe_new_caret {
            // When buffer_mut goes out of scope, it will be dropped &
            // validation performed.
            {
                let buffer_mut = buffer.get_mut(engine.viewport());

                // Convert scroll adjusted caret to raw caret by applying scroll offset.
                // Equivalent to: `let caret_raw = *new_caret_scr_adj -
                // *buffer_mut.inner.scr_ofs;`
                let caret_raw = new_caret_scr_adj + *buffer_mut.inner.scr_ofs;
                *buffer_mut.inner.caret_raw = caret_raw;
            }
        }

        buffer.clear_selection();
    }
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
        line.insert_chunk_at_col(caret_scr_adj.col_index, chunk);

    // When buffer_mut goes out of scope, it will be dropped & validation performed.
    {
        let buffer_mut = buffer.get_mut(engine.viewport());

        // Replace existing line w/ new line.
        let _unused: GCString = std::mem::replace(
            &mut buffer_mut.inner.lines[row_index.as_usize()],
            new_line_content.grapheme_string(),
        );

        let new_line_content_display_width =
            buffer_mut.inner.lines[row_index.as_usize()].display_width;

        // Update caret position.
        scroll_editor_content::inc_caret_col_by(
            buffer_mut.inner.caret_raw,
            buffer_mut.inner.scr_ofs,
            chunk_display_width,
            new_line_content_display_width,
            buffer_mut.inner.vp.col_width,
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
        for row_index in 0..=max_row_index {
            if buffer.get_lines().get(row_index).is_none() {
                // When buffer_mut goes out of scope, it will be dropped & validation
                // performed.
                {
                    let buffer_mut = buffer.get_mut(engine.viewport());
                    buffer_mut.inner.lines.push("".grapheme_string());
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
        let new_content = chunk.grapheme_string();
        let _unused: GCString = std::mem::replace(
            &mut buffer_mut.inner.lines[row_index_scr_adj],
            new_content,
        );

        let line_content = &buffer_mut.inner.lines[row_index_scr_adj];
        let line_content_display_width = line_content.display_width;
        let col_amt = GCString::width(chunk);

        // Update caret position.
        scroll_editor_content::inc_caret_col_by(
            buffer_mut.inner.caret_raw,
            buffer_mut.inner.scr_ofs,
            col_amt,
            line_content_display_width,
            buffer_mut.inner.vp.col_width,
        );
    }

    None
}
