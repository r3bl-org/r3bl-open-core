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

use r3bl_core::{CommonResult,
                InlineString,
                ItemsOwned,
                ast,
                ast_line,
                height,
                new_style,
                tui_color};
use r3bl_tui::{DefaultIoDevices,
               choose,
               readline_async::{HowToChoose, StyleSheet}};
use smallvec::smallvec;
use try_delete_branch_user_choice::Selection::{self, Delete, ExitProgram};

use crate::{AnalyticsAction,
            giti::{SuccessReport,
                   clap_config::BranchSubcommand,
                   git::try_get_local_branches,
                   ui_strings::UIStrings,
                   ui_templates::{multi_select_instruction_header,
                                  report_unknown_error_and_propagate,
                                  single_select_instruction_header}},
            report_analytics};

pub async fn try_delete_branch() -> CommonResult<SuccessReport> {
    report_analytics::start_task_to_generate_event(
        "".to_string(),
        AnalyticsAction::GitiBranchDelete,
    );

    let mut try_run_command_result = SuccessReport {
        branch_subcommand: Some(BranchSubcommand::Delete),
        ..Default::default()
    };

    let default_header_style = new_style!(
        color_fg: {tui_color!(frozen_blue)} color_bg: {tui_color!(moonlight_blue)}
    );

    let header = {
        let last_line = ast_line![ast(
            UIStrings::PleaseSelectBranchesYouWantToDelete.to_string(),
            default_header_style
        )];
        multi_select_instruction_header(last_line)
    };

    if let Ok(branches) = try_get_local_branches() {
        let mut default_io_devices = DefaultIoDevices::default();
        let branches = choose(
            header,
            branches,
            Some(height(20)),
            None,
            HowToChoose::Multiple,
            StyleSheet::default(),
            default_io_devices.as_mut_tuple(),
        )
        .await?;

        let num_of_branches = branches.len();

        if num_of_branches == 0 {
            return Ok(try_run_command_result);
        }

        let branches_to_delete = branches.join(", ");

        let (confirm_branch_deletion_header, confirm_deletion_options) = {
            let mut confirm_deletion_options: ItemsOwned =
                smallvec![UIStrings::Exit.to_string().into()];
            if num_of_branches == 1 {
                let branch_name = &branches[0];
                let branch_name = branch_name.to_string();
                confirm_deletion_options
                    .insert(0, UIStrings::YesDeleteBranch.to_string().into());
                (
                    UIStrings::ConfirmDeletingOneBranch { branch_name }.to_string(),
                    confirm_deletion_options,
                )
            } else {
                confirm_deletion_options
                    .insert(0, UIStrings::YesDeleteBranches.to_string().into());
                (
                    UIStrings::ConfirmDeletingMultipleBranches {
                        num_of_branches,
                        branches_to_delete,
                    }
                    .to_string(),
                    confirm_deletion_options,
                )
            }
        };

        let header = {
            let last_line =
                ast_line![ast(confirm_branch_deletion_header, default_header_style)];
            single_select_instruction_header(last_line)
        };

        let mut default_io_devices = DefaultIoDevices::default();
        let selected_delete_or_exit = choose(
            header,
            confirm_deletion_options,
            Some(height(20)),
            None,
            HowToChoose::Single,
            StyleSheet::default(),
            default_io_devices.as_mut_tuple(),
        )
        .await?;

        match Selection::from(selected_delete_or_exit) {
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
                            try_run_command_result.maybe_deleted_branches = Some({
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
                        try_delete_branch_inner::display_error_message(branches, None);
                        return report_unknown_error_and_propagate(
                            command,
                            miette::miette!(error),
                        );
                    }
                }
            }

            ExitProgram => (),
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
                selected[0] == UIStrings::YesDeleteBranch.to_string();
            let selected_to_delete_multiple_branches =
                selected[0] == UIStrings::YesDeleteBranches.to_string();
            let selected_to_exit = selected[0] == UIStrings::Exit.to_string();

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
                        &UIStrings::FailedToDeleteBranch {
                            branch_name: branch.to_string(),
                            error_message: String::from_utf8_lossy(&output.stderr).into(),
                        }
                        .to_string(),
                    )
                    .println();
                } else {
                    let branches = branches.join(",\n ╴");
                    fg_guards_red(
                        &UIStrings::FailedToDeleteBranches {
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
                    &UIStrings::FailedToRunCommandToDeleteBranches { branches }
                        .to_string(),
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
            b = fg_slate_gray(&UIStrings::Deleted.to_string()),
        );
    }

    pub fn display_all_branches_deleted_success_messages(branches: &ItemsOwned) {
        for branch in branches {
            println!(
                " ✅ {a} {b}",
                a = fg_lizard_green(branch),
                b = fg_slate_gray(&UIStrings::Deleted.to_string()),
            );
        }
    }
}
