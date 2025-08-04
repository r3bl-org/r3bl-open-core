/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

use r3bl_tui::{CommandRunResult, CommonResult, ReadlineAsyncContext, ReadlineEvent};

use crate::giti::{BranchNewDetails, CommandRunDetails, git, local_branch_ops, ui_str};

/// The main function for `giti branch new` command.
///
/// # Errors
///
/// Returns an error if:
/// - Git operations fail
/// - User input fails
/// - Branch creation fails (branch already exists, invalid name, etc.)
pub async fn handle_branch_new_command(
    maybe_branch_name: Option<String>,
) -> CommonResult<CommandRunResult<CommandRunDetails>> {
    match maybe_branch_name {
        Some(branch_name) => command_execute::create_new_branch(branch_name).await,
        None => user_interaction::prompt_for_branch_name().await,
    }
}

mod details {
    use super::{BranchNewDetails, CommandRunDetails};

    pub fn empty() -> CommandRunDetails {
        CommandRunDetails::BranchNew(BranchNewDetails {
            maybe_created_branch: None,
        })
    }

    pub fn with_details(created_branch: String) -> CommandRunDetails {
        CommandRunDetails::BranchNew(BranchNewDetails {
            maybe_created_branch: Some(created_branch),
        })
    }
}

mod command_execute {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub async fn create_new_branch(
        branch_name: String,
    ) -> CommonResult<CommandRunResult<CommandRunDetails>> {
        let (res, _cmd) = local_branch_ops::try_get_local_branches().await;
        let (_, branch_info) = res?;

        if let local_branch_ops::BranchExists::Yes =
            branch_info.exists_locally(&branch_name)
        {
            let string =
                ui_str::branch_create_display::info_branch_already_exists(&branch_name);
            let it = CommandRunResult::Noop(string, details::with_details(branch_name));
            return Ok(it);
        }

        let (res_output, cmd) = git::try_create_and_switch_to_branch(&branch_name).await;
        match res_output {
            Ok(()) => {
                let it = CommandRunResult::Run(
                    ui_str::branch_create_display::info_create_success(&branch_name),
                    details::with_details(branch_name),
                    cmd,
                );
                Ok(it)
            }
            Err(report) => {
                let string =
                    ui_str::branch_create_display::error_failed_to_create_new_branch(
                        &branch_name,
                    );
                let it = CommandRunResult::Fail(string, cmd, report);
                Ok(it)
            }
        }
    }
}

mod user_interaction {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub async fn prompt_for_branch_name()
    -> CommonResult<CommandRunResult<CommandRunDetails>> {
        let prompt_text =
            ui_str::branch_create_display::enter_branch_name_you_want_to_create();

        let mut rl_ctx = ReadlineAsyncContext::try_new(Some(&prompt_text))
            .await?
            .ok_or_else(|| miette::miette!("Failed to create terminal"))?;

        // The loop is just to handle the resize event.
        loop {
            let evt = rl_ctx.read_line().await?;
            match evt {
                ReadlineEvent::Line(branch_name) => {
                    rl_ctx.request_shutdown(None).await?;
                    rl_ctx.await_shutdown().await;

                    return command_execute::create_new_branch(branch_name).await;
                }
                ReadlineEvent::Eof | ReadlineEvent::Interrupted => {
                    rl_ctx.request_shutdown(None).await?;
                    rl_ctx.await_shutdown().await;

                    let it = CommandRunResult::Noop(
                        ui_str::branch_create_display::info_no_branch_created(),
                        details::empty(),
                    );

                    return Ok(it);
                }
                ReadlineEvent::Resized => { /* Do nothing */ }
            }
        }
    }
}
