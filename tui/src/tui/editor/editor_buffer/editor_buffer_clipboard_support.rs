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
    use r3bl_rs_utils_core::{call_if_true, log_debug, ChUnit};

    use crate::*;
    pub fn copy_selection(buffer: &mut EditorBuffer) {
        let text = buffer.get_as_string(); // Get the entire text from the file
        let selection_map = buffer.get_selection_map(); // Get the selection map

        // Initialize an empty string to store the copied text
        let mut copied_text = String::new();

        let map = selection_map.map.clone();
        let mut row_indexes: Vec<u16> = map.keys().map(|k| u16::from(k.value)).collect();
        row_indexes.sort(); // Sort the RowIndex for sequential input.
        let row_indexes: Vec<ChUnit> = row_indexes
            .iter()
            .map(|&x| r3bl_rs_utils_core::ChUnit::from(x))
            .collect();

        for row_idx in row_indexes {
            if let Some(selection_range) = map.get(&row_idx) {
                let start_idx =
                    u16::from(selection_range.start_display_col_index) as usize;
                let end_idx = u16::from(selection_range.end_display_col_index) as usize;
                let row_idx = u16::from(row_idx.value) as usize;

                // Extract the selected text for the current row
                let selected_text = text
                    .lines()
                    .nth(row_idx)
                    .and_then(|line| line.get(start_idx..end_idx))
                    .unwrap_or("");
                copied_text.push_str(selected_text);
                copied_text.push('\n');
            }
        }
        copy_to_clipboard(copied_text);
    }

    fn copy_to_clipboard(text_to_copy: String) {
        // Create a clipboard context.
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        // Copy the text to the clipboard.
        ctx.set_contents(text_to_copy.to_owned()).unwrap();
        call_if_true!(
            DEBUG_TUI_COPY_PASTE,
            log_debug(format!(
                "\n🍕🍕🍕 Selected Text was copied: \n{}",
                text_to_copy
            ))
        );
    }
}
