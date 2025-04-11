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

use r3bl_core::{AST,
                CommonResult,
                InlineString,
                InlineVec,
                ItemsOwned,
                ast,
                ast_line,
                height,
                items_owned_to_vec_string,
                new_style,
                tui_color};
use r3bl_tui::{DefaultIoDevices,
               Header,
               choose,
               readline_async::{HowToChoose, StyleSheet}};
use smallvec::smallvec;
use try_delete_branch_user_choice::Selection::{self};

use crate::{AnalyticsAction,
            giti::{SuccessReport,
                   clap_config::BranchSubcommand,
                   git::{self},
                   ui_strings::{self, UIStrings},
                   ui_templates::{multi_select_instruction_header,
                                  report_unknown_error_and_propagate,
                                  single_select_instruction_header}},
            report_analytics};

pub async fn try_delete_branch() -> CommonResult<SuccessReport> {
    report_analytics::start_task_to_generate_event(
        "".to_string(),
        AnalyticsAction::GitiBranchDelete,
    );

    // Only proceed if some local branches exist (can't delete anything if there aren't
    // any).
    if let Ok(branches) = git::try_get_local_branches()
        && !branches.is_empty()
    {
        let branches = select_branches_to_delete(branches).await?;
        // If the user didn't select any branches, we don't need to do anything.
        if branches.is_empty() {
            return get_success_report();
        }

        let (confirm_header, confirm_options) = create_confirmation_prompt(&branches);
        let selected_action =
            get_user_confirmation(confirm_header, confirm_options).await?;

        if let Selection::Delete = selected_action {
            return delete_selected_branches(branches).await;
        }
    }

    get_success_report()
}

fn get_success_report() -> CommonResult<SuccessReport> {
    Ok(SuccessReport {
        maybe_deleted_branches: None,
        branch_subcommand: Some(BranchSubcommand::Delete),
    })
}

fn create_multi_select_header() -> impl Into<Header> {
    let default_header_style = new_style!(
        color_fg: {tui_color!(frozen_blue)} color_bg: {tui_color!(moonlight_blue)}
    );
    let last_line = ast_line![ast(
        UIStrings::PleaseSelectBranchesYouWantToDelete.to_string(),
        default_header_style
    )];
    multi_select_instruction_header(last_line)
}

async fn select_branches_to_delete(branches: ItemsOwned) -> CommonResult<ItemsOwned> {
    let header = create_multi_select_header();
    let mut default_io_devices = DefaultIoDevices::default();
    choose(
        header,
        branches,
        Some(height(20)),
        None,
        HowToChoose::Multiple,
        StyleSheet::default(),
        default_io_devices.as_mut_tuple(),
    )
    .await
}

fn create_confirmation_prompt(branches: &ItemsOwned) -> (String, ItemsOwned) {
    let num_of_branches = branches.len();

    let mut confirm_deletion_options: ItemsOwned =
        smallvec![UIStrings::Exit.to_string().into()];

    if num_of_branches == 1 {
        let branch_name = &branches[0];
        confirm_deletion_options.insert(0, UIStrings::YesDeleteBranch.to_string().into());

        // Return tuple.
        (
            UIStrings::ConfirmDeletingOneBranch {
                branch_name: branch_name.to_string(),
            }
            .to_string(),
            confirm_deletion_options,
        )
    } else {
        confirm_deletion_options
            .insert(0, UIStrings::YesDeleteBranches.to_string().into());

        // Return tuple.
        (
            ui_strings::display_confirm_deleting_multiple_branches(
                num_of_branches,
                items_owned_to_vec_string(branches),
            ),
            confirm_deletion_options,
        )
    }
}

async fn get_user_confirmation(
    header_text: String,
    options: ItemsOwned,
) -> CommonResult<Selection> {
    // Define styles for the header first line, and subsequent lines (which are branches
    // that are selected for deletion).
    let default_header_style = new_style!(
        color_fg: {tui_color!(frozen_blue)} color_bg: {tui_color!(moonlight_blue)}
    );
    let branch_to_delete_style = new_style!(
        color_fg: {tui_color!(yellow)} color_bg: {tui_color!(moonlight_blue)}
    );

    // Apply one style to the first line of the header, and another style to the rest of
    // the lines. Then prefix with the instruction header.
    let header = {
        let mut header_last_lines = header_text.lines();
        let mut header_last_lines_fmt: InlineVec<InlineVec<AST>> = smallvec![];

        if let Some(first_line) = header_last_lines.next() {
            let first_line = ast_line![ast(first_line, default_header_style)];
            header_last_lines_fmt.push(first_line);
        }

        for line in header_last_lines {
            let line = ast_line![ast(line, branch_to_delete_style)];
            header_last_lines_fmt.push(line);
        }

        single_select_instruction_header(header_last_lines_fmt)
    };

    let mut default_io_devices = DefaultIoDevices::default();
    let selected_option = choose(
        header,
        options,
        Some(height(20)),
        None,
        HowToChoose::Single,
        StyleSheet::default(),
        default_io_devices.as_mut_tuple(),
    )
    .await?;

    Ok(Selection::from(selected_option))
}

async fn delete_selected_branches(branches: ItemsOwned) -> CommonResult<SuccessReport> {
    // Get an empty success report. This will be updated with the deleted branches (if
    // that is successful).
    let mut success_report = get_success_report()?;

    // Delete the branches.
    let (res_output, mut cmd) = git::try_delete_branches(&branches);

    // Handle the result of the delete command.
    match res_output {
        Ok(output) if output.status.success() => {
            // Update success report with deleted branches.
            success_report.maybe_deleted_branches = Some({
                let mut acc: ItemsOwned = smallvec![];
                for branch in &branches {
                    acc.push(branch.clone());
                }
                acc
            });

            // Display success message.
            if branches.len() == 1 {
                try_delete_branch_inner::display_one_branch_deleted_success_message(
                    &branches,
                );
            } else {
                try_delete_branch_inner::display_all_branches_deleted_success_messages(
                    &branches,
                );
            }
        }
        Ok(output) => {
            try_delete_branch_inner::display_error_message(branches, Some(output));
        }
        Err(error) => {
            try_delete_branch_inner::display_error_message(branches, None);
            return report_unknown_error_and_propagate(&mut cmd, miette::miette!(error));
        }
    }

    Ok(success_report)
}

mod try_delete_branch_user_choice {
    use super::*;

    pub enum Selection {
        Delete,
        ExitProgram,
    }

    impl From<ItemsOwned> for Selection {
        fn from(selected: ItemsOwned) -> Selection {
            if selected.is_empty() {
                return Selection::ExitProgram;
            }

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
