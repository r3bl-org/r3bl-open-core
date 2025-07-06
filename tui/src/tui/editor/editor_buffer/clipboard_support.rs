/*
 *   Copyright (c) 2023-2025 R3BL LLC
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

use std::error::Error;

use super::EditorBuffer;
use crate::{editor_engine::engine_internal_api, md_parser::constants::NEW_LINE, usize,
            EditorArgsMut, InlineVecStr, DEBUG_TUI_COPY_PASTE};

pub type ClipboardResult<T> = Result<T, Box<dyn Error + Send + Sync + 'static>>;

/// Abstraction for the clipboard service for dependency injection. This trait is
/// implemented by both a test clipboard service and a system clipboard service.
pub trait ClipboardService {
    fn try_to_put_content_into_clipboard(
        &mut self,
        content: String,
    ) -> ClipboardResult<()>;
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
        if let Some(sel_range) = sel_list.get(row_index) {
            if let Some(line) = lines.get(usize(*row_index)) {
                let sel_text = sel_range.clip_to_range(line);
                vec_str.push(sel_text);
            }
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

pub fn paste_from_clipboard(
    args: EditorArgsMut<'_>,
    clipboard_service_provider: &mut impl ClipboardService,
) {
    let result = clipboard_service_provider.try_to_get_content_from_clipboard();
    match result {
        Ok(clipboard_text) => {
            // If the clipboard text does not contain a new line, then insert the text.
            if clipboard_text.contains(NEW_LINE) {
                let lines = clipboard_text.split(NEW_LINE);
                let line_count = lines.clone().count();
                for (line_index, line) in lines.enumerate() {
                    engine_internal_api::insert_str_at_caret(
                        EditorArgsMut {
                            engine: args.engine,
                            buffer: args.buffer,
                        },
                        line,
                    );
                    // This is not the last line, so insert a new line.
                    if line_index < line_count - 1 {
                        engine_internal_api::insert_new_line_at_caret(EditorArgsMut {
                            engine: args.engine,
                            buffer: args.buffer,
                        });
                    }
                }
            } else {
                engine_internal_api::insert_str_at_caret(
                    EditorArgsMut {
                        engine: args.engine,
                        buffer: args.buffer,
                    },
                    clipboard_text.as_str(),
                );
            }

            DEBUG_TUI_COPY_PASTE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "📋📋📋 Text was pasted from clipboard",
                    clipboard_text = %clipboard_text
                );
            });
        }

        Err(error) => {
            DEBUG_TUI_COPY_PASTE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "📋📋📋 Failed to paste the text from clipboard",
                    error = ?error
                );
            });
        }
    }
}
