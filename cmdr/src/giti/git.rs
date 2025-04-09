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

use std::process::Command;

use r3bl_core::{CommonResult, ItemsOwned};
use smallvec::smallvec;

use super::UIStrings;
use crate::giti::report_unknown_error_and_propagate;

// Get the current branch name.
pub fn try_get_current_branch() -> CommonResult<String> {
    let mut command = Command::new("git");
    let command: &mut Command = command.args(["branch", "--show-current"]);

    let result_output = command.output();

    let current_branch = match result_output {
        // Can't even execute output(), something unknown has gone wrong. Propagate the
        // error.
        Err(error) => {
            return report_unknown_error_and_propagate(command, miette::miette!(error));
        }
        Ok(output) => {
            let output_string = String::from_utf8_lossy(&output.stdout);
            output_string.to_string().trim_end_matches('\n').to_string()
        }
    };

    Ok(current_branch)
}

// Get all the local branches. Prefix the current branch with `(current)`.
pub fn try_get_local_branches() -> CommonResult<ItemsOwned> {
    let branches = try_execute_git_command_to_get_branches()?;

    let current_branch = try_get_current_branch()?;

    let mut branches_vec = smallvec![];
    for branch in branches {
        if branch == current_branch {
            branches_vec.push(UIStrings::CurrentBranch { branch }.to_string().into());
        } else {
            branches_vec.push(branch.into());
        }
    }

    Ok(branches_vec)
}

fn try_execute_git_command_to_get_branches() -> CommonResult<Vec<String>> {
    // Create command.
    let mut command = Command::new("git");
    let command: &mut Command = command.args(["branch", "--format", "%(refname:short)"]);

    // Execute command.
    let result_output = command.output();

    // Process command execution results.
    match result_output {
        // Can't even execute output(), something unknown has gone wrong. Propagate the
        // error.
        Err(error) => report_unknown_error_and_propagate(command, miette::miette!(error)),
        Ok(output) => {
            let output_string = String::from_utf8_lossy(&output.stdout);
            let mut branches = vec![];
            for line in output_string.lines() {
                branches.push(line.to_string());
            }
            Ok(branches)
        }
    }
}
