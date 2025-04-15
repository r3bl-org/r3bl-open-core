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

use std::process::Output;

use miette::IntoDiagnostic;
use r3bl_core::{CommonResult,
                fg_frozen_blue,
                fg_guards_red,
                fg_lizard_green,
                fg_silver_metallic,
                fg_slate_gray};
use r3bl_tui::{ReadlineAsync, ReadlineEvent};

use crate::giti::{BranchNewDetails,
                  CommandRunDetails,
                  CommandRunResult,
                  git,
                  ui_strings::UIStrings};

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
        let (res, _cmd) = git::try_get_local_branches();
        let branches = res?;
        let branches_trimmed: Vec<String> = branches
            .iter()
            .map(|branch| branch.trim_start_matches("(current) ").to_string())
            .collect();

        if branches_trimmed.contains(&branch_name) {
            let string = user_message_display::fmt_branch_already_exists(&branch_name);
            let it = CommandRunResult::DidNotRun(
                Some(string),
                details::with_details(branch_name),
            );
            return Ok(it);
        }

        let (res_output, cmd) = git::try_create_and_switch_to_branch(&branch_name);

        match res_output {
            Ok(output) if output.status.success() => {
                let it = CommandRunResult::RanSuccessfully(
                    user_message_display::fmt_successful_new_branch_creation(
                        &branch_name,
                    ),
                    details::with_details(branch_name),
                );
                Ok(it)
            }
            Ok(output) => {
                let string = user_message_display::fmt_failed_to_create_new_branch(
                    &branch_name,
                    Some(output.clone()),
                );
                let it = CommandRunResult::RanUnsuccessfully(string, cmd, output);
                Ok(it)
            }
            Err(error) => {
                let string = user_message_display::fmt_failed_to_create_new_branch(
                    &branch_name,
                    None,
                );
                let it = CommandRunResult::FailedToRun(string, cmd, error);
                Ok(it)
            }
        }
    }
}

mod user_message_display {
    use super::*;

    pub fn fmt_branch_already_exists(branch_name: &str) -> String {
        format!(
            "{}",
            fg_slate_gray(
                &UIStrings::BranchAlreadyExists {
                    branch_name: branch_name.to_string(),
                }
                .to_string(),
            )
        )
    }

    pub fn fmt_failed_to_create_new_branch(
        branch_name: &str,
        maybe_output: Option<Output>,
    ) -> String {
        // maybe_output is some.
        if let Some(output) = maybe_output {
            let output_string = String::from_utf8_lossy(&output.stderr).into();
            format!(
                "{}",
                fg_guards_red(
                    &UIStrings::FailedToRunCommandToCreateBranch {
                        branch_name: branch_name.into(),
                        error_message: output_string,
                    }
                    .to_string(),
                )
            )
        }
        // maybe_output is none.
        else {
            format!(
                "{}",
                fg_guards_red(
                    &UIStrings::FailedToCreateAndSwitchToBranch {
                        branch_name: branch_name.into(),
                    }
                    .to_string(),
                )
            )
        }
    }

    pub fn fmt_successful_new_branch_creation(branch_name: &str) -> String {
        format!(
            "{a}{b}",
            a = fg_slate_gray(&UIStrings::CreatedAndSwitchedToNewBranch.to_string()),
            b = fg_lizard_green(&format!("✅ {branch_name}"))
        )
    }

    pub fn fmt_no_new_branch_created() -> String {
        format!(
            "{}",
            fg_silver_metallic(&UIStrings::NoNewBranchWasCreated.to_string())
        )
    }
}

mod user_interaction {
    use super::*;

    pub async fn prompt_for_branch_name() -> CommonResult<CommandRunResult> {
        let prompt_text =
            fg_frozen_blue(&UIStrings::EnterBranchNameYouWantToCreate.to_string())
                .to_string();
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
                        Some(user_message_display::fmt_no_new_branch_created()),
                        details::empty(),
                    );
                    return Ok(it);
                }
                ReadlineEvent::Resized => { /* Do nothing */ }
            }
        }
    }
}
