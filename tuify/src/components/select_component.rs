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

use std::io::{Result, *};

use crossterm::{cursor::*, queue, style::*, terminal::*};
use r3bl_ansi_color::TransformColor;
use r3bl_rs_utils_core::*;

use crate::*;

pub struct SelectComponent<W: Write> {
    pub write: W,
}

impl<W: Write> FunctionComponent<W, State> for SelectComponent<W> {
    fn get_write(&mut self) -> &mut W { &mut self.write }

    /// If there are more items than the max display height, then we only use max display
    /// height. Otherwise we can shrink the display height to the number of items.
    fn calculate_viewport_height(&self, state: &mut State) -> ChUnit {
        if state.items.len() > state.max_display_height.into() {
            state.max_display_height
        } else {
            state.items.len().into()
        }
    }

    /// Allocate space and print the lines. The bring the cursor back to the start of the
    /// lines.
    fn render(&mut self, state: &mut State) -> Result<()> {
        // TODO: use styles for selected and unselected
        // Setup the required data.
        let fg_color = r3bl_ansi_color::Color::Rgb(200, 200, 1).as_ansi256().index;
        let bg_color = r3bl_ansi_color::Color::Rgb(100, 60, 150).as_ansi256().index;

        let start_display_col_offset = 1;

        // If there are more items than the max display height, then we only use max
        // display height. Otherwise we can shrink the display height to the number of
        // items.
        let viewport_height: ChUnit = self.calculate_viewport_height(state);
        let viewport_width: ChUnit = state.max_display_width;

        call_if_true!(TRACE, {
            log_debug(
                format!(
                    "render()::state: {:?}, display_height:{}",
                    state.raw_caret_row_index, viewport_height
                )
                .blue()
                .to_string(),
            );
        });

        self.allocate_viewport_height_space(state)?;

        let data_row_index_start = *state.scroll_offset_row_index;

        let writer = self.get_write();

        // 00: figure out how to print header

        // Print each line in viewport.
        for viewport_row_index in 0..*viewport_height {
            let data_row_index: usize =
                (data_row_index_start + viewport_row_index).into();
            let caret_row_scroll_adj =
                ch!(viewport_row_index) + state.scroll_offset_row_index;
            let is_selected = ch!(caret_row_scroll_adj) == state.get_selected_index();

            let row_prefix = {
                let padding_left = " ".repeat(start_display_col_offset);
                let index = caret_row_scroll_adj + 1;
                if is_selected {
                    format!("{padding_left} > {index} ")
                } else {
                    format!("{padding_left}   {index} ")
                }
            };

            let data_item = &state.items[data_row_index];
            // Invert colors for selected items.
            let (fg_color, bg_color) = if state.selected_items.contains(data_item) {
                (bg_color, fg_color)
            } else {
                (fg_color, bg_color)
            };

            let data_item = format!("{row_prefix}{data_item}");
            let data_item = clip_string_to_width_with_ellipsis(data_item, viewport_width);

            queue! {
                writer,
                // Bring the caret back to the start of line.
                MoveToColumn(0),
                // Reset the colors that may have been set by the previous command.
                ResetColor,
                // Set the colors for the text.
                SetForegroundColor(Color::AnsiValue(fg_color)),
                SetBackgroundColor(Color::AnsiValue(bg_color)),
                // Clear the current line.
                Clear(ClearType::CurrentLine),
                // Print the text.
                Print(format!("{data_item}")),
                // Move to next line.
                MoveToNextLine(1),
                // Reset the colors.
                ResetColor,
            }?;
        }

        // Move the cursor back up.
        queue! {
            writer,
            MoveToPreviousLine(*viewport_height),
        }?;

        writer.flush()?;

        Ok(())
    }
}

fn clip_string_to_width_with_ellipsis(line: String, viewport_width: ChUnit) -> String {
    let width = viewport_width.into();
    if line.len() > width {
        format!("{}...", &line[..width - 3])
    } else {
        line.to_string()
    }
}
