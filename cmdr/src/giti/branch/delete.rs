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

use r3bl_core::{CommonResult, InlineString, ItemsOwned, ast, new_style, tui_color};
use r3bl_tui::terminal_async::{HowToChoose, StyleSheet, choose};
use smallvec::smallvec;
use try_delete_branch_user_choice::Selection::{self, Delete, ExitProgram};

use crate::{AnalyticsAction,
            giti::{CommandSuccessfulResponse,
                   clap_config::BranchSubcommand,
                   giti_ui_templates::report_unknown_error_and_propagate,
                   multi_select_instruction_header,
                   single_select_instruction_header,
                   ui_strings::UIStrings::{ConfirmDeletingMultipleBranches,
                                           ConfirmDeletingOneBranch,
                                           CurrentBranch,
                                           Deleted,
                                           Exit,
                                           FailedToDeleteBranch,
                                           FailedToDeleteBranches,
                                           FailedToRunCommandToDeleteBranches,
                                           PleaseSelectBranchesYouWantToDelete,
                                           YesDeleteBranch,
                                           YesDeleteBranches}},
            report_analytics};

pub fn try_delete_branch() -> CommonResult<CommandSuccessfulResponse> {
    report_analytics::start_task_to_generate_event(
        "".to_string(),
        AnalyticsAction::GitiBranchDelete,
    );

    let mut try_run_command_result = CommandSuccessfulResponse {
        branch_subcommand: Some(BranchSubcommand::Delete),
        ..Default::default()
    };

    let default_header_style = new_style!(
        color_fg: {tui_color!(frozen_blue)} color_bg: {tui_color!(moonlight_blue)}
    );

    let select_branches_header_text = &PleaseSelectBranchesYouWantToDelete.to_string();

    let instructions_and_branches_to_delete = {
        let mut lines = multi_select_instruction_header();
        let header_line = ast(select_branches_header_text, default_header_style);
        lines.push(smallvec![header_line]);
        lines
    };

    if let Ok(branches) = get_branches() {
        let maybe_selected_branches = choose(
            instructions_and_branches_to_delete,
            branches,
            Some(20),
            None,
            HowToChoose::Multiple,
            StyleSheet::default(),
        );

        if let Some(branches) = maybe_selected_branches {
            let branches_to_delete = branches.join(", ");
            let num_of_branches = branches.len();

            let (confirm_branch_deletion_header, confirm_deletion_options) = {
                let mut confirm_deletion_options: ItemsOwned =
                    smallvec![Exit.to_string().into()];
                if num_of_branches == 1 {
                    let branch_name = &branches[0];
                    let branch_name = branch_name.to_string();
                    confirm_deletion_options
                        .insert(0, YesDeleteBranch.to_string().into());
                    (
                        ConfirmDeletingOneBranch { branch_name }.to_string(),
                        confirm_deletion_options,
                    )
                } else {
                    confirm_deletion_options
                        .insert(0, YesDeleteBranches.to_string().into());
                    (
                        ConfirmDeletingMultipleBranches {
                            num_of_branches,
                            branches_to_delete,
                        }
                        .to_string(),
                        confirm_deletion_options,
                    )
                }
            };

            let instructions_and_confirm_deletion_options = {
                let mut lines = single_select_instruction_header();
                let header_line =
                    ast(&confirm_branch_deletion_header, default_header_style);
                lines.push(smallvec![header_line]);
                lines
            };

            let maybe_selected_delete_or_exit = choose(
                instructions_and_confirm_deletion_options,
                confirm_deletion_options,
                Some(20),
                None,
                HowToChoose::Single,
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
                                    // Add branches to deleted branches.
                                    try_run_command_result.maybe_deleted_branches =
                                        Some({
                                            let mut it: ItemsOwned = smallvec![];
                                            for branch in &branches {
                                                it.push(branch.clone());
                                            }
                                            it
                                        });
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
                                return report_unknown_error_and_propagate(
                                    command, error,
                                );
                            }
                        }
                    }

                    ExitProgram => (),
                }
            }
        }
    }

    Ok(try_run_command_result)
}

mod try_delete_branch_user_choice {
    use super::*;

    pub enum Selection {
        Delete,
        ExitProgram,
    }

    impl From<ItemsOwned> for Selection {
        fn from(selected: ItemsOwned) -> Selection {
            let selected_to_delete_one_branch =
                selected[0] == YesDeleteBranch.to_string();
            let selected_to_delete_multiple_branches =
                selected[0] == YesDeleteBranches.to_string();
            let selected_to_exit = selected[0] == Exit.to_string();

            if selected_to_delete_one_branch || selected_to_delete_multiple_branches {
                return Selection::Delete;
            }
            if selected_to_exit {
                return Selection::ExitProgram;
            }
            Selection::ExitProgram
        }
    }
}

mod try_delete_branch_inner {
    use r3bl_core::{fg_guards_red, fg_lizard_green, fg_slate_gray};

    use super::*;

    pub fn display_error_message(
        branches: ItemsOwned,
        maybe_output: Option<std::process::Output>,
    ) {
        match maybe_output {
            Some(output) => {
                if branches.len() == 1 {
                    let branch = &branches[0];
                    fg_guards_red(
                        &FailedToDeleteBranch {
                            branch_name: branch.to_string(),
                            error_message: String::from_utf8_lossy(&output.stderr).into(),
                        }
                        .to_string(),
                    )
                    .println();
                } else {
                    let branches = branches.join(",\n ╴");
                    fg_guards_red(
                        &FailedToDeleteBranches {
                            branches,
                            error_message: String::from_utf8_lossy(&output.stderr)
                                .to_string(),
                        }
                        .to_string(),
                    )
                    .println();
                }
            }
            None => {
                let branches = branches.join(",\n ╴");
                fg_guards_red(
                    &FailedToRunCommandToDeleteBranches { branches }.to_string(),
                )
                .println();
            }
        }
    }

    /// Create a [Command] to delete all the given branches. Does not execute the command.
    pub fn create_git_command_to_delete_branches(branches: &ItemsOwned) -> Command {
        let mut command = Command::new("git");
        command.args(["branch", "-D"]);
        for branch in branches {
            command.arg(branch.to_string());
        }
        command
    }

    pub fn display_one_branch_deleted_success_message(branches: &[InlineString]) {
        let branch_name = &branches[0].to_string();
        println!(
            " ✅ {a} {b}",
            a = fg_lizard_green(branch_name),
            b = fg_slate_gray(&Deleted.to_string()),
        );
    }

    pub fn display_all_branches_deleted_success_messages(branches: &ItemsOwned) {
        for branch in branches {
            println!(
                " ✅ {a} {b}",
                a = fg_lizard_green(branch),
                b = fg_slate_gray(&Deleted.to_string()),
            );
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

// Get all the branches to check out to. prefix current branch with `(current)`.
pub fn get_branches() -> CommonResult<ItemsOwned> {
    let branches = try_execute_git_command_to_get_branches()?;
    // If branch name is current_branch, then append `(current)` in front of it.
    // Create command.
    let mut command = Command::new("git");
    let show_current_branch_command: &mut Command =
        command.args(["branch", "--show-current"]);

    let current_branch_result_output = show_current_branch_command.output();

    let current_branch = match current_branch_result_output {
        Ok(output) => {
            let output_string = String::from_utf8_lossy(&output.stdout);
            output_string.to_string().trim_end_matches('\n').to_string()
        }
        // Can't even execute output(), something unknown has gone wrong. Propagate the
        // error.
        Err(error) => {
            return report_unknown_error_and_propagate(
                show_current_branch_command,
                error,
            );
        }
    };

    let mut branches_vec = smallvec![];
    for branch in branches {
        if branch == current_branch {
            branches_vec.push(CurrentBranch { branch }.to_string().into());
        } else {
            branches_vec.push(branch.into());
        }
    }

    Ok(branches_vec)
}

pub fn try_get_current_branch() -> CommonResult<String> {
    // If branch name is current_branch, then append `(current)` in front of it.
    // Create command.
    let mut command = Command::new("git");
    let command: &mut Command = command.args(["branch", "--show-current"]);

    let result_output = command.output();

    let current_branch = match result_output {
        // Can't even execute output(), something unknown has gone wrong. Propagate the
        // error.
        Err(error) => {
            return report_unknown_error_and_propagate(command, error);
        }
        Ok(output) => {
            let output_string = String::from_utf8_lossy(&output.stdout);
            output_string.to_string().trim_end_matches('\n').to_string()
        }
    };

    Ok(current_branch)
}
