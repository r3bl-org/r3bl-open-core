// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Shared test infrastructure for git module tests.

use crate::{InlineString, Run, TempDir, command, ok,
            script::git::types::{git_command_names::{GIT_CMD_ADD, GIT_CMD_COMMIT,
                                                     GIT_CMD_CONFIG, GIT_CMD_INIT,
                                                     GIT_PROGRAM},
                                 git_config_keys::{GIT_CONFIG_COMMIT_GPGSIGN,
                                                   GIT_CONFIG_FLAG_LOCAL,
                                                   GIT_CONFIG_INIT_DEFAULT_BRANCH,
                                                   GIT_CONFIG_USER_EMAIL,
                                                   GIT_CONFIG_USER_NAME},
                                 test_config::{TEST_DEFAULT_BRANCH, TEST_EMAIL,
                                               TEST_GPG_SIGN_DISABLED,
                                               TEST_INITIAL_COMMIT_MSG, TEST_USER_NAME}},
            try_create_temp_dir_and_cd, try_get_current_branch_name, try_write_file};

/// Helper function to setup a basic git repository with an initial commit. Returns a
/// tuple of (`temp_dir_root`, `initial_branch_name`). When the `temp_dir_root` is
/// dropped it will remove that folder.
///
/// This function also uses [`crate::try_cd()`] so make sure to wrap all tests that
/// call this function with [`serial_test`] or use the isolated test runner.
pub async fn helper_setup_git_repo_with_commit() -> miette::Result<(
    /* temp_dir_root: don't drop this immediately using `_` */ TempDir,
    /* initial_branch_name */ InlineString,
)> {
    let (tmp_dir_root, git_folder) = try_create_temp_dir_and_cd!("git_test_repo");

    // First run git init.
    command!(program => GIT_PROGRAM, args => GIT_CMD_INIT)
        .run()
        .await?;

    // Configure initial branch name to be `main`.
    command!(program => GIT_PROGRAM, args => GIT_CMD_CONFIG, GIT_CONFIG_FLAG_LOCAL, GIT_CONFIG_INIT_DEFAULT_BRANCH, TEST_DEFAULT_BRANCH)
        .run()
        .await?;

    // Configure git user for commit. This is necessary to create a commit. This test
    // assumes an environment where no prior local or global git config has been
    // created.
    command!(program => GIT_PROGRAM, args => GIT_CMD_CONFIG, GIT_CONFIG_USER_EMAIL, TEST_EMAIL)
        .run()
        .await?;
    command!(program => GIT_PROGRAM, args => GIT_CMD_CONFIG, GIT_CONFIG_USER_NAME, TEST_USER_NAME)
        .run()
        .await?;

    // Disable commit signing to avoid issues with missing keys in the test
    // environment.
    command!(program => GIT_PROGRAM, args => GIT_CMD_CONFIG, GIT_CONFIG_COMMIT_GPGSIGN, TEST_GPG_SIGN_DISABLED)
        .run()
        .await?;

    // Create and commit a file to have an initial commit.
    try_write_file(git_folder, "initial.txt", "initial content")?;
    command!(program => GIT_PROGRAM, args => GIT_CMD_ADD, "initial.txt")
        .run()
        .await?;
    command!(program => GIT_PROGRAM, args => GIT_CMD_COMMIT, "-m", TEST_INITIAL_COMMIT_MSG)
        .run()
        .await?;

    // Get current branch name.
    let initial_branch = try_get_current_branch_name().await.0?;
    assert_eq!(initial_branch.as_str(), TEST_DEFAULT_BRANCH);

    ok!((tmp_dir_root, initial_branch))
}
