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

use r3bl_tui::{AST,
               AnsiStyledText,
               InlineVec,
               TuiStyle,
               ast_line,
               ast_lines,
               fg_lavender,
               fg_light_yellow_green,
               fg_sky_blue,
               new_style,
               tui_color};

const FIRST_COLUMN_WIDTH: usize = 20;

/// Helper function to format two strings into columns.
/// The first column has a fixed width defined by `COLUMN_WIDTH`.
fn fmt_two_col(col1: &str, col2: &str) -> String {
    format!("{col1:<FIRST_COLUMN_WIDTH$} {col2}")
}

/// This is the instruction header for the multi select list. It is used when the user can
/// select multiple items from the list. The instructions are displayed at the top of the
/// list. This is easily converted into a [r3bl_tui::choose_impl::Header::MultiLine].
///
/// The last line is passed in as a parameter to allow for customization. This is useful
/// when the list is long and the instructions are at the top.
pub fn prefix_multi_select_instruction_header(
    last_lines: InlineVec<InlineVec<AST>>,
) -> InlineVec<InlineVec<AnsiStyledText>> {
    let text_up_and_down = fmt_two_col("Up or down:", "navigate");
    let text_space = fmt_two_col("Space:", "select or deselect item");
    let text_esc = fmt_two_col("Esc or Ctrl+C:", "exit program");
    let text_return_key = fmt_two_col("Return:", "confirm selection");

    let up_and_down = fg_light_yellow_green(text_up_and_down).bg_night_blue();
    let space = fg_light_yellow_green(text_space).bg_night_blue();
    let esc = fg_lavender(text_esc).bg_night_blue();
    let return_key = fg_sky_blue(text_return_key).bg_night_blue();

    let mut acc = ast_lines![
        ast_line![up_and_down],
        ast_line![space],
        ast_line![esc],
        ast_line![return_key],
    ];

    last_lines.iter().for_each(|line| acc.push(line.clone()));

    acc
}

/// This is the instruction header for the single select list. It is used when the user
/// can only select one item from the list. The instructions are displayed at the top of
/// the list. This is easily converted into a [r3bl_tui::choose_impl::Header::MultiLine].
pub fn prefix_single_select_instruction_header(
    last_lines: InlineVec<InlineVec<AST>>,
) -> InlineVec<InlineVec<AST>> {
    let text_up_or_down = fmt_two_col("Up or down:", "navigate");
    let text_esc = fmt_two_col("Esc or Ctrl+C:", "exit program");
    let text_return_key = fmt_two_col("Return:", "confirm selection");

    let up_or_down = fg_light_yellow_green(text_up_or_down).bg_night_blue();
    let esc = fg_lavender(text_esc).bg_night_blue();
    let return_key = fg_sky_blue(text_return_key).bg_night_blue();

    let mut acc =
        ast_lines![ast_line![up_or_down], ast_line![esc], ast_line![return_key],];

    last_lines.iter().for_each(|line| acc.push(line.clone()));

    acc
}

pub fn header_style_default() -> TuiStyle {
    new_style! (
        color_fg: {tui_color! (frozen_blue)} color_bg: {tui_color! (moonlight_blue)}
    )
}

pub fn header_style_primary() -> TuiStyle {
    new_style!(
        color_fg: {tui_color!(yellow)} color_bg: {tui_color!(moonlight_blue)}
    )
}
