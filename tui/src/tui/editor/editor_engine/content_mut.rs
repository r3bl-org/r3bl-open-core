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

use super::{DeleteSelectionWith, scroll_editor_content};
use crate::{caret_locate::{locate_col, CaretColLocationInLine}, caret_scr_adj, caret_scroll_index, col, empty_check_early_return, len, multiline_disabled_check_early_return, row, validate_buffer_mut::EditorBufferMutWithDrop, width, CaretScrAdj, ColIndex, ColWidth, EditorArgsMut, EditorBuffer, EditorEngine, InlineString, InlineVec, RowIndex, SelectionList, SelectionRange, Width, ZeroCopyGapBuffer};

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

/// Inserts multiple lines of text at the caret position in a single atomic operation.
///
/// # Performance Characteristics
///
/// This function provides significant performance improvements over inserting lines
/// individually by leveraging the `EditorBufferMutWithDrop` pattern:
///
/// ## How It Works
/// 1. **Single mutable borrow**: Creates one `EditorBufferMutWithDrop` instance that
///    holds the buffer lock for the entire operation.
/// 2. **Batch processing**: All lines and newlines are inserted while holding this single
///    lock.
/// 3. **Deferred validation**: The expensive validation operations (caret bounds
///    checking, scroll position validation, selection range updates) only run once when
///    the `EditorBufferMutWithDrop` is dropped at the end of the function.
///
/// ## Performance Comparison
/// - **Individual insertions**: O(n) validations for n lines (each insert triggers
///   validation)
/// - **Batch insertion**: O(1) validation regardless of line count
///
/// ## Implementation Details
/// The function inserts each line at the current caret position, then adds a newline
/// (except after the last line). The caret is automatically advanced after each
/// insertion. All of this happens within a single mutable borrow scope, ensuring
/// atomicity and performance.
///
/// # Arguments
/// * `args` - Contains mutable references to the editor buffer and engine
/// * `lines` - Vector of string slices to insert, with newlines added between them
pub fn insert_lines_batch_at_caret(args: EditorArgsMut<'_>, lines: &[&str]) {
    let EditorArgsMut { buffer, engine } = args;

    if lines.is_empty() {
        return;
    }

    // Get a single mutable reference to avoid multiple validations
    let mut buffer_mut = buffer.get_mut(engine.viewport());

    // Process all lines in a single transaction
    let line_count = lines.len();

    for (index, line_content) in lines.iter().enumerate() {
        let current_caret_scr_adj =
            *buffer_mut.inner.caret_raw + *buffer_mut.inner.scr_ofs;
        let row_index = current_caret_scr_adj.row_index;

        // Insert the line content at current position
        if buffer_mut.inner.lines.get_line_content(row_index).is_some() {
            // Insert into existing line - we need to use the trait methods
            if let Some(chunk_width) = buffer_mut.inner.lines.insert_at_col(
                row_index,
                current_caret_scr_adj.col_index,
                line_content,
            ) {
                // Update caret position
                let new_line_display_width = buffer_mut
                    .inner
                    .lines
                    .get_line_display_width(row_index)
                    .unwrap_or(width(0));
                scroll_editor_content::inc_caret_col_by(
                    buffer_mut.inner.caret_raw,
                    buffer_mut.inner.scr_ofs,
                    chunk_width,
                    new_line_display_width,
                    buffer_mut.inner.vp.col_width,
                );
            }
        } else {
            // Create new line
            fill_in_missing_lines_up_to_row_impl(&mut buffer_mut, row_index);
            if buffer_mut.inner.lines.get_line_content(row_index).is_some() {
                buffer_mut.inner.lines.set_line(row_index, line_content);

                // Update caret position
                let line_display_width = buffer_mut
                    .inner
                    .lines
                    .get_line_display_width(row_index)
                    .unwrap_or(width(0));
                let col_amt = width(line_content.len());
                scroll_editor_content::inc_caret_col_by(
                    buffer_mut.inner.caret_raw,
                    buffer_mut.inner.scr_ofs,
                    col_amt,
                    line_display_width,
                    buffer_mut.inner.vp.col_width,
                );
            }
        }

        // Insert newline between lines (but not after the last line)
        if index < line_count - 1 {
            // Insert newline logic similar to insert_new_line_at_caret
            match locate_col_impl(&buffer_mut) {
                CaretColLocationInLine::AtEnd => {
                    // Insert new line at end
                    let new_row_index = scroll_editor_content::inc_caret_row(
                        buffer_mut.inner.caret_raw,
                        buffer_mut.inner.scr_ofs,
                        buffer_mut.inner.vp.row_height,
                    );

                    scroll_editor_content::reset_caret_col(
                        buffer_mut.inner.caret_raw,
                        buffer_mut.inner.scr_ofs,
                    );

                    buffer_mut.inner.lines.insert_line(new_row_index);
                }
                CaretColLocationInLine::AtStart => {
                    // Insert new line at start
                    let cur_row_index = (*buffer_mut.inner.caret_raw
                        + *buffer_mut.inner.scr_ofs)
                        .row_index;
                    buffer_mut.inner.lines.insert_line(cur_row_index);

                    scroll_editor_content::inc_caret_row(
                        buffer_mut.inner.caret_raw,
                        buffer_mut.inner.scr_ofs,
                        buffer_mut.inner.vp.row_height,
                    );
                }
                CaretColLocationInLine::InMiddle => {
                    // Split line in middle
                    let caret_scr_adj =
                        *buffer_mut.inner.caret_raw + *buffer_mut.inner.scr_ofs;
                    let row_index = caret_scr_adj.row_index.as_usize();

                    if let Some(right_content) = buffer_mut.inner.lines.split_line_at_col(
                        caret_scr_adj.row_index,
                        caret_scr_adj.col_index,
                    ) {
                        buffer_mut.inner.lines.insert_line(row(row_index + 1));
                        buffer_mut
                            .inner
                            .lines
                            .set_line(row(row_index + 1), &right_content);

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

    // The EditorBufferMutWithDrop will perform validation once when it's dropped
}

/// Helper function to locate caret position when we already have `buffer_mut`
fn locate_col_impl(
    buffer_mut: &EditorBufferMutWithDrop<'_>,
) -> CaretColLocationInLine {
    let caret_scr_adj = *buffer_mut.inner.caret_raw + *buffer_mut.inner.scr_ofs;
    let row_index = caret_scr_adj.row_index;

    if let Some(line_width) = buffer_mut.inner.lines.get_line_display_width(row_index) {
        let col_index = caret_scr_adj.col_index;

        if col_index == col(0) {
            CaretColLocationInLine::AtStart
        } else if col_index >= caret_scroll_index::col_index_for_width(line_width) {
            CaretColLocationInLine::AtEnd
        } else {
            CaretColLocationInLine::InMiddle
        }
    } else {
        CaretColLocationInLine::AtEnd
    }
}

/// Helper function to fill missing lines when we already have `buffer_mut`
fn fill_in_missing_lines_up_to_row_impl(
    buffer_mut: &mut EditorBufferMutWithDrop<'_>,
    row_index: RowIndex,
) {
    let max_row_index = row_index.as_usize();

    if buffer_mut
        .inner
        .lines
        .get_line_content(row(max_row_index))
        .is_none()
    {
        for row_idx in 0..=max_row_index {
            if buffer_mut
                .inner
                .lines
                .get_line_content(row(row_idx))
                .is_none()
            {
                buffer_mut.inner.lines.push_line("");
            }
        }
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
            buffer_mut.inner.lines.push_line("");
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
    #[allow(clippy::wildcard_imports)]
    use super::*;

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

            buffer_mut.inner.lines.insert_line(new_row_index);
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
            buffer_mut.inner.lines.insert_line(cur_row_index);
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

        let caret_adj = buffer.get_caret_scr_adj();

        // When buffer_mut goes out of scope, it will be dropped & validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());

            if let Some(right_content) = buffer_mut
                .inner
                .lines
                .split_line_at_col(caret_adj.row_index, caret_adj.col_index)
            {
                let row_index = caret_adj.row_index.as_usize();

                buffer_mut.inner.lines.insert_line(row(row_index + 1));
                buffer_mut
                    .inner
                    .lines
                    .set_line(row(row_index + 1), &right_content);

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
    #[allow(clippy::wildcard_imports)]
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
        let caret_scr_adj = buffer.get_caret_scr_adj();

        // When buffer_mut goes out of scope, it will be dropped &
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            let row_index = caret_scr_adj.row_index;

            // Delete one character at the caret position
            buffer_mut.inner.lines.delete_at_col(
                row_index,
                caret_scr_adj.col_index,
                len(1),
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
        let caret_scr_adj = buffer.get_caret_scr_adj();

        // When buffer_mut goes out of scope, it will be dropped &
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            let row_index = caret_scr_adj.row_index;

            // Merge the current line with the next line
            buffer_mut.inner.lines.merge_with_next_line(row_index);
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
                seg_result.width,
            )?;
        }
        None => {
            backspace_at_caret_helper::backspace_at_start_of_line(buffer, engine)?;
        }
    }

    None
}

mod backspace_at_caret_helper {
    #[allow(clippy::wildcard_imports)]
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
        _segment_width: ColWidth,
    ) -> Option<()> {
        // When buffer_mut goes out of scope, it will be dropped &
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            let cur_row_index =
                (*buffer_mut.inner.caret_raw + *buffer_mut.inner.scr_ofs).row_index;

            // Delete the segment at the specified column
            buffer_mut.inner.lines.delete_at_col(
                cur_row_index,
                delete_at_this_display_col,
                len(1),  // Delete 1 segment, regardless of its display width
            );

            let new_line_content_display_width = buffer_mut
                .inner
                .lines
                .get_line_display_width(cur_row_index)
                .unwrap_or(width(0));

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
        let caret_scr_adj = buffer.get_caret_scr_adj();
        let prev_row_index = caret_scr_adj.row_index - row(1);

        // A line above the caret exists.
        let prev_line_display_width =
            buffer.get_line_display_width_at_row_index(prev_row_index);

        // When buffer_mut goes out of scope, it will be dropped &
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());

            // Merge the previous line with the current line
            buffer_mut.inner.lines.merge_with_next_line(prev_row_index);

            let new_line_content_display_width = buffer_mut
                .inner
                .lines
                .get_line_display_width(prev_row_index)
                .unwrap_or(width(0));

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
        &lines_to_replace,
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
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub fn analyze_selections(
        buffer: &EditorBuffer,
        selection_map: &SelectionList,
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

    /// Prepares the replacement text for a line that has a partial selection to be
    /// deleted.
    ///
    /// This function extracts the parts of a line that should be kept when deleting a
    /// selected portion, then concatenates them to form the replacement line content.
    ///
    /// # Visual Example
    ///
    /// ```text
    /// Original line: "Hello [selected text] World"
    ///                      ^              ^
    ///                      |              |
    ///               start of selection   end of selection
    ///
    /// keep_before_selection_range: "Hello "     (keep this part)
    /// keep_after_selection_range:  " World"     (keep this part)
    /// Final result:                "Hello World" (concatenate before + after)
    /// ```
    ///
    /// # Arguments
    /// * `lines` - The gap buffer containing all line data
    /// * `selected_row_index` - The row index of the line being processed
    /// * `selection_range` - The range of text selected for deletion
    /// * `end_col_index` - The column index where the selection ends
    /// * `line_width` - The total display width of the line
    ///
    /// # Returns
    /// * `Some(InlineString)` - The concatenated text that should remain after deletion
    /// * `None` - If the line doesn't exist or cannot be processed
    fn prepare_partial_line_replacement(
        lines: &ZeroCopyGapBuffer,
        selected_row_index: RowIndex,
        selection_range: SelectionRange,
        end_col_index: ColIndex,
        line_width: Width,
    ) -> Option<InlineString> {
        let line_with_info = lines.get_line_with_info(selected_row_index)?;

        // Create selection ranges for the parts we want to keep
        let keep_before_selection_range = SelectionRange::new(
            caret_scr_adj(col(0) + selected_row_index),
            caret_scr_adj(
                col(selection_range
                    .get_start_display_col_index_as_width()
                    .as_usize())
                    + selected_row_index,
            ),
        );
        let keep_after_selection_range = SelectionRange::new(
            caret_scr_adj(end_col_index + selected_row_index),
            caret_scr_adj(col(line_width.as_usize()) + selected_row_index),
        );

        let keep_before_selected_str =
            keep_before_selection_range.clip_to_range_str(line_with_info);
        let keep_after_selected_str =
            keep_after_selection_range.clip_to_range_str(line_with_info);

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
        lines_to_replace: &HashMap<RowIndex, InlineString>,
    ) {
        // When buffer_mut goes out of scope, it will be dropped &
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());

            // Replace lines, before removing them (to prevent indices from being
            // invalidated)
            for row_index in lines_to_replace.keys() {
                let new_line_content = &lines_to_replace[row_index];
                buffer_mut
                    .inner
                    .lines
                    .set_line(*row_index, new_line_content);
            }

            // Remove lines in inverse order, in order to preserve the validity of indices
            lines_to_remove.reverse();
            for row_index in lines_to_remove {
                buffer_mut.inner.lines.remove_line(row_index);
            }
        }
    }

    pub fn restore_caret_and_clear_selection(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        selection_map: &SelectionList,
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

    // When buffer_mut goes out of scope, it will be dropped & validation performed.
    {
        let buffer_mut = buffer.get_mut(engine.viewport());

        // Insert chunk at the specified position
        if let Some(chunk_display_width) = buffer_mut.inner.lines.insert_at_col(
            row_index,
            caret_scr_adj.col_index,
            chunk,
        ) {
            let new_line_content_display_width = buffer_mut
                .inner
                .lines
                .get_line_display_width(row_index)
                .unwrap_or(width(0));

            // Update caret position.
            scroll_editor_content::inc_caret_col_by(
                buffer_mut.inner.caret_raw,
                buffer_mut.inner.scr_ofs,
                chunk_display_width,
                new_line_content_display_width,
                buffer_mut.inner.vp.col_width,
            );
        }
    }

    None
}

/// Insert empty lines up to the row index.
fn fill_in_missing_lines_up_to_row(args: EditorArgsMut<'_>, row_index: RowIndex) {
    let EditorArgsMut { buffer, engine } = args;

    let max_row_index = row_index.as_usize();

    // Fill in any missing lines.
    if buffer
        .get_lines()
        .get_line_content(row(max_row_index))
        .is_none()
    {
        for row_idx in 0..=max_row_index {
            if buffer.get_lines().get_line_content(row(row_idx)).is_none() {
                // When buffer_mut goes out of scope, it will be dropped & validation
                // performed.
                {
                    let buffer_mut = buffer.get_mut(engine.viewport());
                    buffer_mut.inner.lines.push_line("");
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
    let row_index = caret_scr_adj.row_index;

    // Make sure there's a line at caret_adj_row.
    let _unused = buffer.get_lines().get_line_content(row_index)?;

    // When buffer_mut goes out of scope, it will be dropped & validation performed.
    {
        let buffer_mut = buffer.get_mut(engine.viewport());

        // Actually set the content to the correct line.
        buffer_mut.inner.lines.set_line(row_index, chunk);

        let line_content_display_width = buffer_mut
            .inner
            .lines
            .get_line_display_width(row_index)
            .unwrap_or(width(0));
        let col_amt = width(chunk.len());

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

#[cfg(test)]
mod tests {
    use smallvec::smallvec;

    use crate::{CaretDirection, DEFAULT_SYN_HI_FILE_EXT, EditorArgsMut, EditorBuffer,
                EditorEvent, GCStringOwned, assert_eq2, caret_scr_adj,
                clipboard_service::clipboard_test_fixtures::TestClipboard,
                col,
                editor::{editor_test_fixtures::{assert, mock_real_objects_for_editor},
                         sizing::VecEditorContentLines},
                editor_engine::engine_internal_api,
                row, width};

    #[test]
    fn editor_delete() {
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

        // Remove the "a" on the last line.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ❱          │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::Delete,
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(2)));

        // Move to the end of the 2nd line. Press delete.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 ❱ab        │
        //   └──⮬───────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::Delete,
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().len().as_usize(), 2);
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(2) + row(1)));

        // Move to the end of the 1st line.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱abcab     │
        //   └───⮬──────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::Delete,
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().len().as_usize(), 1);
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(3) + row(0)));
        assert::line_at_caret(&buffer, "abcab");
    }

    #[test]
    fn editor_backspace() {
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

        // Remove the "a" on the last line.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ❱          │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(2)));

        // Remove the last line.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 ❱ab        │
        //   └──⮬───────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(2) + row(1)));

        // Move caret to start of 2nd line. Then press backspace.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱abcab     │
        //   └───⮬──────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::MoveCaret(CaretDirection::Left),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(1)));
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().len().as_usize(), 1);
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(3) + row(0)));
        assert::line_at_caret(&buffer, "abcab");

        // Move caret to end of line. Insert "😃". Then move caret to end of line.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱abcab😃   │
        //   └───────⮬──┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::InsertString("😃".into()),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(7) + row(0)));

        // Press backspace.
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Backspace,
            &mut TestClipboard::default(),
        );
        assert::line_at_caret(&buffer, "abcab");
    }

    #[test]
    fn editor_insert_new_line() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Starts w/ an empty line.
        assert_eq2!(buffer.get_lines().len().as_usize(), 1);

        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱          │
        //   └⮬─────────┘
        //   C0123456789
        assert_eq2!(buffer.get_lines().len().as_usize(), 1);
        assert::none_is_at_caret(&buffer);

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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(0)));

        // Insert new line (at end of line).
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 ❱          │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertNewLine],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().len().as_usize(), 2);
        assert::none_is_at_caret(&buffer);
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(1)));

        // Insert "a".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 ❱a         │
        //   └─⮬────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('a')],
            &mut TestClipboard::default(),
        );

        // Move caret left.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 ❱a         │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Left)],
            &mut TestClipboard::default(),
        );
        assert::str_is_at_caret(&buffer, "a");

        // Insert new line (at start of line).
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 │          │
        // 2 ❱a         │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertNewLine],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().len().as_usize(), 3);
        assert::str_is_at_caret(&buffer, "a");
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(2)));

        // Move caret right, insert "b".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 │          │
        // 2 ❱ab        │
        //   └──⮬───────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::InsertChar('b'),
            ],
            &mut TestClipboard::default(),
        );

        assert::none_is_at_caret(&buffer);
        assert_eq2!(
            engine_internal_api::line_at_caret_to_string(&buffer,).unwrap().0,
            "ab"
        );

        // Move caret left, insert new line (at middle of line).
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 │          │
        // 2 │a         │
        // 3 ❱b         │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::InsertNewLine,
            ],
            &mut TestClipboard::default(),
        );
        assert::str_is_at_caret(&buffer, "b");
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(3)));
        assert_eq2!(buffer.get_lines().len().as_usize(), 4);

        // Move caret to end of prev line. Press enter. `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 │          │
        // 2 │a         │
        // 3 ❱          │
        // 4 │b         │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::InsertNewLine,
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().len().as_usize(), 5);
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(3)));
    }

    #[test]
    fn editor_insertion() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Move caret to col: FlexBoxId::from(0), row: 0. Insert "a".
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱a░        │
        //   └─⮬────────┘
        //   C0123456789
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(0)));
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('a')],
            &mut TestClipboard::default(),
        );
        let expected: VecEditorContentLines = smallvec!["a".into()];
        assert_eq2!(buffer.get_lines().to_gc_string_vec(), expected.into_iter().collect::<Vec<_>>());
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(0)));

        // Move caret to col: FlexBoxId::from(0), row: 1. Insert "b".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 ❱b░        │
        //   └─⮬────────┘
        //   C0123456789
        engine_internal_api::insert_new_line_at_caret(EditorArgsMut {
            buffer: &mut buffer,
            engine: &mut engine,
        });
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('b')],
            &mut TestClipboard::default(),
        );
        let expected: VecEditorContentLines = smallvec!["a".into(), "b".into()];
        assert_eq2!(buffer.get_lines().to_gc_string_vec(), expected.into_iter().collect::<Vec<_>>());
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(1)));

        // Move caret to col: FlexBoxId::from(0), row: 3. Insert "😀" (unicode width = 2).
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 │b         │
        // 2 │          │
        // 3 ❱😀░       │
        //   └──⮬───────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertNewLine,
                EditorEvent::InsertNewLine,
                EditorEvent::InsertChar('😀'),
            ],
            &mut TestClipboard::default(),
        );
        let expected: VecEditorContentLines =
            smallvec!["a".into(), "b".into(), "".into(), "😀".into()];
        assert_eq2!(buffer.get_lines().to_gc_string_vec(), expected.into_iter().collect::<Vec<_>>());
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(2) + row(3)));

        // Insert "d".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 │b         │
        // 2 │          │
        // 3 ❱😀d░      │
        //   └───⮬──────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('d')],
            &mut TestClipboard::default(),
        );
        let expected: VecEditorContentLines =
            smallvec!["a".into(), "b".into(), "".into(), "😀d".into()];
        assert_eq2!(buffer.get_lines().to_gc_string_vec(), expected.into_iter().collect::<Vec<_>>());
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(3) + row(3)));

        // Insert "🙏🏽" (unicode width = 2).
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 │b         │
        // 2 │          │
        // 3 ❱😀d🙏🏽░    │
        //   └─────⮬────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("🙏🏽".into())],
            &mut TestClipboard::default(),
        );
        assert_eq2!(width(2), GCStringOwned::width("🙏🏽"));
        let expected: VecEditorContentLines =
            smallvec!["a".into(), "b".into(), "".into(), "😀d🙏🏽".into()];
        assert_eq2!(buffer.get_lines().to_gc_string_vec(), expected.into_iter().collect::<Vec<_>>());
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(5) + row(3)));
    }

    #[test]
    fn test_insert_lines_batch_at_caret_basic() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        let lines = vec!["line1", "line2", "line3"];
        engine_internal_api::insert_str_batch_at_caret(
            EditorArgsMut {
                engine: &mut engine,
                buffer: &mut buffer,
            },
            &lines,
        );

        assert_eq2!(buffer.get_lines().len().as_usize(), 3);
        assert_eq2!(
            buffer.get_lines().get_line_content(row(0)).unwrap(),
            "line1"
        );
        assert_eq2!(
            buffer.get_lines().get_line_content(row(1)).unwrap(),
            "line2"
        );
        assert_eq2!(
            buffer.get_lines().get_line_content(row(2)).unwrap(),
            "line3"
        );

        // Caret should be at the end of the last line
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(5) + row(2)));
    }

    #[test]
    fn test_insert_lines_batch_with_empty_lines() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        let lines = vec!["line1", "", "line3"];
        engine_internal_api::insert_str_batch_at_caret(
            EditorArgsMut {
                engine: &mut engine,
                buffer: &mut buffer,
            },
            &lines,
        );

        assert_eq2!(buffer.get_lines().len().as_usize(), 3);
        assert_eq2!(
            buffer.get_lines().get_line_content(row(0)).unwrap(),
            "line1"
        );
        assert_eq2!(buffer.get_lines().get_line_content(row(1)).unwrap(), "");
        assert_eq2!(
            buffer.get_lines().get_line_content(row(2)).unwrap(),
            "line3"
        );
    }

    #[test]
    fn test_insert_lines_batch_at_middle_of_line() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // First insert some initial content
        buffer.init_with(vec!["existing content".to_string()]);

        // Move caret to middle of line (after "existing")
        let buffer_mut = buffer.get_mut(engine.viewport());
        buffer_mut.inner.caret_raw.col_index = col(8); // Position after "existing"
        drop(buffer_mut);

        // Insert new lines
        let lines = vec!["NEW1", "NEW2"];
        engine_internal_api::insert_str_batch_at_caret(
            EditorArgsMut {
                engine: &mut engine,
                buffer: &mut buffer,
            },
            &lines,
        );

        // The batch insert behavior when inserting in the middle of a line:
        // When inserting multiple lines in the middle of a line, it appears the behavior
        // splits the line and inserts all new content together
        assert_eq2!(buffer.get_lines().len().as_usize(), 2);

        // First, let's check what we actually have
        let lines = buffer.get_lines();
        if !lines.is_empty() {
            assert_eq2!(lines.get_line_content(row(0)).unwrap(), "existingNEW1");
        }
        if lines.len().as_usize() >= 2 {
            assert_eq2!(lines.get_line_content(row(1)).unwrap(), "NEW2 content");
        }
    }

    #[test]
    fn test_batch_vs_individual_insert_result_equivalence() {
        // Test that batch insert produces same result as individual inserts
        let mut buffer1 = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine1 = mock_real_objects_for_editor::make_editor_engine();

        let mut buffer2 = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine2 = mock_real_objects_for_editor::make_editor_engine();

        let lines = vec!["first", "second", "third"];

        // Method 1: Batch insert
        engine_internal_api::insert_str_batch_at_caret(
            EditorArgsMut {
                engine: &mut engine1,
                buffer: &mut buffer1,
            },
            &lines,
        );

        // Method 2: Individual inserts
        engine_internal_api::insert_str_at_caret(
            EditorArgsMut {
                engine: &mut engine2,
                buffer: &mut buffer2,
            },
            "first",
        );
        engine_internal_api::insert_new_line_at_caret(EditorArgsMut {
            engine: &mut engine2,
            buffer: &mut buffer2,
        });
        engine_internal_api::insert_str_at_caret(
            EditorArgsMut {
                engine: &mut engine2,
                buffer: &mut buffer2,
            },
            "second",
        );
        engine_internal_api::insert_new_line_at_caret(EditorArgsMut {
            engine: &mut engine2,
            buffer: &mut buffer2,
        });
        engine_internal_api::insert_str_at_caret(
            EditorArgsMut {
                engine: &mut engine2,
                buffer: &mut buffer2,
            },
            "third",
        );

        // Both methods should produce identical results
        assert_eq2!(buffer1.get_lines(), buffer2.get_lines());
        assert_eq2!(buffer1.get_caret_scr_adj(), buffer2.get_caret_scr_adj());
    }

    #[test]
    fn test_insert_lines_batch_empty_vector() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        let lines: Vec<&str> = vec![];
        engine_internal_api::insert_str_batch_at_caret(
            EditorArgsMut {
                engine: &mut engine,
                buffer: &mut buffer,
            },
            &lines,
        );

        // Buffer should remain unchanged with one empty line
        assert_eq2!(buffer.get_lines().len().as_usize(), 1);
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "");
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(0)));
    }

    #[test]
    fn test_insert_lines_batch_large_content() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Create a large batch of lines
        let lines: Vec<String> = (0..100).map(|i| format!("Line number {i}")).collect();
        let lines_refs: Vec<&str> = lines.iter().map(String::as_str).collect();

        engine_internal_api::insert_str_batch_at_caret(
            EditorArgsMut {
                engine: &mut engine,
                buffer: &mut buffer,
            },
            &lines_refs,
        );

        assert_eq2!(buffer.get_lines().len().as_usize(), 100);
        assert_eq2!(
            buffer.get_lines().get_line_content(row(0)).unwrap(),
            "Line number 0"
        );
        assert_eq2!(
            buffer.get_lines().get_line_content(row(99)).unwrap(),
            "Line number 99"
        );

        // Caret should be at the end of the last line
        let last_line_len = "Line number 99".len();
        assert_eq2!(
            buffer.get_caret_scr_adj(),
            caret_scr_adj(col(last_line_len) + row(99))
        );
    }

    #[test]
    fn test_backspace_emoji_at_end_of_line() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Start with "abcab" and add emoji
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("abcab😃".into()),
            ],
            &mut TestClipboard::default(),
        );
        
        // Verify initial state
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "abcab😃");
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(7) + row(0))); // 5 + 2 for emoji width

        // Backspace should delete the emoji
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Backspace,
            &mut TestClipboard::default(),
        );
        
        // Verify the emoji was deleted
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "abcab");
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(5) + row(0)));
    }

    #[test]
    fn test_backspace_emoji_in_middle_of_line() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Create line with emoji in middle: "Hello 😃 World"
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("Hello 😃 World".into()),
            ],
            &mut TestClipboard::default(),
        );
        
        // Move caret to position after emoji (before " World")
        // "Hello " = 6 cols, emoji = 2 cols, so position 8
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::MoveCaret(CaretDirection::Left),
            ],
            &mut TestClipboard::default(),
        );
        
        // Caret should be at position 8 (after emoji)
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(8) + row(0)));
        
        // Backspace should delete the emoji
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Backspace,
            &mut TestClipboard::default(),
        );
        
        // Verify result
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "Hello  World");
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(6) + row(0)));
    }

    #[test]
    fn test_backspace_multiple_emojis() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Create line with multiple emojis
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("👋😀🎉".into()),
            ],
            &mut TestClipboard::default(),
        );
        
        // Each emoji has width 2, so total width is 6
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(6) + row(0)));
        
        // First backspace deletes 🎉
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Backspace,
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "👋😀");
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(4) + row(0)));
        
        // Second backspace deletes 😀
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Backspace,
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "👋");
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(2) + row(0)));
        
        // Third backspace deletes 👋
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Backspace,
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "");
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(0)));
    }

    #[test]
    fn test_backspace_mixed_width_characters() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Mix of ASCII, emoji, and other Unicode
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("a😃b世界c".into()),
            ],
            &mut TestClipboard::default(),
        );
        
        // Width: a=1, 😃=2, b=1, 世=2, 界=2, c=1, total=9
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(9) + row(0)));
        
        // Backspace 'c'
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Backspace,
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "a😃b世界");
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(8) + row(0)));
        
        // Backspace '界' (width 2)
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Backspace,
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "a😃b世");
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(6) + row(0)));
        
        // Backspace '世' (width 2)
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Backspace,
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "a😃b");
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(4) + row(0)));
    }

    #[test]
    fn test_backspace_family_emoji_zwj_sequence() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Test with family emoji (uses zero-width joiners)
        // This is a single grapheme cluster despite being multiple codepoints
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("test👨‍👩‍👧‍👦end".into()),
            ],
            &mut TestClipboard::default(),
        );
        
        // Move to before "end"
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
        
        // Backspace should delete the entire family emoji as one unit
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Backspace,
            &mut TestClipboard::default(),
        );
        
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "testend");
    }

    #[test]
    fn test_delete_emoji_forward() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Create line with emoji
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("Hello😃World".into()),
            ],
            &mut TestClipboard::default(),
        );
        
        // Move caret to position before emoji
        for _ in 0..6 { // "World" + 1 to get before emoji
            EditorEvent::apply_editor_event(
                &mut engine,
                &mut buffer,
                EditorEvent::MoveCaret(CaretDirection::Left),
                &mut TestClipboard::default(),
            );
        }
        
        // Caret should be at position 5 (after "Hello")
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(5) + row(0)));
        
        // Delete forward should remove the emoji
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Delete,
            &mut TestClipboard::default(),
        );
        
        // Verify result
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "HelloWorld");
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(5) + row(0)));
    }

    #[test]
    fn test_backspace_unicode_emoji_at_end() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "Hello😃"
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("Hello😃".into())],
            &mut TestClipboard::default(),
        );
        
        // Caret should be after emoji (at column 7 = 5 for "Hello" + 2 for emoji)
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(7) + row(0)));

        // Backspace to delete emoji
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        
        // Verify emoji was deleted
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "Hello");
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(5) + row(0)));
    }

    #[test]
    fn test_backspace_unicode_emoji_in_middle() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "ab😃cd"
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("ab😃cd".into())],
            &mut TestClipboard::default(),
        );
        
        // Move caret to after emoji (column 4 = 2 + 2)
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::MoveCaret(CaretDirection::Left),
            ],
            &mut TestClipboard::default(),
        );
        
        // Caret should be at column 4
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(4) + row(0)));

        // Backspace to delete emoji
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        
        // Verify emoji was deleted
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "abcd");
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(2) + row(0)));
    }

    #[test]
    fn test_backspace_unicode_multiple_emojis() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "👋😀🎉"
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("👋😀🎉".into())],
            &mut TestClipboard::default(),
        );
        
        // Each emoji has width 2, so total width is 6
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(6) + row(0)));

        // Backspace three times to delete all emojis
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::Backspace,
                EditorEvent::Backspace,
                EditorEvent::Backspace,
            ],
            &mut TestClipboard::default(),
        );
        
        // Verify all emojis were deleted
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "");
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(0)));
    }

    #[test]
    fn test_backspace_unicode_mixed_content() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "a😃b世界c"
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("a😃b世界c".into())],
            &mut TestClipboard::default(),
        );
        
        // Width: a=1, 😃=2, b=1, 世=2, 界=2, c=1 = total 9
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(9) + row(0)));

        // Backspace to delete 'c'
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "a😃b世界");
        
        // Backspace to delete '界'
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "a😃b世");
        
        // Backspace to delete '世'
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "a😃b");
        
        // Backspace to delete 'b'
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "a😃");
        
        // Backspace to delete emoji
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "a");
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(0)));
    }

    #[test]
    fn test_backspace_unicode_at_beginning_of_line() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert two lines "😃Hello" and "World"
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("😃Hello".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("World".into()),
            ],
            &mut TestClipboard::default(),
        );
        
        // Move caret to beginning of second line
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Home],
            &mut TestClipboard::default(),
        );
        
        // Caret should be at beginning of second line
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(1)));
        
        // Backspace at beginning of line should merge with previous line
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        
        // Lines should be merged
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "😃HelloWorld");
        // Caret should be at the merge point (after "😃Hello" = column 7)
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(7) + row(0)));
    }

    #[test]
    fn test_backspace_unicode_regression_abcab_emoji() {
        // This is the exact regression test for the bug we fixed
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "abcab😃" (the exact string from the bug report)
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("abcab😃".into())],
            &mut TestClipboard::default(),
        );
        
        // Caret should be at column 7 (5 for "abcab" + 2 for emoji)
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(7) + row(0)));

        // Backspace to delete emoji
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        
        // Verify emoji was deleted correctly
        assert_eq2!(buffer.get_lines().get_line_content(row(0)).unwrap(), "abcab");
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(5) + row(0)));
    }
}
