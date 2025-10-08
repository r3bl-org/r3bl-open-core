// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{DEBUG_TUI_COPY_PASTE, EditorBuffer, InlineVecStr};
use std::error::Error;

pub type ClipboardResult<T> = Result<T, Box<dyn Error + Send + Sync + 'static>>;

/// Abstraction for the clipboard service for dependency injection. This trait is
/// implemented by both a test clipboard service and a system clipboard service.
pub trait ClipboardService {
    /// # Errors
    ///
    /// Returns an error if the clipboard operation fails.
    fn try_to_put_content_into_clipboard(
        &mut self,
        content: String,
    ) -> ClipboardResult<()>;
    /// # Errors
    ///
    /// Returns an error if the clipboard operation fails.
    fn try_to_get_content_from_clipboard(&mut self) -> ClipboardResult<String>;
}

pub fn copy_to_clipboard(
    buffer: &EditorBuffer,
    clipboard_service_provider: &mut impl ClipboardService,
) {
    let lines = buffer.get_lines();
    let sel_list = buffer.get_selection_list();

    // Initialize an empty string to store the copied text.
    let mut vec_str = InlineVecStr::new();

    // Sort the row indices so that the copied text is in the correct order.
    let row_indices = sel_list.get_ordered_indices();

    // Iterate through the sorted row indices, and copy the selected text.
    for row_index in row_indices {
        if let Some(sel_range) = sel_list.get(row_index)
            && let Some(line_with_info) = lines.get_line(row_index)
        {
            // Use the new zero-copy clip_to_range_str method.
            let sel_text = sel_range.clip_to_range_str(line_with_info);
            vec_str.push(sel_text);
        }
    }

    let result =
        clipboard_service_provider.try_to_put_content_into_clipboard(vec_str.join("\n"));
    if let Err(error) = result {
        DEBUG_TUI_COPY_PASTE.then(|| {
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "📋📋📋 Failed to copy selected text to clipboard",
                error = ?error,
            );
        });
    }
}

#[cfg(test)]
mod tests {

    use crate::{CaretDirection, DEFAULT_SYN_HI_FILE_EXT, EditorBuffer, EditorEvent,
                SelectionAction, assert_eq2,
                clipboard_service::clipboard_test_fixtures::TestClipboard,
                editor::test_fixtures_editor::mock_real_objects_for_editor};

    #[test]
    fn test_copy() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();
        // Buffer has two lines.
        // Row Index : 0 , Column Length : 12
        // Row Index : 1 , Column Length : 12
        buffer.init_with(["abc r3bl xyz", "pqr rust uvw"]);
        let mut test_clipboard = TestClipboard::default();
        // Single Line copying.
        {
            // Current Caret Position : [row : 0, col : 0]
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::End)],
                &mut test_clipboard,
            );
            // Current Caret Position : [row : 0, col : 12]

            // Copying the contents from Selection.
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Copy],
                &mut test_clipboard,
            );
            let content = test_clipboard.content.clone();
            assert_eq2!(content, "abc r3bl xyz".to_string());
        }

        // Multi-line Copying.
        {
            // Current Caret Position : [row : 0, col : 12]
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::PageDown)],
                &mut test_clipboard,
            );
            // Current Caret Position : [row : 1, col : 12]

            // Copying the contents from Selection.
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Copy],
                &mut test_clipboard,
            );

            let content = test_clipboard.content;
            /* cspell:disable-next-line */
            assert_eq2!(content, "abc r3bl xyz\npqr rust uvw".to_string());
        }
    }

    #[test]
    fn test_paste() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Buffer has two lines.
        // Row Index : 0 , Column Length : 12
        // Row Index : 1 , Column Length : 12
        buffer.init_with(["abc r3bl xyz", "pqr rust uvw"]);

        // Single Line Pasting.
        {
            let mut test_clipboard = TestClipboard {
                content: "copied text ".to_string(),
            };

            // Current Caret Position : [row : 0, col : 0]
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right); 4], /* Move caret by 4 positions */
                &mut test_clipboard,
            );

            // Current Caret Position : [row : 0, col : 4]
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Paste],
                &mut test_clipboard,
            );

            let new_lines = vec!["abc copied text r3bl xyz", "pqr rust uvw"];
            assert_eq2!(
                buffer.get_lines().to_gc_string_vec(),
                new_lines.into_iter().map(Into::into).collect::<Vec<_>>()
            );
        }

        // Multi-line Pasting.
        {
            // Current Caret Position : [row : 0, col : 4]
            let mut test_clipboard = TestClipboard {
                content: "old line\nnew line ".to_string(),
            };

            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Paste],
                &mut test_clipboard,
            );

            let new_lines = vec![
                "abc copied text old line",
                "new line r3bl xyz",
                "pqr rust uvw",
            ];
            assert_eq2!(
                buffer.get_lines().to_gc_string_vec(),
                new_lines.into_iter().map(Into::into).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn test_cut() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Buffer has two lines.
        // Row Index : 0 , Column Length : 12
        // Row Index : 1 , Column Length : 12
        buffer.init_with(["abc r3bl xyz", "pqr rust uvw"]);

        // Single Line cutting.
        {
            let mut test_clipboard = TestClipboard::default();

            // Current Caret Position : [row : 0, col : 0]
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::End)],
                &mut test_clipboard,
            );
            // Current Caret Position : [row : 0, col : 12]

            // Cutting the contents from Selection and pasting to clipboard.
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Cut],
                &mut test_clipboard,
            );
            // Current Caret Position : [row : 0, col : 0]

            let content = test_clipboard.content.clone();
            assert_eq2!(content, "abc r3bl xyz".to_string()); // copied to clipboard

            let new_lines = vec![
                "pqr rust uvw", // First line 'abc r3bl xyz' is cut
            ];
            assert_eq2!(
                buffer.get_lines().to_gc_string_vec(),
                new_lines.into_iter().map(Into::into).collect::<Vec<_>>()
            );
        }

        // Multi-line Cutting.
        {
            let mut test_clipboard = TestClipboard::default();

            buffer.init_with(["abc r3bl xyz", "pqr rust uvw"]);
            // Current Caret Position : [row : 0, col : 0]
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Down)],
                &mut test_clipboard,
            );
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right); 4], /* Move caret by 4 positions */
                &mut test_clipboard,
            );
            // Current Caret Position : [row : 1, col : 4]
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::PageUp)], /* Select by pressing PageUp */
                &mut test_clipboard,
            );
            // Current Caret Position : [row : 0, col : 4]

            // Cutting the contents from Selection and pasting to clipboard.
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Cut],
                &mut test_clipboard,
            );

            let content = test_clipboard.content;
            /* cspell:disable-next-line */
            assert_eq2!(content, "r3bl xyz\npqr ".to_string()); // copied to clipboard
            let new_lines = vec!["abc ", "rust uvw"];
            assert_eq2!(
                buffer.get_lines().to_gc_string_vec(),
                new_lines.into_iter().map(Into::into).collect::<Vec<_>>()
            );
        }
    }
}
