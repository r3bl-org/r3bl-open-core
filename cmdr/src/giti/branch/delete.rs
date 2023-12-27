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

use r3bl_ansi_color::{AnsiStyledText, Color, Style};
use r3bl_rs_utils_core::CommonResult;
use r3bl_tuify::{select_from_list_with_multi_line_header,
                 SelectionMode,
                 StyleSheet,
                 DELETE_BRANCH,
                 DELETE_BRANCHES,
                 EXIT};
use try_delete_branch_user_choice::Selection::{self, *};

use crate::giti::{giti_ui_templates::report_unknown_error_and_propagate,
                  multi_select_instruction_header,
                  single_select_instruction_header};

pub fn try_delete_branch() -> CommonResult<()> {
    let default_header_style = [
        Style::Foreground(Color::Rgb(171, 204, 242)),
        Style::Background(Color::Rgb(31, 36, 46)),
    ];

    let instructions_and_branches_to_delete = {
        let mut instructions_and_branches_to_delete = multi_select_instruction_header();
        let header = AnsiStyledText {
            text: " Please select branches you want to delete",
            style: &default_header_style,
        };
        instructions_and_branches_to_delete.push(vec![header]);
        instructions_and_branches_to_delete
    };

    let branches = try_execute_git_command_to_get_branches()?;

    let maybe_selected_branches = select_from_list_with_multi_line_header(
        instructions_and_branches_to_delete,
        branches,
        Some(20),
        None,
        SelectionMode::Multiple,
        StyleSheet::default(),
    );

    if let Some(branches) = maybe_selected_branches {
        let branches_to_delete = branches.join(", ");
        let num_of_branches = branches.len();

        let (confirm_branch_deletion_header, confirm_deletion_options) = {
            let mut confirm_deletion_options: Vec<String> = vec![EXIT.to_string()];
            if num_of_branches == 1 {
                confirm_deletion_options.insert(0, DELETE_BRANCH.to_string());
                (
                    format!("Confirm deleting 1 branch: {}", branches_to_delete),
                    confirm_deletion_options,
                )
            } else {
                confirm_deletion_options.insert(0, DELETE_BRANCHES.to_string());
                (
                    format!(
                        "Confirm deleting {} branches: {}?",
                        num_of_branches, branches_to_delete
                    ),
                    confirm_deletion_options,
                )
            }
        };

        let instructions_and_confirm_deletion_options = {
            let mut instructions_and_confirm_deletion_header =
                single_select_instruction_header();
            let header = AnsiStyledText {
                text: &confirm_branch_deletion_header,
                style: &default_header_style,
            };
            instructions_and_confirm_deletion_header.push(vec![header]);
            instructions_and_confirm_deletion_header
        };

        let maybe_selected_delete_or_exit = select_from_list_with_multi_line_header(
            instructions_and_confirm_deletion_options,
            confirm_deletion_options,
            Some(20),
            None,
            SelectionMode::Single,
            StyleSheet::default(),
        );

        if let Some(selected) = maybe_selected_delete_or_exit {
            match Selection::from(selected) {
                Delete => {
                    let command: &mut Command =
                        &mut try_delete_branch_inner::create_git_command_to_delete_branches(
                            &branches,
                        );
                    let result_output = command.output();

                    match result_output {
                        Ok(output) => {
                            // Got output, check exit code for success (known errors).
                            if output.status.success() {
                                if num_of_branches == 1 {
                                    try_delete_branch_inner::display_one_branch_deleted_success_message(&branches);
                                } else {
                                    try_delete_branch_inner::display_all_branches_deleted_success_messages(
                                        &branches,
                                    );
                                }
                            } else {
                                try_delete_branch_inner::display_error_message(
                                    branches,
                                    Some(output),
                                );
                            }
                        }
                        Err(error) => {
                            // Can't even execute output(), something unknown has gone
                            // wrong. Propagate the error.
                            try_delete_branch_inner::display_error_message(
                                branches, None,
                            );
                            return report_unknown_error_and_propagate(command, error);
                        }
                    }
                }

                Exit => return Ok(println!("You chose not to delete any branches.")),
            }
        }
    }

    return Ok(());
}

mod try_delete_branch_user_choice {
    use super::*;

    pub enum Selection {
        Delete,
        Exit,
    }

    impl From<Vec<String>> for Selection {
        fn from(selected: Vec<String>) -> Selection {
            let selected_to_delete_one_branch = selected[0] == DELETE_BRANCH.to_string();
            let selected_to_delete_multiple_branches =
                selected[0] == DELETE_BRANCHES.to_string();
            let selected_to_exit = selected[0] == EXIT.to_string();

            if selected_to_delete_one_branch || selected_to_delete_multiple_branches {
                return Selection::Delete;
            }
            if selected_to_exit {
                return Selection::Exit;
            }
            Selection::Exit
        }
    }
}

mod try_delete_branch_inner {
    use r3bl_tuify::{FAILED_COLOR, LIGHT_GRAY_COLOR, SUCCESS_COLOR};

    use super::*;

    pub fn display_error_message(
        branches: Vec<String>,
        maybe_output: Option<std::process::Output>,
    ) {
        match maybe_output {
            Some(output) => {
                if branches.len() == 1 {
                    let branch = &branches[0];
                    AnsiStyledText {
                        text: &format!(
                            "Failed to delete branch: {branch}!\n\n{}",
                            String::from_utf8_lossy(&output.stderr)
                        ),
                        style: &[Style::Foreground(FAILED_COLOR)],
                    }
                    .println();
                } else {
                    let branches = branches.join(",\n ╴");
                    AnsiStyledText {
                        text: &format!(
                            "Failed to delete branches:\n ╴{branches}!\n\n{}",
                            String::from_utf8_lossy(&output.stderr)
                        ),
                        style: &[Style::Foreground(FAILED_COLOR)],
                    }
                    .println();
                }
            }
            None => {
                let branches = branches.join(",\n ╴");
                AnsiStyledText {
                    text: &format!(
                        "Failed to run command to delete branches:\n ╴{branches}!"
                    ),
                    style: &[Style::Foreground(FAILED_COLOR)],
                }
                .println();
            }
        }
    }

    /// Create a [Command] to delete all the given branches. Does not execute the command.
    pub fn create_git_command_to_delete_branches(branches: &Vec<String>) -> Command {
        let mut command = Command::new("git");
        command.args(["branch", "-D"]);
        for branch in branches {
            command.arg(branch);
        }
        command
    }

    pub fn display_one_branch_deleted_success_message(branches: &Vec<String>) {
        let branch_name = &branches[0].to_string();
        let deleted_branch = AnsiStyledText {
            text: branch_name,
            style: &[Style::Foreground(SUCCESS_COLOR)],
        };
        let deleted = AnsiStyledText {
            text: "deleted",
            style: &[Style::Foreground(LIGHT_GRAY_COLOR)],
        };
        AnsiStyledText {
            text: &format!("✅ {deleted_branch} {deleted}").as_str(),
            style: &[Style::Foreground(SUCCESS_COLOR)],
        }
        .println();
    }

    pub fn display_all_branches_deleted_success_messages(branches: &Vec<String>) {
        for branch in branches {
            let deleted_branch = AnsiStyledText {
                text: branch,
                style: &[Style::Foreground(SUCCESS_COLOR)],
            };
            let deleted = AnsiStyledText {
                text: "deleted",
                style: &[Style::Foreground(LIGHT_GRAY_COLOR)],
            };
            AnsiStyledText {
                text: &format!("✅ {deleted_branch} {deleted}").as_str(),
                style: &[Style::Foreground(SUCCESS_COLOR)],
            }
            .println();
        }
    }
}

pub fn try_execute_git_command_to_get_branches() -> CommonResult<Vec<String>> {
    // Create command.
    let mut command = Command::new("git");
    let command: &mut Command = command.args(["branch", "--format", "%(refname:short)"]);

    // Execute command.
    let result_output = command.output();

    // Process command execution results.
    match result_output {
        // Can't even execute output(), something unknown has gone wrong. Propagate the
        // error.
        Err(error) => report_unknown_error_and_propagate(command, error),
        Ok(output) => {
            let output_string = String::from_utf8_lossy(&output.stdout);
            let mut branches = vec![];
            for line in output_string.lines() {
                branches.push(line.to_string());
            }
            Ok(branches)
        }
    }
}
