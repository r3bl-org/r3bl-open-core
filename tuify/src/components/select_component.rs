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

use crossterm::{cursor::*, queue, style::*};
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
    fn calculate_viewport_height(&self, state: & State) -> ChUnit {
        if state.items.len() + HEADER_HEIGHT > state.max_display_height.into() {
            state.max_display_height
        } else {
            (state.items.len() + HEADER_HEIGHT).into()
        }
    }

    /// Allocate space and print the lines. The bring the cursor back to the start of the
    /// lines.
    fn render(&mut self, state: & State, shared_global_data: &mut SharedGlobalData) -> Result<()> {
        let initial_inline_row_offset = get_inline_row_index();

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

        let data_row_index_start = *state.scroll_offset_row_index;

        let writer = self.get_write();

        let origin_pos = position!(col_index:0, row_index:0);
        let mut rel_pos = position!(col_index:0, row_index:0);
        // Print header.
        let header_text =
            format!("{}{}", " ".repeat(start_display_col_offset), state.header);
        let header_text = clip_string_to_width_with_ellipsis(header_text, viewport_width);

        let mut ops = render_ops! {
            @new
            RenderOp::MoveCursorPositionRelTo(origin_pos, rel_pos)
        };
        ops += RenderOp::ResetColor;
        ops += RenderOp::ApplyColors(Some(header_style));
        ops += RenderOp::PaintTextWithAttributes(
            UnicodeString::new(&header_text).pad_end_with_spaces_to_fit_width(SPACER, viewport_width),
            Some(header_style)
        );
        rel_pos.add_row(1);

        // Print each line in viewport.
        for viewport_row_index in 0..*viewport_height-1 {
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


            ops += RenderOp::MoveCursorPositionRelTo(origin_pos, rel_pos);
            ops += RenderOp::ResetColor;
            ops += RenderOp::ApplyColors(Some(data_style));
            ops += RenderOp::PaintTextWithAttributes(
                UnicodeString::new(&data_item).pad_end_with_spaces_to_fit_width(SPACER, viewport_width),
                Some(data_style)
            );
            rel_pos.add_row(1);
        }

        paint(ops, &state, FlushKind::JustFlush, shared_global_data );

        // Move the cursor back up.
        // `If` is required since MoveToPreviousLine(0) also results
        // in movement.
        if get_inline_row_index() != initial_inline_row_offset {
            queue! (
                writer,
                MoveToPreviousLine((get_inline_row_index() - initial_inline_row_offset).into()),
            )?;
        }

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
