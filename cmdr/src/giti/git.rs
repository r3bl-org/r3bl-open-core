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
use r3bl_core::{CommonResult, InlineString, InlineVec, ItemsOwned};

use super::CURRENT_PREFIX;

/// This is a type alias for the result of a git command. The tuple contains:
/// 1. The result of the command.
/// 2. The command itself.
pub type ResultAndCommand<T> = (CommonResult<T>, Command);

pub mod modified_unstaged_file_ops {
    use super::*;

    pub fn try_check_for_modified_unstaged_files() -> ResultAndCommand<Output> {
        let mut command = Command::new("git");
        command.args(["status", "--porcelain"]);
        (command.output().into_diagnostic(), command)
    }

    #[derive(Debug, Clone, Copy)]
    pub enum ModifiedUnstagedFiles {
        Exist,
        DoNotExist,
    }

    pub fn try_check() -> CommonResult<ModifiedUnstagedFiles> {
        let (res_output, _cmd) = try_check_for_modified_unstaged_files();
        let output = res_output?;
        if output.status.success() && output.stdout.is_empty() {
            Ok(ModifiedUnstagedFiles::DoNotExist)
        } else {
            Ok(ModifiedUnstagedFiles::Exist)
        }
    }

    /// Parses the output of a Git command to extract a list of modified files.
    ///
    /// # Example
    /// ```rust
    /// # use std::os::unix::process::ExitStatusExt;
    /// # use std::process::Output;
    /// # use std::process::ExitStatus;
    /// # use r3bl_cmdr::giti::modified_unstaged_file_ops::get_modified_file_list;
    ///
    /// let output = Output {
    ///     stdout: b"MM file1.txt\nM file2.txt\n file3.txt".to_vec(),
    ///     stderr: vec![],
    ///     status: std::process::ExitStatus::from_raw(0),
    /// };
    ///
    /// let modified_files = get_modified_file_list(output);
    /// assert_eq!(
    ///     modified_files,
    ///     vec![
    ///         "    - file1.txt".to_string(),
    ///         "    - file2.txt".to_string(),
    ///         "    - file3.txt".to_string()
    ///     ]
    /// );
    /// ```
    pub fn get_modified_file_list(output: Output) -> Vec<String> {
        let modified_files = String::from_utf8_lossy(&output.stdout);

        // Early return if there are no modified files.
        if modified_files.is_empty() {
            return vec![];
        }

        let mut acc =
            Vec::with_capacity(/* size hint */ modified_files.lines().count());

        // Remove all the spaces from start and end of each modified file.
        let modified_files_vec = modified_files
            .trim()
            .split('\n')
            .map(|line| line.trim())
            .collect::<Vec<&str>>();

        // Remove all the "MM" and " M" from modified files.
        // "M" means unstaged files. "MM" means staged files.
        for output in &modified_files_vec {
            if output.starts_with("MM ") {
                let modified_output = output.replace("MM", "");
                let modified_output = modified_output.trim_start();
                let modified_output = format!("    - {}", modified_output);
                acc.push(modified_output);
            } else if output.starts_with("M ") {
                let modified_output = output.replace("M ", "");
                let modified_output = modified_output.trim_start();
                let modified_output = format!("    - {}", modified_output);
                acc.push(modified_output);
            } else {
                let modified_output = output.trim_start();
                let modified_output = format!("    - {}", modified_output);
                acc.push(modified_output);
            }
        }

        acc
    }
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

pub mod local_branch_ops {
    use super::*;

    /// Get all the local branches. In the list of branches that are returned, the current
    /// branch is prefixed with [CURRENT_PREFIX].
    pub fn try_get_local_branches() -> ResultAndCommand<ItemsOwned> {
        let (res, cmd) = try_execute_git_command_to_get_branches();
        let Ok(branches) = res else {
            return (res, cmd);
        };

        let (res, cmd) = try_get_current_branch();
        let Ok(current_branch) = res else {
            return (Err(miette::miette!(res.unwrap_err())), cmd);
        };

        let mut items_owned = ItemsOwned::with_capacity(branches.len());
        for branch in branches {
            match branch == current_branch {
                // If the branch is the current branch, prefix it with "(current)".
                true => {
                    items_owned.push(mark_branch_current(&branch));
                }
                // If the branch is not the current branch, just add it to the list.
                false => {
                    items_owned.push(branch);
                }
            }
        }

        (Ok(items_owned), cmd)
    }

    pub fn mark_branch_current(branch_name: &str) -> InlineString {
        let mut acc = InlineString::new();
        use std::fmt::Write as _;
        _ = write!(acc, "{CURRENT_PREFIX} {branch_name}");
        acc
    }

    pub enum LocalBranch {
        Exists,
        DoesNotExist,
    }

    /// Checks if a given branch name exists in the list of branches.
    /// - The list of branches is produced by
    ///   [super::local_branch_ops::try_get_local_branches()].
    /// - The current branch has a [CURRENT_PREFIX] at the start of it. So this prefix is
    ///   removed when the check is performed.
    pub fn exists_locally(branch_name: &str, branches: &ItemsOwned) -> LocalBranch {
        let branches_trimmed = branches
            .iter()
            .map(|branch| branch.trim_start_matches(CURRENT_PREFIX))
            .collect::<InlineVec<&str>>();

        if branches_trimmed.contains(&branch_name) {
            LocalBranch::Exists
        } else {
            LocalBranch::DoesNotExist
        }
    }
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
            let mut branches = ItemsOwned::with_capacity(output_string.lines().count());
            for line in output_string.lines() {
                branches.push(line.into());
            }
            (Ok(branches), command)
        }
    }
}
