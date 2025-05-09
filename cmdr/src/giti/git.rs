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

use r3bl_tui::{CommonResult, InlineString, ItemsOwned, Run, command};
use tokio::process::Command;

use super::CURRENT_BRANCH_PREFIX;

/// This is a type alias for the result of a git command. The tuple contains:
/// 1. The result of the command.
/// 2. The command itself.
pub type ResultAndCommand<T> = (CommonResult<T>, Command);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RepoStatus {
    Dirty,
    Clean,
}

/// Runs `git status --porcelain` and reports whether the git repo is clean or not. It is
/// not clean if files exist that aren't committed yet, and are staged, unstaged,
/// untracked.
pub async fn try_is_working_directory_clean() -> ResultAndCommand<RepoStatus> {
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
        RepoStatus::Clean
    } else {
        RepoStatus::Dirty
    };

    (Ok(status), cmd)
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

pub async fn try_checkout_existing_local_branch(
    branch_name: &str,
) -> ResultAndCommand<()> {
    let mut cmd = command!(
        program => "git",
        args => "checkout", branch_name
    );

    let res_output = cmd.run().await;
    let Ok(_) = res_output else {
        let report = res_output.unwrap_err();
        let err = Err(report);
        return (err, cmd);
    };

    (Ok(()), cmd)
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

/// This is a wrapper over `git branch` functionality.
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

    pub(super) async fn try_execute_git_command_to_get_branches()
    -> ResultAndCommand<ItemsOwned> {
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
}

/// This module contains test functions for validating various Git-related operations,
/// such as checking the working directory status, retrieving the current branch name,
/// switching branches, and managing temporary directories for tests.
///
/// The code leverages `tokio::test` for asynchronous testing and uses helper functions
/// and temporary directory utilities to simulate Git repositories. Each test ensures
/// the target functionality behaves as expected in both valid and error scenarios.
///
/// # Modules
///
/// - **helper_setup_git_repo_with_commit**: A utility function to set up a Git repository
///   with an initial commit. This helper is used to reduce boilerplate for initializing
///   Git repositories in multiple tests.
///
/// # Tests
///
/// - **test_try_is_working_directory_clean**: Validates the
///   `try_is_working_directory_clean` function:
///     - Before Git initialization, the function should return an error.
///     - After initialization and creating files, it should correctly identify the
///       working directory as clean or dirty.
///     - Staged but uncommitted changes should mark the repository as dirty, reverting to
///       clean after a commit.
///
/// - **test_try_get_current_branch_name**: Confirms that the
///   `try_get_current_branch_name` function works as intended for retrieving the active
///   branch name:
///     - Fails when executed outside a Git repository.
///     - Successfully retrieves the branch name in an initialized repository.
///     - Verifies the branch name updates correctly when switching between branches.
///
/// - **test_try_checkout_existing_local_branch**: Ensures the
///   `try_checkout_existing_local_branch` function works as expected for switching
///   branches:
///     - Successfully checks out an existing branch.
///     - Fails gracefully when trying to check out a non-existent branch.
///
/// # Dependencies:
///
/// - **TempDir**: Used to create and manage temporary directories during testing.
///   Automatically cleans up directories when dropped.
///
/// - **fs_paths!**: Macro for handling file and folder paths conveniently during test
///   setup.
///
/// - **commands!**: Utility macro for running shell commands (`git` in this case)
///   asynchronously.
///
/// - **r3bl_tui**: Provides helper utilities such as temporary directory management and
///   path creation.
///
/// # Note:
///
/// Each test ensures proper cleanup of temporary folders upon completion. Temporary Git
/// repositories are automatically removed to maintain an isolated test environment.
///
/// Errors encountered during asynchronous calls (e.g., invalid Git operations) are
/// correctly propagated and asserted to verify robustness in edge cases.
#[cfg(test)]
mod tests {
    use std::fs::write;

    use r3bl_tui::{MkdirOptions::CreateIntermediateDirectories,
                   TempDir,
                   create_temp_dir,
                   fs_paths,
                   inline_string,
                   try_cd,
                   try_mkdir};

    use super::*;

    /// Tests [super::try_is_working_directory_clean()] function to verify it correctly
    /// detects clean and dirty repository states. Does not use
    /// [helper_setup_git_repo_with_commit()] helper function.
    #[tokio::test]
    async fn test_try_is_working_directory_clean() {
        let tmp_dir_root = create_temp_dir().unwrap();

        // Setup git folder.
        let git_folder = fs_paths!(with_root: tmp_dir_root => "test_git_folder");
        try_mkdir(&git_folder, CreateIntermediateDirectories).unwrap();

        // Change to this folder.
        try_cd(&git_folder).unwrap();

        // Assert that running command will error out before git init.
        assert!(try_is_working_directory_clean().await.0.is_err());

        // Run git init.
        _ = command!(program => "git", args => "init")
            .run()
            .await
            .unwrap();

        // Assert that the working directory is clean after git init.
        assert_eq!(
            try_is_working_directory_clean().await.0.unwrap(),
            RepoStatus::Clean
        );

        // Create a file.
        write(
            fs_paths!(with_root: git_folder => "test_file.txt"),
            "test content",
        )
        .unwrap();

        // Assert that the working directory is dirty after creating a file.
        assert_eq!(
            try_is_working_directory_clean().await.0.unwrap(),
            RepoStatus::Dirty
        );

        // Stage the file.
        _ = command!(program => "git", args => "add", "test_file.txt")
            .run()
            .await
            .unwrap();

        // Repo is still dirty (changes are staged but not committed).
        assert_eq!(
            try_is_working_directory_clean().await.0.unwrap(),
            RepoStatus::Dirty
        );

        // Configure git user for commit.
        _ = command!(program => "git", args => "config", "user.email", "test@example.com")
            .run()
            .await
            .unwrap();
        _ = command!(program => "git", args => "config", "user.name", "Test User")
            .run()
            .await
            .unwrap();

        // Commit the changes.
        _ = command!(program => "git", args => "commit", "-m", "Initial commit")
            .run()
            .await
            .unwrap();

        // Assert that the working directory is clean after committing.
        assert_eq!(
            try_is_working_directory_clean().await.0.unwrap(),
            RepoStatus::Clean
        );
    } // Drop temp_dir_root here (which cleans up that folder).

    /// Helper function to setup a basic git repository with an initial commit Returns a
    /// tuple of (temp_dir_root, git_folder_path, initial_branch_name). When the
    /// `temp_dir_root` is dropped it will clean remove that folder.
    async fn helper_setup_git_repo_with_commit() -> (
        /* temp_dir_root: don't drop this immediately using `_` */ TempDir,
        /* initial_branch_name */ InlineString,
    ) {
        let tmp_dir_root = create_temp_dir().unwrap();

        // Setup git folder.
        let git_folder = fs_paths!(with_root: tmp_dir_root => "git_test_repo");
        try_mkdir(&git_folder, CreateIntermediateDirectories).unwrap();

        // Change to this folder.
        try_cd(&git_folder).unwrap();

        // First run git init.
        _ = command!(program => "git", args => "init")
            .run()
            .await
            .unwrap();

        // Configure initial branch name to be `main`.
        _ = command!(program => "git", args => "config", "--local", "init.defaultBranch", "main")
            .run()
            .await
            .unwrap();

        // Configure git user for commit.
        _ = command!(program => "git", args => "config", "user.email", "test@example.com")
            .run()
            .await
            .unwrap();
        _ = command!(program => "git", args => "config", "user.name", "Test User")
            .run()
            .await
            .unwrap();

        // Create and commit a file to have an initial commit.
        write(
            fs_paths!(with_root: git_folder => "initial.txt"),
            "initial content",
        )
        .unwrap();
        _ = command!(program => "git", args => "add", "initial.txt")
            .run()
            .await
            .unwrap();
        _ = command!(program => "git", args => "commit", "-m", "Initial commit")
            .run()
            .await
            .unwrap();

        // Get current branch name.
        let initial_branch = try_get_current_branch_name().await.0.unwrap();

        assert_eq!(initial_branch.as_str(), "main");

        (tmp_dir_root, initial_branch)
    }

    /// Tests [super::try_get_current_branch_name()] function to verify it correctly
    /// retrieves the current branch name.
    #[tokio::test]
    async fn test_try_get_current_branch_name() {
        // Create an empty temp dir to verify that running the command fails.
        {
            // Create a temporary directory.
            let temp_dir_root = create_temp_dir().unwrap();

            // Setup git sub folder.
            let git_folder = fs_paths!(with_root: temp_dir_root => "test_current_branch");
            try_mkdir(&git_folder, CreateIntermediateDirectories).unwrap();

            // Change to this sub folder.
            try_cd(&git_folder).unwrap();

            // Assert that running command will error out before `git init` is run.
            assert!(try_get_current_branch_name().await.0.is_err());
        } // Drop temp_dir_root here (which cleans up that folder).

        // Setup new git folder.
        {
            let (
                /* don't drop this immediately using `_` */ _temp_dir_root,
                initial_branch_name,
            ) = helper_setup_git_repo_with_commit().await;

            // Get initial branch name.
            let (res, _) = try_get_current_branch_name().await;
            {
                let name = res.unwrap();
                assert_eq!(name, initial_branch_name);
            }

            // Create and switch to a new branch.
            _ = command!(program => "git", args => "checkout", "-b", "feature-branch")
                .run()
                .await
                .unwrap();

            // Get current branch name after switch.
            let (res, _) = try_get_current_branch_name().await;
            let new_feature_branch = res.unwrap();

            // Verify branch name has changed.
            assert_eq!(new_feature_branch, "feature-branch");
            assert_ne!(new_feature_branch, initial_branch_name);
        } // Drop _temp_dir_root here (which cleans up that folder).
    }

    /// Tests [super::try_checkout_existing_local_branch()] function to verify it
    /// correctly switches to an existing branch.
    #[tokio::test]
    async fn test_try_checkout_existing_local_branch() {
        let (
            /* don't drop this immediately using `_` */ _temp_dir_root,
            initial_branch,
        ) = helper_setup_git_repo_with_commit().await;

        // Create a new branch (without switching to it).
        _ = command!(program => "git", args => "branch", "test-branch")
            .run()
            .await
            .unwrap();

        // Checkout the test branch.
        let (res, _) = try_checkout_existing_local_branch("test-branch").await;
        assert!(res.is_ok());

        // Get current branch after checkout.
        let (res, _) = try_get_current_branch_name().await;
        let current_branch = res.unwrap();

        // Verify current branch is now the test branch.
        assert_eq!(current_branch, "test-branch");
        assert_ne!(current_branch, initial_branch);

        // Try to checkout a non-existent branch (should fail).
        let (res, _) = try_checkout_existing_local_branch("nonexistent-branch").await;
        assert!(res.is_err());
    } // Drop _temp_dir_root and clean up folder.

    /// Tests [super::try_create_and_switch_to_branch()], and
    /// [super::local_branch_ops::try_get_local_branches] function to verify it correctly
    /// creates a new branch and switches to it.
    #[tokio::test]
    async fn test_try_create_and_switch_to_branch() {
        let (
            /* don't drop this immediately using `_` */ _temp_dir_root,
            initial_branch,
        ) = helper_setup_git_repo_with_commit().await;

        // Create a new branch and switch to it.
        let (res, _) = try_create_and_switch_to_branch("new-feature").await;
        assert!(res.is_ok());

        // Get current branch after creating and switching.
        let (res, _) = try_get_current_branch_name().await;
        let current_branch = res.unwrap();

        // Verify current branch is now the new branch.
        assert_eq!(current_branch, "new-feature");
        assert_ne!(current_branch, initial_branch);

        // Check that the branch exists in the list of branches.
        let (res, _) = local_branch_ops::try_get_local_branches().await;
        let (_, branch_info) = res.unwrap();

        assert_eq!(
            branch_info.exists_locally("new-feature"),
            local_branch_ops::BranchExists::Yes
        );
    } // Drop _temp_dir_root and clean up folder.

    /// Tests [super::try_delete_branches()] and
    /// [super::local_branch_ops::try_get_local_branches()] functions to verify it
    /// correctly deletes branches.
    #[tokio::test]
    async fn test_try_delete_branches() {
        let (
            /* don't drop this immediately using `_` */ _temp_dir_root,
            initial_branch,
        ) = helper_setup_git_repo_with_commit().await;

        // Should fail to delete the current branch.
        let (res, _) = try_delete_branches(&initial_branch.into()).await;
        assert!(res.is_err());

        // Create some branches.
        _ = command!(program => "git", args => "branch", "branch1")
            .run()
            .await
            .unwrap();
        _ = command!(program => "git", args => "branch", "branch2")
            .run()
            .await
            .unwrap();
        _ = command!(program => "git", args => "branch", "branch3")
            .run()
            .await
            .unwrap();

        // Verify branches exist.
        let (res, _) = local_branch_ops::try_get_local_branches().await;
        let (_, branch_info) = res.unwrap();

        assert_eq!(
            branch_info.exists_locally("main"),
            local_branch_ops::BranchExists::Yes
        );

        assert_eq!(
            branch_info.exists_locally("branch1"),
            local_branch_ops::BranchExists::Yes
        );
        assert_eq!(
            branch_info.exists_locally("branch2"),
            local_branch_ops::BranchExists::Yes
        );
        assert_eq!(
            branch_info.exists_locally("branch3"),
            local_branch_ops::BranchExists::Yes
        );

        // Delete branches.
        let (res, _) = try_delete_branches(&(&["branch1", "branch2"]).into()).await;
        assert!(res.is_ok());

        // Verify branches are deleted
        let (res, _) = local_branch_ops::try_get_local_branches().await;
        let (_, branch_info) = res.unwrap();

        assert_eq!(
            branch_info.exists_locally("branch1"),
            local_branch_ops::BranchExists::No
        );
        assert_eq!(
            branch_info.exists_locally("branch2"),
            local_branch_ops::BranchExists::No
        );
        assert_eq!(
            branch_info.exists_locally("branch3"),
            local_branch_ops::BranchExists::Yes
        );
    } // Drop _temp_dir_root and clean up folder.

    /// Tests [super::local_branch_ops::try_get_local_branches()] function to verify it
    /// correctly lists branches and distinguishes the current branch.
    #[tokio::test]
    async fn test_try_get_local_branches() {
        let (
            /* don't drop this immediately using `_` */ _temp_dir_root,
            initial_branch,
        ) = helper_setup_git_repo_with_commit().await;

        // Create some branches.
        _ = command!(program => "git", args => "branch", "branch1")
            .run()
            .await
            .unwrap();
        _ = command!(program => "git", args => "branch", "branch2")
            .run()
            .await
            .unwrap();

        // Get local branches.
        let (res, _) = local_branch_ops::try_get_local_branches().await;
        let (items_owned, branch_info) = res.unwrap();
        // Verify `items_owned` list is correct.
        {
            // Verify current branch is the same as the initial branch.
            assert_eq!(branch_info.current_branch, initial_branch);

            // Verify current branch is `(◕‿◕) main` and is in `items_owned`.
            assert!(
                items_owned.contains(&inline_string!(
                    "{CURRENT_BRANCH_PREFIX} {initial_branch}"
                ))
            );

            // Verify other branches are in the list.
            assert!(items_owned.iter().any(|branch| branch == "branch1"));
            assert!(items_owned.iter().any(|branch| branch == "branch2"));
        }

        // Verify all branches are in the list.
        assert_eq!(
            branch_info.exists_locally(initial_branch.as_str()),
            local_branch_ops::BranchExists::Yes
        );
        assert_eq!(
            branch_info.exists_locally("branch1"),
            local_branch_ops::BranchExists::Yes
        );
        assert_eq!(
            branch_info.exists_locally("branch2"),
            local_branch_ops::BranchExists::Yes
        );

        // Switch to another branch.
        _ = command!(program => "git", args => "checkout", "branch1")
            .run()
            .await
            .unwrap();

        // Get local branches again.
        let (res, _) = local_branch_ops::try_get_local_branches().await;
        let (items_owned, branch_info) = res.unwrap();
        {
            // Verify the current branch is now "branch1".
            assert_eq!(branch_info.current_branch.as_str(), "branch1");

            // Verify the marked current branch in items_owned contains "branch1".
            assert!(
                items_owned.contains(&inline_string!("{CURRENT_BRANCH_PREFIX} branch1"))
            );

            // Verify other branches are in the list.
            assert!(items_owned.iter().any(|branch| branch == "main"));
            assert!(items_owned.iter().any(|branch| branch == "branch2"));
        }
    } // Drop _temp_dir_root and clean up folder.

    /// Tests [local_branch_ops::LocalBranchInfo] methods including `exists_locally()`,
    /// `mark_branch_current()`, and `trim_current_prefix_from_branch()`.
    #[tokio::test]
    async fn test_local_branch_info_methods() {
        // Test exists_locally method.
        let branch_info = local_branch_ops::LocalBranchInfo {
            current_branch: "main".into(),
            other_branches: (&["develop", "feature/x"]).into(),
        };

        assert_eq!(
            branch_info.exists_locally("main"),
            local_branch_ops::BranchExists::Yes
        );
        assert_eq!(
            branch_info.exists_locally("develop"),
            local_branch_ops::BranchExists::Yes
        );
        assert_eq!(
            branch_info.exists_locally("feature/x"),
            local_branch_ops::BranchExists::Yes
        );
        assert_eq!(
            branch_info.exists_locally("nonexistent"),
            local_branch_ops::BranchExists::No
        );

        // Test mark_branch_current method.
        let marked = local_branch_ops::LocalBranchInfo::mark_branch_current("main");
        assert_eq!(marked, inline_string!("{CURRENT_BRANCH_PREFIX} main"));

        // Test trim_current_prefix_from_branch method.
        let formatted = inline_string!("{CURRENT_BRANCH_PREFIX} main");
        let trimmed = local_branch_ops::LocalBranchInfo::trim_current_prefix_from_branch(
            &formatted,
        );
        assert_eq!(trimmed, "main");

        // Test trim_current_prefix_from_branch doesn't affect strings without prefix.
        let unchanged =
            local_branch_ops::LocalBranchInfo::trim_current_prefix_from_branch("develop");
        assert_eq!(unchanged, "develop");
    } // Drop _temp_dir_root and clean up folder.

    /// Tests [super::local_branch_ops::try_execute_git_command_to_get_branches()]
    /// function to verify it correctly lists all branches from the git repository.
    #[tokio::test]
    async fn test_try_execute_git_command_to_get_branches() {
        let (
            /* don't drop this immediately using `_` */ _temp_dir_root,
            initial_branch,
        ) = helper_setup_git_repo_with_commit().await;

        // Create some branches
        _ = command!(program => "git", args => "branch", "branch1")
            .run()
            .await
            .unwrap();
        _ = command!(program => "git", args => "branch", "branch2")
            .run()
            .await
            .unwrap();

        // Get all branches
        let (res, _) =
            super::local_branch_ops::try_execute_git_command_to_get_branches().await;
        let branches = res.unwrap();

        // Verify all branches are listed
        assert!(branches.iter().any(|b| b == &initial_branch));
        assert!(branches.iter().any(|b| b == "branch1"));
        assert!(branches.iter().any(|b| b == "branch2"));
        assert_eq!(branches.len(), 3); // initial + 2 created branches
    }
} // Drop _temp_dir_root and clean up folder.
