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
                  CommandExecutionReport,
                  common_types::report_error_and_propagate,
                  git,
                  ui_strings::UIStrings};

/// The main function for `giti branch new` command.
pub async fn try_new(
    maybe_branch_name: Option<String>,
) -> CommonResult<CommandExecutionReport> {
    match maybe_branch_name {
        Some(branch_name) => command_execute::create_new_branch(branch_name),
        None => user_interaction::prompt_for_branch_name().await,
    }
}

mod report {
    use super::*;

    pub fn empty() -> CommonResult<CommandExecutionReport> {
        Ok(CommandExecutionReport::BranchNew(BranchNewDetails {
            maybe_created_branch: None,
        }))
    }

    pub fn with_details(created_branch: String) -> CommonResult<CommandExecutionReport> {
        Ok(CommandExecutionReport::BranchNew(BranchNewDetails {
            maybe_created_branch: Some(created_branch),
        }))
    }
}

mod command_execute {
    use super::*;

    pub fn create_new_branch(branch_name: String) -> CommonResult<CommandExecutionReport> {
        let (res, _cmd) = git::try_get_local_branches();
        let branches = res?;
        let branches_trimmed: Vec<String> = branches
            .iter()
            .map(|branch| branch.trim_start_matches("(current) ").to_string())
            .collect();

        if branches_trimmed.contains(&branch_name) {
            fg_slate_gray(&UIStrings::BranchAlreadyExists { branch_name }.to_string())
                .println();
            return report::empty();
        }

        let (res_output, mut cmd) = git::try_create_and_switch_to_branch(&branch_name);

        match res_output {
            // Command executed successfully.
            Ok(output) if output.status.success() => {
                display_message_to_user::display_successful_new_branch_creation(
                    &branch_name,
                );
                report::with_details(branch_name)
            }
            // Command executed but failed.
            Ok(output) => {
                display_message_to_user::display_failed_to_create_new_branch(
                    &branch_name,
                    Some(output),
                );
                report_error_and_propagate(
                    &mut cmd,
                    miette::miette!("Error creating branch"),
                )
            }
            // Command failed to execute.
            Err(error) => {
                display_message_to_user::display_failed_to_create_new_branch(
                    &branch_name,
                    None,
                );
                report_error_and_propagate(&mut cmd, miette::miette!(error))
            }
        }
    }
}

mod display_message_to_user {
    use super::*;

    pub fn display_failed_to_create_new_branch(
        branch_name: &str,
        maybe_output: Option<Output>,
    ) {
        // maybe_output is some.
        if let Some(output) = maybe_output {
            let output_string = String::from_utf8_lossy(&output.stderr).into();
            fg_guards_red(
                &UIStrings::FailedToRunCommandToCreateBranch {
                    branch_name: branch_name.into(),
                    error_message: output_string,
                }
                .to_string(),
            )
            .println();
        }
        // maybe_output is none.
        else {
            fg_guards_red(
                &UIStrings::FailedToCreateAndSwitchToBranch {
                    branch_name: branch_name.into(),
                }
                .to_string(),
            )
            .println();
        }
    }

    pub fn display_successful_new_branch_creation(branch_name: &str) {
        println!(
            "{a}{b}",
            a = fg_slate_gray(&UIStrings::CreatedAndSwitchedToNewBranch.to_string()),
            b = fg_lizard_green(&format!("✅ {branch_name}"))
        );
    }
}

mod user_interaction {
    use super::*;

    pub async fn prompt_for_branch_name() -> CommonResult<CommandExecutionReport> {
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
                    fg_silver_metallic(&UIStrings::NoNewBranchWasCreated.to_string())
                        .println();
                    return report::empty();
                }
                ReadlineEvent::Resized => { /* Do nothing */ }
            }
        }
    }
}
