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
use r3bl_tui::{CommandRunResult, CommonResult, DefaultIoDevices, ast, ast_line, choose,
               height, inline_vec,
               readline_async::{HowToChoose, StyleSheet}};

use crate::{giti::{BranchCheckoutDetails, CommandRunDetails, RepoStatus,
                   git::{self},
                   try_is_working_directory_clean, ui_str},
            prefix_single_select_instruction_header};

/// The main function for `giti branch checkout` command.
pub async fn handle_branch_checkout_command(
    maybe_branch_name: Option<String>,
) -> CommonResult<CommandRunResult<CommandRunDetails>> {
    match maybe_branch_name {
        Some(ref branch_name) => {
            command_execute::checkout_branch_if_not_current(branch_name).await
        }
        None => user_interaction::handle_branch_selection().await,
    }
}

mod details {
    use super::{BranchCheckoutDetails, CommandRunDetails};

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

mod command_execute {
    use super::{CommandRunDetails, CommandRunResult, CommonResult, RepoStatus, details,
                git, try_is_working_directory_clean, ui_str};

    pub async fn checkout_branch_if_not_current(
        branch_name: &str,
    ) -> CommonResult<CommandRunResult<CommandRunDetails>> {
        use git::local_branch_ops::BranchExists;

        let (res, _cmd) = git::local_branch_ops::try_get_local_branches().await;
        let (_, branch_info) = res?;

        // Early return if the branch does not exist locally.
        match branch_info.exists_locally(branch_name) {
            BranchExists::No => {
                let it = CommandRunResult::Noop(
                    ui_str::branch_checkout_display::error_branch_does_not_exist_msg(
                        branch_name,
                    ),
                    details::empty(),
                );
                return Ok(it);
            }
            BranchExists::Yes => { /* do nothing and continue */ }
        }

        // Early return if the branch_name is already checked out.
        let (res, _cmd) = git::try_get_current_branch_name().await;
        let current_branch = res?;

        if branch_name == current_branch.as_str() {
            let it = CommandRunResult::Noop(
                ui_str::branch_checkout_display::info_already_on_current_branch_msg(
                    &current_branch,
                ),
                details::empty(),
            );
            return Ok(it);
        }

        // Early return if there are modified files.
        if let (Ok(RepoStatus::Dirty), _cmd) = try_is_working_directory_clean().await {
            let it = CommandRunResult::Noop(
                ui_str::modified_files_display::warn_modified_files_exist_msg(
                    branch_name,
                ),
                details::empty(),
            );
            return Ok(it);
        }

        checkout_branch(branch_name, &current_branch).await
    }

    pub async fn checkout_branch(
        branch_name: &str,
        current_branch: &str,
    ) -> CommonResult<CommandRunResult<CommandRunDetails>> {
        let (res_output, cmd) =
            git::try_checkout_existing_local_branch(branch_name).await;
        match res_output {
            Ok(()) => {
                let it = CommandRunResult::Run(
                    ui_str::branch_checkout_display::info_checkout_success_msg(
                        branch_name,
                        current_branch,
                    ),
                    details::with_details(branch_name.into()),
                    cmd,
                );
                Ok(it)
            }
            Err(report) => {
                let it = CommandRunResult::Fail(
                    ui_str::branch_checkout_display::error_failed_to_checkout_branch_msg(
                        branch_name,
                    ),
                    cmd,
                    report,
                );
                Ok(it)
            }
        }
    }
}

mod user_interaction {
    use super::{CommandRunDetails, CommandRunResult, CommonResult, DefaultIoDevices,
                HowToChoose, StyleSheet, ast, ast_line, choose, command_execute,
                details, git, height, inline_vec,
                prefix_single_select_instruction_header, ui_str};

    pub async fn handle_branch_selection()
    -> CommonResult<CommandRunResult<CommandRunDetails>> {
        let (res, _cmd) = git::local_branch_ops::try_get_local_branches().await;

        // Early return if the command fails.
        let Ok((_, branch_info)) = res else {
            return Ok(noop());
        };

        // Early return if there are no branches to select from.
        if branch_info.other_branches.is_empty() {
            return Ok(noop());
        }

        let header_with_instructions = {
            let last_line = ast_line![ast(
                ui_str::branch_checkout_display::select_branch_to_switch_to_msg_raw(),
                crate::common::ui_templates::header_style_default()
            )];
            prefix_single_select_instruction_header(inline_vec![last_line])
        };
        let mut default_io_devices = DefaultIoDevices::default();
        let maybe_user_choice = choose(
            header_with_instructions,
            branch_info.other_branches,
            Some(height(20)),
            None,
            HowToChoose::Single,
            StyleSheet::default(),
            default_io_devices.as_mut_tuple(),
        )
        .await? // Propagate UI errors (e.g., I/O errors).
        .into_iter() // Convert InlineVec<InlineString> to iterator.
        .next(); // Get the first (and only) selected item, if any.

        // Early return if the user did not select a branch.
        let Some(user_choice) = maybe_user_choice else {
            return Ok(noop());
        };

        // Actually checkout the user selected branch.
        command_execute::checkout_branch(&user_choice, &branch_info.current_branch).await
    }

    fn noop() -> CommandRunResult<CommandRunDetails> {
        CommandRunResult::Noop(
            ui_str::branch_checkout_display::no_suitable_branch_available_msg(),
            details::empty(),
        )
    }
}
