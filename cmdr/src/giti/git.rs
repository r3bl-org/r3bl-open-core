// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use r3bl_tui::{CommonResult, InlineString, ItemsOwned, Run, command};
use tokio::process::Command;

use super::CURRENT_BRANCH_PREFIX;

/// This is a type alias for the result of a git command. The tuple contains:
/// 1. The result of the command.
/// 2. The command itself.
pub type ResultAndCommand<T> = (CommonResult<T>, Command);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Information about local git branches:
    /// - The currently checked out branch.
    /// - List of other local branches (excluding the current one).
    #[derive(Debug, PartialEq, Eq, Clone)]
    pub struct LocalBranchInfo {
        pub current_branch: InlineString,
        pub other_branches: ItemsOwned,
    }

    #[derive(Debug, PartialEq, Eq, Clone)]
    pub enum BranchExists {
        Yes,
        No,
    }

    /// Get all the local branches as a tuple.
    ///
    /// 1. The first item in the tuple contains the current branch is prefixed with
    ///    [`CURRENT_BRANCH_PREFIX`].
    ///
    ///   ```text
    ///   [
    ///     "(◕‿◕) main",
    ///     "tuifyasync",
    ///   ]
    ///   ```
    ///
    /// 2. The second item in the tuple contains [`LocalBranchInfo`].
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
        #[must_use]
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
        #[must_use]
        pub fn mark_branch_current(branch_name: &str) -> InlineString {
            use std::fmt::Write;
            let mut acc = InlineString::new();
            // We don't care about the result of this operation.
            write!(acc, "{CURRENT_BRANCH_PREFIX} {branch_name}").ok();
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
        #[must_use]
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

/// These tests are all run in an isolated process to prevent flakiness when tests are run
/// in parallel. The issue is that when these tests are run by cargo test (in parallel in
/// the SAME process), it leads to undefined behavior and flaky test failures, since the
/// current working directory is changed per process, and all the tests are running in
/// parallel in the same process.
///
/// By running all these tests in an isolated process, we ensure that any changes to
/// the current working directory are completely isolated and cannot affect other tests.
#[cfg(test)]
mod tests {
    use r3bl_tui::{TempDir, inline_string, inline_vec, ok, try_create_temp_dir_and_cd,
                   try_write_file, with_saved_pwd};

    use super::*;

    /// Helper function to setup a basic git repository with an initial commit Returns a
    /// tuple of (`temp_dir_root`, `git_folder_path`, `initial_branch_name`). When the
    /// `temp_dir_root` is dropped it will clean remove that folder.
    ///
    /// This function also uses [`r3bl_tui::try_cd()`] so make sure to wrap all tests that
    /// call this function with [`serial_test`].
    async fn helper_setup_git_repo_with_commit() -> miette::Result<(
        /* temp_dir_root: don't drop this immediately using `_` */ TempDir,
        /* initial_branch_name */ InlineString,
    )> {
        let (tmp_dir_root, git_folder) = try_create_temp_dir_and_cd!("git_test_repo");

        // First run git init.
        command!(program => "git", args => "init").run().await?;

        // Configure initial branch name to be `main`.
        command!(program => "git", args => "config", "--local", "init.defaultBranch", "main")
            .run()
            .await?;

        // Configure git user for commit. This is necessary to create a commit. This test
        // assumes an environment where no prior local or global git config has been
        // created.
        command!(program => "git", args => "config", "user.email", "test@example.com")
            .run()
            .await?;
        command!(program => "git", args => "config", "user.name", "Test User")
            .run()
            .await?;

        // Create and commit a file to have an initial commit.
        try_write_file(git_folder, "initial.txt", "initial content")?;
        command!(program => "git", args => "add", "initial.txt")
            .run()
            .await?;
        command!(program => "git", args => "commit", "-m", "Initial commit")
            .run()
            .await?;

        // Get current branch name.
        let initial_branch = try_get_current_branch_name().await.0?;
        assert_eq!(initial_branch.as_str(), "main");

        ok!((tmp_dir_root, initial_branch))
    }

    // Tests [super::try_is_working_directory_clean()] function to verify it correctly
    // detects clean and dirty repository states. Does not use
    // [helper_setup_git_repo_with_commit()] helper function.
    async fn test_try_is_working_directory_clean() -> miette::Result<()> {
        with_saved_pwd!({
            let (_temp_dir_root, git_folder) =
                try_create_temp_dir_and_cd!("test_git_folder");

            // Assert that running command will error out before git init.
            assert!(try_is_working_directory_clean().await.0.is_err());

            // Run git init.
            let _unused: Vec<_> =
                command!(program => "git", args => "init").run().await?;

            // Assert that the working directory is clean after git init.
            assert_eq!(try_is_working_directory_clean().await.0?, RepoStatus::Clean);

            // Create a file.
            try_write_file(git_folder, "test_file.txt", "test content")?;

            // Assert that the working directory is dirty after creating a file.
            assert_eq!(try_is_working_directory_clean().await.0?, RepoStatus::Dirty);

            // Stage the file.
            let _unused: Vec<_> =
                command!(program => "git", args => "add", "test_file.txt")
                    .run()
                    .await?;

            // Repo is still dirty (changes are staged but not committed).
            assert_eq!(try_is_working_directory_clean().await.0?, RepoStatus::Dirty);

            // Configure git user for commit. This is necessary to create a commit. This
            // test assumes an environment where no prior local or global git
            // config has been created.
            let _unused: Vec<_> = command!(program => "git", args => "config", "user.email", "test@example.com")
                .run().await?;
            let _unused: Vec<_> =
                command!(program => "git", args => "config", "user.name", "Test User")
                    .run()
                    .await?;

            // Commit the changes.
            let _unused: Vec<_> =
                command!(program => "git", args => "commit", "-m", "Initial commit")
                    .run()
                    .await?;

            // Assert that the working directory is clean after committing.
            assert_eq!(try_is_working_directory_clean().await.0?, RepoStatus::Clean);

            ok!()
        })
    }

    // Tests [super::try_get_current_branch_name()] function to verify it correctly
    // retrieves the current branch name.
    async fn test_try_get_current_branch_name() -> miette::Result<()> {
        with_saved_pwd!({
            // Create an empty temp dir to verify that running the command fails.
            {
                // Create a temp dir and cd to it.
                let _temp_dir_root = try_create_temp_dir_and_cd!();

                // Assert that running command will error out before `git init` is run.
                assert!(try_get_current_branch_name().await.0.is_err());
            } // Drop temp_dir_root here (which cleans up that folder).

            // Setup new git folder.
            {
                let (
                    /* don't drop this immediately using `_` */ _temp_dir_root,
                    initial_branch_name,
                ) = helper_setup_git_repo_with_commit().await?;

                // Get initial branch name.
                let name = try_get_current_branch_name().await.0?;
                assert_eq!(name, initial_branch_name);

                // Create and switch to a new branch.
                let _unused: Vec<_> = command!(program => "git", args => "checkout", "-b", "feature-branch")
                    .run()
                    .await?;

                // Get current branch name after switch.
                let new_feature_branch = try_get_current_branch_name().await.0?;

                // Verify branch name has changed.
                assert_eq!(new_feature_branch, "feature-branch");
                assert_ne!(new_feature_branch, initial_branch_name);
            } // Drop _temp_dir_root here (which cleans up that folder).

            ok!()
        })
    }

    // Tests [super::try_checkout_existing_local_branch()] function to verify it
    // correctly switches to an existing branch.
    async fn test_try_checkout_existing_local_branch() -> miette::Result<()> {
        with_saved_pwd!({
            let (
                /* don't drop this immediately using `_` */ _temp_dir_root,
                initial_branch,
            ) = helper_setup_git_repo_with_commit().await?;

            // Create a new branch (without switching to it).
            let _unused: Vec<_> =
                command!(program => "git", args => "branch", "test-branch")
                    .run()
                    .await?;

            // Checkout the test branch.
            let res = try_checkout_existing_local_branch("test-branch").await.0;
            assert!(res.is_ok());

            // Get current branch after checkout.
            let current_branch = try_get_current_branch_name().await.0?;

            // Verify current branch is now the test branch.
            assert_eq!(current_branch, "test-branch");
            assert_ne!(current_branch, initial_branch);

            // Try to checkout a non-existent branch (should fail).
            let res = try_checkout_existing_local_branch("nonexistent-branch")
                .await
                .0;
            assert!(res.is_err());

            ok!()
        })
    }

    // Tests [super::try_create_and_switch_to_branch()], and
    // [super::local_branch_ops::try_get_local_branches] function to verify it correctly
    // creates a new branch and switches to it.
    async fn test_try_create_and_switch_to_branch() -> miette::Result<()> {
        with_saved_pwd!({
            let (
                /* don't drop this immediately using `_` */ _temp_dir_root,
                initial_branch,
            ) = helper_setup_git_repo_with_commit().await?;

            // Create a new branch and switch to it.
            let res = try_create_and_switch_to_branch("new-feature").await.0;
            assert!(res.is_ok());

            // Get current branch after creating and switching.
            let current_branch = try_get_current_branch_name().await.0?;

            // Verify current branch is now the new branch.
            assert_eq!(current_branch, "new-feature");
            assert_ne!(current_branch, initial_branch);

            // Ensure the branch exists in the list of branches.
            let (_, branch_info) = local_branch_ops::try_get_local_branches().await.0?;
            assert_eq!(
                branch_info.exists_locally("new-feature"),
                local_branch_ops::BranchExists::Yes
            );

            ok!()
        })
    }

    // Tests [super::try_delete_branches()] and
    // [super::local_branch_ops::try_get_local_branches()] functions to verify it
    // correctly deletes branches.
    async fn test_try_delete_branches() -> miette::Result<()> {
        with_saved_pwd!({
            let (
                /* don't drop this immediately using `_` */ _temp_dir_root,
                initial_branch,
            ) = helper_setup_git_repo_with_commit().await?;

            // Should fail to delete the current branch.
            let res = try_delete_branches(&initial_branch.into()).await.0;
            assert!(res.is_err());

            // Create some branches.
            let _unused: Vec<_> = command!(program => "git", args => "branch", "branch1")
                .run()
                .await?;
            let _unused: Vec<_> = command!(program => "git", args => "branch", "branch2")
                .run()
                .await?;
            let _unused: Vec<_> = command!(program => "git", args => "branch", "branch3")
                .run()
                .await?;

            // Verify branches exist.
            let (_, branch_info) = local_branch_ops::try_get_local_branches().await.0?;

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
            let res = try_delete_branches(&inline_vec!["branch1", "branch2"].into())
                .await
                .0;
            assert!(res.is_ok());

            // Verify branches are deleted
            let (_, branch_info) = local_branch_ops::try_get_local_branches().await.0?;

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

            ok!()
        })
    }

    // Tests [super::local_branch_ops::try_get_local_branches()] function to verify it
    // correctly lists branches and distinguishes the current branch.
    async fn test_try_get_local_branches() -> miette::Result<()> {
        with_saved_pwd!({
            let (
                /* don't drop this immediately using `_` */ _temp_dir_root,
                initial_branch,
            ) = helper_setup_git_repo_with_commit().await?;

            // Create some branches.
            let _unused: Vec<_> = command!(program => "git", args => "branch", "branch1")
                .run()
                .await?;
            let _unused: Vec<_> = command!(program => "git", args => "branch", "branch2")
                .run()
                .await?;

            // Get local branches.
            let (items_owned, branch_info) =
                local_branch_ops::try_get_local_branches().await.0?;
            // Verify `items_owned` list is correct.
            {
                // Verify current branch is the same as the initial branch.
                assert_eq!(branch_info.current_branch, initial_branch);

                // Verify current branch is `(◕‿◕) main` and is in `items_owned`.
                assert!(items_owned.contains(&inline_string!(
                    "{CURRENT_BRANCH_PREFIX} {initial_branch}"
                )));

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
            let _unused: Vec<_> =
                command!(program => "git", args => "checkout", "branch1")
                    .run()
                    .await?;

            // Get local branches again.
            let (items_owned, branch_info) =
                local_branch_ops::try_get_local_branches().await.0?;
            {
                // Verify the current branch is now "branch1".
                assert_eq!(branch_info.current_branch.as_str(), "branch1");

                // Verify the marked current branch in items_owned contains "branch1".
                assert!(
                    items_owned
                        .contains(&inline_string!("{CURRENT_BRANCH_PREFIX} branch1"))
                );

                // Verify other branches are in the list.
                assert!(items_owned.iter().any(|branch| branch == "main"));
                assert!(items_owned.iter().any(|branch| branch == "branch2"));
            }

            ok!()
        })
    }

    // Tests [local_branch_ops::LocalBranchInfo] methods including `exists_locally()`,
    // `mark_branch_current()`, and `trim_current_prefix_from_branch()`.
    fn test_local_branch_info_methods() -> miette::Result<()> {
        with_saved_pwd!({
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
            let trimmed =
                local_branch_ops::LocalBranchInfo::trim_current_prefix_from_branch(
                    &formatted,
                );
            assert_eq!(trimmed, "main");

            // Test trim_current_prefix_from_branch doesn't affect strings without prefix.
            let unchanged =
                local_branch_ops::LocalBranchInfo::trim_current_prefix_from_branch(
                    "develop",
                );
            assert_eq!(unchanged, "develop");

            ok!()
        })
    }

    // Tests [super::local_branch_ops::try_execute_git_command_to_get_branches()]
    // function to verify it correctly lists all branches from the git repository.
    async fn test_try_execute_git_command_to_get_branches() -> miette::Result<()> {
        with_saved_pwd!({
            let (
                /* don't drop this immediately using `_` */ _temp_dir_root,
                initial_branch,
            ) = helper_setup_git_repo_with_commit().await?;

            // Create some branches
            let _unused: Vec<_> = command!(program => "git", args => "branch", "branch1")
                .run()
                .await?;
            let _unused: Vec<_> = command!(program => "git", args => "branch", "branch2")
                .run()
                .await?;

            // Get all branches
            let branches =
                super::local_branch_ops::try_execute_git_command_to_get_branches()
                    .await
                    .0?;

            // Verify all branches are listed
            assert!(branches.iter().any(|b| b == &initial_branch));
            assert!(branches.iter().any(|b| b == "branch1"));
            assert!(branches.iter().any(|b| b == "branch2"));
            assert_eq!(branches.len(), 3); // initial + 2 created branches

            ok!()
        })
    }

    // XMARK: Process isolated test

    /// This function runs all the tests that change the current working directory
    /// sequentially. This ensures that the current working directory is
    /// only changed in a controlled manner, eliminating flakiness when tests are run in
    /// parallel.
    ///
    /// This function is called by `test_all_git_functions_in_isolated_process()` to run
    /// the tests in an isolated process.
    async fn run_all_git_tests_sequentially_impl() -> miette::Result<()> {
        // Run each test in sequence
        test_try_is_working_directory_clean().await?;
        test_try_get_current_branch_name().await?;
        test_try_checkout_existing_local_branch().await?;
        test_try_create_and_switch_to_branch().await?;
        test_try_delete_branches().await?;
        test_try_get_local_branches().await?;
        test_local_branch_info_methods()?;
        test_try_execute_git_command_to_get_branches().await?;

        ok!()
    }

    /// This test function runs all the tests that change the current working directory
    /// in an isolated process. This ensures that the current working directory is
    /// only changed in a completely isolated environment, eliminating any potential
    /// side effects on other tests running in parallel.
    ///
    /// The issue is that when these tests are run by cargo test (in parallel in the SAME
    /// process), it leads to undefined behavior and flaky test failures, since the
    /// current working directory is changed per process, and all the tests are
    /// running in parallel in the same process.
    ///
    /// By running all these tests in an isolated process, we ensure that any changes to
    /// the current working directory are completely isolated and cannot affect other
    /// tests.
    #[tokio::test]
    async fn test_all_git_functions_in_isolated_process() {
        if std::env::var("ISOLATED_TEST_RUNNER").is_ok() {
            // This is the actual test running in the isolated process
            if let Err(err) = run_all_git_tests_sequentially_impl().await {
                eprintln!("Test failed with error: {err}");
                std::process::exit(1);
            }
            // If we reach here without errors, exit normally
            std::process::exit(0);
        }

        // This is the test coordinator - spawn the actual test in a new process
        let current_exe = std::env::current_exe().unwrap();
        let mut cmd = std::process::Command::new(&current_exe);
        cmd.env("ISOLATED_TEST_RUNNER", "1")
            .env("RUST_BACKTRACE", "1") // Get better error info
            .args([
                "--test-threads",
                "1",
                "test_all_git_functions_in_isolated_process",
            ]);

        let output = cmd.output().expect("Failed to run isolated test");

        // Check if the child process exited successfully or if there's a panic message in
        // stderr
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success()
            || stderr.contains("panicked at")
            || stderr.contains("Test failed with error")
        {
            // These statements are important for IDEs providing hyperlinks to the line in
            // the test sources above which failed.
            eprintln!("Exit status: {:?}", output.status);
            eprintln!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
            eprintln!("Stderr: {stderr}");

            panic!(
                "Isolated test failed with status code {:?}: {}",
                output.status.code(),
                stderr
            );
        }
    }
}
