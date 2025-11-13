// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{RepoStatus, ResultAndCommand, Run, command,
            script::git::types::{git_command_args::{GIT_ARG_GIT_DIR, GIT_ARG_PORCELAIN},
                                 git_command_names::{GIT_CMD_REV_PARSE,
                                                     GIT_CMD_STATUS, GIT_PROGRAM}}};

/// Runs `git status --porcelain` and reports whether the git repo is clean or not. It is
/// not clean if files exist that aren't committed yet, and are staged, unstaged,
/// untracked.
pub async fn try_is_working_directory_clean() -> ResultAndCommand<RepoStatus> {
    let mut cmd = command!(
        program => GIT_PROGRAM,
        args => GIT_CMD_STATUS, GIT_ARG_PORCELAIN
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

/// Check if we're in a git repository.
pub async fn try_is_git_repo() -> ResultAndCommand<bool> {
    let mut cmd = command!(
        program => GIT_PROGRAM,
        args => GIT_CMD_REV_PARSE, GIT_ARG_GIT_DIR
    );

    let res_output = cmd.run().await;
    let is_repo = res_output.map(|_output| true).unwrap_or(false);

    (Ok(is_repo), cmd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{command, ok,
                script::git::{test_fixtures::helper_setup_git_repo_with_commit,
                              types::{git_command_names::{GIT_CMD_ADD, GIT_CMD_COMMIT,
                                                           GIT_CMD_CONFIG, GIT_CMD_INIT},
                                       git_config_keys::{GIT_CONFIG_COMMIT_GPGSIGN,
                                                         GIT_CONFIG_USER_EMAIL,
                                                         GIT_CONFIG_USER_NAME},
                                       test_config::{TEST_EMAIL,
                                                     TEST_ENV_ISOLATED_TEST_RUNNER,
                                                     TEST_GPG_SIGN_DISABLED,
                                                     TEST_INITIAL_COMMIT_MSG,
                                                     TEST_USER_NAME}}},
                try_create_temp_dir_and_cd, try_write_file, with_saved_pwd};

    async fn test_try_is_working_directory_clean() -> miette::Result<()> {
        with_saved_pwd!(async {
            let (_temp_dir_root, git_folder) =
                try_create_temp_dir_and_cd!("test_git_folder");

            // Assert that running command will error out before git init.
            assert!(try_is_working_directory_clean().await.0.is_err());

            // Run git init.
            let _unused: Vec<_> = command!(program => GIT_PROGRAM, args => GIT_CMD_INIT)
                .run()
                .await?;

            // Assert that the working directory is clean after git init.
            assert_eq!(try_is_working_directory_clean().await.0?, RepoStatus::Clean);

            // Create a file.
            try_write_file(git_folder, "test_file.txt", "test content")?;

            // Assert that the working directory is dirty after creating a file.
            assert_eq!(try_is_working_directory_clean().await.0?, RepoStatus::Dirty);

            // Stage the file.
            let _unused: Vec<_> =
                command!(program => GIT_PROGRAM, args => GIT_CMD_ADD, "test_file.txt")
                    .run()
                    .await?;

            // Repo is still dirty (changes are staged but not committed).
            assert_eq!(try_is_working_directory_clean().await.0?, RepoStatus::Dirty);

            // Configure git user for commit. This is necessary to create a commit. This
            // test assumes an environment where no prior local or global git
            // config has been created.
            let _unused: Vec<_> = command!(program => GIT_PROGRAM, args => GIT_CMD_CONFIG, GIT_CONFIG_USER_EMAIL, TEST_EMAIL)
                .run().await?;
            let _unused: Vec<_> =
                command!(program => GIT_PROGRAM, args => GIT_CMD_CONFIG, GIT_CONFIG_USER_NAME, TEST_USER_NAME)
                    .run()
                    .await?;

            // Disable commit signing to avoid issues with missing keys in the test
            // environment.
            let _unused: Vec<_> =
                command!(program => GIT_PROGRAM, args => GIT_CMD_CONFIG, GIT_CONFIG_COMMIT_GPGSIGN, TEST_GPG_SIGN_DISABLED)
                    .run()
                    .await?;

            // Commit the changes.
            let _unused: Vec<_> =
                command!(program => GIT_PROGRAM, args => GIT_CMD_COMMIT, "-m", TEST_INITIAL_COMMIT_MSG)
                    .run()
                    .await?;

            // Assert that the working directory is clean after committing.
            assert_eq!(try_is_working_directory_clean().await.0?, RepoStatus::Clean);

            ok!(())
        })
    }

    async fn test_try_is_git_repo() -> miette::Result<()> {
        with_saved_pwd!(async {
            // Test in a git repo.
            {
                let (_temp_dir_root, _initial_branch) =
                    helper_setup_git_repo_with_commit().await?;

                let (is_repo, _) = try_is_git_repo().await;
                assert!(is_repo?);
            }

            // Test in a non-git directory.
            {
                let _temp_dir_root = try_create_temp_dir_and_cd!();
                let (is_repo, _) = try_is_git_repo().await;
                assert!(!is_repo?);
            }

            ok!(())
        })
    }

    async fn run_status_ops_tests() -> miette::Result<()> {
        test_try_is_working_directory_clean().await?;
        test_try_is_git_repo().await?;
        ok!(())
    }

    #[tokio::test]
    async fn test_status_ops_in_isolated_process() {
        if std::env::var(TEST_ENV_ISOLATED_TEST_RUNNER).is_ok() {
            // This is the actual test running in the isolated process.
            if let Err(err) = run_status_ops_tests().await {
                eprintln!("Test failed with error: {err}");
                std::process::exit(1);
            }
            std::process::exit(0);
        }

        // This is the test coordinator - spawn the actual test in a new process.
        let current_exe = std::env::current_exe().unwrap();
        let mut cmd = std::process::Command::new(&current_exe);
        cmd.env(TEST_ENV_ISOLATED_TEST_RUNNER, "1")
            .env("RUST_BACKTRACE", "1")
            .args([
                "--test-threads",
                "1",
                "--nocapture",
                "test_status_ops_in_isolated_process",
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
