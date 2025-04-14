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

use std::process::Output;

use branch_checkout_formatting::add_spaces_to_end_of_string;
use r3bl_core::{AnsiStyledText,
                ChUnit,
                CommonResult,
                GCString,
                InlineString,
                InlineVec,
                ItemsOwned,
                ast,
                ast_line,
                ast_lines,
                fg_guards_red,
                fg_lizard_green,
                fg_slate_gray,
                get_terminal_width,
                height,
                new_style,
                tui_color,
                usize};
use r3bl_tui::{DefaultIoDevices,
               Header,
               choose,
               readline_async::{HowToChoose, StyleSheet}};
use smallvec::smallvec;

use crate::giti::{BranchCheckoutDetails,
                  CommandExecutionReport,
                  common_types::report_error_and_propagate,
                  git::{self},
                  ui_strings::UIStrings};

mod report {
    use super::*;

    pub fn empty() -> CommonResult<CommandExecutionReport> {
        Ok(CommandExecutionReport::BranchCheckout(BranchCheckoutDetails {
            maybe_checked_out_branch: None,
        }))
    }

    pub fn with_details(branch_name: String) -> CommonResult<CommandExecutionReport> {
        Ok(CommandExecutionReport::BranchCheckout(BranchCheckoutDetails {
            maybe_checked_out_branch: Some(branch_name),
        }))
    }
}

/// The main function for `giti branch new` command.
pub async fn try_checkout(
    maybe_branch_name: Option<String>,
) -> CommonResult<CommandExecutionReport> {
    match maybe_branch_name {
        Some(branch_name) => command_execute::checkout_branch_if_not_current(branch_name),
        None => user_interaction::handle_branch_selection().await,
    }
}

mod command_execute {
    use super::*;

    pub fn checkout_branch_if_not_current(
        branch_name: String,
    ) -> CommonResult<CommandExecutionReport> {
        let (res, _cmd) = git::try_get_local_branches();
        let branches = res?;

        // Early return if the branch does not exist locally.
        match branch_exists::check(&branches, &branch_name) {
            branch_exists::LocalBranch::DoesNotExist => {
                display_message_to_user::branch_does_not_exist(&branch_name);
                return report::empty();
            }
            _ => { /* do nothing and continue */ }
        }

        // Early return if the branch_name is already checked out.
        let (res, _cmd) = git::try_get_current_branch();
        let current_branch = res?;

        if branch_name == current_branch {
            display_message_to_user::already_on_branch(&current_branch);
            return report::empty();
        }

        // Early return if there are modified files.
        if has_modified_files()? {
            return report::empty();
        }

        checkout_branch(&branch_name, &current_branch)
    }

    pub fn checkout_branch(
        branch_name: &str,
        current_branch: &str,
    ) -> CommonResult<CommandExecutionReport> {
        let (res_output, mut cmd) = git::try_create_and_switch_to_branch(branch_name);
        match res_output {
            // Command executed successfully.
            Ok(output) if output.status.success() => {
                display_checkout_success_message(branch_name, current_branch);
                report::with_details(branch_name.into())
            }
            // Command executed but failed.
            Ok(output) => {
                user_interaction::display_error_message(branch_name, Some(output));
                report_error_and_propagate(
                    &mut cmd,
                    miette::miette!("Error checking out branch"),
                )
            }
            // Command failed to execute.
            Err(error) => {
                user_interaction::display_error_message(branch_name, None);
                report_error_and_propagate(&mut cmd, miette::miette!(error))
            }
        }
    }
}

mod branch_exists {
    use super::*;

    pub enum LocalBranch {
        Exists,
        DoesNotExist,
    }

    pub fn check(branches: &ItemsOwned, branch_name: &str) -> LocalBranch {
        let branches_trimmed = branches
            .iter()
            .map(|branch| branch.trim_start_matches("(current) "))
            .collect::<InlineVec<&str>>();

        if branches_trimmed.contains(&branch_name) {
            LocalBranch::Exists
        } else {
            LocalBranch::DoesNotExist
        }
    }
}

mod display_message_to_user {
    use super::*;

    pub fn already_on_branch(current_branch: &str) {
        println!(
            "{a}{b}",
            a = fg_slate_gray(&UIStrings::AlreadyOnCurrentBranch.to_string()),
            b = fg_lizard_green(current_branch)
        );
    }

    pub fn branch_does_not_exist(branch_name: &str) {
        fg_guards_red(
            &UIStrings::BranchDoesNotExist {
                branch_name: branch_name.to_string(),
            }
            .to_string(),
        )
        .println();
    }

    pub fn no_suitable_branch_is_available_for_checkout() {
        fg_slate_gray(&UIStrings::NoSuitableBranchIsAvailableForCheckout.to_string())
            .println();
    }
}

mod user_interaction {
    use super::*;

    pub fn display_error_message(branch: &str, maybe_output: Option<Output>) {
        match maybe_output {
            Some(output) => {
                fg_guards_red(
                    &UIStrings::FailedToSwitchToBranch {
                        branch: branch.to_string(),
                        error_message: String::from_utf8_lossy(&output.stderr)
                            .to_string(),
                    }
                    .to_string(),
                )
                .println();
            }
            None => {
                fg_guards_red(
                    &UIStrings::NoBranchGotCheckedOut {
                        branch: branch.to_string(),
                    }
                    .to_string(),
                )
                .println();
            }
        }
    }

    pub async fn handle_branch_selection() -> CommonResult<CommandExecutionReport> {
        let (res, _cmd) = git::try_get_local_branches();
        if let Ok(branches) = res {
            let header = create_branch_selection_header();

            let (res, _cmd) = git::try_get_current_branch();
            let current_branch = res?;

            if let Some(selected_branch) =
                prompt_user_to_select_branch(header, branches).await?
            {
                command_execute::checkout_branch(&selected_branch, &current_branch)
            } else {
                report::empty()
            }
        } else {
            report::empty()
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
            display_message_to_user::no_suitable_branch_is_available_for_checkout();
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
            UIStrings::SelectBranchToSwitchTo.to_string(),
            new_style!(
                color_fg: {tui_color!(frozen_blue)}
                color_bg: {tui_color!(moonlight_blue)}
            )
        )]]
    }
}

// 00: [ ] move this to mod modified_files
fn has_modified_files() -> CommonResult<bool> {
    let (res_output, _cmd) = git::try_check_for_modified_unstaged_files();
    if let Ok(output) = res_output {
        if output.status.success() {
            let modified_files =
                branch_checkout_formatting::get_formatted_modified_files(output);
            if !modified_files.is_empty() {
                display_modified_files_warning(&modified_files);
                return Ok(true);
            }
        }
    }

    Ok(false)
}

// 00: [ ] move this to mod modified_files
fn display_modified_files_warning(modified_files: &ItemsOwned) {
    let terminal_width = *get_terminal_width();
    let style = new_style!(
        color_fg: {tui_color!(orange)} color_bg: {tui_color!(night_blue)}
    );

    let message = if modified_files.len() == 1 {
        branch_checkout_formatting::add_spaces_to_end_of_string(
            &UIStrings::ModifiedFileOnCurrentBranch.to_string(),
            terminal_width,
        )
    } else {
        add_spaces_to_end_of_string(
            &UIStrings::ModifiedFilesOnCurrentBranch.to_string(),
            terminal_width,
        )
    };

    ast(&message, style).println();

    for file in modified_files {
        let file = add_spaces_to_end_of_string(file, terminal_width);
        fg_slate_gray(&file).bg_night_blue().println();
    }

    let warning = add_spaces_to_end_of_string(
        &UIStrings::PleaseCommitChangesBeforeSwitchingBranches.to_string(),
        terminal_width,
    );
    ast(&warning, style).println();
}

// 00: [ ] move this to mod modified_files
mod branch_checkout_formatting {
    use super::*;

    pub fn add_spaces_to_end_of_string(string: &str, terminal_width: ChUnit) -> String {
        let string_length = GCString::width(string);
        let spaces_to_add = terminal_width - *string_length;
        let spaces = " ".repeat(usize(spaces_to_add));
        let string = format!("{}{}", string, spaces);
        string
    }

    pub fn get_formatted_modified_files(output: std::process::Output) -> ItemsOwned {
        let mut return_vec = smallvec![];

        let modified_files = String::from_utf8_lossy(&output.stdout).to_string();

        // Early return if there are no modified files.
        if modified_files.is_empty() {
            return return_vec;
        }

        // Remove all the spaces from start and end of each modified file.
        let modified_files = modified_files.trim();
        let modified_files_vec = modified_files
            .split('\n')
            .map(|output| output.trim())
            .collect::<InlineVec<&str>>();

        // Remove all the "MM" and " M" from modified files.
        // "M" means unstaged files. "MM" means staged files.
        for output in &modified_files_vec {
            if output.starts_with("MM ") {
                let modified_output = output.replace("MM", "");
                let modified_output = modified_output.trim_start();
                let modified_output = format!("    - {}", modified_output);
                return_vec.push(modified_output.into());
            } else if output.starts_with("M ") {
                let modified_output = output.replace("M ", "");
                let modified_output = modified_output.trim_start();
                let modified_output = format!("    - {}", modified_output);
                return_vec.push(modified_output.into());
            } else {
                let modified_output = output.trim_start();
                let modified_output = format!("    - {}", modified_output);
                return_vec.push(modified_output.into());
            }
        }
        return_vec
    }
}

// 00: [ ] move this to mod display_message_to_user
fn display_checkout_success_message(branch_name: &str, current_branch: &str) {
    if branch_name == current_branch {
        println!(
            "{a}{b}",
            a = fg_slate_gray(&UIStrings::AlreadyOnCurrentBranch.to_string()),
            b = fg_lizard_green(branch_name)
        );
    } else {
        println!(
            "{a}{b}",
            a = fg_slate_gray(&UIStrings::SwitchedToBranch.to_string()),
            b = fg_lizard_green(branch_name)
        );
    }
}
