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

use r3bl_ansi_color::{ASTStyle, AnsiStyledText};
use r3bl_core::CommonResult;
use r3bl_tuify::{SelectionMode, StyleSheet, select_from_list_with_multi_line_header};
use smallvec::smallvec;
use try_delete_branch_user_choice::Selection::{self, Delete, ExitProgram};

use crate::{AnalyticsAction,
            color_constants::DefaultColors::{FrozenBlue,
                                             GuardsRed,
                                             LizardGreen,
                                             MoonlightBlue,
                                             SlateGrey},
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

    let default_header_style = smallvec::smallvec![
        ASTStyle::Foreground(FrozenBlue.as_ansi_color()),
        ASTStyle::Background(MoonlightBlue.as_ansi_color()),
    ];

    let select_branches_header_text = &PleaseSelectBranchesYouWantToDelete.to_string();

    let instructions_and_branches_to_delete = {
        let mut instructions_and_branches_to_delete = multi_select_instruction_header();
        let header = AnsiStyledText {
            text: select_branches_header_text,
            style: default_header_style.clone(),
        };
        instructions_and_branches_to_delete.push(vec![header]);
        instructions_and_branches_to_delete
    };

    if let Ok(branches) = get_branches() {
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
                let mut confirm_deletion_options: Vec<String> = vec![Exit.to_string()];
                if num_of_branches == 1 {
                    let branch_name = &branches[0];
                    let branch_name = branch_name.to_string();
                    confirm_deletion_options.insert(0, YesDeleteBranch.to_string());
                    (
                        ConfirmDeletingOneBranch { branch_name }.to_string(),
                        confirm_deletion_options,
                    )
                } else {
                    confirm_deletion_options.insert(0, YesDeleteBranches.to_string());
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
                let mut instructions_and_confirm_deletion_header =
                    single_select_instruction_header();
                let header = AnsiStyledText {
                    text: &confirm_branch_deletion_header,
                    style: default_header_style,
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
                                    // Add branches to deleted branches.
                                    try_run_command_result.maybe_deleted_branches =
                                        Some({
                                            let mut it = smallvec![];
                                            for branch in &branches {
                                                it.push(branch.into());
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

    impl From<Vec<String>> for Selection {
        fn from(selected: Vec<String>) -> Selection {
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
    use super::*;

    pub fn display_error_message(
        branches: Vec<String>,
        maybe_output: Option<std::process::Output>,
    ) {
        let ferrari_red = GuardsRed.as_ansi_color();
        match maybe_output {
            Some(output) => {
                if branches.len() == 1 {
                    let branch = &branches[0];
                    AnsiStyledText {
                        text: &FailedToDeleteBranch {
                            branch_name: branch.clone(),
                            error_message: String::from_utf8_lossy(&output.stderr)
                                .to_string(),
                        }
                        .to_string(),
                        style: smallvec::smallvec![ASTStyle::Foreground(ferrari_red)],
                    }
                    .println();
                } else {
                    let branches = branches.join(",\n ╴");
                    AnsiStyledText {
                        text: &FailedToDeleteBranches {
                            branches,
                            error_message: String::from_utf8_lossy(&output.stderr)
                                .to_string(),
                        }
                        .to_string(),
                        style: smallvec::smallvec![ASTStyle::Foreground(ferrari_red)],
                    }
                    .println();
                }
            }
            None => {
                let branches = branches.join(",\n ╴");
                AnsiStyledText {
                    text: &FailedToRunCommandToDeleteBranches { branches }.to_string(),
                    style: smallvec::smallvec![ASTStyle::Foreground(ferrari_red)],
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

    pub fn display_one_branch_deleted_success_message(branches: &[String]) {
        let lizard_green = LizardGreen.as_ansi_color();
        let branch_name = &branches[0].to_string();
        let deleted_branch = AnsiStyledText {
            text: branch_name,
            style: smallvec::smallvec![ASTStyle::Foreground(lizard_green)],
        };
        let deleted = AnsiStyledText {
            text: &Deleted.to_string(),
            style: smallvec::smallvec![ASTStyle::Foreground(SlateGrey.as_ansi_color())],
        };
        println!(" ✅ {deleted_branch} {deleted}");
    }

    pub fn display_all_branches_deleted_success_messages(branches: &Vec<String>) {
        let lizard_green = LizardGreen.as_ansi_color();
        for branch in branches {
            let deleted_branch = AnsiStyledText {
                text: branch,
                style: smallvec::smallvec![ASTStyle::Foreground(lizard_green)],
            };
            let deleted = AnsiStyledText {
                text: &Deleted.to_string(),
                style: smallvec::smallvec![ASTStyle::Foreground(
                    SlateGrey.as_ansi_color()
                )],
            };
            println!(" ✅ {deleted_branch} {deleted}");
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
pub fn get_branches() -> CommonResult<Vec<String>> {
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

    let mut branches_vec = vec![];
    for branch in branches {
        if branch == current_branch {
            branches_vec.push(CurrentBranch { branch }.to_string());
        } else {
            branches_vec.push(branch.to_string());
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
