/*
 *   Copyright (c) 2023-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   You may not use this file except in compliance with the License.
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

use r3bl_core::{AnsiStyledText,
                CommonResult,
                InlineString,
                InlineVec,
                ItemsOwned,
                ast,
                ast_line,
                ast_lines,
                height,
                new_style,
                tui_color};
use r3bl_tui::{DefaultIoDevices,
               Header,
               choose,
               readline_async::{HowToChoose, StyleSheet}};

use crate::giti::{BranchCheckoutDetails,
                  CommandRunDetails,
                  CommandRunResult,
                  git::{self},
                  modified_unstaged_file_ops,
                  ui_str};

mod details {
    use super::*;

    pub fn empty() -> CommandRunDetails {
        CommandRunDetails::BranchCheckout(BranchCheckoutDetails {
            maybe_checked_out_branch: None,
        })
    }

    pub fn with_details(branch_name: String) -> CommandRunDetails {
        CommandRunDetails::BranchCheckout(BranchCheckoutDetails {
            maybe_checked_out_branch: Some(branch_name),
        })
    }
}

/// The main function for `giti branch new` command.
pub async fn try_checkout(
    maybe_branch_name: Option<String>,
) -> CommonResult<CommandRunResult> {
    match maybe_branch_name {
        Some(ref branch_name) => {
            command_execute::checkout_branch_if_not_current(branch_name)
        }
        None => user_interaction::handle_branch_selection().await,
    }
}

mod command_execute {
    use super::*;

    pub fn checkout_branch_if_not_current(
        branch_name: &str,
    ) -> CommonResult<CommandRunResult> {
        let (res, _cmd) = git::local_branch_ops::try_get_local_branches();
        let branches = res?;

        // Early return if the branch does not exist locally.
        match git::local_branch_ops::exists_locally(branch_name, branches) {
            git::local_branch_ops::LocalBranch::DoesNotExist => {
                let it = CommandRunResult::DidNotRun(
                    ui_str::branch_checkout_display::error_branch_does_not_exist(
                        branch_name,
                    ),
                    details::empty(),
                );
                return Ok(it);
            }
            _ => { /* do nothing and continue */ }
        }

        // Early return if the branch_name is already checked out.
        let (res, _cmd) = git::try_get_current_branch();
        let current_branch = res?;

        if branch_name == current_branch.as_str() {
            let it = CommandRunResult::DidNotRun(
                ui_str::branch_checkout_display::info_already_on_current_branch(
                    &current_branch,
                ),
                details::empty(),
            );
            return Ok(it);
        }

        // Early return if there are modified files.
        if let (Ok(modified_unstaged_file_ops::ModifiedUnstagedFiles::Exist), _cmd) =
            modified_unstaged_file_ops::try_check()
        {
            let it = CommandRunResult::DidNotRun(

                    ui_str::modified_files_display::warn_modified_files_on_current_branch_exist(
                        branch_name,
                    ),

                details::empty(),
            );
            return Ok(it);
        }

        checkout_branch(branch_name, &current_branch)
    }

    pub fn checkout_branch(
        branch_name: &str,
        current_branch: &str,
    ) -> CommonResult<CommandRunResult> {
        let (res_output, cmd) = git::try_create_and_switch_to_branch(branch_name);
        match res_output {
            Ok(output) if output.status.success() => {
                let it = CommandRunResult::RanSuccessfully(
                    ui_str::branch_checkout_display::info_checkout_success(
                        branch_name,
                        current_branch,
                    ),
                    details::with_details(branch_name.into()),
                    cmd,
                );
                Ok(it)
            }
            Ok(output) => {
                let text =
                    ui_str::branch_checkout_display::error_failed_to_checkout_branch(
                        branch_name,
                        Some(output.clone()),
                    );
                let it = CommandRunResult::RanUnsuccessfully(text, cmd, output);
                Ok(it)
            }
            Err(error) => {
                let string =
                    ui_str::branch_checkout_display::error_failed_to_checkout_branch(
                        branch_name,
                        None,
                    );
                let it = CommandRunResult::FailedToRun(string, cmd, error);
                Ok(it)
            }
        }
    }
}

mod user_interaction {
    use super::*;

    pub async fn handle_branch_selection() -> CommonResult<CommandRunResult> {
        let (res, _cmd) = git::local_branch_ops::try_get_local_branches();
        if let Ok(branches) = res {
            let header = create_branch_selection_header();

            let (res, _cmd) = git::try_get_current_branch();
            let current_branch = res?;

            match prompt_user_to_select_branch(header, branches).await? {
                Some(selected_branch) => {
                    command_execute::checkout_branch(&selected_branch, &current_branch)
                }
                None => {
                    let it: CommandRunResult = CommandRunResult::DidNotRun(
                        ui_str::branch_checkout_display::info_no_suitable_branch_is_available_for_checkout(),
                        details::empty(),
                    );
                    Ok(it)
                }
            }
        } else {
            let it = CommandRunResult::DidNotRun(
                ui_str::branch_checkout_display::info_no_suitable_branch_is_available_for_checkout(),
                details::empty(),
            );
            Ok(it)
        }
    }

    async fn prompt_user_to_select_branch(
        arg_header: impl Into<Header>,
        branches: ItemsOwned,
    ) -> CommonResult<Option<String>> {
        let mut default_io_devices = DefaultIoDevices::default();

        // Remove the current branch from the list of branches.
        let branches_with_current_removed = branches
            .iter()
            .filter(|branch| !branch.contains("(current)"))
            .cloned()
            .collect::<InlineVec<InlineString>>();

        // There are no branches to select from, so return None.
        if branches_with_current_removed.is_empty() {
            return Ok(None);
        }

        let selected_branch = choose(
            arg_header,
            branches_with_current_removed,
            Some(height(20)),
            None,
            HowToChoose::Single,
            StyleSheet::default(),
            default_io_devices.as_mut_tuple(),
        )
        .await?;

        Ok(selected_branch.first().map(|branch| branch.to_string()))
    }

    fn create_branch_selection_header() -> InlineVec<InlineVec<AnsiStyledText>> {
        ast_lines![ast_line![ast(
            ui_str::branch_checkout_display::select_branch_to_switch_to(),
            new_style!(
                color_fg: {tui_color!(frozen_blue)}
                color_bg: {tui_color!(moonlight_blue)}
            )
        )]]
    }
}
