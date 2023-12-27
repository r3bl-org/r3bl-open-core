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

use r3bl_ansi_color::{AnsiStyledText, Style};
use r3bl_rs_utils_core::{log_error,
                         CommonError,
                         CommonErrorType,
                         CommonResult,
                         UnicodeString};
use r3bl_tui::{ColorWheel, GradientGenerationPolicy, TextColorizationPolicy};
use r3bl_tuify::{select_from_list, SelectionMode, StyleSheet, LIGHT_GRAY_COLOR};

pub fn multi_select_instruction_header() {
    AnsiStyledText {
        text: &format!(
            "{}{}{}{}",
            "┆ Up or Down:      navigate\n",
            "┆ Space:           select or unselect branches\n",
            "┆ Return:          confirm selection\n",
            "┆ Esc:             exit program\n",
        ),
        style: &[Style::Foreground(LIGHT_GRAY_COLOR)],
    }
    .println();
}

pub fn single_select_instruction_header() {
    AnsiStyledText {
        text: &format!(
            "{}{}{}",
            "┆ Up or Down:      navigate\n",
            "┆ Return:          confirm selection\n",
            "┆ Esc:             exit program\n",
        ),
        style: &[Style::Foreground(LIGHT_GRAY_COLOR)],
    }
    .println();
}

pub fn ask_user_to_select_from_list(
    options: Vec<String>,
    header: String,
    selection_mode: SelectionMode,
) -> Option<Vec<String>> {
    let max_height_row_count = 20;
    let max_width_col_count = 0;
    let style = StyleSheet::default();
    let user_input = select_from_list(
        header,
        options,
        max_height_row_count,
        max_width_col_count,
        selection_mode,
        style,
    );
    user_input
}

pub fn show_exit_message() {
    println!("{}", {
        let goodbye_to_user = match var("USER") {
            Ok(username) => {
                format!("Goodbye, {} 👋 🦜. Thanks for using giti!", username)
            }
            Err(_) => "Thanks for using giti! 👋 🦜".to_owned(),
        };

        let please_star_us = format!(
            "{}\n{}",
            "Please star r3bl-open-core repo on GitHub!",
            "🌟 https://github.com/r3bl-org/r3bl-open-core\n"
        );

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

    let error_msg = format!(
        "Error executing command: '{} {}'. Error: {}",
        program_name_to_string, command_args_to_string, command_output_error
    );

    log_error(error_msg.clone());
    CommonError::new::<T>(CommonErrorType::CommandExecutionError, &error_msg)
}
