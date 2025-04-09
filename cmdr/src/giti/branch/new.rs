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

use std::process::Command;

use miette::IntoDiagnostic;
use r3bl_core::{CommonResult,
                fg_frozen_blue,
                fg_guards_red,
                fg_lizard_green,
                fg_silver_metallic,
                fg_slate_gray};
use r3bl_tui::{ReadlineAsync, ReadlineEvent};

use crate::giti::{self,
                  SuccessReport,
                  clap_config::BranchSubcommand,
                  ui_strings::UIStrings,
                  ui_templates::report_unknown_error_and_propagate};

pub async fn try_make_new_branch(
    maybe_branch_name: Option<String>,
) -> CommonResult<SuccessReport> {
    match maybe_branch_name {
        Some(branch_name) => handle_branch_creation(branch_name),
        None => prompt_for_branch_name().await,
    }
}

fn success_report() -> CommonResult<SuccessReport> {
    Ok(SuccessReport {
        maybe_deleted_branches: None,
        branch_subcommand: Some(BranchSubcommand::New),
    })
}

fn handle_branch_creation(branch_name: String) -> CommonResult<SuccessReport> {
    let branches = giti::try_get_local_branches()?;
    let branches_trimmed: Vec<String> = branches
        .iter()
        .map(|branch| branch.trim_start_matches("(current) ").to_string())
        .collect();

    if branches_trimmed.contains(&branch_name) {
        fg_slate_gray(&UIStrings::BranchAlreadyExists { branch_name }.to_string())
            .println();
        return success_report();
    }

    let git_command =
        &mut create_git_command_to_create_and_switch_to_branch(&branch_name);

    match git_command.output() {
        Ok(output) if output.status.success() => {
            display_successful_new_branch_creation(&branch_name)
        }
        Ok(_) | Err(_) => {
            display_failed_to_create_new_branch(&branch_name);
            return report_unknown_error_and_propagate(
                git_command,
                miette::miette!("Error creating branch"),
            );
        }
    }

    success_report()
}

async fn prompt_for_branch_name() -> CommonResult<SuccessReport> {
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
                return handle_branch_creation(branch_name);
            }
            ReadlineEvent::Eof | ReadlineEvent::Interrupted => {
                rl_async.exit(None).await.into_diagnostic()?;
                fg_silver_metallic(&UIStrings::NoNewBranchWasCreated.to_string())
                    .println();
                return success_report();
            }
            ReadlineEvent::Resized => { /* Do nothing */ }
        }
    }
}

fn display_failed_to_create_new_branch(branch_name: &str) {
    fg_guards_red(
        &UIStrings::FailedToCreateAndSwitchToBranch {
            branch_name: branch_name.to_string(),
        }
        .to_string(),
    )
    .println();
}

fn display_successful_new_branch_creation(branch_name: &str) {
    println!(
        "{a}{b}",
        a = fg_slate_gray(&UIStrings::CreatedAndSwitchedToNewBranch.to_string()),
        b = fg_lizard_green(&format!("✅ {branch_name}"))
    );
}

fn create_git_command_to_create_and_switch_to_branch(branch_name: &str) -> Command {
    let mut command = Command::new("git");
    command.args(["checkout", "-b", branch_name]);
    command
}
