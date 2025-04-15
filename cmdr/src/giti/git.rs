/*
 *   Copyright (c) 2025 R3BL LLC
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

use std::process::{Command, Output};

use miette::IntoDiagnostic;
use r3bl_core::{CommonResult, InlineString, ItemsOwned};
use smallvec::smallvec;

use super::UIStrings;

/// This is a type alias for the result of a git command. The tuple contains:
/// 1. The result of the command.
/// 2. The command itself.
pub type ResultAndCommand<T> = (CommonResult<T>, Command);

pub fn try_check_for_modified_unstaged_files() -> ResultAndCommand<Output> {
    let mut command = Command::new("git");
    command.args(["status", "--porcelain"]);
    (command.output().into_diagnostic(), command)
}

pub fn try_create_and_switch_to_branch(branch_name: &str) -> ResultAndCommand<Output> {
    let mut command = Command::new("git");
    command.args(["checkout", "-b", branch_name]);
    (command.output().into_diagnostic(), command)
}

pub fn try_delete_branches(branches: &ItemsOwned) -> ResultAndCommand<Output> {
    let mut command = Command::new("git");
    command.args(["branch", "-D"]);
    for branch in branches {
        command.arg(branch.to_string());
    }
    (command.output().into_diagnostic(), command)
}

// Get the current branch name. It is returned in an [r3bl_core::InlineVec] with only a
// single item.
pub fn try_get_current_branch() -> ResultAndCommand<InlineString> {
    let mut command = Command::new("git");
    command.args(["branch", "--show-current"]);

    let result_output = command.output();

    let current_branch = match result_output {
        // Can't even execute output(), something unknown has gone wrong. Propagate the
        // error.
        Err(error) => {
            return (Err(miette::miette!(error)), command);
        }
        Ok(output) => {
            let output_string = String::from_utf8_lossy(&output.stdout);
            output_string.to_string().trim_end_matches('\n').to_string()
        }
    };

    (Ok(current_branch.into()), command)
}

// Get all the local branches. Prefix the current branch with `(current)`.
pub fn try_get_local_branches() -> ResultAndCommand<ItemsOwned> {
    let (res, cmd) = try_execute_git_command_to_get_branches();
    let Ok(branches) = res else {
        return (res, cmd);
    };

    let (res, cmd) = try_get_current_branch();
    let Ok(current_branch) = res else {
        return (Err(miette::miette!(res.unwrap_err())), cmd);
    };

    let mut items_owned = smallvec![];
    for branch in branches {
        match branch == current_branch {
            // If the branch is the current branch, prefix it with "(current)".
            true => {
                items_owned.push(
                    UIStrings::CurrentBranch {
                        branch: branch.to_string(),
                    }
                    .to_string()
                    .into(),
                );
            }
            // If the branch is not the current branch, just add it to the list.
            false => {
                items_owned.push(branch);
            }
        }
    }

    (Ok(items_owned), cmd)
}

fn try_execute_git_command_to_get_branches() -> ResultAndCommand<ItemsOwned> {
    // Create command.
    let mut command = Command::new("git");
    command.args(["branch", "--format", "%(refname:short)"]);

    // Execute command.
    let res_output = command.output();

    // Process command execution results.
    match res_output {
        // Can't even execute output(), something unknown has gone wrong. Propagate the
        // error.
        Err(error) => (Err(miette::miette!(error)), command),
        Ok(output) => {
            let output_string = String::from_utf8_lossy(&output.stdout);
            let mut branches = smallvec![];
            for line in output_string.lines() {
                branches.push(line.into());
            }
            (Ok(branches), command)
        }
    }
}
