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

use miette::IntoDiagnostic;
use r3bl_core::CommonResult;
use r3bl_tui::{ReadlineAsync, ReadlineEvent};

use crate::giti::{BranchNewDetails, CommandRunDetails, CommandRunResult, git, ui_str};

/// The main function for `giti branch new` command.
pub async fn try_new(
    maybe_branch_name: Option<String>,
) -> CommonResult<CommandRunResult> {
    match maybe_branch_name {
        Some(branch_name) => command_execute::create_new_branch(branch_name),
        None => user_interaction::prompt_for_branch_name().await,
    }
}

mod details {
    use super::*;

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
    use super::*;

    pub fn create_new_branch(branch_name: String) -> CommonResult<CommandRunResult> {
        let (res, _cmd) = git::local_branch_ops::try_get_local_branches();
        let branches = res?;
        let branches_trimmed: Vec<String> = branches
            .iter()
            .map(|branch| branch.trim_start_matches("(current) ").to_string())
            .collect();

        if branches_trimmed.contains(&branch_name) {
            let string =
                ui_str::branch_create_display::info_branch_already_exists(&branch_name);
            let it =
                CommandRunResult::DidNotRun(string, details::with_details(branch_name));
            return Ok(it);
        }

        let (res_output, cmd) = git::try_create_and_switch_to_branch(&branch_name);

        match res_output {
            Ok(output) if output.status.success() => {
                let it = CommandRunResult::RanSuccessfully(
                    ui_str::branch_create_display::info_create_success(&branch_name),
                    details::with_details(branch_name),
                );
                Ok(it)
            }
            Ok(output) => {
                let string =
                    ui_str::branch_create_display::error_failed_to_create_new_branch(
                        &branch_name,
                        Some(output.clone()),
                    );
                let it = CommandRunResult::RanUnsuccessfully(string, cmd, output);
                Ok(it)
            }
            Err(error) => {
                let string =
                    ui_str::branch_create_display::error_failed_to_create_new_branch(
                        &branch_name,
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
    use crate::giti::ui_str;

    pub async fn prompt_for_branch_name() -> CommonResult<CommandRunResult> {
        let prompt_text =
            ui_str::branch_create_display::enter_branch_name_you_want_to_create();
        let mut rl_async = ReadlineAsync::try_new(Some(&prompt_text))?
            .ok_or_else(|| miette::miette!("Failed to create terminal"))?;

        // The loop is just to handle the resize event.
        loop {
            let evt = rl_async.read_line().await?;
            match evt {
                ReadlineEvent::Line(branch_name) => {
                    rl_async.exit(None).await.into_diagnostic()?;
                    return command_execute::create_new_branch(branch_name);
                }
                ReadlineEvent::Eof | ReadlineEvent::Interrupted => {
                    rl_async.exit(None).await.into_diagnostic()?;
                    let it = CommandRunResult::DidNotRun(
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
