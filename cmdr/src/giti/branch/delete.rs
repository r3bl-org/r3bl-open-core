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

use std::process::Output;

use r3bl_core::{AST,
                CommonResult,
                InlineVec,
                ItemsOwned,
                ast,
                ast_line,
                fg_guards_red,
                fg_lizard_green,
                fg_slate_gray,
                height,
                items_owned_to_vec_string,
                new_style,
                tui_color};
use r3bl_tui::{DefaultIoDevices,
               Header,
               choose,
               readline_async::{HowToChoose, StyleSheet}};
use smallvec::smallvec;

use crate::{AnalyticsAction,
            giti::{BranchDeleteDetails,
                   CommandRunDetails,
                   CommandRunResult,
                   git::{self},
                   ui_strings::{self, UIStrings},
                   ui_templates::{multi_select_instruction_header,
                                  single_select_instruction_header}},
            report_analytics};

/// The main function for `giti branch delete` command.
pub async fn try_delete() -> CommonResult<CommandRunResult> {
    report_analytics::start_task_to_generate_event(
        "".to_string(),
        AnalyticsAction::GitiBranchDelete,
    );

    // Only proceed if some local branches exist (can't delete anything if there aren't
    // any).
    let (res, _cmd) = git::try_get_local_branches();
    if let Ok(branches) = res
        && !branches.is_empty()
    {
        let branches = user_interaction::select_branches_to_delete(branches).await?;

        // If the user didn't select any branches, we don't need to do anything.
        if branches.is_empty() {
            return Ok(CommandRunResult::DidNotRun(None, details::empty()));
        }

        let (confirm_header, confirm_options) =
            user_interaction::create_confirmation_prompt(&branches);
        let selected_action =
            user_interaction::get_user_confirmation(confirm_header, confirm_options)
                .await?;

        if let parse_user_choice::Selection::Delete = selected_action {
            return command_execute::delete_selected_branches(branches).await;
        }
    }

    Ok(CommandRunResult::DidNotRun(None, details::empty()))
}

mod details {
    use super::*;

    pub fn empty() -> CommandRunDetails {
        let it = BranchDeleteDetails {
            maybe_deleted_branches: None,
        };
        CommandRunDetails::BranchDelete(it)
    }

    pub fn with_details(branches: ItemsOwned) -> CommandRunDetails {
        if branches.is_empty() {
            empty()
        } else {
            let it = BranchDeleteDetails {
                maybe_deleted_branches: Some(branches),
            };
            CommandRunDetails::BranchDelete(it)
        }
    }
}

mod user_interaction {
    use super::*;

    pub fn create_multi_select_header() -> impl Into<Header> {
        let default_header_style = new_style!(
            color_fg: {tui_color!(frozen_blue)} color_bg: {tui_color!(moonlight_blue)}
        );
        let last_line = ast_line![ast(
            UIStrings::PleaseSelectBranchesYouWantToDelete.to_string(),
            default_header_style
        )];
        multi_select_instruction_header(last_line)
    }

    pub async fn select_branches_to_delete(
        branches: ItemsOwned,
    ) -> CommonResult<ItemsOwned> {
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

    pub fn create_confirmation_prompt(branches: &ItemsOwned) -> (String, ItemsOwned) {
        let num_of_branches = branches.len();

        let mut confirm_deletion_options: ItemsOwned =
            smallvec![UIStrings::Exit.to_string().into()];

        if num_of_branches == 1 {
            let branch_name = &branches[0];
            confirm_deletion_options
                .insert(0, UIStrings::YesDeleteBranch.to_string().into());

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

    pub async fn get_user_confirmation(
        header_text: String,
        options: ItemsOwned,
    ) -> CommonResult<parse_user_choice::Selection> {
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

        Ok(parse_user_choice::Selection::from(selected_option))
    }
}

mod command_execute {
    use super::*;

    pub async fn delete_selected_branches(
        branches_to_delete: ItemsOwned,
    ) -> CommonResult<CommandRunResult> {
        let (res_output, cmd) = git::try_delete_branches(&branches_to_delete);
        match res_output {
            Ok(output) if output.status.success() => {
                let it = CommandRunResult::RanSuccessfully(
                    user_message_display::fmt_branches_deleted_success_message(
                        &branches_to_delete,
                    ),
                    details::with_details(branches_to_delete.clone()),
                );
                Ok(it)
            }
            Ok(output) => {
                let it = CommandRunResult::RanUnsuccessfully(
                    user_message_display::fmt_error_message(
                        branches_to_delete,
                        Some(output.clone()),
                    ),
                    cmd,
                    output,
                );
                Ok(it)
            }
            Err(error) => {
                let it = CommandRunResult::FailedToRun(
                    user_message_display::fmt_error_message(branches_to_delete, None),
                    cmd,
                    error,
                );
                Ok(it)
            }
        }
    }
}

mod parse_user_choice {
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

mod user_message_display {
    use super::*;

    pub fn fmt_error_message(
        branches: ItemsOwned,
        maybe_output: Option<Output>,
    ) -> String {
        // maybe_output is some.
        if let Some(output) = maybe_output {
            if branches.len() == 1 {
                let branch = &branches[0];
                format!(
                    "{}",
                    fg_guards_red(
                        &UIStrings::FailedToDeleteBranch {
                            branch_name: branch.to_string(),
                            error_message: String::from_utf8_lossy(&output.stderr).into(),
                        }
                        .to_string(),
                    )
                )
            } else {
                let branches = branches.join(",\n ╴");
                format!(
                    "{}",
                    fg_guards_red(
                        &UIStrings::FailedToDeleteBranches {
                            branches,
                            error_message: String::from_utf8_lossy(&output.stderr)
                                .to_string(),
                        }
                        .to_string(),
                    )
                )
            }
        }
        // maybe_output is none.
        else {
            let branches = branches.join(",\n ╴");
            format!(
                "{}",
                fg_guards_red(
                    &UIStrings::FailedToRunCommandToDeleteBranches { branches }
                        .to_string(),
                )
            )
        }
    }

    pub fn fmt_branches_deleted_success_message(branches: &ItemsOwned) -> String {
        if branches.len() == 1 {
            fmt_one_branch_deleted_success_message(branches)
        } else {
            fmt_all_branches_deleted_success_messages(branches)
        }
    }

    fn fmt_one_branch_deleted_success_message(branches: &ItemsOwned) -> String {
        let branch_name = &branches[0].to_string();
        format!(
            " ✅ {a} {b}",
            a = fg_lizard_green(branch_name),
            b = fg_slate_gray(&UIStrings::Deleted.to_string()),
        )
    }

    fn fmt_all_branches_deleted_success_messages(branches: &ItemsOwned) -> String {
        branches
            .iter()
            .map(|branch| {
                format!(
                    " ✅ {a} {b}",
                    a = fg_lizard_green(branch),
                    b = fg_slate_gray(&UIStrings::Deleted.to_string()),
                )
            })
            .collect::<String>()
    }
}
