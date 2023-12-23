/*
 *   Copyright (c) 2023 R3BL LLC
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

use crossterm::style::Stylize;
use r3bl_rs_utils_core::{call_if_true, ch, log_debug, UnicodeString};

use super::*;
use crate::*;

type ClipboardResult<T> = Result<T, Box<dyn Error + Send + Sync + 'static>>;

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
    let lines: &Vec<UnicodeString> = buffer.get_lines();
    let selection_map = buffer.get_selection_map();

    // Initialize an empty string to store the copied text.
    let mut vec_str: Vec<&str> = vec![];

    // Sort the row indices so that the copied text is in the correct order.
    let row_indices = selection_map.get_ordered_indices();

    // Iterate through the sorted row indices, and copy the selected text.
    for row_index in row_indices {
        if let Some(selection_range) = selection_map.map.get(&row_index) {
            if let Some(line) = lines.get(ch!(@to_usize row_index)) {
                let selected_text = line.clip_to_range(*selection_range);
                vec_str.push(selected_text);
            }
        }
    }

    let result =
        clipboard_service_provider.try_to_put_content_into_clipboard(vec_str.join("\n"));
    if let Err(error) = result {
        call_if_true!(DEBUG_TUI_COPY_PASTE, {
            log_debug(
                format!(
                    "\n📋📋📋 Failed to copy selected text to clipboard: {0}",
                    /* 0 */
                    format!("{error}").white(),
                )
                .on_dark_red()
                .to_string(),
            )
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
            if !clipboard_text.contains(&"\n") {
                EditorEngineInternalApi::insert_str_at_caret(
                    EditorArgsMut {
                        editor_engine: args.editor_engine,
                        editor_buffer: args.editor_buffer,
                    },
                    clipboard_text.as_str(),
                );
            }
            // If the clipboard text contains a new line, then insert the text line by line.
            else {
                let lines = clipboard_text.split("\n");
                let line_count = lines.clone().count();
                for (line_index, line) in lines.enumerate() {
                    EditorEngineInternalApi::insert_str_at_caret(
                        EditorArgsMut {
                            editor_engine: args.editor_engine,
                            editor_buffer: args.editor_buffer,
                        },
                        line,
                    );
                    // This is not the last line, so insert a new line.
                    if line_index < line_count - 1 {
                        EditorEngineInternalApi::insert_new_line_at_caret(
                            EditorArgsMut {
                                editor_engine: args.editor_engine,
                                editor_buffer: args.editor_buffer,
                            },
                        );
                    }
                }
            }

            call_if_true!(DEBUG_TUI_COPY_PASTE, {
                log_debug(
                    format!(
                        "\n📋📋📋 Text was pasted from clipboard: \n{0}",
                        /* 0 */
                        clipboard_text.clone().dark_red()
                    )
                    .black()
                    .on_green()
                    .to_string(),
                )
            });
        }

        Err(error) => {
            call_if_true!(DEBUG_TUI_COPY_PASTE, {
                log_debug(
                    format!(
                        "\n📋📋📋 Failed to paste the text from clipboard: {0}",
                        /* 0 */
                        format!("{error}").white(),
                    )
                    .on_dark_red()
                    .to_string(),
                )
            });
        }
    }
}

pub mod test_clipboard_service_provider {
    use super::{ClipboardResult, ClipboardService};

    #[derive(Debug, Default)]
    pub struct TestClipboard {
        pub content: String,
    }

    impl ClipboardService for TestClipboard {
        fn try_to_put_content_into_clipboard(
            &mut self,
            content: String,
        ) -> ClipboardResult<()> {
            self.content = content;
            Ok(())
        }

        fn try_to_get_content_from_clipboard(&mut self) -> ClipboardResult<String> {
            Ok(self.content.clone())
        }
    }
}

pub mod system_clipboard_service_provider {
    use copypasta_ext::{copypasta::ClipboardProvider, x11_fork::ClipboardContext};
    use crossterm::style::Stylize;
    use r3bl_rs_utils_core::{call_if_true, log_debug, throws};

    use super::{ClipboardResult, ClipboardService};
    use crate::DEBUG_TUI_COPY_PASTE;

    pub struct SystemClipboard;

    impl ClipboardService for SystemClipboard {
        fn try_to_put_content_into_clipboard(
            &mut self,
            content: String,
        ) -> ClipboardResult<()> {
            throws!({
                let mut ctx = ClipboardContext::new()?;
                ctx.set_contents(content.clone())?;

                call_if_true!(DEBUG_TUI_COPY_PASTE, {
                    log_debug(
                        format!(
                            "\n📋📋📋 Selected Text was copied to clipboard: \n{0}",
                            /* 0 */
                            content.dark_red()
                        )
                        .black()
                        .on_green()
                        .to_string(),
                    )
                });
            })
        }

        fn try_to_get_content_from_clipboard(&mut self) -> ClipboardResult<String> {
            let mut ctx = ClipboardContext::new()?;
            let content = ctx.get_contents()?;

            Ok(content)
        }
    }
}
