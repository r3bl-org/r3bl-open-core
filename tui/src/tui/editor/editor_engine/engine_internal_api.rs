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

//! Functions that implement the internal (not re-exported in `mod.rs`) & functional API
//! of the editor engine. See [`mod@super::engine_public_api`] for the public event based
//! API.

use super::{DeleteSelectionWith, SelectMode, caret_mut, content_mut};
use crate::{EditorArgsMut, EditorBuffer, EditorEngine, GCString, clipboard_support,
            clipboard_support::ClipboardService};

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

#[must_use]
pub fn line_at_caret_to_string(buffer: &EditorBuffer) -> Option<&GCString> {
    buffer.line_at_caret_scr_adj()
}

pub fn insert_str_at_caret(args: EditorArgsMut<'_>, chunk: &str) {
    content_mut::insert_chunk_at_caret(args, chunk);
}

/// Inserts multiple lines of text at the caret position in a single batch operation.
///
/// # Performance Benefits
/// This function is significantly more efficient than inserting lines individually
/// because:
///
/// 1. **Single validation pass**: The editor buffer validation (scroll position, caret
///    bounds, selection ranges, etc.) only happens once when the batch operation
///    completes, rather than after each line insertion.
///
/// 2. **Atomic operation**: All lines are inserted within a single
///    `EditorBufferMutWithDrop` scope, which defers validation until the scope ends.
///
/// 3. **Reduced overhead**: For N lines, this reduces the operation count from 2N-1 (N
///    line insertions + N-1 newline insertions, each with validation) to just 1 batch
///    operation with a single validation.
///
/// # Example
/// ```ignore
/// // Slow approach - validates after each operation:
/// for line in lines {
///     insert_str_at_caret(args, line);      // Validates
///     insert_new_line_at_caret(args);       // Validates again
/// }
///
/// // Fast approach - validates once at the end:
/// insert_str_batch_at_caret(args, lines);   // Validates once
/// ```
///
/// # Arguments
/// * `args` - Mutable references to the editor engine and buffer
/// * `lines` - Vector of string slices to insert, with newlines automatically added
///   between them
pub fn insert_str_batch_at_caret(args: EditorArgsMut<'_>, lines: &[&str]) {
    content_mut::insert_lines_batch_at_caret(args, lines);
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
    clipboard_support::copy_to_clipboard(buffer, clipboard);
}

#[cfg(test)]
mod tests {
    use crate::{DEFAULT_SYN_HI_FILE_EXT, DeleteSelectionWith, EditorBuffer, GCStringExt,
                SelectMode, assert_eq2, caret_raw, col,
                editor::editor_test_fixtures::mock_real_objects_for_editor,
                editor_engine::engine_internal_api, row,
                system_clipboard_service_provider::clipboard_test_fixtures::TestClipboard};

    #[test]
    fn test_select_all() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let _engine = mock_real_objects_for_editor::make_editor_engine();

        // Add some content
        buffer.init_with(vec![
            "line 1".to_string(),
            "line 2".to_string(),
            "line 3".to_string(),
        ]);

        // Select all
        engine_internal_api::select_all(&mut buffer, SelectMode::Enabled);

        // Check that all lines are selected
        let selection_list = buffer.get_selection_list();
        assert_eq2!(selection_list.len(), 3);
        assert!(selection_list.get(row(0)).is_some());
        assert!(selection_list.get(row(1)).is_some());
        assert!(selection_list.get(row(2)).is_some());
    }

    #[test]
    fn test_clear_selection() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let _engine = mock_real_objects_for_editor::make_editor_engine();

        // Add some content and select all
        buffer.init_with(vec!["line 1".to_string(), "line 2".to_string()]);
        engine_internal_api::select_all(&mut buffer, SelectMode::Enabled);

        // Verify selection exists
        assert_eq2!(buffer.get_selection_list().len(), 2);

        // Clear selection
        engine_internal_api::clear_selection(&mut buffer);

        // Verify selection is cleared
        assert_eq2!(buffer.get_selection_list().len(), 0);
    }

    #[test]
    fn test_delete_selected() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Add some content
        buffer.init_with(vec![
            "line 1".to_string(),
            "line 2".to_string(),
            "line 3".to_string(),
        ]);

        // Select line 2
        let buffer_mut = buffer.get_mut(engine.viewport());
        *buffer_mut.inner.caret_raw = caret_raw(col(0) + row(1));
        drop(buffer_mut);

        engine_internal_api::select_all(&mut buffer, SelectMode::Enabled);

        // Delete selected content
        engine_internal_api::delete_selected(
            &mut buffer,
            &mut engine,
            DeleteSelectionWith::Delete,
        );

        // Should have no lines left after deleting all
        assert_eq2!(buffer.get_lines().len(), 0);
    }

    #[test]
    fn test_copy_editor_selection_to_clipboard() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut test_clipboard = TestClipboard::default();

        // Add some content
        buffer.init_with(vec![
            "line 1".to_string(),
            "line 2".to_string(),
            "line 3".to_string(),
        ]);

        // Select all
        engine_internal_api::select_all(&mut buffer, SelectMode::Enabled);

        // Copy to clipboard
        engine_internal_api::copy_editor_selection_to_clipboard(
            &buffer,
            &mut test_clipboard,
        );

        // Check clipboard content
        assert_eq2!(test_clipboard.content, "line 1\nline 2\nline 3");
    }

    #[test]
    fn test_delete_selected_with_partial_selection() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Add some content
        buffer.init_with(vec!["hello world".to_string(), "second line".to_string()]);

        // Move caret to position (5, 0) - after "hello"
        let buffer_mut = buffer.get_mut(engine.viewport());
        *buffer_mut.inner.caret_raw = caret_raw(col(5) + row(0));
        drop(buffer_mut);

        // Select from current position to end of line
        engine_internal_api::end(&mut buffer, &mut engine, SelectMode::Enabled);

        // Delete selected content
        engine_internal_api::delete_selected(
            &mut buffer,
            &mut engine,
            DeleteSelectionWith::Delete,
        );

        // Should have "hello" on first line and "second line" on second
        assert_eq2!(buffer.get_lines().len(), 2);
        assert_eq2!(buffer.get_lines()[0], "hello".grapheme_string());
        assert_eq2!(buffer.get_lines()[1], "second line".grapheme_string());
    }

    #[test]
    fn test_navigation_with_selection() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Add content
        buffer.init_with(vec![
            "first line".to_string(),
            "second line".to_string(),
            "third line".to_string(),
        ]);

        // Start at beginning
        let buffer_mut = buffer.get_mut(engine.viewport());
        *buffer_mut.inner.caret_raw = caret_raw(col(0) + row(0));
        drop(buffer_mut);

        // Select right 5 characters
        for _ in 0..5 {
            engine_internal_api::right(&mut buffer, &mut engine, SelectMode::Enabled);
        }

        // Should have selected "first"
        let selection_list = buffer.get_selection_list();
        assert_eq2!(selection_list.len(), 1);
        assert!(selection_list.get(row(0)).is_some());

        // Move down with selection
        engine_internal_api::down(&mut buffer, &mut engine, SelectMode::Enabled);

        // Should now have selection on two lines
        let selection_list = buffer.get_selection_list();
        assert_eq2!(selection_list.len(), 2);
        assert!(selection_list.get(row(0)).is_some());
        assert!(selection_list.get(row(1)).is_some());
    }

    #[test]
    fn test_line_at_caret_to_string() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let engine = mock_real_objects_for_editor::make_editor_engine();

        // Add content
        buffer.init_with(vec!["first line".to_string(), "second line".to_string()]);

        // Test at first line
        let line = engine_internal_api::line_at_caret_to_string(&buffer);
        assert_eq2!(line.unwrap(), &"first line".grapheme_string());

        // Move to second line
        let buffer_mut = buffer.get_mut(engine.viewport());
        *buffer_mut.inner.caret_raw = caret_raw(col(0) + row(1));
        drop(buffer_mut);

        let line = engine_internal_api::line_at_caret_to_string(&buffer);
        assert_eq2!(line.unwrap(), &"second line".grapheme_string());
    }

    #[test]
    fn test_page_navigation() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Add many lines
        let lines: Vec<String> = (0..50).map(|i| format!("Line {i}")).collect();
        buffer.init_with(lines);

        // Start at top
        let buffer_mut = buffer.get_mut(engine.viewport());
        *buffer_mut.inner.caret_raw = caret_raw(col(0) + row(0));
        drop(buffer_mut);

        // Page down
        engine_internal_api::page_down(&mut buffer, &mut engine, SelectMode::Disabled);

        // Should have moved down (exact amount depends on viewport height)
        let caret_pos = buffer.get_caret_scr_adj();
        assert!(caret_pos.row_index > row(0));

        // Page up
        engine_internal_api::page_up(&mut buffer, &mut engine, SelectMode::Disabled);

        // Should be back near the top
        let caret_pos = buffer.get_caret_scr_adj();
        assert_eq2!(caret_pos.row_index, row(0));
    }

    #[test]
    fn test_home_end_navigation() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Add content
        buffer.init_with(vec!["Hello, World!".to_string()]);

        // Move to middle of line
        let buffer_mut = buffer.get_mut(engine.viewport());
        *buffer_mut.inner.caret_raw = caret_raw(col(7) + row(0));
        drop(buffer_mut);

        // Test home
        engine_internal_api::home(&mut buffer, &mut engine, SelectMode::Disabled);
        assert_eq2!(buffer.get_caret_scr_adj().col_index, col(0));

        // Test end
        engine_internal_api::end(&mut buffer, &mut engine, SelectMode::Disabled);
        assert_eq2!(buffer.get_caret_scr_adj().col_index, col(13)); // Length of "Hello, World!"
    }
}
