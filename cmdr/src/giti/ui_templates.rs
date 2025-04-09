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

use std::{env::var, process::Command};

use r3bl_core::{AnsiStyledText,
                ColorWheel,
                CommonError,
                CommonErrorType,
                CommonResult,
                GradientGenerationPolicy,
                InlineVec,
                TextColorizationPolicy,
                ast_line,
                ast_lines,
                fg_slate_gray};

use crate::{giti::ui_strings::UIStrings::{ErrorExecutingCommand,
                                          GoodbyeThanksForUsingGiti,
                                          GoodbyeThanksForUsingGitiUsername,
                                          PleaseStarUs},
            upgrade_check};

/// This is the instruction header for the multi select list. It is used when the user can
/// select multiple items from the list. The instructions are displayed at the top of the
/// list. This is easily converted into a [r3bl_tui::choose_impl::Header::MultiLine].
///
/// The last line is passed in as a parameter to allow for customization. This is useful
/// when the list is long and the instructions are at the top.
pub fn multi_select_instruction_header<'a>(
    last_line: InlineVec<AnsiStyledText<'a>>,
) -> InlineVec<InlineVec<AnsiStyledText<'a>>> {
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
pub fn single_select_instruction_header<'a>(
    last_line: InlineVec<AnsiStyledText<'a>>,
) -> InlineVec<InlineVec<AnsiStyledText<'a>>> {
    let text_up_or_down = " Up or down:     navigate";
    let text_esc = " Esc or Ctrl+C:  exit program";
    let text_return_key = " Return:         confirm selection";

    let up_or_down = fg_slate_gray(text_up_or_down).bg_night_blue();
    let esc = fg_slate_gray(text_esc).bg_night_blue();
    let return_key = fg_slate_gray(text_return_key).bg_night_blue();

    ast_lines![
        ast_line![up_or_down],
        ast_line![esc],
        ast_line![return_key],
        last_line,
    ]
}

pub fn show_exit_message() {
    if upgrade_check::is_update_required() {
        println!("{}", {
            let plain_text_exit_msg = format!(
                "{}\n{}",
                "💿 A new version of giti is available.",
                "Run `cargo install r3bl-cmdr` to upgrade 🙌."
            );

            ColorWheel::default().colorize_into_string(
                &plain_text_exit_msg,
                GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                TextColorizationPolicy::ColorEachCharacter(None),
                None,
            )
        });
    } else {
        println!("{}", {
            let goodbye_to_user = match var("USER") {
                Ok(username) => {
                    GoodbyeThanksForUsingGitiUsername { username }.to_string()
                }
                Err(_) => GoodbyeThanksForUsingGiti.to_string(),
            };

            let please_star_us = PleaseStarUs.to_string();
            let plain_text_exit_msg = format!("{goodbye_to_user}\n{please_star_us}");

            ColorWheel::lolcat_into_string(&plain_text_exit_msg, None)
        });
    }
}

/// Call this function when you can't even execute [Command::output] and something unknown
/// has gone wrong. Propagate the error to the caller since it is not recoverable and can't
/// be handled.
pub fn report_unknown_error_and_propagate<T>(
    command: &mut Command,
    command_output_error: miette::Report,
) -> CommonResult<T> {
    let program_name_to_string: String =
        command.get_program().to_string_lossy().to_string();

    let command_args_to_string: String = {
        let mut it = vec![];
        for item in command.get_args() {
            it.push(item.to_string_lossy().to_string());
        }
        it.join(" ")
    };

    let error_msg = ErrorExecutingCommand {
        program_name_to_string,
        command_args_to_string,
        command_output_error,
    }
    .to_string();

    // % is Display, ? is Debug.
    tracing::error!(message = error_msg);
    CommonError::new_error_result::<T>(CommonErrorType::CommandExecutionError, &error_msg)
}
