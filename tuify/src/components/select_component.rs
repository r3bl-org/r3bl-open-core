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

use std::io::{Result, Write};

use crossterm::{cursor::{MoveToColumn, MoveToNextLine, MoveToPreviousLine},
                queue,
                style::{Attribute,
                        Print,
                        ResetColor,
                        SetBackgroundColor,
                        SetForegroundColor,
                        Stylize},
                terminal::{Clear, ClearType}};
use r3bl_ansi_color::AnsiStyledText;
use r3bl_rs_utils_core::{call_if_true, ch, log_debug, throws, ChUnit, UnicodeString};

use crate::{apply_style,
            get_crossterm_color_based_on_terminal_capabilities,
            get_terminal_width,
            set_attribute,
            FunctionComponent,
            Header,
            SelectionMode,
            State,
            StyleSheet,
            DEVELOPMENT_MODE};

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

impl<W: Write> FunctionComponent<W, State<'_>> for SelectComponent<W> {
    fn get_write(&mut self) -> &mut W { &mut self.write }

    // Header can be either a single line or a multi line.
    fn calculate_header_viewport_height(&self, state: &mut State<'_>) -> ChUnit {
        match state.get_header() {
            Header::Single => ch!(1),
            Header::Multiple => ch!(state.multi_line_header.len()),
        }
    }

    /// If there are more items than the max display height, then we only use max display
    /// height. Otherwise we can shrink the display height to the number of items.
    /// This does NOT include the header.
    fn calculate_items_viewport_height(&self, state: &mut State<'_>) -> ChUnit {
        if state.items.len() > ch!(@to_usize state.max_display_height) {
            state.max_display_height
        } else {
            ch!(state.items.len())
        }
    }

    /// Allocate space and print the lines. The bring the cursor back to the start of the
    /// lines.
    fn render(&mut self, state: &mut State<'_>) -> Result<()> {
        throws!({
            // Setup the required data.
            let focused_and_selected_style = self.style.focused_and_selected_style;
            let focused_style = self.style.focused_style;
            let unselected_style = self.style.unselected_style;
            let selected_style = self.style.selected_style;
            let single_line_header_style = self.style.header_style;
            let start_display_col_offset = 1;
            let header_viewport_height: ChUnit =
                self.calculate_header_viewport_height(state);

            // If there are more items than the max display height, then we only use max
            // display height. Otherwise we can shrink the display height to the number of
            // items.
            let items_viewport_height: ChUnit =
                self.calculate_items_viewport_height(state);

            let viewport_width: ChUnit = {
                // Try to get the terminal width from state first (since it should be set
                // when resize events occur). If that is not set, then get the terminal
                // width directly.
                let terminal_width = match state.window_size {
                    Some(size) => size.col_count,
                    None => ch!(get_terminal_width()),
                };

                // Do not exceed the max display width (if it is set).
                if state.max_display_width == ch!(0)
                    || state.max_display_width > ch!(terminal_width)
                {
                    ch!(terminal_width)
                } else {
                    state.max_display_width
                }
            };

            call_if_true!(DEVELOPMENT_MODE, {
                log_debug(
                      format!(
                          "🍎🍎🍎\n render()::state: \n\t[raw_caret_row_index: {}, scroll_offset_row_index: {}], \n\theader_viewport_height: {}, items_viewport_height:{}, viewport_width:{}",
                          state.raw_caret_row_index, state.scroll_offset_row_index, header_viewport_height, items_viewport_height, viewport_width
                      )
                      .blue()
                      .to_string(),
                  );
            });

            self.allocate_viewport_height_space(state)?;

            let data_row_index_start = *state.scroll_offset_row_index;

            let writer = self.get_write();

            match state.get_header() {
                Header::Single => {
                    let mut header_text = format!(
                        "{}{}",
                        " ".repeat(start_display_col_offset),
                        state.header
                    );

                    header_text =
                        clip_string_to_width_with_ellipsis(header_text, viewport_width);

                    queue! {
                        writer,
                        // Bring the caret back to the start of line.
                        MoveToColumn(0),
                        // Reset the colors that may have been set by the previous command.
                        ResetColor,
                        // Set the colors for the text.
                        apply_style!(single_line_header_style => fg_color),
                        apply_style!(single_line_header_style => bg_color),
                        // Style the text.
                        apply_style!(single_line_header_style => bold),
                        apply_style!(single_line_header_style => italic),
                        apply_style!(single_line_header_style => dim),
                        apply_style!(single_line_header_style => underline),
                        apply_style!(single_line_header_style => reverse),
                        apply_style!(single_line_header_style => hidden),
                        apply_style!(single_line_header_style => strikethrough),
                        // Clear the current line.
                        Clear(ClearType::CurrentLine),
                        // Print the text.
                        Print(header_text),
                        // Move to next line.
                        MoveToNextLine(1),
                        // Reset the colors.
                        ResetColor,
                    }?;
                }
                Header::Multiple => {
                    // Subtract 3 from viewport width because we need to add "..." to the
                    // end of the line.
                    let mut available_space_col_count: ChUnit = viewport_width - 3;
                    // This is the vector of vectors of AnsiStyledText we want to print to
                    // the screen.
                    let mut multi_line_header_clipped_vec: Vec<Vec<AnsiStyledText<'_>>> =
                        Vec::new();
                    let mut maybe_clipped_text_vec: Vec<Vec<String>> = Vec::new();

                    for header_line in state.multi_line_header.iter() {
                        let mut header_line_modified = vec![];

                        'inner: for last_span in header_line.iter() {
                            let span_text = last_span.text;
                            let span_as_unicode_string = UnicodeString::from(span_text);
                            let unicode_string_width =
                                span_as_unicode_string.display_width;

                            if unicode_string_width > available_space_col_count {
                                // Clip the text to available space.
                                let clipped_text = span_as_unicode_string
                                    .clip_to_width(ch!(0), available_space_col_count);
                                let clipped_text = format!("{clipped_text}...");
                                header_line_modified.push(clipped_text.to_owned());
                                break 'inner;
                            } else {
                                available_space_col_count -= unicode_string_width;

                                // If last item in the header, then fill the remaining
                                // space with spaces.
                                let maybe_header_line_last_span: Option<
                                    &AnsiStyledText<'_>,
                                > = header_line.last();

                                if let Some(header_line_last_span) =
                                    maybe_header_line_last_span
                                {
                                    if last_span == header_line_last_span {
                                        // Because text is not clipped, we add back the 3 we subtracted
                                        // earlier for the "...".
                                        let num_of_spaces: ChUnit =
                                            available_space_col_count + ch!(3);
                                        let span_with_spaces = span_text.to_owned()
                                            + &" ".repeat(num_of_spaces.into());
                                        header_line_modified.push(span_with_spaces);
                                    } else {
                                        header_line_modified.push(span_text.to_owned());
                                    }
                                }
                            };
                        }

                        // Reset the available space.
                        available_space_col_count = viewport_width - 3;
                        maybe_clipped_text_vec.push(header_line_modified);
                    }

                    // Replace the text inside vector of vectors of AnsiStyledText with
                    // the clipped text.
                    let zipped = maybe_clipped_text_vec
                        .iter()
                        .zip(state.multi_line_header.iter());
                    zipped.for_each(|(clipped_text_vec, header_span_vec)| {
                        let mut ansi_styled_text_vec: Vec<AnsiStyledText<'_>> =
                            Vec::new();
                        let zipped = clipped_text_vec.iter().zip(header_span_vec.iter());
                        zipped.for_each(|(clipped_text, header_span)| {
                            ansi_styled_text_vec.push(AnsiStyledText {
                                text: clipped_text,
                                style: header_span.style,
                            });
                        });
                        multi_line_header_clipped_vec.push(ansi_styled_text_vec);
                    });

                    let multi_line_header_text = multi_line_header_clipped_vec
                        .iter()
                        .map(|header_line| {
                            header_line
                                .iter()
                                .map(|header_span| header_span.to_string())
                                .collect::<Vec<String>>()
                                .join("")
                        })
                        .collect::<Vec<String>>()
                        .join("\r\n");

                    queue! {
                        writer,
                        // Bring the caret back to the start of line.
                        MoveToColumn(0),
                        // Reset the colors that may have been set by the previous command.
                        ResetColor,
                        // Clear the current line.
                        Clear(ClearType::CurrentLine),
                        // Print each AnsiStyledText.
                        Print(multi_line_header_text),
                        // Move to next line.
                        MoveToNextLine(1),
                        // Reset the colors.
                        ResetColor,
                    }?;
                }
            }

            // Print each line in viewport.
            for viewport_row_index in 0..*items_viewport_height {
                let data_row_index: usize =
                    (data_row_index_start + viewport_row_index).into();
                let caret_row_scroll_adj =
                    ch!(viewport_row_index) + state.scroll_offset_row_index;
                let data_item = &state.items[data_row_index];

                // Invert colors for selected items.
                enum SelectionStateStyle {
                    FocusedAndSelected,
                    Focused,
                    Selected,
                    Unselected,
                }

                let is_selected = state.selected_items.contains(data_item);
                let is_focused = ch!(caret_row_scroll_adj) == state.get_focused_index();

                let selection_state = match (is_focused, is_selected) {
                    (true, true) => SelectionStateStyle::FocusedAndSelected,
                    (true, false) => SelectionStateStyle::Focused,
                    (false, true) => SelectionStateStyle::Selected,
                    (false, false) => SelectionStateStyle::Unselected,
                };

                let data_style = match selection_state {
                    SelectionStateStyle::FocusedAndSelected => focused_and_selected_style,
                    SelectionStateStyle::Focused => focused_style,
                    SelectionStateStyle::Selected => selected_style,
                    SelectionStateStyle::Unselected => unselected_style,
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
                let data_item: String =
                    clip_string_to_width_with_ellipsis(data_item, viewport_width);
                let data_item_display_width: ChUnit =
                    UnicodeString::from(&data_item).display_width;
                let padding_right = if data_item_display_width < viewport_width {
                    " ".repeat(ch!(@to_usize (viewport_width - data_item_display_width)))
                } else {
                    "".to_string()
                };

                queue! {
                    writer,
                    // Bring the caret back to the start of line.
                    MoveToColumn(0),
                    // Reset the colors that may have been set by the previous command.
                    ResetColor,
                    // Clear the current line.
                    Clear(ClearType::CurrentLine),
                    // Set the colors for the text.
                    apply_style!(data_style => fg_color),
                    apply_style!(data_style => bg_color),
                    // Style the text.
                    apply_style!(data_style => bold),
                    apply_style!(data_style => italic),
                    apply_style!(data_style => dim),
                    apply_style!(data_style => underline),
                    apply_style!(data_style => reverse),
                    apply_style!(data_style => hidden),
                    apply_style!(data_style => strikethrough),
                    // Print the text.
                    Print(data_item),
                    // Print the padding text.
                    Print(padding_right),
                    // Move to next line.
                    MoveToNextLine(1),
                    // Reset the colors.
                    ResetColor,
                }?;
            }

            // Move the cursor back up.
            queue! {
                writer,
                MoveToPreviousLine(*items_viewport_height + *header_viewport_height),
            }?;

            writer.flush()?;
        });
    }
}

pub fn clip_string_to_width_with_ellipsis(
    mut header_text: String,
    viewport_width: ChUnit,
) -> String {
    let unicode_string = UnicodeString::from(header_text);
    let unicode_string_width = unicode_string.display_width;
    let available_space_col_count: ChUnit = viewport_width;
    if unicode_string_width > available_space_col_count {
        // Clip the text to available space.
        let clipped_text =
            unicode_string.clip_to_width(ch!(0), available_space_col_count - 3);
        let clipped_text = format!("{clipped_text}...");
        header_text = clipped_text;
    } else {
        header_text = unicode_string.string;
    }
    header_text
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use r3bl_ansi_color::global_color_support::{clear_override, set_override};
    use serial_test::serial;

    use super::*;
    use crate::TestStringWriter;

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

    #[serial]
    #[test]
    fn test_select_component() {
        let mut state = State {
            header: "Header".to_string(),
            items: vec![
                "Item 1".to_string(),
                "Item 2".to_string(),
                "Item 3".to_string(),
            ],
            max_display_height: ch!(5),
            max_display_width: ch!(40),
            raw_caret_row_index: ch!(0),
            scroll_offset_row_index: ch!(0),
            selected_items: vec![],
            selection_mode: SelectionMode::Single,
            ..Default::default()
        };

        state.scroll_offset_row_index = ch!(0);

        let mut writer = TestStringWriter::new();

        let mut component = SelectComponent {
            write: &mut writer,
            style: StyleSheet::default(),
        };

        set_override(r3bl_ansi_color::ColorSupport::Ansi256);
        component.render(&mut state).unwrap();

        let generated_output = writer.get_buffer().to_string();

        println!(
            "generated_output = writer.get_buffer(): \n\n{:#?}\n\n",
            generated_output
        );

        let expected_output = "\u{1b}[4F\u{1b}[1G\u{1b}[0m\u{1b}[38;5;153m\u{1b}[48;5;235m\u{1b}[21m\u{1b}[23m\u{1b}[22m\u{1b}[24m\u{1b}[27m\u{1b}[28m\u{1b}[29m\u{1b}[2K Header\u{1b}[1E\u{1b}[0m\u{1b}[1G\u{1b}[0m\u{1b}[2K\u{1b}[38;5;46m\u{1b}[48;5;233m\u{1b}[21m\u{1b}[23m\u{1b}[22m\u{1b}[24m\u{1b}[27m\u{1b}[28m\u{1b}[29m  ◉ Item 1                              \u{1b}[1E\u{1b}[0m\u{1b}[1G\u{1b}[0m\u{1b}[2K\u{1b}[38;5;250m\u{1b}[48;5;233m\u{1b}[21m\u{1b}[23m\u{1b}[22m\u{1b}[24m\u{1b}[27m\u{1b}[28m\u{1b}[29m  ◌ Item 2                              \u{1b}[1E\u{1b}[0m\u{1b}[1G\u{1b}[0m\u{1b}[2K\u{1b}[38;5;250m\u{1b}[48;5;233m\u{1b}[21m\u{1b}[23m\u{1b}[22m\u{1b}[24m\u{1b}[27m\u{1b}[28m\u{1b}[29m  ◌ Item 3                              \u{1b}[1E\u{1b}[0m\u{1b}[4F";
        assert_eq!(generated_output, expected_output);

        clear_override();
    }
}
