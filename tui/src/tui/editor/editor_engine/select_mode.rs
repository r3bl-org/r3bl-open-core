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
use std::cmp::Ordering;

use r3bl_core::CaretScrAdj;

use crate::{handle_selection_multiline_caret_movement,
            handle_selection_multiline_caret_movement_hit_top_or_bottom_of_document,
            handle_selection_single_line_caret_movement,
            EditorBuffer};

#[derive(Clone, Copy, Debug)]
pub enum SelectMode {
    Enabled,
    Disabled,
}

impl SelectMode {
    // BUG: [ ] introduce scroll adjusted type
    pub fn get_caret_display_position_scroll_adjusted(
        &self,
        buffer: &EditorBuffer,
    ) -> Option<CaretScrAdj> {
        match self {
            SelectMode::Enabled => Some(buffer.get_caret_scr_adj()),
            _ => None,
        }
    }

    /// Manage the selection based on the movement of the caret:
    /// - <kbd>Shift + Left</kbd>
    /// - <kbd>Shift + Right</kbd>
    /// - <kbd>Shift + Home</kbd>
    /// - <kbd>Shift + End</kbd>
    ///
    /// # Arguments
    /// - `editor_buffer` - The buffer to update. This holds the selection map.
    /// - `maybe_previous_caret_display_pos` - The previous position of the caret.
    ///    - This maybe [None] if [SelectMode] is [SelectMode::Disabled].
    ///    - This has to be [Some] if [SelectMode::Enabled].
    /// - `maybe_current_caret_display_pos` - The current position of the caret.
    ///    - This maybe [None] if [SelectMode] is [SelectMode::Disabled].
    ///    - This has to be [Some] if [SelectMode::Enabled].
    pub fn handle_selection_single_line_caret_movement(
        &self,
        editor_buffer: &mut EditorBuffer,
        maybe_previous_caret_display_position: Option<CaretScrAdj>,
        maybe_current_caret_display_position: Option<CaretScrAdj>,
    ) -> Option<()> {
        match self {
            // Cancel the selection. We don't care about the caret positions (they maybe
            // None or not).
            SelectMode::Disabled => editor_buffer.clear_selection(),
            // Create or update the selection w/ the caret positions (which can't be
            // None).
            SelectMode::Enabled => {
                let previous = maybe_previous_caret_display_position?;
                let current = maybe_current_caret_display_position?;

                if previous.row_index != current.row_index {
                    return None;
                }

                handle_selection_single_line_caret_movement(
                    editor_buffer,
                    previous.row_index, // Same as `current.row_index`.
                    previous.col_index,
                    current.col_index,
                )
            }
        };

        None
    }

    pub fn update_selection_based_on_caret_movement_in_multiple_lines(
        &self,
        buffer: &mut EditorBuffer,
        maybe_prev_caret_scr_adj: Option<CaretScrAdj>,
        maybe_curr_caret_scr_adj: Option<CaretScrAdj>,
    ) -> Option<()> {
        match self {
            // Cancel the selection. We don't care about the caret positions (they maybe
            // None or not).
            SelectMode::Disabled => buffer.clear_selection(),
            // Create or update the selection w/ the caret positions (which can't be
            // None).
            SelectMode::Enabled => {
                let prev = maybe_prev_caret_scr_adj?;
                let curr = maybe_curr_caret_scr_adj?;

                match prev.row_index.cmp(&curr.row_index) {
                     Ordering::Equal => handle_selection_multiline_caret_movement_hit_top_or_bottom_of_document(
                         buffer,
                         prev,
                         curr,
                     ),
                     _ => handle_selection_multiline_caret_movement(
                         buffer,
                         prev,
                         curr,
                     ),
                 }
            }
        };

        None
    }
}

#[derive(Clone, PartialEq, Copy)]
pub enum DeleteSelectionWith {
    Backspace,
    Delete,
    AnyKey,
}
