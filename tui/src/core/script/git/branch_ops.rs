// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{InlineString, ItemsOwned, ResultAndCommand, Run, command,
            script::git::types::{LocalBranchInfo,
                                 git_command_args::{GIT_ARG_CREATE_BRANCH,
                                                    GIT_ARG_DELETE_FORCE,
                                                    GIT_ARG_FORMAT,
                                                    GIT_ARG_REFNAME_SHORT,
                                                    GIT_ARG_SHOW_CURRENT},
                                 git_command_names::{GIT_CMD_BRANCH, GIT_CMD_CHECKOUT,
                                                     GIT_PROGRAM}}};

pub async fn try_get_current_branch_name() -> ResultAndCommand<InlineString> {
    let mut cmd = command!(
        program => GIT_PROGRAM,
        args => GIT_CMD_BRANCH, GIT_ARG_SHOW_CURRENT,
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
        program => GIT_PROGRAM,
        args => GIT_CMD_CHECKOUT, branch_name
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
        program => GIT_PROGRAM,
        args => GIT_CMD_CHECKOUT, GIT_ARG_CREATE_BRANCH, branch_name
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
        program => GIT_PROGRAM,
        args => GIT_CMD_BRANCH, GIT_ARG_DELETE_FORCE,
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

/// Get all the local branches as a tuple.
///
/// 1. The first item in the tuple contains the current branch is prefixed with
///    `CURRENT_BRANCH_PREFIX`.
///
///   ```text
///   [
///     "(◕‿◕) main",
///     "tuifyasync",
///   ]
///   ```
///
/// 2. The second item in the tuple contains [`LocalBranchInfo`].
pub async fn try_get_local_branches() -> ResultAndCommand<(ItemsOwned, LocalBranchInfo)> {
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

/// Returns information about local git branches:
/// 1. The currently checked out branch.
/// 2. List of other local branches (excluding the current one).
async fn try_get_branch_info() -> ResultAndCommand<LocalBranchInfo> {
    // Get all branches first.
    let (res, cmd) = try_execute_git_command_to_get_branches().await;
    let Ok(all_branches) = res else {
        let report = res.unwrap_err();
        return (Err(report), cmd);
    };

    // Get current branch.
    let (res, _cmd) = try_get_current_branch_name().await;
    let Ok(current_branch) = res else {
        let report = res.unwrap_err();
        return (Err(report), cmd);
    };

    // Filter out current branch from all branches to get other branches.
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
        program => GIT_PROGRAM,
        args => GIT_CMD_BRANCH, GIT_ARG_FORMAT, GIT_ARG_REFNAME_SHORT,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{command, inline_vec, ok,
                script::git::{test_fixtures::helper_setup_git_repo_with_commit,
                              types::{BranchExists,
                                      git_ui_strings::CURRENT_BRANCH_PREFIX,
                                      test_config::TEST_ENV_ISOLATED_TEST_RUNNER}},
                try_create_temp_dir_and_cd, with_saved_pwd};

    async fn test_try_get_current_branch_name() -> miette::Result<()> {
        with_saved_pwd!(async {
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
                let _unused: Vec<_> = command!(program => GIT_PROGRAM, args => "checkout", "-b", "feature-branch")
                    .run()
                    .await?;

                // Get current branch name after switch.
                let new_feature_branch = try_get_current_branch_name().await.0?;

                // Verify branch name has changed.
                assert_eq!(new_feature_branch, "feature-branch");
                assert_ne!(new_feature_branch, initial_branch_name);
            } // Drop _temp_dir_root here (which cleans up that folder).

            ok!(())
        })
    }

    async fn test_try_checkout_existing_local_branch() -> miette::Result<()> {
        with_saved_pwd!(async {
            let (
                /* don't drop this immediately using `_` */ _temp_dir_root,
                initial_branch,
            ) = helper_setup_git_repo_with_commit().await?;

            // Create a new branch (without switching to it).
            let _unused: Vec<_> =
                command!(program => GIT_PROGRAM, args => "branch", "test-branch")
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

            ok!(())
        })
    }

    async fn test_try_create_and_switch_to_branch() -> miette::Result<()> {
        with_saved_pwd!(async {
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
            let (_, branch_info) = crate::try_get_local_branches().await.0?;
            assert_eq!(
                branch_info.exists_locally("new-feature"),
                crate::BranchExists::Yes
            );

            ok!(())
        })
    }

    async fn test_try_delete_branches() -> miette::Result<()> {
        with_saved_pwd!(async {
            let (
                /* don't drop this immediately using `_` */ _temp_dir_root,
                initial_branch,
            ) = helper_setup_git_repo_with_commit().await?;

            // Should fail to delete the current branch.
            let res = try_delete_branches(&initial_branch.into()).await.0;
            assert!(res.is_err());

            // Create some branches.
            let _unused: Vec<_> =
                command!(program => GIT_PROGRAM, args => "branch", "branch1")
                    .run()
                    .await?;
            let _unused: Vec<_> =
                command!(program => GIT_PROGRAM, args => "branch", "branch2")
                    .run()
                    .await?;
            let _unused: Vec<_> =
                command!(program => GIT_PROGRAM, args => "branch", "branch3")
                    .run()
                    .await?;

            // Verify branches exist.
            let (_, branch_info) = crate::try_get_local_branches().await.0?;

            assert_eq!(branch_info.exists_locally("main"), crate::BranchExists::Yes);

            assert_eq!(
                branch_info.exists_locally("branch1"),
                crate::BranchExists::Yes
            );
            assert_eq!(
                branch_info.exists_locally("branch2"),
                crate::BranchExists::Yes
            );
            assert_eq!(
                branch_info.exists_locally("branch3"),
                crate::BranchExists::Yes
            );

            // Delete branches.
            let res = try_delete_branches(&inline_vec!["branch1", "branch2"].into())
                .await
                .0;
            assert!(res.is_ok());

            // Verify branches are deleted.
            let (_, branch_info) = crate::try_get_local_branches().await.0?;

            assert_eq!(
                branch_info.exists_locally("branch1"),
                crate::BranchExists::No
            );
            assert_eq!(
                branch_info.exists_locally("branch2"),
                crate::BranchExists::No
            );
            assert_eq!(
                branch_info.exists_locally("branch3"),
                crate::BranchExists::Yes
            );

            ok!(())
        })
    }

    async fn test_try_get_local_branches() -> miette::Result<()> {
        with_saved_pwd!(async {
            let (
                /* don't drop this immediately using `_` */ _temp_dir_root,
                initial_branch,
            ) = helper_setup_git_repo_with_commit().await?;

            // Create some branches.
            let _unused: Vec<_> =
                command!(program => GIT_PROGRAM, args => "branch", "branch1")
                    .run()
                    .await?;
            let _unused: Vec<_> =
                command!(program => GIT_PROGRAM, args => "branch", "branch2")
                    .run()
                    .await?;

            // Get local branches.
            let (items_owned, branch_info) = try_get_local_branches().await.0?;
            // Verify `items_owned` list is correct.
            {
                // Verify current branch is the same as the initial branch.
                assert_eq!(branch_info.current_branch, initial_branch);

                // Verify current branch is marked correctly and is in `items_owned`.
                assert!(items_owned.contains(&LocalBranchInfo::mark_branch_current(
                    initial_branch.as_str()
                )));

                // Verify other branches are in the list.
                assert!(items_owned.iter().any(|branch| branch == "branch1"));
                assert!(items_owned.iter().any(|branch| branch == "branch2"));
            }

            // Verify all branches are in the list.
            assert_eq!(
                branch_info.exists_locally(initial_branch.as_str()),
                BranchExists::Yes
            );
            assert_eq!(branch_info.exists_locally("branch1"), BranchExists::Yes);
            assert_eq!(branch_info.exists_locally("branch2"), BranchExists::Yes);

            // Switch to another branch.
            let _unused: Vec<_> =
                command!(program => GIT_PROGRAM, args => "checkout", "branch1")
                    .run()
                    .await?;

            // Get local branches again.
            let (items_owned, branch_info) = try_get_local_branches().await.0?;
            {
                // Verify the current branch is now "branch1".
                assert_eq!(branch_info.current_branch.as_str(), "branch1");

                // Verify the marked current branch in items_owned contains "branch1".
                assert!(
                    items_owned
                        .contains(&LocalBranchInfo::mark_branch_current("branch1"))
                );

                // Verify other branches are in the list.
                assert!(items_owned.iter().any(|branch| branch == "main"));
                assert!(items_owned.iter().any(|branch| branch == "branch2"));
            }

            ok!(())
        })
    }

    async fn test_local_branch_info_methods() -> miette::Result<()> {
        with_saved_pwd!(async {
            // Test exists_locally method.
            let branch_info = LocalBranchInfo {
                current_branch: "main".into(),
                other_branches: (&["develop", "feature/x"]).into(),
            };

            assert_eq!(branch_info.exists_locally("main"), BranchExists::Yes);
            assert_eq!(branch_info.exists_locally("develop"), BranchExists::Yes);
            assert_eq!(branch_info.exists_locally("feature/x"), BranchExists::Yes);
            assert_eq!(branch_info.exists_locally("nonexistent"), BranchExists::No);

            // Test mark_branch_current method.
            let marked = LocalBranchInfo::mark_branch_current("main");
            assert_eq!(marked, format!("{CURRENT_BRANCH_PREFIX} main"));

            // Test trim_current_prefix_from_branch method.
            let formatted = LocalBranchInfo::mark_branch_current("main");
            let trimmed = LocalBranchInfo::trim_current_prefix_from_branch(&formatted);
            assert_eq!(trimmed, "main");

            // Test trim_current_prefix_from_branch doesn't affect strings without prefix.
            let unchanged = LocalBranchInfo::trim_current_prefix_from_branch("develop");
            assert_eq!(unchanged, "develop");

            ok!(())
        })
    }

    async fn test_try_execute_git_command_to_get_branches() -> miette::Result<()> {
        with_saved_pwd!(async {
            let (
                /* don't drop this immediately using `_` */ _temp_dir_root,
                initial_branch,
            ) = helper_setup_git_repo_with_commit().await?;

            // Create some branches.
            let _unused: Vec<_> =
                command!(program => GIT_PROGRAM, args => "branch", "branch1")
                    .run()
                    .await?;
            let _unused: Vec<_> =
                command!(program => GIT_PROGRAM, args => "branch", "branch2")
                    .run()
                    .await?;

            // Get all branches.
            let branches = try_execute_git_command_to_get_branches().await.0?;

            // Verify all branches are listed.
            assert!(branches.iter().any(|b| b == &initial_branch));
            assert!(branches.iter().any(|b| b == "branch1"));
            assert!(branches.iter().any(|b| b == "branch2"));
            assert_eq!(branches.len(), 3); // initial + 2 created branches

            ok!(())
        })
    }

    async fn run_branch_ops_tests() -> miette::Result<()> {
        test_try_get_current_branch_name().await?;
        test_try_checkout_existing_local_branch().await?;
        test_try_create_and_switch_to_branch().await?;
        test_try_delete_branches().await?;
        test_try_get_local_branches().await?;
        test_local_branch_info_methods().await?;
        test_try_execute_git_command_to_get_branches().await?;
        ok!(())
    }

    #[tokio::test]
    async fn test_branch_ops_in_isolated_process() {
        crate::suppress_wer_dialogs();
        if std::env::var(TEST_ENV_ISOLATED_TEST_RUNNER).is_ok() {
            if let Err(err) = run_branch_ops_tests().await {
                eprintln!("Test failed with error: {err}");
                std::process::exit(1);
            }
            std::process::exit(0);
        }

        let mut cmd = crate::new_isolated_test_command();
        cmd.env(TEST_ENV_ISOLATED_TEST_RUNNER, "1")
            .env("RUST_BACKTRACE", "1")
            .args([
                "--test-threads",
                "1",
                "--nocapture",
                "test_branch_ops_in_isolated_process",
            ]);

        let output = cmd.output().expect("Failed to run isolated test");
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success()
            || stderr.contains("panicked at")
            || stderr.contains("Test failed with error")
        {
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
