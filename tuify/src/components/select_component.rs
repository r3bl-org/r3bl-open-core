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
use r3bl_rs_utils_core::*;

use crate::*;

pub struct SelectComponent<W: Write> {
    pub write: W,
    pub style: StyleSheet,
}

const IS_FOCUSED: &str = " › ";
const IS_NOT_FOCUSED: &str = "   ";
const MULTI_SELECT_IS_SELECTED: &str = "✔";
const MULTI_SELECT_IS_NOT_SELECTED: &str = "☐";
const SINGLE_SELECT_IS_SELECTED: &str = "◉";
const SINGLE_SELECT_IS_NOT_SELECTED: &str = "◌";

impl<W: Write> FunctionComponent<W, State> for SelectComponent<W> {
    fn get_write(&mut self) -> &mut W { &mut self.write }

    /// If there are more items than the max display height, then we only use max display
    /// height. Otherwise we can shrink the display height to the number of items.
    /// This does NOT include the header.
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
        // Setup the required data.
        let normal_style = self.style.normal_style;
        let header_style = self.style.header_style;
        let selected_style = self.style.selected_style;

        let start_display_col_offset = 1;

        // If there are more items than the max display height, then we only use max
        // display height. Otherwise we can shrink the display height to the number of
        // items.
        let viewport_height: ChUnit = self.calculate_viewport_height(state);
        let viewport_width: ChUnit = state.max_display_width;

        call_if_true!(TRACE, {
            log_debug(
                format!(
                    "render()::state: \n\t[raw_caret_row_index: {}, scroll_offset_row_index: {}], \n\tdisplay_height:{}",
                    state.raw_caret_row_index, state.scroll_offset_row_index, viewport_height
                )
                .blue()
                .to_string(),
            );
        });

        self.allocate_viewport_height_space(state)?;

        let data_row_index_start = *state.scroll_offset_row_index;

        let writer = self.get_write();

        // Print header.
        queue! {
            writer,
            // Bring the caret back to the start of line.
            MoveToColumn(0),
            // Reset the colors that may have been set by the previous command.
            ResetColor,
            // Set the colors for the text.
            apply_style!(header_style => fg_color),
            apply_style!(header_style => bg_color),
            // Style the text.
            apply_style!(header_style => bold),
            apply_style!(header_style => italic),
            apply_style!(header_style => dim),
            apply_style!(header_style => underline),
            apply_style!(header_style => reverse),
            apply_style!(header_style => hidden),
            apply_style!(header_style => strikethrough),
            // Clear the current line.
            Clear(ClearType::CurrentLine),
            // Print the text.
            Print(format!("{}{}", " ".repeat(start_display_col_offset), state.header)),
            // Move to next line.
            MoveToNextLine(1),
            // Reset the colors.
            ResetColor,
        }?;

        // Print each line in viewport.
        for viewport_row_index in 0..*viewport_height {
            let data_row_index: usize =
                (data_row_index_start + viewport_row_index).into();
            let caret_row_scroll_adj =
                ch!(viewport_row_index) + state.scroll_offset_row_index;
            let is_focused = ch!(caret_row_scroll_adj) == state.get_focused_index();

            let data_item = &state.items[data_row_index];

            // Invert colors for selected items.
            let is_selected = state.selected_items.contains(data_item);
            let data_style = if is_selected {
                selected_style
            } else {
                normal_style
            };

            let row_prefix = match state.selection_mode {
                SelectionMode::Single => {
                    let padding_left = " ".repeat(start_display_col_offset);
                    if is_focused {
                        format!("{padding_left} {SINGLE_SELECT_IS_SELECTED} ")
                    } else {
                        format!("{padding_left} {SINGLE_SELECT_IS_NOT_SELECTED} ")
                    }
                }
                SelectionMode::Multiple => {
                    let padding_left = " ".repeat(start_display_col_offset);
                    match (is_focused, is_selected) {
                        (true, true) => {
                            format!("{padding_left} {IS_FOCUSED} {MULTI_SELECT_IS_SELECTED} ")
                        }
                        (true, false) => format!(
                            "{padding_left} {IS_FOCUSED} {MULTI_SELECT_IS_NOT_SELECTED} "
                        ),
                        (false, true) => format!(
                            "{padding_left} {IS_NOT_FOCUSED} {MULTI_SELECT_IS_SELECTED} "
                        ),
                        (false, false) => format!(
                            "{padding_left} {IS_NOT_FOCUSED} {MULTI_SELECT_IS_NOT_SELECTED} "
                        ),
                    }
                }
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
                apply_style!(data_style => bg_color),
                apply_style!(data_style => bg_color),
                // Style the text.
                apply_style!(data_style => bold),
                apply_style!(data_style => italic),
                apply_style!(data_style => dim),
                apply_style!(data_style => underline),
                apply_style!(data_style => reverse),
                apply_style!(data_style => hidden),
                apply_style!(data_style => strikethrough),
                // Clear the current line.
                Clear(ClearType::CurrentLine),
                // Print the text.
                Print(data_item.to_string()),
                // Move to next line.
                MoveToNextLine(1),
                // Reset the colors.
                ResetColor,
            }?;
        }

        // Move the cursor back up.
        queue! {
            writer,
            MoveToPreviousLine(*viewport_height + 1),
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

#[cfg(test)]
mod tests {
    use std::io::{Result, Write};

    use pretty_assertions::assert_eq;

    use super::*;

    struct StringWriter {
        buffer: String,
    }

    impl StringWriter {
        fn new() -> Self {
            StringWriter {
                buffer: String::new(),
            }
        }

        fn get_buffer(&self) -> &str { &self.buffer }
    }

    impl Write for StringWriter {
        fn write(&mut self, buf: &[u8]) -> Result<usize> {
            let result = std::str::from_utf8(buf);
            match result {
                Ok(value) => {
                    self.buffer.push_str(value);
                    Ok(buf.len())
                }
                Err(_) => Ok(0),
            }
        }

        fn flush(&mut self) -> Result<()> { Ok(()) }
    }

    #[test]
    fn test_clip_string_to_width_with_ellipsis() {
        let line = "This is a long line that needs to be clipped".to_string();
        let clipped_line =
            clip_string_to_width_with_ellipsis(line.clone(), ChUnit::new(20));
        assert_eq!(clipped_line, "This is a long li...");

        let short_line = "This is a short line".to_string();
        let clipped_short_line =
            clip_string_to_width_with_ellipsis(short_line.clone(), ChUnit::new(20));
        assert_eq!(clipped_short_line, "This is a short line");
    }

    #[test]
    fn test_select_component() {
        let mut state = State {
            items: vec![
                "Item 1".to_string(),
                "Item 2".to_string(),
                "Item 3".to_string(),
            ],
            ..State::default()
        };

        state.scroll_offset_row_index = ch!(0);

        let mut writer = StringWriter::new();

        let mut component = SelectComponent {
            write: &mut writer,
            style: StyleSheet::default(),
        };

        component.render(&mut state).unwrap();

        let expected_output = "\u{1b}[1F\u{1b}[1G\u{1b}[0m\u{1b}[38;2;50;50;50m\u{1b}[48;2;150;150;150m\u{1b}[1m\u{1b}[23m\u{1b}[22m\u{1b}[24m\u{1b}[27m\u{1b}[28m\u{1b}[29m\u{1b}[2K \u{1b}[1E\u{1b}[0m\u{1b}[1F";
        assert_eq!(writer.get_buffer(), expected_output);
    }
}
