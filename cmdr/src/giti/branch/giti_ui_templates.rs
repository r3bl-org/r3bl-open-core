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

use std::{env::var, process::Command};

use r3bl_ansi_color::{AnsiStyledText, Color, Style};
use r3bl_rs_utils_core::{log_error,
                         CommonError,
                         CommonErrorType,
                         CommonResult,
                         UnicodeString};
use r3bl_tui::{ColorWheel, GradientGenerationPolicy, TextColorizationPolicy};
use r3bl_tuify::SLATE_GRAY;

use crate::giti::ui_strings::UIStrings::*;

pub fn multi_select_instruction_header() -> Vec<Vec<AnsiStyledText<'static>>> {
    let up_and_down = AnsiStyledText {
        text: " Up or down:     navigate",
        style: &[
            Style::Foreground(SLATE_GRAY),
            Style::Background(Color::Rgb(14, 17, 23)),
        ],
    };

    let space = AnsiStyledText {
        text: " Space:          select or deselect item",
        style: &[
            Style::Foreground(SLATE_GRAY),
            Style::Background(Color::Rgb(14, 17, 23)),
        ],
    };

    let esc = AnsiStyledText {
        text: " Esc or Ctrl+C:  exit program",
        style: &[
            Style::Foreground(SLATE_GRAY),
            Style::Background(Color::Rgb(14, 17, 23)),
        ],
    };

    let return_key = AnsiStyledText {
        text: " Return:         confirm selection",
        style: &[
            Style::Foreground(SLATE_GRAY),
            Style::Background(Color::Rgb(14, 17, 23)),
        ],
    };

    vec![vec![up_and_down], vec![space], vec![esc], vec![return_key]]
}

pub fn single_select_instruction_header() -> Vec<Vec<AnsiStyledText<'static>>> {
    let up_or_down = AnsiStyledText {
        text: " Up or down:     navigate",
        style: &[
            Style::Foreground(SLATE_GRAY),
            Style::Background(Color::Rgb(14, 17, 23)),
        ],
    };
    let esc = AnsiStyledText {
        text: " Esc or Ctrl+C:  exit program",
        style: &[
            Style::Foreground(SLATE_GRAY),
            Style::Background(Color::Rgb(14, 17, 23)),
        ],
    };

    let return_key = AnsiStyledText {
        text: " Return:         confirm selection",
        style: &[
            Style::Foreground(SLATE_GRAY),
            Style::Background(Color::Rgb(14, 17, 23)),
        ],
    };

    vec![vec![up_or_down], vec![esc], vec![return_key]]
}

pub fn show_exit_message() {
    println!("{}", {
        let goodbye_to_user = match var("USER") {
            Ok(username) => GoodbyeThanksForUsingGitiUsername { username }.to_string(),
            Err(_) => GoodbyeThanksForUsingGiti.to_string(),
        };

        let please_star_us = PleaseStarUs.to_string();
        let plain_text_exit_msg = format!("{goodbye_to_user}\n{please_star_us}");

        let unicode_string = UnicodeString::from(plain_text_exit_msg);
        let mut color_wheel = ColorWheel::default();
        let lolcat_exit_msg = color_wheel.colorize_into_string(
            &unicode_string,
            GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
            TextColorizationPolicy::ColorEachCharacter(None),
        );

        lolcat_exit_msg
    });
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

    log_error(error_msg.clone());
    CommonError::new::<T>(CommonErrorType::CommandExecutionError, &error_msg)
}
