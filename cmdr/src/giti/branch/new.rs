/*
 *   Copyright (c) 2024 R3BL LLC
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

use r3bl_ansi_color::{AnsiStyledText, Style};
use r3bl_core::CommonResult;
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};

use crate::{color_constants::DefaultColors::{FrozenBlue,
                                             GuardsRed,
                                             LizardGreen,
                                             SilverMetallic,
                                             SlateGray},
            giti::{self,
                   CommandSuccessfulResponse,
                   UIStrings::{BranchAlreadyExists,
                               CreatedAndSwitchedToNewBranch,
                               EnterBranchNameYouWantToCreate,
                               FailedToCreateAndSwitchToBranch,
                               NoNewBranchWasCreated},
                   clap_config::BranchSubcommand,
                   report_unknown_error_and_propagate}};

pub fn try_make_new_branch(
    maybe_branch_name: Option<String>,
) -> CommonResult<CommandSuccessfulResponse> {
    let response = CommandSuccessfulResponse {
        maybe_deleted_branches: None,
        branch_subcommand: Some(BranchSubcommand::New),
    };

    match maybe_branch_name {
        Some(branch_name) => {
            // If this branch already exists, then show error message.
            let branches = giti::get_branches()?;
            let branches_trimmed: Vec<String> = branches
                .iter()
                .map(|branch| branch.trim_start_matches("(current) ").to_string())
                .collect();
            if branches_trimmed.contains(&branch_name) {
                let branch_already_exists =
                    BranchAlreadyExists { branch_name }.to_string();
                AnsiStyledText {
                    text: &branch_already_exists,
                    style: &[Style::Foreground(SlateGray.as_ansi_color())],
                }
                .println();
                return Ok(response);
            }

            // If this branch doesn't exist, then create it. Create a git command to
            // create a new branch and check it out.
            let git_command_to_create_and_switch_to_branch: &mut Command =
                &mut create_git_command_to_create_and_switch_to_branch(&branch_name);
            let result_create_new_branch =
                git_command_to_create_and_switch_to_branch.output();
            match result_create_new_branch {
                Ok(new_branch_output) => {
                    if new_branch_output.status.success() {
                        display_successful_new_branch_creation(&branch_name);
                    } else {
                        display_failed_to_create_new_branch(&branch_name);
                    }
                }
                Err(error) => {
                    display_failed_to_create_new_branch(&branch_name);
                    return report_unknown_error_and_propagate(
                        git_command_to_create_and_switch_to_branch,
                        error,
                    );
                }
            }
        }
        None => {
            let mut line_editor = Reedline::create();
            let prompt_text = AnsiStyledText {
                text: &EnterBranchNameYouWantToCreate.to_string(),
                style: &[Style::Foreground(FrozenBlue.as_ansi_color())],
            }
            .to_string();
            let prompt = DefaultPrompt::new(
                DefaultPromptSegment::Basic(prompt_text),
                DefaultPromptSegment::Empty,
            );

            // Ask the user to type in the name of the branch they want to create.
            let result_signal = line_editor.read_line(&prompt);
            match result_signal {
                Ok(Signal::Success(branch_name)) => {
                    let git_command_to_create_and_switch_to_branch: &mut Command =
                        &mut create_git_command_to_create_and_switch_to_branch(
                            &branch_name,
                        );
                    let result_create_new_branch =
                        git_command_to_create_and_switch_to_branch.output();
                    match result_create_new_branch {
                        Ok(new_branch_output) => {
                            if new_branch_output.status.success() {
                                display_successful_new_branch_creation(&branch_name);
                            } else {
                                display_failed_to_create_new_branch(&branch_name);
                            }
                        }
                        Err(error) => {
                            display_failed_to_create_new_branch(&branch_name);
                            return report_unknown_error_and_propagate(
                                git_command_to_create_and_switch_to_branch,
                                error,
                            );
                        }
                    }
                }
                Ok(Signal::CtrlC) => {
                    AnsiStyledText {
                        text: &NoNewBranchWasCreated.to_string(),
                        style: &[Style::Foreground(SilverMetallic.as_ansi_color())],
                    }
                    .println();
                }
                _ => {}
            }
        }
    }

    Ok(response)
}

fn display_failed_to_create_new_branch(branch_name: &str) {
    AnsiStyledText {
        text: &FailedToCreateAndSwitchToBranch {
            branch_name: branch_name.to_string(),
        }
        .to_string(),
        style: &[Style::Foreground(GuardsRed.as_ansi_color())],
    }
    .println();
}

fn display_successful_new_branch_creation(branch_name: &str) {
    let created_and_switched_to_new_branch = AnsiStyledText {
        text: &CreatedAndSwitchedToNewBranch.to_string(),
        style: &[Style::Foreground(SlateGray.as_ansi_color())],
    };
    let branch_name = AnsiStyledText {
        text: &format!("✅ {branch_name}"),
        style: &[Style::Foreground(LizardGreen.as_ansi_color())],
    };
    println!("{created_and_switched_to_new_branch}{branch_name}");
}

fn create_git_command_to_create_and_switch_to_branch(branch_name: &str) -> Command {
    let mut command_to_create_and_switch_to_branch = Command::new("git");
    command_to_create_and_switch_to_branch.args(["checkout", "-b", branch_name]);
    command_to_create_and_switch_to_branch
}
