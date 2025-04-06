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

use r3bl_core::{ASTColor,
                AnsiStyledText,
                ColorWheel,
                CommonError,
                CommonErrorType,
                CommonResult,
                GradientGenerationPolicy,
                InlineVec,
                TextColorizationPolicy,
                fg_rgb_color,
                tui_color};
use smallvec::smallvec;

use crate::{giti::ui_strings::UIStrings::{ErrorExecutingCommand,
                                          GoodbyeThanksForUsingGiti,
                                          GoodbyeThanksForUsingGitiUsername,
                                          PleaseStarUs},
            upgrade_check};

pub fn multi_select_instruction_header<'a>() -> InlineVec<InlineVec<AnsiStyledText<'a>>> {
    let slate_gray: ASTColor = tui_color!(slate_grey).into();
    let night_blue: ASTColor = tui_color!(night_blue).into();

    let text_up_and_down = " Up or down:     navigate";
    let text_space = " Space:          select or deselect item";
    let text_esc = " Esc or Ctrl+C:  exit program";
    let text_return_key = " Return:         confirm selection";

    let up_and_down = fg_rgb_color(slate_gray, text_up_and_down).bg_rgb_color(night_blue);
    let space = fg_rgb_color(slate_gray, text_space).bg_rgb_color(night_blue);
    let esc = fg_rgb_color(slate_gray, text_esc).bg_rgb_color(night_blue);
    let return_key = fg_rgb_color(slate_gray, text_return_key).bg_rgb_color(night_blue);

    smallvec![
        smallvec![up_and_down],
        smallvec![space],
        smallvec![esc],
        smallvec![return_key]
    ]
}

pub fn single_select_instruction_header<'a>() -> InlineVec<InlineVec<AnsiStyledText<'a>>>
{
    let slate_gray: ASTColor = tui_color!(slate_grey).into();
    let night_blue: ASTColor = tui_color!(night_blue).into();

    let text_up_or_down = " Up or down:     navigate";
    let text_esc = " Esc or Ctrl+C:  exit program";
    let text_return_key = " Return:         confirm selection";

    let up_or_down = fg_rgb_color(slate_gray, text_up_or_down).bg_rgb_color(night_blue);
    let esc = fg_rgb_color(slate_gray, text_esc).bg_rgb_color(night_blue);
    let return_key = fg_rgb_color(slate_gray, text_return_key).bg_rgb_color(night_blue);

    smallvec![smallvec![up_or_down], smallvec![esc], smallvec![return_key]]
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
    command_output_error: std::io::Error,
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
