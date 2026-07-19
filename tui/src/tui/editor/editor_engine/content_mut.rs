// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{DeleteSelectionWith, scroll_editor_content};
use crate::{CCaret, CCol, CRow, CWidth, CursorBoundsCheck, CursorPositionBoundsStatus,
            EditorArgsMut, EditorBuffer, EditorEngine, InlineString, InlineVec,
            RangeExt, SelectionContainer, SelectionLine, ZeroCopyGapBuffer, c_col,
            c_len, c_row, c_sel_line, c_width, empty_check_early_return, locate_col,
            multiline_disabled_check_early_return, validate_buffer_mut::EditorBufferMut};
use rustc_hash::FxHashMap;

/// Inserts a single-line string into the editor buffer at the current caret position.
///
/// If the line at the caret's row already exists, this function inserts the text into
/// that line at the caret's column position. If the row does not yet exist, missing empty
/// lines are created up to that row index and the text is placed on the newly created
/// line.
///
/// **Caret & Viewport Movement**
/// - The canvas caret (`c_caret`) column is incremented by the display width of the
///   inserted text.
/// - The viewport origin (`vp_origin`) is panned automatically if the caret moves past
///   the right edge of the visible viewport.
///
/// **Comparison with [`insert_multiple_lines_at_caret`]**
/// - [`insert_into_single_line_at_caret`] is optimized for single-line text entry (e.g.,
///   typing individual characters or pasting single-line text).
/// - For multi-line text containing newline characters, use
///   [`insert_multiple_lines_at_caret`] instead, which executes within a single batch
///   transaction and avoids repeated validation passes.
///
/// # Arguments
/// * `args`: Contains mutable references to the [`EditorBuffer`] and [`EditorEngine`].
/// * `text`: The string slice to insert (must not contain newlines).
///
/// [`EditorBuffer`]: crate::EditorBuffer
/// [`EditorEngine`]: crate::EditorEngine
/// [`insert_multiple_lines_at_caret`]: fn@crate::editor_engine::content_mut::insert_multiple_lines_at_caret
pub fn insert_into_single_line_at_caret(args: EditorArgsMut<'_>, text: &str) {
    let EditorArgsMut { buffer, engine } = args;

    let c_caret = buffer.get_c_caret();
    let row_index = c_caret.row_index;

    if buffer.get_line_at_row_index(row_index).is_some() {
        insert_into_single_line_at_caret_helper::insert_into_existing_line(
            EditorArgsMut::new(buffer, engine),
            c_caret,
            text,
        );
    } else {
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            insert_into_single_line_at_caret_helper::fill_in_missing_lines(
                buffer_mut.inner.lines,
                row_index,
            );
        }
        insert_into_single_line_at_caret_helper::insert_into_new_line(
            EditorArgsMut::new(buffer, engine),
            c_caret,
            text,
        );
    }
}

/// Inserts multiple lines of text at the caret position in a single atomic operation.
///
/// # Performance Characteristics
///
/// This function provides significant performance improvements over inserting lines
/// individually by leveraging the [`EditorBufferMutWithDrop`] pattern:
///
/// ## How It Works
/// 1. **Single mutable borrow**: Creates one [`EditorBufferMutWithDrop`] instance that
///    holds the buffer lock for the entire operation.
/// 2. **Batch processing**: All lines and newlines are inserted while holding this single
///    lock.
/// 3. **Deferred validation**: The expensive validation operations (caret bounds
///    checking, scroll position validation, selection range updates) only run once when
///    the [`EditorBufferMutWithDrop`] is dropped at the end of the function.
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
///
/// [`EditorBufferMutWithDrop`]: crate::EditorBufferMutWithDrop
#[allow(clippy::too_many_lines)]
pub fn insert_multiple_lines_at_caret(args: EditorArgsMut<'_>, lines: &[&str]) {
    let EditorArgsMut { buffer, engine } = args;

    if lines.is_empty() {
        return;
    }

    // Get a single mutable reference to avoid multiple validations.
    let mut buffer_mut = buffer.get_mut(engine.viewport());

    // Process all lines in a single transaction.
    let line_count = lines.len();

    let EditorBufferMut {
        lines: buf_lines,
        c_caret,
        viewport,
        ..
    } = &mut buffer_mut.inner;

    for (index, line_content) in lines.iter().enumerate() {
        let current_c_caret = **c_caret;
        let row_index = current_c_caret.row_index;

        // Insert the line content at current position.
        if buf_lines.get_line_content(row_index).is_some() {
            // Insert into existing line - we need to use the trait methods.
            if let Some(chunk_width) = buf_lines.insert_at_col(
                row_index,
                current_c_caret.col_index,
                line_content,
            ) {
                // Update caret position.
                let new_line_display_width = buf_lines
                    .get_line_display_width(row_index)
                    .unwrap_or(c_width(0));
                scroll_editor_content::horiz_caret_movement::inc_c_caret_col_by(
                    c_caret,
                    viewport,
                    chunk_width,
                    new_line_display_width,
                );
            }
        } else {
            // Create new line.
            insert_into_single_line_at_caret_helper::fill_in_missing_lines(
                buf_lines, row_index,
            );
            if buf_lines.get_line_content(row_index).is_some() {
                buf_lines.set_line(row_index, line_content);

                // Update caret position.
                let line_display_width = buf_lines
                    .get_line_display_width(row_index)
                    .unwrap_or(c_width(0));
                let col_amt = c_width(line_content.len());
                scroll_editor_content::horiz_caret_movement::inc_c_caret_col_by(
                    c_caret,
                    viewport,
                    col_amt,
                    line_display_width,
                );
            }
        }

        // Insert newline between lines (but not after the last line)
        if index < line_count - 1 {
            // Insert newline logic similar to insert_new_line_at_caret.
            let caret = **c_caret;
            let cursor_status = if let Some(line_width) =
                buf_lines.get_line_display_width(caret.row_index)
            {
                line_width.check_cursor_position_bounds(caret.col_index)
            } else {
                CursorPositionBoundsStatus::AtEnd
            };

            match cursor_status {
                CursorPositionBoundsStatus::AtEnd
                | CursorPositionBoundsStatus::Beyond => {
                    // Insert new line at end.
                    let new_row_index =
                        scroll_editor_content::vert_caret_movement::inc_c_caret_row(
                            c_caret, viewport,
                        );

                    scroll_editor_content::horiz_caret_movement::reset_c_caret_col(
                        c_caret, viewport,
                    );

                    buf_lines.insert_line(new_row_index);
                }
                CursorPositionBoundsStatus::AtStart => {
                    // Insert new line at start.
                    let cur_row_index = c_caret.row_index;
                    buf_lines.insert_line(cur_row_index);

                    scroll_editor_content::vert_caret_movement::inc_c_caret_row(
                        c_caret, viewport,
                    );
                }
                CursorPositionBoundsStatus::Within => {
                    // Split line in middle.
                    let caret = **c_caret;
                    if let Some(right_content) =
                        buf_lines.split_line_at_col(caret.row_index, caret.col_index)
                    {
                        let next_row_index = caret.row_index + 1;
                        buf_lines.insert_line(next_row_index);
                        buf_lines.set_line(next_row_index, &right_content);

                        scroll_editor_content::vert_caret_movement::inc_c_caret_row(
                            c_caret, viewport,
                        );

                        scroll_editor_content::horiz_caret_movement::reset_c_caret_col(
                            c_caret, viewport,
                        );
                    }
                }
            }
        }
    }

    // The EditorBufferMutWithDrop will perform validation once when it's dropped.
}

mod insert_into_single_line_at_caret_helper {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub fn insert_into_existing_line(
        args: EditorArgsMut<'_>,
        c_caret: CCaret,
        text: &str,
    ) -> Option<()> {
        let EditorArgsMut { buffer, engine } = args;

        let row_index = c_caret.row_index;

        // When buffer_mut goes out of scope, it will be dropped & validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());

            // Insert text at the specified position.
            if let Some(chunk_display_width) =
                buffer_mut
                    .inner
                    .lines
                    .insert_at_col(row_index, c_caret.col_index, text)
            {
                let new_line_content_display_width = buffer_mut
                    .inner
                    .lines
                    .get_line_display_width(row_index)
                    .unwrap_or(c_width(0));

                // Update caret position.
                scroll_editor_content::horiz_caret_movement::inc_c_caret_col_by(
                    buffer_mut.inner.c_caret,
                    buffer_mut.inner.viewport,
                    chunk_display_width,
                    new_line_content_display_width,
                );
            }
        }

        None
    }

    pub fn insert_into_new_line(
        args: EditorArgsMut<'_>,
        c_caret: CCaret,
        text: &str,
    ) -> Option<()> {
        let EditorArgsMut { buffer, engine } = args;
        let row_index = c_caret.row_index;

        // Make sure there's a line at caret_adj_row.
        let _unused = buffer.get_lines().get_line_content(row_index)?;

        // When buffer_mut goes out of scope, it will be dropped & validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());

            // Actually set the content to the correct line.
            buffer_mut.inner.lines.set_line(row_index, text);

            let line_content_display_width = buffer_mut
                .inner
                .lines
                .get_line_display_width(row_index)
                .unwrap_or(c_width(0));
            let col_amt = c_width(text.len());

            // Update caret position.
            scroll_editor_content::horiz_caret_movement::inc_c_caret_col_by(
                buffer_mut.inner.c_caret,
                buffer_mut.inner.viewport,
                col_amt,
                line_content_display_width,
            );
        }

        None
    }

    /// Fills in missing empty lines up to `row_index` in the gap buffer.
    pub fn fill_in_missing_lines(lines: &mut ZeroCopyGapBuffer, row_index: CRow) {
        let None = lines.get_line_content(row_index) else {
            return;
        };

        let row_range = c_row(0)..=row_index;
        for row_idx in row_range.as_index_iter() {
            if lines.get_line_content(row_idx).is_none() {
                lines.push_line("");
            }
        }
    }
}

/// Inserts a newline character at the current caret position, splitting lines or adding
/// empty lines.
///
/// Depending on where the caret is positioned relative to the current line's text bounds:
///
/// **1. At End or Beyond (EOL)**
/// Adds a new empty line directly below the current line and moves the caret to the start
/// of that new line:
/// ```text
/// Caret: ▲, ►
/// R ┌──────────┐      R ┌──────────┐
/// 0 ►abc       │  ->  0 │abc       │
///   └───▲──────┘      1 ►          │
///   C0123456789         └▲─────────┘
///                       C0123456789
/// ```
///
/// **2. At Start (SOL)**
/// Inserts a new empty line at the current row index, pushing down existing content, and
/// moves the caret to the next line:
/// ```text
/// Caret: ▲, ►
/// R ┌──────────┐      R ┌──────────┐
/// 0 ►abc       │  ->  0 │          │
///   └▲─────────┘      1 ►abc       │
///   C0123456789         └▲─────────┘
///                       C0123456789
/// ```
///
/// **3. In the Middle (Within)**
/// Splits the current line at the caret column, placing right-hand text onto a new row
/// below:
/// ```text
/// Caret: ▲, ►
/// R ┌──────────┐      R ┌──────────┐
/// 0 ►abc       │  ->  0 │a         │
///   └──▲───────┘      1 ►bc        │
///   C0123456789         └▲─────────┘
///                       C0123456789
/// ```
///
/// # Arguments
/// * `args`: Contains mutable references to the [`EditorBuffer`] and [`EditorEngine`].
///
/// [`EditorBuffer`]: crate::EditorBuffer
/// [`EditorEngine`]: crate::EditorEngine
pub fn insert_new_line_at_caret(args: EditorArgsMut<'_>) {
    let EditorArgsMut { buffer, engine } = args;

    multiline_disabled_check_early_return!(engine, @Nothing);

    if buffer.is_empty() {
        // When buffer_mut goes out of scope, it will be dropped and
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.lines.push_line("");
        }
        return;
    }

    match locate_col(buffer) {
        CursorPositionBoundsStatus::AtEnd | CursorPositionBoundsStatus::Beyond => {
            insert_new_line_at_caret_helper::insert_new_line_at_end_of_current_line(
                EditorArgsMut::new(buffer, engine),
            );
        }
        CursorPositionBoundsStatus::AtStart => {
            insert_new_line_at_caret_helper::insert_new_line_at_start_of_current_line(
                EditorArgsMut::new(buffer, engine),
            );
        }
        CursorPositionBoundsStatus::Within => {
            insert_new_line_at_caret_helper::insert_new_line_at_middle_of_current_line(
                EditorArgsMut::new(buffer, engine),
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

        // When buffer_mut goes out of scope, it will be dropped and
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());

            let new_row_index =
                scroll_editor_content::vert_caret_movement::inc_c_caret_row(
                    buffer_mut.inner.c_caret,
                    buffer_mut.inner.viewport,
                );

            scroll_editor_content::horiz_caret_movement::reset_c_caret_col(
                buffer_mut.inner.c_caret,
                buffer_mut.inner.viewport,
            );

            buffer_mut.inner.lines.insert_line(new_row_index);
        }
    }

    // Handle inserting a new line at the start of the current line.
    pub fn insert_new_line_at_start_of_current_line(args: EditorArgsMut<'_>) {
        let EditorArgsMut { buffer, engine } = args;

        // When buffer_mut goes out of scope, it will be dropped and
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            let cur_row_index = buffer_mut.inner.c_caret.row_index;
            buffer_mut.inner.lines.insert_line(cur_row_index);
        }

        // When buffer_mut goes out of scope, it will be dropped and
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            scroll_editor_content::vert_caret_movement::inc_c_caret_row(
                buffer_mut.inner.c_caret,
                buffer_mut.inner.viewport,
            );
        }
    }

    // Handle inserting a new line at the middle of the current line.
    pub fn insert_new_line_at_middle_of_current_line(args: EditorArgsMut<'_>) {
        let EditorArgsMut { buffer, engine } = args;

        let c_caret = buffer.get_c_caret();

        // When buffer_mut goes out of scope, it will be dropped & validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());

            if let Some(right_content) = buffer_mut
                .inner
                .lines
                .split_line_at_col(c_caret.row_index, c_caret.col_index)
            {
                let next_row_index = c_caret.row_index + 1;
                buffer_mut.inner.lines.insert_line(next_row_index);
                buffer_mut
                    .inner
                    .lines
                    .set_line(next_row_index, &right_content);

                scroll_editor_content::vert_caret_movement::inc_c_caret_row(
                    buffer_mut.inner.c_caret,
                    buffer_mut.inner.viewport,
                );

                scroll_editor_content::horiz_caret_movement::reset_c_caret_col(
                    buffer_mut.inner.c_caret,
                    buffer_mut.inner.viewport,
                );
            }
        }
    }
}

/// Deletes a single character forward (to the right of the caret) or merges lines.
///
/// **Deletion Behaviors**
/// - **Middle/Start of line**: Deletes the grapheme cluster immediately at the caret's
///   column position without moving the caret.
/// - **End of line**: Merges the current line with the contents of the line below it.
///
/// ```text
/// Case 1 (Middle):
/// Caret: ▲, ►
/// R ┌──────────┐      R ┌──────────┐
/// 0 ►abc       │  ->  0 ►ac        │
///   └──▲───────┘        └──▲───────┘
///   C0123456789         C0123456789
///
/// Case 2 (End of line):
/// Caret: ▲, ►
/// R ┌──────────┐      R ┌──────────┐
/// 0 ►abc       │  ->  0 ►abcdef    │
/// 1 │def       │        └───▲──────┘
///   └───▲──────┘        C0123456789
///   C0123456789
/// ```
///
/// # Arguments
/// * `buffer`: Mutable reference to the [`EditorBuffer`].
/// * `engine`: Mutable reference to the [`EditorEngine`].
///
/// # Returns
/// * `Some(())` if deletion succeeded, or `None` if the buffer is empty or deletion could
///   not be performed.
///
/// [`EditorBuffer`]: crate::EditorBuffer
/// [`EditorEngine`]: crate::EditorEngine
pub fn delete_at_caret(
    buffer: &mut EditorBuffer,
    engine: &mut EditorEngine,
) -> Option<()> {
    empty_check_early_return!(buffer, @None);
    if buffer.get_seg_at_caret().is_some() {
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
    /// Caret : ▲, ►
    /// R ┌──────────┐
    /// 0 ►abc       │
    /// 1 │ab        │
    /// 2 │a         │
    ///   └─▲────────┘
    ///   C0123456789
    /// ```
    pub fn delete_in_middle_of_line(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
    ) -> Option<()> {
        let c_caret = buffer.get_c_caret();

        // When buffer_mut goes out of scope, it will be dropped and
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            let row_index = c_caret.row_index;

            // Delete one character at the caret position.
            buffer_mut
                .inner
                .lines
                .delete_at_col(row_index, c_caret.col_index, c_len(1));
        }

        None
    }

    /// ```text
    /// Caret : ▲, ►
    /// R ┌──────────┐
    /// 0 ►abc       │
    /// 1 │ab        │
    /// 2 │a         │
    ///   └───▲──────┘
    ///   C0123456789
    /// ```
    pub fn delete_at_end_of_line(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
    ) -> Option<()> {
        let c_caret = buffer.get_c_caret();

        // When buffer_mut goes out of scope, it will be dropped and
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            let row_index = c_caret.row_index;

            // Merge the current line with the next line.
            buffer_mut.inner.lines.merge_with_next_line(row_index);
        }

        None
    }
}

/// Deletes a single character backward (to the left of the caret) or merges with the
/// previous line.
///
/// **Backspace Behaviors**
/// - **Middle/End of line**: Deletes the grapheme cluster immediately to the left of the
///   caret and moves the caret column backward.
/// - **Start of line**: Merges the current line into the end of the previous line above
///   it, moving the caret to the join position.
///
/// ```text
/// Case 1 (Middle):
/// Caret: ▲, ►
/// R ┌──────────┐      R ┌──────────┐
/// 0 ►abc       │  ->  0 ►ac        │
///   └───▲──────┘        └──▲───────┘
///   C0123456789         C0123456789
///
/// Case 2 (Start of line):
/// Caret: ▲, ►
/// R ┌──────────┐      R ┌──────────┐
/// 0 │abc       │  ->  0 ►abcdef    │
/// 1 ►def       │        └───▲──────┘
///   └▲─────────┘        C0123456789
///   C0123456789
/// ```
///
/// # Arguments
/// * `buffer`: Mutable reference to the [`EditorBuffer`].
/// * `engine`: Mutable reference to the [`EditorEngine`].
///
/// # Returns
/// * `Some(())` if backspace succeeded, or `None` if the buffer is empty or backspace
///   could not be performed.
///
/// [`EditorBuffer`]: crate::EditorBuffer
/// [`EditorEngine`]: crate::EditorEngine
pub fn backspace_at_caret(
    buffer: &mut EditorBuffer,
    engine: &mut EditorEngine,
) -> Option<()> {
    empty_check_early_return!(buffer, @None);

    match buffer.get_seg_to_left_of_caret() {
        Some(seg_result) => {
            backspace_at_caret_helper::backspace_in_middle_of_line(
                buffer,
                engine,
                seg_result.start_display_col_index,
                seg_result.display_width,
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
    /// Caret : ▲, ►
    /// R ┌──────────┐
    /// 0 ►abc       │
    /// 1 │ab        │
    /// 2 │a         │
    ///   └─▲────────┘
    ///   C0123456789
    /// ```
    pub fn backspace_in_middle_of_line(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        delete_at_this_display_col: CCol,
        _segment_width: CWidth,
    ) -> Option<()> {
        // When buffer_mut goes out of scope, it will be dropped and
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            let cur_row_index = buffer_mut.inner.c_caret.row_index;

            // Delete the segment at the specified column.
            buffer_mut.inner.lines.delete_at_col(
                cur_row_index,
                delete_at_this_display_col,
                c_len(1), // Delete 1 segment, regardless of its display width
            );

            let new_line_content_display_width = buffer_mut
                .inner
                .lines
                .get_line_display_width(cur_row_index)
                .unwrap_or(c_width(0));

            scroll_editor_content::horiz_caret_movement::set_c_caret_col_to(
                delete_at_this_display_col,
                buffer_mut.inner.c_caret,
                buffer_mut.inner.viewport,
                new_line_content_display_width,
            );
        }

        None
    }

    /// ```text
    /// Caret : ▲, ►
    /// R ┌──────────┐
    /// 0 │abc       │
    /// 1 ►ab        │
    /// 2 │a         │
    ///   └▲─────────┘
    ///   C0123456789
    /// ```
    pub fn backspace_at_start_of_line(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
    ) -> Option<()> {
        let c_caret = buffer.get_c_caret();
        let prev_row_index = c_caret.row_index - 1;

        // A line above the caret exists.
        let prev_line_display_width = buffer
            .get_lines()
            .get_line_display_width_at_row_index(prev_row_index);

        // When buffer_mut goes out of scope, it will be dropped and
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());

            // Merge the previous line with the current line.
            buffer_mut.inner.lines.merge_with_next_line(prev_row_index);

            let new_line_content_display_width = buffer_mut
                .inner
                .lines
                .get_line_display_width_at_row_index(prev_row_index);

            scroll_editor_content::vert_caret_movement::dec_c_caret_row(
                buffer_mut.inner.c_caret,
                buffer_mut.inner.viewport,
            );

            scroll_editor_content::horiz_caret_movement::set_c_caret_col_to(
                prev_line_display_width.eol_cursor_position(),
                buffer_mut.inner.c_caret,
                buffer_mut.inner.viewport,
                new_line_content_display_width,
            );
        }

        None
    }
}

/// Deletes all currently selected text ranges across single or multiple lines in the
/// buffer.
///
/// **Deletion Lifecycle**
/// 1. **Analyze selections**: Classifies each selected line into complete line removals
///    (when the entire line width is selected) or partial line replacements (when keeping
///    text before/after).
/// 2. **Apply mutations**: Updates partially selected lines first, then removes fully
///    selected lines in reverse row order to preserve line index validity.
/// 3. **Restore caret & clear**: Repositions the caret to the start of the former
///    selection and clears the selection container.
///
/// ```text
/// Before:
/// Caret: ▲, ►
/// R ┌──────────────────────┐
/// 0 ►Hello [selected] World│
///   └────────────────▲─────┘
///   C0123456789012345678901
///              1         2
///
/// After:
/// Caret: ▲, ►
/// R ┌──────────────────────┐
/// 0 ►Hello World           │
///   └──────▲───────────────┘
///   C0123456789012345678901
///              1         2
/// ```
///
/// # Arguments
/// * `buffer`: Mutable reference to the [`EditorBuffer`].
/// * `engine`: Mutable reference to the [`EditorEngine`].
/// * `with`: Determines how the new caret position is calculated upon deleting the
///   selection.
///
/// # Returns
/// * `Some(())` if selection was deleted, or `None` if the buffer is empty or no text was
///   selected.
///
/// [`EditorBuffer`]: crate::EditorBuffer
/// [`EditorEngine`]: crate::EditorEngine
pub fn delete_selected(
    buffer: &mut EditorBuffer,
    engine: &mut EditorEngine,
    with: DeleteSelectionWith,
) -> Option<()> {
    // Early return if any of the following are met.
    empty_check_early_return!(buffer, @None);
    if buffer.get_selection_container().is_empty() {
        return None;
    }

    let selection_map = buffer.get_selection_container().clone();

    // Analyze selections and prepare deletion operations.
    let (lines_to_remove, lines_to_replace) =
        delete_selected_helper::analyze_selections(buffer, &selection_map);

    // Apply the deletions.
    delete_selected_helper::apply_deletions(
        buffer,
        engine,
        lines_to_remove,
        &lines_to_replace,
    );

    // Restore caret position and clear selection.
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
        selection_container: &SelectionContainer,
    ) -> (InlineVec<CRow>, FxHashMap<CRow, InlineString>) {
        let lines = buffer.get_lines();
        let mut vec_row_indices_to_remove = InlineVec::<CRow>::new();
        let mut map_lines_to_replace = FxHashMap::<CRow, InlineString>::default();

        for selection_line in selection_container.iter() {
            let selected_row_index = selection_line.row;
            let line_width = buffer
                .get_lines()
                .get_line_display_width_at_row_index(selected_row_index);

            let (start_col_index, end_col_index) =
                (selection_line.get_start(), selection_line.get_end());

            // Check if entire line should be removed.
            if should_remove_entire_line(start_col_index, end_col_index, line_width) {
                vec_row_indices_to_remove.push(selected_row_index);
                continue;
            }

            // Skip if selection range is empty.
            if selection_line.get_start() == selection_line.get_end() {
                continue;
            }

            // Prepare partial line replacement.
            if let Some(remaining_text) = prepare_partial_line_replacement(
                lines,
                selected_row_index,
                selection_line.clone(),
                end_col_index,
                line_width,
            ) {
                map_lines_to_replace.insert(selected_row_index, remaining_text);
            }
        }

        (vec_row_indices_to_remove, map_lines_to_replace)
    }

    fn should_remove_entire_line(
        start_col_index: CCol,
        end_col_index: CCol,
        line_width: CWidth,
    ) -> bool {
        let starts_at_beginning = start_col_index == c_col(0);
        let ends_at_or_beyond_eol = end_col_index >= line_width.eol_cursor_position();
        starts_at_beginning && ends_at_or_beyond_eol
    }

    /// Prepares the replacement text for a line that has a partial selection to be
    /// deleted.
    ///
    /// This function extracts the parts of a line that should be kept when deleting a
    /// selected portion, then concatenates them to form the replacement line content.
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
        selected_row_index: CRow,
        selection_range: SelectionLine,
        end_col_index: CCol,
        line_width: CWidth,
    ) -> Option<InlineString> {
        let line_with_info = lines.get_line(selected_row_index)?;

        // Create selection ranges for the parts we want to keep.
        let start_col = selection_range.get_start();
        let keep_before_selection_range =
            c_sel_line(selected_row_index, c_col(0)..start_col);
        let keep_after_selection_range = c_sel_line(
            selected_row_index,
            end_col_index..line_width.eol_cursor_position(),
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
        mut lines_to_remove: InlineVec<CRow>,
        lines_to_replace: &FxHashMap<CRow, InlineString>,
    ) {
        // When buffer_mut goes out of scope, it will be dropped and
        // validation performed.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());

            // Replace lines, before removing them (to prevent indices from being.
            // invalidated)
            for row_index in lines_to_replace.keys() {
                let new_line_content = &lines_to_replace[row_index];
                buffer_mut
                    .inner
                    .lines
                    .set_line(*row_index, new_line_content);
            }

            // Remove lines in inverse order, in order to preserve the validity of
            // indices.
            lines_to_remove.reverse();
            for row_index in lines_to_remove {
                buffer_mut.inner.lines.remove_line(row_index);
            }
        }
    }

    pub fn restore_caret_and_clear_selection(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        selection_map: &SelectionContainer,
        with: DeleteSelectionWith,
    ) {
        // Restore caret position to start of selection range.
        let maybe_new_caret = selection_map.get_c_caret_at_start_of_range(with);

        if let Some(new_c_caret) = maybe_new_caret {
            // When buffer_mut goes out of scope, it will be dropped &.
            // validation performed.
            {
                let buffer_mut = buffer.get_mut(engine.viewport());

                // Set canvas caret to start of selection range.
                *buffer_mut.inner.c_caret = new_c_caret;
            }
        }

        buffer.clear_selection();
    }
}

#[cfg(test)]
mod tests {
    //! Content mutation tests.
    //!
    //! # Implementation Note: Intentional Use of Raw `usize`
    //!
    //! Test assertions use `.as_usize()` for comparison with numeric literals.
    //! This is legitimate for test validation and doesn't require type-safe bounds
    //! checking.

    use crate::{CaretDirection, DEFAULT_SYN_HI_FILE_EXT, EditorArgsMut, EditorBuffer,
                EditorEvent, FileExtensionToken, GCStringOwned, assert_eq2, c_caret,
                c_col, c_row,
                clipboard_test_fixtures::TestClipboard,
                editor::test_fixtures_editor::{assert, mock_real_objects_for_editor},
                editor_engine::engine_internal_api,
                vp_width};

    #[test]
    fn editor_delete() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "abc\nab\na".
        //
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ►a         │
        //   └─▲────────┘
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
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(1) + c_row(2)));

        // Remove the "a" on the last line.
        //
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ►          │
        //   └▲─────────┘
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
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(0) + c_row(2)));

        // Move to the end of the 2nd line. Press delete.
        //
        // R ┌──────────┐
        // 0 │abc       │
        // 1 ►ab        │
        //   └──▲───────┘
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
        assert_eq2!(buffer.get_lines().get_line_count().as_usize(), 2);
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(2) + c_row(1)));

        // Move to the end of the 1st line.
        //
        // R ┌──────────┐
        // 0 ►abcab     │
        //   └───▲──────┘
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
        assert_eq2!(buffer.get_lines().get_line_count().as_usize(), 1);
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(3) + c_row(0)));
        assert::line_at_caret(&buffer, "abcab");
    }

    #[test]
    fn editor_backspace() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "abc\nab\na".
        //
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ►a         │
        //   └─▲────────┘
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
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(1) + c_row(2)));

        // Remove the "a" on the last line.
        //
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ►          │
        //   └▲─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(0) + c_row(2)));

        // Remove the last line.
        //
        // R ┌──────────┐
        // 0 │abc       │
        // 1 ►ab        │
        //   └──▲───────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(2) + c_row(1)));

        // Move caret to start of 2nd line. Then press backspace.
        //
        // R ┌──────────┐
        // 0 ►abcab     │
        //   └───▲──────┘
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
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(0) + c_row(1)));
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().get_line_count().as_usize(), 1);
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(3) + c_row(0)));
        assert::line_at_caret(&buffer, "abcab");

        // Move caret to end of line. Insert "😃". Then move caret to end of line.
        //
        // R ┌──────────┐
        // 0 ►abcab😃   │
        //   └───────▲──┘
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
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(7) + c_row(0)));

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
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Starts w/ an empty line.
        assert_eq2!(buffer.get_lines().get_line_count().as_usize(), 1);

        //
        // R ┌──────────┐
        // 0 ►          │
        //   └▲─────────┘
        //   C0123456789
        assert_eq2!(buffer.get_lines().get_line_count().as_usize(), 1);
        assert::none_is_at_caret(&buffer);

        // Insert "a".
        //
        // R ┌──────────┐
        // 0 ►a         │
        //   └─▲────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('a')],
            &mut TestClipboard::default(),
        );
        assert::none_is_at_caret(&buffer);
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(1) + c_row(0)));

        // Insert new line (at end of line).
        //
        // R ┌──────────┐
        // 0 │a         │
        // 1 ►          │
        //   └▲─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertNewLine],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().get_line_count().as_usize(), 2);
        assert::none_is_at_caret(&buffer);
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(0) + c_row(1)));

        // Insert "a".
        //
        // R ┌──────────┐
        // 0 │a         │
        // 1 ►a         │
        //   └─▲────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('a')],
            &mut TestClipboard::default(),
        );

        // Move caret left.
        //
        // R ┌──────────┐
        // 0 │a         │
        // 1 ►a         │
        //   └▲─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Left)],
            &mut TestClipboard::default(),
        );
        assert::str_is_at_caret(&buffer, "a");

        // Insert new line (at start of line).
        //
        // R ┌──────────┐
        // 0 │a         │
        // 1 │          │
        // 2 ►a         │
        //   └▲─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertNewLine],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().get_line_count().as_usize(), 3);
        assert::str_is_at_caret(&buffer, "a");
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(0) + c_row(2)));

        // Move caret right, insert "b".
        //
        // R ┌──────────┐
        // 0 │a         │
        // 1 │          │
        // 2 ►ab        │
        //   └──▲───────┘
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
            engine_internal_api::line_at_caret_to_string(&buffer,)
                .expect("conversion error")
                .content(),
            "ab"
        );

        // Move caret left, insert new line (at middle of line).
        //
        // R ┌──────────┐
        // 0 │a         │
        // 1 │          │
        // 2 │a         │
        // 3 ►b         │
        //   └▲─────────┘
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
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(0) + c_row(3)));
        assert_eq2!(buffer.get_lines().get_line_count().as_usize(), 4);

        // Move caret to end of prev line. Press enter. `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 │          │
        // 2 │a         │
        // 3 ►          │
        // 4 │b         │
        //   └▲─────────┘
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
        assert_eq2!(buffer.get_lines().get_line_count().as_usize(), 5);
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(0) + c_row(3)));
    }

    #[test]
    fn editor_insertion() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Move caret to col: FlexBoxId::from(0), row: 0. Insert "a".
        //
        // R ┌──────────┐
        // 0 ►a         │
        //   └─▲────────┘
        //   C0123456789
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(0) + c_row(0)));
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('a')],
            &mut TestClipboard::default(),
        );
        let expected = vec!["a"];
        assert_eq2!(
            buffer.get_lines().to_gc_string_vec(),
            expected.into_iter().map(Into::into).collect::<Vec<_>>()
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(1) + c_row(0)));

        // Move caret to col: FlexBoxId::from(0), row: 1. Insert "b".
        //
        // R ┌──────────┐
        // 0 │a         │
        // 1 ►b         │
        //   └─▲────────┘
        //   C0123456789
        engine_internal_api::insert_new_line_at_caret(EditorArgsMut::new(
            &mut buffer,
            &mut engine,
        ));
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('b')],
            &mut TestClipboard::default(),
        );
        let expected = vec!["a", "b"];
        assert_eq2!(
            buffer.get_lines().to_gc_string_vec(),
            expected.into_iter().map(Into::into).collect::<Vec<_>>()
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(1) + c_row(1)));

        // Move caret to col: FlexBoxId::from(0), row: 3. Insert "😀" (unicode width = 2).
        //
        // R ┌──────────┐
        // 0 │a         │
        // 1 │b         │
        // 2 │          │
        // 3 ►😀        │
        //   └──▲───────┘
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
        let expected = vec!["a", "b", "", "😀"];
        assert_eq2!(
            buffer.get_lines().to_gc_string_vec(),
            expected.into_iter().map(Into::into).collect::<Vec<_>>()
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(2) + c_row(3)));

        // Insert "d".
        //
        // R ┌──────────┐
        // 0 │a         │
        // 1 │b         │
        // 2 │          │
        // 3 ►😀d       │
        //   └───▲──────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('d')],
            &mut TestClipboard::default(),
        );
        let expected = vec!["a", "b", "", "😀d"];
        assert_eq2!(
            buffer.get_lines().to_gc_string_vec(),
            expected.into_iter().map(Into::into).collect::<Vec<_>>()
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(3) + c_row(3)));

        // Insert "🙏🏽" (unicode width = 2).
        //
        // R ┌──────────┐
        // 0 │a         │
        // 1 │b         │
        // 2 │          │
        // 3 ►😀d🙏🏽     │
        //   └─────▲────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("🙏🏽".into())],
            &mut TestClipboard::default(),
        );
        assert_eq2!(vp_width(2), GCStringOwned::from("🙏🏽").width());
        let expected = vec!["a", "b", "", "😀d🙏🏽"];
        assert_eq2!(
            buffer.get_lines().to_gc_string_vec(),
            expected.into_iter().map(Into::into).collect::<Vec<_>>()
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(5) + c_row(3)));
    }

    #[test]
    fn test_insert_multiple_lines_at_caret_basic() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        let lines = vec!["line1", "line2", "line3"];
        engine_internal_api::insert_multiple_lines_at_caret(
            EditorArgsMut::new(&mut buffer, &mut engine),
            &lines,
        );

        assert_eq2!(buffer.get_lines().get_line_count().as_usize(), 3);
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "line1"
        );
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(1))
                .expect("conversion error"),
            "line2"
        );
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(2))
                .expect("conversion error"),
            "line3"
        );

        // Caret should be at the end of the last line.
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(5) + c_row(2)));
    }

    #[test]
    fn test_insert_multiple_lines_with_empty_lines() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        let lines = vec!["line1", "", "line3"];
        engine_internal_api::insert_multiple_lines_at_caret(
            EditorArgsMut::new(&mut buffer, &mut engine),
            &lines,
        );

        assert_eq2!(buffer.get_lines().get_line_count().as_usize(), 3);
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "line1"
        );
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(1))
                .expect("conversion error"),
            ""
        );
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(2))
                .expect("conversion error"),
            "line3"
        );
    }

    #[test]
    fn test_insert_multiple_lines_at_middle_of_line() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // First insert some initial content.
        buffer.init_with(vec!["existing content".to_string()]);

        // Move caret to middle of line (after "existing")
        let buffer_mut = buffer.get_mut(engine.viewport());
        buffer_mut.inner.c_caret.col_index = c_col(8); // Position after "existing"
        drop(buffer_mut);

        // Insert new lines.
        let lines = vec!["NEW1", "NEW2"];
        engine_internal_api::insert_multiple_lines_at_caret(
            EditorArgsMut::new(&mut buffer, &mut engine),
            &lines,
        );

        // The batch insert behavior when inserting in the middle of a line:
        // When inserting multiple lines in the middle of a line, it appears the behavior
        // splits the line and inserts all new content together.
        assert_eq2!(buffer.get_lines().get_line_count().as_usize(), 2);

        // First, let's check what we actually have.
        let lines = buffer.get_lines();
        if !lines.is_empty() {
            assert_eq2!(
                lines.get_line_content(c_row(0)).expect("conversion error"),
                "existingNEW1"
            );
        }
        if lines.get_line_count().as_usize() >= 2 {
            assert_eq2!(
                lines.get_line_content(c_row(1)).expect("conversion error"),
                "NEW2 content"
            );
        }
    }

    #[test]
    fn test_multiple_lines_vs_individual_insert_result_equivalence() {
        // Test that batch insert produces same result as individual inserts.
        let mut buffer1 =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine1 = mock_real_objects_for_editor::make_editor_engine();

        let mut buffer2 =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine2 = mock_real_objects_for_editor::make_editor_engine();

        let lines = vec!["first", "second", "third"];

        // Method 1: Batch insert
        engine_internal_api::insert_multiple_lines_at_caret(
            EditorArgsMut::new(&mut buffer1, &mut engine1),
            &lines,
        );

        // Method 2: Individual inserts
        engine_internal_api::insert_into_single_line_at_caret(
            EditorArgsMut::new(&mut buffer2, &mut engine2),
            "first",
        );
        engine_internal_api::insert_new_line_at_caret(EditorArgsMut::new(
            &mut buffer2,
            &mut engine2,
        ));
        engine_internal_api::insert_into_single_line_at_caret(
            EditorArgsMut::new(&mut buffer2, &mut engine2),
            "second",
        );
        engine_internal_api::insert_new_line_at_caret(EditorArgsMut::new(
            &mut buffer2,
            &mut engine2,
        ));
        engine_internal_api::insert_into_single_line_at_caret(
            EditorArgsMut::new(&mut buffer2, &mut engine2),
            "third",
        );

        // Both methods should produce identical results.
        assert_eq2!(buffer1.get_lines(), buffer2.get_lines());
        assert_eq2!(buffer1.get_c_caret(), buffer2.get_c_caret());
    }

    #[test]
    fn test_insert_multiple_lines_empty_vector() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        let lines: Vec<&str> = vec![];
        engine_internal_api::insert_multiple_lines_at_caret(
            EditorArgsMut::new(&mut buffer, &mut engine),
            &lines,
        );

        // Buffer should remain unchanged with one empty line.
        assert_eq2!(buffer.get_lines().get_line_count().as_usize(), 1);
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            ""
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(0) + c_row(0)));
    }

    #[test]
    fn test_insert_multiple_lines_large_content() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Create a large batch of lines.
        let lines: Vec<String> = (0..100).map(|i| format!("Line number {i}")).collect();
        let lines_refs: Vec<&str> = lines.iter().map(String::as_str).collect();

        engine_internal_api::insert_multiple_lines_at_caret(
            EditorArgsMut::new(&mut buffer, &mut engine),
            &lines_refs,
        );

        assert_eq2!(buffer.get_lines().get_line_count().as_usize(), 100);
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "Line number 0"
        );
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(99))
                .expect("conversion error"),
            "Line number 99"
        );

        // Caret should be at the end of the last line.
        let last_line_len = "Line number 99".len();
        assert_eq2!(
            buffer.get_c_caret(),
            c_caret(c_col(last_line_len) + c_row(99))
        );
    }

    #[test]
    fn test_backspace_emoji_at_end_of_line() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Start with "abcab" and add emoji.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("abcab😃".into())],
            &mut TestClipboard::default(),
        );

        // Verify initial state.
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "abcab😃"
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(7) + c_row(0))); // 5 + 2 for emoji width

        // Backspace should delete the emoji.
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Backspace,
            &mut TestClipboard::default(),
        );

        // Verify the emoji was deleted.
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "abcab"
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(5) + c_row(0)));
    }

    #[test]
    fn test_backspace_emoji_in_middle_of_line() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Create line with emoji in middle: "Hello 😃 World".
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("Hello 😃 World".into())],
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
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(8) + c_row(0)));

        // Backspace should delete the emoji.
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Backspace,
            &mut TestClipboard::default(),
        );

        // Verify result
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "Hello  World"
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(6) + c_row(0)));
    }

    #[test]
    fn test_backspace_multiple_emojis() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Create line with multiple emojis.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("👋😀🎉".into())],
            &mut TestClipboard::default(),
        );

        // Each emoji has width 2, so total width is 6.
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(6) + c_row(0)));

        // First backspace deletes 🎉.
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Backspace,
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "👋😀"
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(4) + c_row(0)));

        // Second backspace deletes 😀.
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Backspace,
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "👋"
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(2) + c_row(0)));

        // Third backspace deletes 👋.
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Backspace,
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            ""
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(0) + c_row(0)));
    }

    #[test]
    fn test_backspace_mixed_width_characters() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Mix of ASCII, emoji, and other Unicode.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("a😃b世界c".into())],
            &mut TestClipboard::default(),
        );

        // ColWidth: a=1, 😃=2, b=1, 世=2, 界=2, c=1, total=9
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(9) + c_row(0)));

        // Backspace 'c'
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Backspace,
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "a😃b世界"
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(8) + c_row(0)));

        // Backspace '界' (width 2)
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Backspace,
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "a😃b世"
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(6) + c_row(0)));

        // Backspace '世' (width 2)
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Backspace,
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "a😃b"
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(4) + c_row(0)));
    }

    #[test]
    fn test_backspace_family_emoji_zwj_sequence() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Test with family emoji (uses zero-width joiners)
        // This is a single grapheme cluster despite being multiple codepoints.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("test👨‍👩‍👧‍👦end".into())],
            &mut TestClipboard::default(),
        );

        // Move to before "end".
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

        // Backspace should delete the entire family emoji as one unit.
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Backspace,
            &mut TestClipboard::default(),
        );

        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "testend"
        );
    }

    #[test]
    fn test_delete_emoji_forward() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Create line with emoji.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("Hello😃World".into())],
            &mut TestClipboard::default(),
        );

        // Move caret to position before emoji.
        for _ in 0..6 {
            // "World" + 1 to get before emoji.
            EditorEvent::apply_editor_event(
                &mut engine,
                &mut buffer,
                EditorEvent::MoveCaret(CaretDirection::Left),
                &mut TestClipboard::default(),
            );
        }

        // Caret should be at position 5 (after "Hello")
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(5) + c_row(0)));

        // Delete forward should remove the emoji.
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Delete,
            &mut TestClipboard::default(),
        );

        // Verify result
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "HelloWorld"
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(5) + c_row(0)));
    }

    #[test]
    fn test_backspace_unicode_emoji_at_end() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "Hello😃"
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("Hello😃".into())],
            &mut TestClipboard::default(),
        );

        // Caret should be after emoji (at column 7 = 5 for "Hello" + 2 for emoji)
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(7) + c_row(0)));

        // Backspace to delete emoji.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );

        // Verify emoji was deleted.
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "Hello"
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(5) + c_row(0)));
    }

    #[test]
    fn test_backspace_unicode_emoji_in_middle() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
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

        // Caret should be at column 4.
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(4) + c_row(0)));

        // Backspace to delete emoji.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );

        // Verify emoji was deleted.
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "abcd"
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(2) + c_row(0)));
    }

    #[test]
    fn test_backspace_unicode_multiple_emojis() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "👋😀🎉"
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("👋😀🎉".into())],
            &mut TestClipboard::default(),
        );

        // Each emoji has width 2, so total width is 6.
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(6) + c_row(0)));

        // Backspace three times to delete all emojis.
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

        // Verify all emojis were deleted.
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            ""
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(0) + c_row(0)));
    }

    #[test]
    fn test_backspace_unicode_mixed_content() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "a😃b世界c"
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("a😃b世界c".into())],
            &mut TestClipboard::default(),
        );

        // ColWidth: a=1, 😃=2, b=1, 世=2, 界=2, c=1 = total 9
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(9) + c_row(0)));

        // Backspace to delete 'c'.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "a😃b世界"
        );

        // Backspace to delete '界'.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "a😃b世"
        );

        // Backspace to delete '世'.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "a😃b"
        );

        // Backspace to delete 'b'.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "a😃"
        );

        // Backspace to delete emoji.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "a"
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(1) + c_row(0)));
    }

    #[test]
    fn test_backspace_unicode_at_beginning_of_line() {
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert two lines "😃Hello" and "World".
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

        // Move caret to beginning of second line.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Home],
            &mut TestClipboard::default(),
        );

        // Caret should be at beginning of second line.
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(0) + c_row(1)));

        // Backspace at beginning of line should merge with previous line.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );

        // Lines should be merged.
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "😃HelloWorld"
        );
        // Caret should be at the merge point (after "😃Hello" = column 7)
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(7) + c_row(0)));
    }

    #[test]
    fn test_backspace_unicode_regression_abcab_emoji() {
        // This is the exact regression test for the bug we fixed.
        let mut buffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "abcab😃" (the exact string from the bug report)
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("abcab😃".into())],
            &mut TestClipboard::default(),
        );

        // Caret should be at column 7 (5 for "abcab" + 2 for emoji)
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(7) + c_row(0)));

        // Backspace to delete emoji.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );

        // Verify emoji was deleted correctly.
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "abcab"
        );
        assert_eq2!(buffer.get_c_caret(), c_caret(c_col(5) + c_row(0)));
    }
}
