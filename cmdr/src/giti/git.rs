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

use super::CURRENT_BRANCH_PREFIX;

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

    /// Information about local git branches:
    /// - The currently checked out branch.
    /// - List of other local branches (excluding the current one).
    #[derive(Debug, PartialEq, Clone)]
    pub struct LocalBranchInfo {
        pub current_branch: InlineString,
        pub other_branches: ItemsOwned,
    }

    #[derive(Debug, PartialEq, Clone)]
    pub enum BranchExists {
        Yes,
        No,
    }

    /// Get all the local branches as a tuple.
    ///
    /// 1. The first item in the tuple contains the current branch is prefixed with
    ///    [CURRENT_BRANCH_PREFIX].
    ///
    ///   ```text
    ///   [
    ///     "(◕‿◕) main",
    ///     "tuifyasync",
    ///   ]
    ///   ```
    ///
    /// 2. The second item in the tuple contains [LocalBranchInfo].
    pub async fn try_get_local_branches()
    -> ResultAndCommand<(ItemsOwned, LocalBranchInfo)> {
        let (res, cmd) = try_get_branch_info().await;
        let Ok(info) = res else {
            let report = res.unwrap_err();
            return (Err(report), cmd);
        };

        let mut items_owned = ItemsOwned::with_capacity(info.other_branches.len() + 1);

        // Add current branch with prefix.
        items_owned.push(LocalBranchInfo::mark_branch_current(&info.current_branch));

        // Add other branches as is.
        items_owned.extend(info.other_branches.clone());

        let tuple = (items_owned, info);

        (Ok(tuple), cmd)
    }

    impl LocalBranchInfo {
        pub fn exists_locally(&self, branch_name: &str) -> BranchExists {
            if branch_name == self.current_branch.as_str()
                || self.other_branches.iter().any(|b| b == branch_name)
            {
                BranchExists::Yes
            } else {
                BranchExists::No
            }
        }

        /// ### Input
        /// ```text
        /// "main"
        /// ```
        ///
        /// ### Output
        /// ```text
        /// "(◕‿◕) main"
        /// ```
        pub fn mark_branch_current(branch_name: &str) -> InlineString {
            let mut acc = InlineString::new();
            use std::fmt::Write as _;
            _ = write!(acc, "{CURRENT_BRANCH_PREFIX} {branch_name}");
            acc
        }

        /// ### Input
        /// ```text
        /// "(◕‿◕) main"
        /// ```
        ///
        /// ### Output
        /// ```text
        /// "main"
        /// ```
        pub fn trim_current_prefix_from_branch(branch: &str) -> &str {
            branch.trim_start_matches(CURRENT_BRANCH_PREFIX).trim()
        }
    }

    /// Returns information about local git branches:
    /// 1. The currently checked out branch.
    /// 2. List of other local branches (excluding the current one).
    async fn try_get_branch_info() -> ResultAndCommand<LocalBranchInfo> {
        // Get all branches first
        let (res, cmd) = try_execute_git_command_to_get_branches().await;
        let Ok(all_branches) = res else {
            let report = res.unwrap_err();
            return (Err(report), cmd);
        };

        // Get current branch
        let (res, cmd) = try_get_current_branch_name().await;
        let Ok(current_branch) = res else {
            let report = res.unwrap_err();
            return (Err(report), cmd);
        };

        // Filter out current branch from all branches to get other branches
        let other_branches = all_branches
            .into_iter()
            .filter(|branch| branch != &current_branch)
            .collect();

        let info = LocalBranchInfo {
            current_branch,
            other_branches,
        };

        (Ok(info), cmd)
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
