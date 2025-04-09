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
use std::process::Command;

use branch_checkout_formatting::{add_spaces_to_end_of_string,
                                 display_correct_message_after_user_tried_to_checkout,
                                 get_formatted_modified_files};
use r3bl_core::{ChUnit,
                CommonResult,
                GCString,
                InlineVec,
                ast,
                ast_line,
                fg_frozen_blue,
                fg_guards_red,
                fg_lizard_green,
                fg_slate_gray,
                get_terminal_width,
                height,
                new_style,
                tui_color,
                usize};
use r3bl_tui::{DefaultIoDevices,
               choose,
               readline_async::{HowToChoose, StyleSheet}};
use smallvec::smallvec;

use super::{get_branches, try_get_current_branch};
use crate::giti::{SuccessReport,
                  clap_config::BranchSubcommand,
                  ui_strings::UIStrings::{AlreadyOnCurrentBranch,
                                          BranchDoesNotExist,
                                          FailedToSwitchToBranch,
                                          ModifiedFileOnCurrentBranch,
                                          ModifiedFilesOnCurrentBranch,
                                          NoBranchGotCheckedOut,
                                          PleaseCommitChangesBeforeSwitchingBranches,
                                          SelectBranchToSwitchTo,
                                          SwitchedToBranch},
                  ui_templates::{report_unknown_error_and_propagate,
                                 single_select_instruction_header}};

pub async fn try_checkout_branch(
    maybe_branch_name: Option<String>,
) -> CommonResult<SuccessReport> {
    let try_run_command_result = SuccessReport {
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
                fg_guards_red(&BranchDoesNotExist { branch_name }.to_string()).println();
                return Ok(try_run_command_result);
            };

            let current_branch = try_get_current_branch()?;

            // Check if branch_name is the same as current branch. Then early return.
            if branch_name == current_branch {
                println!(
                    "{a}{b}",
                    a = fg_slate_gray(&AlreadyOnCurrentBranch.to_string()),
                    b = fg_lizard_green(&current_branch)
                );
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
                    let modified_files = &get_formatted_modified_files(output);

                    // If user has files that are modified (unstaged or staged), but not committed.
                    if !modified_files.is_empty() {
                        let terminal_width = *get_terminal_width();

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

                        let modified_files_style = new_style!(
                            color_fg: {tui_color!(orange)} color_bg: {tui_color!(night_blue)}
                        );
                        if modified_files.len() == 1 {
                            ast(&one_modified_file, modified_files_style).println();
                        } else {
                            ast(&multiple_modified_files, modified_files_style).println();
                        }

                        for file in modified_files {
                            let file = add_spaces_to_end_of_string(file, terminal_width);
                            fg_slate_gray(&file).bg_night_blue().println();
                        }

                        let please_commit_changes =
                            PleaseCommitChangesBeforeSwitchingBranches.to_string();
                        let please_commit_changes = add_spaces_to_end_of_string(
                            &please_commit_changes,
                            terminal_width,
                        );
                        ast(&please_commit_changes, modified_files_style).println();

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
                        if branch_name == current_branch {
                            println!(
                                "{a}{b}",
                                a = fg_slate_gray(&AlreadyOnCurrentBranch.to_string()),
                                b = fg_lizard_green(&branch_name)
                            );
                        } else {
                            println!(
                                "{a}{b}",
                                a = fg_slate_gray(&SwitchedToBranch.to_string()),
                                b = fg_lizard_green(&branch_name)
                            );
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
                        miette::miette!(error),
                    );
                }
            }
        }

        // The code below will execute if branch_name is not passed as an argument. It
        // displays user all the local branches and asks them to select a branch to check
        // out to.
        None => {
            let _binding_last_line_text = SelectBranchToSwitchTo.to_string();
            let header = {
                let line = ast_line![
                    fg_frozen_blue(&_binding_last_line_text).bg_moonlight_blue()
                ];
                single_select_instruction_header(line)
            };

            let current_branch = try_get_current_branch()?;

            if let Ok(branches) = get_branches() {
                // Ask user to select a branch to check out to.
                let mut default_io_devices = DefaultIoDevices::default();
                let selected_branch = choose(
                    header,
                    branches,
                    Some(height(20)),
                    None,
                    HowToChoose::Single,
                    StyleSheet::default(),
                    default_io_devices.as_mut_tuple(),
                )
                .await?;

                // If user selected a branch, then check out to it.
                if let Some(selected_branch) = selected_branch.first() {
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
                                miette::miette!(error),
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
    use r3bl_core::ItemsOwned;

    use super::*;

    pub fn add_spaces_to_end_of_string(string: &str, terminal_width: ChUnit) -> String {
        let string_length = GCString::width(string);
        let spaces_to_add = terminal_width - *string_length;
        let spaces = " ".repeat(usize(spaces_to_add));
        let string = format!("{}{}", string, spaces);
        string
    }

    pub fn get_formatted_modified_files(output: std::process::Output) -> ItemsOwned {
        let mut return_vec = smallvec![];

        let modified_files = String::from_utf8_lossy(&output.stdout).to_string();

        // Early return if there are no modified files.
        if modified_files.is_empty() {
            return return_vec;
        }

        // Remove all the spaces from start and end of each modified file.
        let modified_files = modified_files.trim();
        let modified_files_vec = modified_files
            .split('\n')
            .map(|output| output.trim())
            .collect::<InlineVec<&str>>();

        // Remove all the "MM" and " M" from modified files.
        // "M" means unstaged files. "MM" means staged files.
        for output in &modified_files_vec {
            if output.starts_with("MM ") {
                let modified_output = output.replace("MM", "");
                let modified_output = modified_output.trim_start();
                let modified_output = format!("    - {}", modified_output);
                return_vec.push(modified_output.into());
            } else if output.starts_with("M ") {
                let modified_output = output.replace("M ", "");
                let modified_output = modified_output.trim_start();
                let modified_output = format!("    - {}", modified_output);
                return_vec.push(modified_output.into());
            } else {
                let modified_output = output.trim_start();
                let modified_output = format!("    - {}", modified_output);
                return_vec.push(modified_output.into());
            }
        }
        return_vec
    }

    pub fn display_correct_message_after_user_tried_to_checkout(
        selected_branch: &str,
        current_branch: String,
    ) {
        if selected_branch == current_branch {
            println!(
                "{a}{b}",
                a = fg_slate_gray(&AlreadyOnCurrentBranch.to_string()),
                b = fg_lizard_green(selected_branch)
            );
        } else {
            println!(
                "{a}{b}",
                a = fg_slate_gray(&SwitchedToBranch.to_string()),
                b = fg_lizard_green(selected_branch)
            );
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
        match maybe_output {
            Some(output) => {
                fg_guards_red(
                    &FailedToSwitchToBranch {
                        branch,
                        error_message: String::from_utf8_lossy(&output.stderr)
                            .to_string(),
                    }
                    .to_string(),
                )
                .println();
            }
            None => {
                fg_guards_red(&NoBranchGotCheckedOut { branch }.to_string()).println();
            }
        }
    }
}
