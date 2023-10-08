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

pub mod copy_to_clipboard {
    use clipboard::{self, ClipboardContext, ClipboardProvider};
    use crossterm::style::Stylize;
    use r3bl_rs_utils_core::{call_if_true,
                             ch,
                             log_debug,
                             ChUnit,
                             CommonResult,
                             UnicodeString};

    use crate::*;
    pub fn copy_selection(buffer: &EditorBuffer) {
        let lines: &Vec<UnicodeString> = buffer.get_lines();
        let selection_map = buffer.get_selection_map();

        // Initialize an empty string to store the copied text.
        let mut vec_str: Vec<&str> = vec![];

        // Sort the row indices so that the copied text is in the correct order.
        let row_indices = {
            let mut key_vec: Vec<ChUnit> = selection_map.map.keys().copied().collect();
            key_vec.sort();
            key_vec
        };

        // Iterate through the sorted row indices, and copy the selected text.
        for row_index in row_indices {
            if let Some(selection_range) = selection_map.map.get(&row_index) {
                if let Some(line) = lines.get(ch!(@to_usize row_index)) {
                    let selected_text = line.clip_to_range(*selection_range);
                    vec_str.push(selected_text);
                }
            }
        }

        if let Err(error) = try_copy_to_clipboard(&vec_str) {
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

    /// Wrap the call to the clipboard crate, so it returns a [CommonResult]. This is due
    /// to the requirement to call `unwrap()` on the [ClipboardContext] object.
    fn try_copy_to_clipboard(vec_str: &[&str]) -> CommonResult<()> {
        let mut context: ClipboardContext = ClipboardProvider::new().unwrap();
        let content = vec_str.join("\n");
        context.set_contents(content.clone()).unwrap();

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

        Ok(())
    }
}
