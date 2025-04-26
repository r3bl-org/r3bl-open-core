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

use r3bl_tui::{CommonResult,
               InlineString,
               InlineVec,
               ItemsOwned,
               Run,
               command,
               inline_string};
use tokio::process::Command;

use super::CURRENT_PREFIX;

/// This is a type alias for the result of a git command. The tuple contains:
/// 1. The result of the command.
/// 2. The command itself.
pub type ResultAndCommand<T> = (CommonResult<T>, Command);

pub mod modified_unstaged_file_ops {
    use super::*;

    #[derive(Debug, Clone, Copy)]
    pub enum ModifiedUnstagedFiles {
        Exist,
        None,
    }

    /// Similar to [try_get_modified_file_list()], but returns [ModifiedUnstagedFiles] state
    /// indicating if there are any modified files.
    pub async fn try_check_exists() -> ResultAndCommand<ModifiedUnstagedFiles> {
        let mut cmd = command!(
            program => "git",
            args => "status", "--porcelain"
        );

        let res_output = cmd.run().await;
        let Ok(output) = res_output else {
            let report = res_output.unwrap_err();
            let err = Err(report);
            return (err, cmd);
        };

        let status = if output.is_empty() {
            ModifiedUnstagedFiles::None
        } else {
            ModifiedUnstagedFiles::Exist
        };

        (Ok(status), cmd)
    }

    /// Similar to [try_check_exists()], but returns a list of modified files.
    pub async fn try_get_modified_file_list() -> ResultAndCommand<InlineVec<InlineString>>
    {
        let mut cmd = command!(
            program => "git",
            args => "status", "--porcelain"
        );

        let res_output = cmd.run().await;
        let Ok(output) = res_output else {
            let report = res_output.unwrap_err();
            let err = Err(report);
            return (err, cmd);
        };

        let modified_files = get_modified_file_list_from_output(output);

        (Ok(modified_files), cmd)
    }

    /// Parses the [std::process:Output]'s `stdout` of a git command to extract a list of
    /// modified files.
    ///
    /// Here's output from this command `git status --porcelain`:
    /// ```text
    /// M  1.code-search
    ///  M cmdr/src/giti/branch/checkout.rs
    ///  M cmdr/src/giti/branch/delete.rs
    ///  M cmdr/src/giti/branch/new.rs
    /// MM cmdr/src/giti/common_types.rs
    ///  M cmdr/src/giti/git.rs
    ///  M core/src/script/command_runner.rs
    /// MM todo.md
    /// ```
    fn get_modified_file_list_from_output(output: Vec<u8>) -> InlineVec<InlineString> {
        let modified_files = String::from_utf8_lossy(&output);

        // Early return if there are no modified files.
        if modified_files.is_empty() {
            return InlineVec::new();
        }

        let mut acc =
            InlineVec::with_capacity(/* size hint */ modified_files.lines().count());

        let lines = modified_files
            .lines()
            .map(|line| line.trim_start().trim())
            .collect::<InlineVec<&str>>();

        // Replace all the "MM " and "M " from modified filenames and replace with prefix.
        // "M" means unstaged files. "MM" means staged files.
        for line in &lines {
            if line.starts_with("MM ") {
                acc.push(inline_string!(
                    "    - {}",
                    line.strip_prefix("MM ").unwrap()
                ));
            } else if line.starts_with("M ") {
                acc.push(inline_string!(
                    "    - {}",
                    line.strip_prefix("M ").unwrap_or(line)
                ));
            } else {
                acc.push(inline_string!("    - {}", line));
            }
        }

        acc
    }
}

pub async fn try_create_and_switch_to_branch(branch_name: &str) -> ResultAndCommand<()> {
    let mut cmd = command!(
        program => "git",
        args => "checkout", "-b", branch_name
    );

    let res_output = cmd.run().await;
    let Ok(_) = res_output else {
        let report = res_output.unwrap_err();
        let err = Err(report);
        return (err, cmd);
    };

    (Ok(()), cmd)
}

pub async fn try_delete_branches(branches: &ItemsOwned) -> ResultAndCommand<()> {
    let mut cmd = command!(
        program => "git",
        args => "branch", "-D",
        + items => branches
    );

    let res_output = cmd.run().await;
    let Ok(_) = res_output else {
        let report = res_output.unwrap_err();
        let err = Err(report);
        return (err, cmd);
    };

    (Ok(()), cmd)
}

pub async fn try_get_current_branch_name() -> ResultAndCommand<InlineString> {
    let mut cmd = command!(
        program => "git",
        args => "branch", "--show-current",
    );

    let res_output = cmd.run().await;
    let Ok(output) = res_output else {
        let report = res_output.unwrap_err();
        let err = Err(report);
        return (err, cmd);
    };

    let current_branch = String::from_utf8_lossy(&output)
        .trim_end_matches('\n')
        .to_string();

    (Ok(current_branch.into()), cmd)
}

pub mod local_branch_ops {
    use super::*;

    /// Get all the local branches. In the list of branches that are returned, the current
    /// branch is prefixed with [CURRENT_PREFIX].
    ///
    /// ```
    /// [
    ///     "(◕‿◕) main",
    ///     "tuifyasync",
    /// ]
    /// ```
    pub async fn try_get_local_branch_names_with_current_marked()
    -> ResultAndCommand<ItemsOwned> {
        // Get branches.
        let (res, cmd) = try_execute_git_command_to_get_branches().await;
        let Ok(branches) = res else {
            let report = res.unwrap_err();
            let err = Err(report);
            return (err, cmd);
        };

        // Get current branch.
        let (res, cmd) = try_get_current_branch_name().await;
        let Ok(current_branch) = res else {
            let report = res.unwrap_err();
            let err = Err(report);
            return (err, cmd);
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

    /// ### Input
    /// ```
    /// "main"
    /// ```
    ///
    /// ### Output
    /// ```
    /// "(◕‿◕) main"
    /// ```
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

    /// ### Input
    /// ```
    /// [
    ///     "(◕‿◕) main",
    ///     "tuifyasync",
    /// ]
    /// ```
    ///
    /// ### Output
    /// ```
    /// [
    ///     "main",
    ///     "tuifyasync",
    /// ]
    /// ```
    pub fn filter_current_branch_prefix_from_branches(
        branches: &ItemsOwned,
    ) -> InlineVec<&str> {
        branches
            .iter()
            .map(|item| trim_current_prefix_from_branch(item))
            .collect()
    }

    // Add this function in local_branch_ops mod:
    pub fn trim_current_prefix_from_branch(branch: &str) -> &str {
        branch.trim_start_matches(CURRENT_PREFIX).trim()
    }

    /// Checks if a given branch name exists in the list of branches.
    /// - The list of branches is produced by [super::local_branch_ops::try_get_local_branch_names_with_current_marked()].
    /// - The current branch has a [CURRENT_PREFIX] at the start of it. So this prefix is removed when
    ///   the check is performed.
    pub fn exists_locally(branch_name: &str, branches: ItemsOwned) -> LocalBranch {
        let branches_trimmed = filter_current_branch_prefix_from_branches(&branches);
        if branches_trimmed.contains(&branch_name) {
            LocalBranch::Exists
        } else {
            LocalBranch::DoesNotExist
        }
    }
}

async fn try_execute_git_command_to_get_branches() -> ResultAndCommand<ItemsOwned> {
    let mut cmd = command!(
        program => "git",
        args => "branch", "--format", "%(refname:short)",
    );

    let res_output = cmd.run().await;
    let Ok(output) = res_output else {
        let report = res_output.unwrap_err();
        let err = Err(report);
        return (err, cmd);
    };

    let output_string = String::from_utf8_lossy(&output);
    let mut branches = ItemsOwned::with_capacity(output_string.lines().count());
    for line in output_string.lines() {
        branches.push(line.into());
    }
    (Ok(branches), cmd)
}
