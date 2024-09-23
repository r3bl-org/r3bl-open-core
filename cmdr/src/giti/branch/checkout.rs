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

use branch_checkout_formatting::{add_spaces_to_end_of_string,
                                 display_correct_message_after_user_tried_to_checkout,
                                 get_formatted_modified_files};
use r3bl_ansi_color::{AnsiStyledText, Style};
use r3bl_rs_utils_core::{ch, CommonResult, UnicodeString};
use r3bl_tuify::{get_terminal_width,
                 select_from_list_with_multi_line_header,
                 SelectionMode,
                 StyleSheet};

use super::{get_branches, try_get_current_branch};
use crate::{color_constants::DefaultColors::{FrozenBlue,
                                             GuardsRed,
                                             LizardGreen,
                                             MoonlightBlue,
                                             NightBlue,
                                             Orange,
                                             SlateGray},
            giti::{clap_config::BranchSubcommand,
                   report_unknown_error_and_propagate,
                   single_select_instruction_header,
                   ui_strings::UIStrings::*,
                   CommandSuccessfulResponse}};
pub fn try_checkout_branch(
    maybe_branch_name: Option<String>,
) -> CommonResult<CommandSuccessfulResponse> {
    let try_run_command_result = CommandSuccessfulResponse {
        maybe_deleted_branches: None,
        branch_subcommand: Some(BranchSubcommand::Checkout),
    };

    // If branch_name is passed as an argument, then check out to it and return early.
    match maybe_branch_name {
        Some(branch_name) => {
            // Check does branch_name match any of the branches.
            let branches = get_branches()?;
            let branches_trimmed: Vec<String> = branches
                .iter()
                .map(|branch| branch.trim_start_matches("(current) ").to_string())
                .collect();

            // If branch_name doesn't match any of the branches, then the branch doesn't exist,  return early.
            if !branches_trimmed.contains(&branch_name) {
                let ferrari_red = GuardsRed.as_ansi_color();
                AnsiStyledText {
                    text: &BranchDoesNotExist { branch_name }.to_string(),
                    style: &[Style::Foreground(ferrari_red)],
                }
                .println();
                return Ok(try_run_command_result);
            };

            let current_branch = try_get_current_branch()?;

            // Check if branch_name is the same as current branch. Then early return.
            if branch_name == current_branch {
                let current_branch_name = AnsiStyledText {
                    text: &current_branch,
                    style: &[Style::Foreground(LizardGreen.as_ansi_color())],
                };
                let slate_gray = SlateGray.as_ansi_color();
                let already_on_branch = AnsiStyledText {
                    text: &AlreadyOnCurrentBranch.to_string(),
                    style: &[Style::Foreground(slate_gray)],
                };
                println!("{already_on_branch}{current_branch_name}");
                return Ok(try_run_command_result);
            }

            // Check for modified unstaged files.
            let command_to_check_for_modified_files: &mut Command =
                &mut create_git_command_to_check_for_modified_unstaged_files();
            let result_output_for_modified_files =
                command_to_check_for_modified_files.output();

            if let Ok(output) = result_output_for_modified_files {
                if output.status.success() {
                    // Format each modified file in modified_files_vec.
                    let modified_files: Vec<String> =
                        get_formatted_modified_files(output);

                    // If user has files that are modified (unstaged or staged), but not committed.
                    if !modified_files.is_empty() {
                        let terminal_width = get_terminal_width();

                        let one_modified_file = &ModifiedFileOnCurrentBranch.to_string();
                        let one_modified_file = add_spaces_to_end_of_string(
                            one_modified_file,
                            terminal_width,
                        );

                        let multiple_modified_files =
                            &ModifiedFilesOnCurrentBranch.to_string();
                        let multiple_modified_files = add_spaces_to_end_of_string(
                            multiple_modified_files,
                            terminal_width,
                        );

                        let modified_filed_text_style = &[
                            Style::Foreground(Orange.as_ansi_color()),
                            Style::Background(NightBlue.as_ansi_color()),
                        ];

                        if modified_files.len() == 1 {
                            AnsiStyledText {
                                text: &one_modified_file,
                                style: modified_filed_text_style,
                            }
                            .println();
                        } else {
                            AnsiStyledText {
                                text: &multiple_modified_files,
                                style: modified_filed_text_style,
                            }
                            .println();
                        }

                        let gray_text_style = &[
                            Style::Foreground(SlateGray.as_ansi_color()),
                            Style::Background(NightBlue.as_ansi_color()),
                        ];

                        for file in &modified_files {
                            let file = add_spaces_to_end_of_string(file, terminal_width);
                            AnsiStyledText {
                                text: &file,
                                style: gray_text_style,
                            }
                            .println();
                        }

                        let please_commit_changes =
                            PleaseCommitChangesBeforeSwitchingBranches.to_string();
                        let please_commit_changes = add_spaces_to_end_of_string(
                            &please_commit_changes,
                            terminal_width,
                        );
                        AnsiStyledText {
                            text: &please_commit_changes,
                            style: &[
                                Style::Foreground(Orange.as_ansi_color()),
                                Style::Background(NightBlue.as_ansi_color()),
                            ],
                        }
                        .println();
                        return Ok(try_run_command_result);
                    }
                }
            }

            // Below code will execute if user doesn't have any modified uncommitted files.
            let checkout_branch_command: &mut Command =
                &mut create_git_command_to_checkout_branch(&branch_name);
            let branch_checkout_result_output = checkout_branch_command.output();

            match branch_checkout_result_output {
                Ok(branch_checkout_output) => {
                    if branch_checkout_output.status.success() {
                        let branch_name_styled = AnsiStyledText {
                            text: &branch_name,
                            style: &[Style::Foreground(LizardGreen.as_ansi_color())],
                        };
                        let slate_gray = SlateGray.as_ansi_color();

                        if branch_name == current_branch {
                            let already_on_branch = AnsiStyledText {
                                text: &AlreadyOnCurrentBranch.to_string(),
                                style: &[Style::Foreground(slate_gray)],
                            };
                            println!("{already_on_branch}{branch_name_styled}");
                        } else {
                            let switched_to = AnsiStyledText {
                                text: &SwitchedToBranch.to_string(),
                                style: &[Style::Foreground(slate_gray)],
                            };
                            println!("{switched_to}{branch_name_styled}");
                        }
                    } else {
                        try_checkout_branch_error::display_error_message(
                            branch_name,
                            Some(branch_checkout_output),
                        );
                    }
                }
                Err(error) => {
                    // Can't even execute output(), something unknown has gone
                    // wrong. Propagate the error.
                    try_checkout_branch_error::display_error_message(branch_name, None);
                    return report_unknown_error_and_propagate(
                        checkout_branch_command,
                        error,
                    );
                }
            }
        }

        // The code below will execute if branch_name is not passed as an argument. It displays user
        // all the local branches and asks them to select a branch to check out to.
        None => {
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
                    let selected_branch =
                        selected_branch.trim_start_matches("(current) ");
                    let checkout_branch_command: &mut Command =
                        &mut create_git_command_to_checkout_branch(selected_branch);
                    let branch_checkout_result_output = checkout_branch_command.output();

                    match branch_checkout_result_output {
                        Ok(branch_checkout_output) => {
                            if branch_checkout_output.status.success() {
                                display_correct_message_after_user_tried_to_checkout(
                                    selected_branch,
                                    current_branch,
                                );
                            } else {
                                try_checkout_branch_error::display_error_message(
                                    selected_branch.to_string(),
                                    Some(branch_checkout_output),
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
                            return report_unknown_error_and_propagate(
                                checkout_branch_command,
                                error,
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(try_run_command_result)
}

mod branch_checkout_formatting {
    use super::*;

    pub fn add_spaces_to_end_of_string(string: &str, terminal_width: usize) -> String {
        let string_length = UnicodeString::from(string).display_width;
        let spaces_to_add = ch!(terminal_width) - string_length;
        let spaces = " ".repeat(ch!(@to_usize spaces_to_add));
        let string = format!("{}{}", string, spaces);
        string
    }

    pub fn get_formatted_modified_files(output: std::process::Output) -> Vec<String> {
        let mut modified_files_vec: Vec<String> = Vec::new();
        let modified_files = String::from_utf8_lossy(&output.stdout).to_string();
        // Early return if there are no modified files.
        if modified_files.is_empty() {
            return modified_files_vec;
        }
        let modified_files = modified_files.trim();
        let modified_files_vector: Vec<&str> = modified_files.split('\n').collect();
        // Remove all the spaces from start and end of each modified file.
        let modified_files_vector: Vec<String> = modified_files_vector
            .iter()
            .map(|output| output.trim().to_string())
            .collect();

        // Remove all the "MM" and " M" from modified files.
        // "M" means unstaged files. "MM" means staged files.
        for output in &modified_files_vector {
            if output.starts_with("MM ") {
                let modified_output = output.replace("MM", "");
                let modified_output = modified_output.trim_start();
                let modified_output = format!("    - {}", modified_output);
                modified_files_vec.push(modified_output);
            } else if output.starts_with("M ") {
                let modified_output = output.replace("M ", "");
                let modified_output = modified_output.trim_start();
                let modified_output = format!("    - {}", modified_output);
                modified_files_vec.push(modified_output);
            } else {
                let modified_output = output.trim_start();
                let modified_output = format!("    - {}", modified_output);
                modified_files_vec.push(modified_output);
            }
        }
        modified_files_vec
    }

    pub fn display_correct_message_after_user_tried_to_checkout(
        selected_branch: &str,
        current_branch: String,
    ) {
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
    }
}

fn create_git_command_to_check_for_modified_unstaged_files() -> Command {
    let mut command = Command::new("git");
    command.args(["status", "--porcelain"]);
    command
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
