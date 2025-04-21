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

use r3bl_tui::{AST, AnsiStyledText, InlineVec, ast_line, ast_lines, fg_slate_gray};

use crate::{giti::ui_str, upgrade_check};

/// This is the instruction header for the multi select list. It is used when the user can
/// select multiple items from the list. The instructions are displayed at the top of the
/// list. This is easily converted into a [r3bl_tui::choose_impl::Header::MultiLine].
///
/// The last line is passed in as a parameter to allow for customization. This is useful
/// when the list is long and the instructions are at the top.
pub fn multi_select_instruction_header(
    last_line: InlineVec<AnsiStyledText>,
) -> InlineVec<InlineVec<AnsiStyledText>> {
    let text_up_and_down = " Up or down:     navigate";
    let text_space = " Space:          select or deselect item";
    let text_esc = " Esc or Ctrl+C:  exit program";
    let text_return_key = " Return:         confirm selection";

    let up_and_down = fg_slate_gray(text_up_and_down).bg_night_blue();
    let space = fg_slate_gray(text_space).bg_night_blue();
    let esc = fg_slate_gray(text_esc).bg_night_blue();
    let return_key = fg_slate_gray(text_return_key).bg_night_blue();

    ast_lines![
        ast_line![up_and_down],
        ast_line![space],
        ast_line![esc],
        ast_line![return_key],
        last_line,
    ]
}

/// This is the instruction header for the single select list. It is used when the user
/// can only select one item from the list. The instructions are displayed at the top of
/// the list. This is easily converted into a [r3bl_tui::choose_impl::Header::MultiLine].
pub fn single_select_instruction_header(
    last_lines: InlineVec<InlineVec<AST>>,
) -> InlineVec<InlineVec<AST>> {
    let text_up_or_down = " Up or down:     navigate";
    let text_esc = " Esc or Ctrl+C:  exit program";
    let text_return_key = " Return:         confirm selection";

    let up_or_down = fg_slate_gray(text_up_or_down).bg_night_blue();
    let esc = fg_slate_gray(text_esc).bg_night_blue();
    let return_key = fg_slate_gray(text_return_key).bg_night_blue();

    let mut acc =
        ast_lines![ast_line![up_or_down], ast_line![esc], ast_line![return_key],];

    last_lines.iter().for_each(|line| acc.push(line.clone()));

    acc
}

pub fn show_exit_message() {
    if upgrade_check::is_update_required() {
        let upgrade_reqd_msg = ui_str::upgrade_required_message();
        println!("{upgrade_reqd_msg}");
    } else {
        let exit_msg = ui_str::goodbye_thanks_for_using_giti();
        println!("{exit_msg}");
    }
}
