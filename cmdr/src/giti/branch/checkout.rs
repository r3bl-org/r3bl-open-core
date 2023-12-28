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

use std::process::Command;

use r3bl_ansi_color::{AnsiStyledText, Style};
use r3bl_rs_utils_core::CommonResult;
use r3bl_tuify::{select_from_list_with_multi_line_header, SelectionMode, StyleSheet};

use super::{get_branches, try_get_current_branch};
use crate::{color_constants::DefaultColors::{FrozenBlue,
                                             GuardsRed,
                                             LizardGreen,
                                             MoonlightBlue,
                                             SlateGray},
            giti::{report_unknown_error_and_propagate,
                   single_select_instruction_header,
                   ui_strings::UIStrings::*,
                   TryRunCommandResult}};

pub fn try_checkout_branch() -> CommonResult<TryRunCommandResult> {
    let try_run_command_result = TryRunCommandResult::default();

    let default_header_style = [
        Style::Foreground(FrozenBlue.as_ansi_color()),
        Style::Background(MoonlightBlue.as_ansi_color()),
    ];

    let select_branch_to_switch_to = &SelectBranchToSwitchTo.to_string();

    let instructions_and_branches = {
        let mut instructions_and_branches = single_select_instruction_header();
        let header = AnsiStyledText {
            text: select_branch_to_switch_to,
            style: &default_header_style,
        };
        instructions_and_branches.push(vec![header]);
        instructions_and_branches
    };

    let current_branch = try_get_current_branch()?;

    if let Ok(branches) = get_branches() {
        // Ask user to select a branch to check out to.
        let maybe_selected_branch = select_from_list_with_multi_line_header(
            instructions_and_branches,
            branches,
            Some(20),
            None,
            SelectionMode::Single,
            StyleSheet::default(),
        );

        // If user selected a branch, then check out to it.
        if let Some(selected_branch) = maybe_selected_branch {
            let selected_branch = &selected_branch[0];
            let selected_branch = selected_branch.trim_start_matches("(current) ");
            let command: &mut Command =
                &mut create_git_command_to_checkout_branch(selected_branch);
            let result_output = command.output();

            match result_output {
                Ok(output) => {
                    if output.status.success() {
                        let branch_name = AnsiStyledText {
                            text: selected_branch,
                            style: &[Style::Foreground(LizardGreen.as_ansi_color())],
                        };
                        let slate_gray = SlateGray.as_ansi_color();

                        if selected_branch == current_branch {
                            let already_on_branch = AnsiStyledText {
                                text: &AlreadyOnCurrentBranch.to_string(),
                                style: &[Style::Foreground(slate_gray)],
                            };
                            println!("{already_on_branch}{branch_name}");
                        } else {
                            let switched_to = AnsiStyledText {
                                text: &SwitchedToBranch.to_string(),
                                style: &[Style::Foreground(slate_gray)],
                            };
                            println!("{switched_to}{branch_name}");
                        }
                    } else {
                        try_checkout_branch_error::display_error_message(
                            selected_branch.to_string(),
                            Some(output),
                        );
                    }
                }
                Err(error) => {
                    // Can't even execute output(), something unknown has gone
                    // wrong. Propagate the error.
                    try_checkout_branch_error::display_error_message(
                        selected_branch.to_string(),
                        None,
                    );
                    return report_unknown_error_and_propagate(command, error);
                }
            }
        }
    }
    return Ok(try_run_command_result);
}

fn create_git_command_to_checkout_branch(branch_name: &str) -> Command {
    let mut command = Command::new("git");
    command.args(["checkout", branch_name]);
    command
}

mod try_checkout_branch_error {
    use super::*;

    pub fn display_error_message(
        branch: String,
        maybe_output: Option<std::process::Output>,
    ) {
        let ferrari_red = GuardsRed.as_ansi_color();
        match maybe_output {
            Some(output) => {
                AnsiStyledText {
                    text: &FailedToSwitchToBranch {
                        branch,
                        error_message: String::from_utf8_lossy(&output.stderr)
                            .to_string(),
                    }
                    .to_string(),
                    style: &[Style::Foreground(ferrari_red)],
                }
                .println();
            }
            None => {
                AnsiStyledText {
                    text: &NoBranchGotCheckedOut { branch }.to_string(),
                    style: &[Style::Foreground(ferrari_red)],
                }
                .println();
            }
        }
    }
}
