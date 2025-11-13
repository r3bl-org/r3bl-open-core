// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Operations for detecting and filtering changed files in git repositories.

use crate::{ResultAndCommand, Run, command,
            script::git::types::{git_command_args::{GIT_ARG_HEAD, GIT_ARG_NAME_ONLY,
                                                    GIT_ARG_NO_COMMIT_ID,
                                                    GIT_ARG_RECURSIVE},
                                 git_command_names::{GIT_CMD_DIFF, GIT_CMD_DIFF_TREE,
                                                     GIT_PROGRAM}}};
use std::path::PathBuf;

/// Get list of changed files matching any of the provided extensions.
///
/// Priority:
/// 1. If there are staged or unstaged changes, return those files
/// 2. If working tree is clean, return files from most recent commit
///
/// # Arguments
///
/// * `extensions` - File extensions to filter by. If empty, returns all changed files.
///
/// # Examples
///
/// ```rust,no_run
/// use r3bl_tui::try_get_changed_files_by_ext;
///
/// # async fn example() {
/// // Get ALL changed files (no filtering)
/// let (files, _cmd) = try_get_changed_files_by_ext(&[]).await;
///
/// // Get changed Rust files only
/// let (files, _cmd) = try_get_changed_files_by_ext(&["rs"]).await;
///
/// // Get changed Rust and TOML files
/// let (files, _cmd) = try_get_changed_files_by_ext(&["rs", "toml"]).await;
/// # }
/// ```
pub async fn try_get_changed_files_by_ext(
    extensions: &[&str],
) -> ResultAndCommand<Vec<PathBuf>> {
    // First check for staged and unstaged files
    let (res_changed_files, cmd) = get_working_tree_changes(extensions).await;
    let Ok(changed_files) = res_changed_files else {
        let report = res_changed_files.unwrap_err();
        return (Err(report), cmd);
    };

    if !changed_files.is_empty() {
        return (Ok(changed_files), cmd);
    }

    // Working tree is clean, check most recent commit
    get_files_from_last_commit(extensions).await
}

/// Get files with staged or unstaged changes matching the extensions.
async fn get_working_tree_changes(extensions: &[&str]) -> ResultAndCommand<Vec<PathBuf>> {
    let mut cmd = command!(
        program => GIT_PROGRAM,
        args => GIT_CMD_DIFF, GIT_ARG_NAME_ONLY, GIT_ARG_HEAD
    );

    let res_output = cmd.run().await;
    let Ok(output) = res_output else {
        let report = res_output.unwrap_err();
        return (Err(report), cmd);
    };

    let files: Vec<PathBuf> = String::from_utf8_lossy(&output)
        .lines()
        .filter(|line| {
            // If extensions is empty, include all files
            if extensions.is_empty() {
                return true;
            }

            // Otherwise, filter by extension
            std::path::Path::new(line)
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| {
                    extensions.iter().any(|&e| ext.eq_ignore_ascii_case(e))
                })
        })
        .map(PathBuf::from)
        .collect();

    (Ok(files), cmd)
}

/// Get files from the most recent commit matching the extensions.
async fn get_files_from_last_commit(
    extensions: &[&str],
) -> ResultAndCommand<Vec<PathBuf>> {
    let mut cmd = command!(
        program => GIT_PROGRAM,
        args => GIT_CMD_DIFF_TREE, GIT_ARG_NO_COMMIT_ID, GIT_ARG_NAME_ONLY, GIT_ARG_RECURSIVE, GIT_ARG_HEAD
    );

    let res_output = cmd.run().await;
    let Ok(output) = res_output else {
        let report = res_output.unwrap_err();
        return (Err(report), cmd);
    };

    let files: Vec<PathBuf> = String::from_utf8_lossy(&output)
        .lines()
        .filter(|line| {
            // If extensions is empty, include all files
            if extensions.is_empty() {
                return true;
            }

            // Otherwise, filter by extension
            std::path::Path::new(line)
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| {
                    extensions.iter().any(|&e| ext.eq_ignore_ascii_case(e))
                })
        })
        .map(PathBuf::from)
        .collect();

    (Ok(files), cmd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{command, ok,
                script::git::types::{git_command_names::{GIT_CMD_ADD, GIT_CMD_COMMIT,
                                                         GIT_CMD_CONFIG, GIT_CMD_INIT},
                                     git_config_keys::{GIT_CONFIG_COMMIT_GPGSIGN,
                                                       GIT_CONFIG_USER_EMAIL,
                                                       GIT_CONFIG_USER_NAME},
                                     test_config::{TEST_EMAIL,
                                                   TEST_ENV_ISOLATED_TEST_RUNNER,
                                                   TEST_GPG_SIGN_DISABLED,
                                                   TEST_INITIAL_COMMIT_MSG,
                                                   TEST_USER_NAME}},
                try_create_temp_dir_and_cd, try_write_file, with_saved_pwd};

    async fn test_try_get_changed_files_by_ext() -> miette::Result<()> {
        with_saved_pwd!(async {
            let (
                /* don't drop this immediately using `_` */ _temp_dir_root,
                _git_folder,
            ) = try_create_temp_dir_and_cd!("test_git_changed_files");

            // Setup git repo.
            command!(program => GIT_PROGRAM, args => GIT_CMD_INIT)
                .run()
                .await?;
            command!(program => GIT_PROGRAM, args => GIT_CMD_CONFIG, GIT_CONFIG_USER_EMAIL, TEST_EMAIL)
                .run()
                .await?;
            command!(program => GIT_PROGRAM, args => GIT_CMD_CONFIG, GIT_CONFIG_USER_NAME, TEST_USER_NAME)
                .run()
                .await?;
            command!(program => GIT_PROGRAM, args => GIT_CMD_CONFIG, GIT_CONFIG_COMMIT_GPGSIGN, TEST_GPG_SIGN_DISABLED)
                .run()
                .await?;

            // Create initial commit.
            try_write_file(&_git_folder, "initial.txt", "initial")?;
            command!(program => GIT_PROGRAM, args => GIT_CMD_ADD, "initial.txt")
                .run()
                .await?;
            command!(program => GIT_PROGRAM, args => GIT_CMD_COMMIT, "-m", TEST_INITIAL_COMMIT_MSG)
                .run()
                .await?;

            // Create some test files.
            try_write_file(&_git_folder, "test.rs", "fn main() {}")?;
            try_write_file(&_git_folder, "config.toml", "[package]")?;
            try_write_file(&_git_folder, "README.md", "# Test")?;
            try_write_file(&_git_folder, "data.json", "{}")?;

            // Stage the test files so they show up in git diff --name-only HEAD.
            command!(program => GIT_PROGRAM, args => GIT_CMD_ADD, "test.rs")
                .run()
                .await?;
            command!(program => GIT_PROGRAM, args => GIT_CMD_ADD, "config.toml")
                .run()
                .await?;
            command!(program => GIT_PROGRAM, args => GIT_CMD_ADD, "README.md")
                .run()
                .await?;
            command!(program => GIT_PROGRAM, args => GIT_CMD_ADD, "data.json")
                .run()
                .await?;

            // Test: Get all changed files (empty extensions array).
            let (all_files, _) = try_get_changed_files_by_ext(&[]).await;
            let all_files = all_files?;
            assert_eq!(all_files.len(), 4);

            // Test: Get only .rs files.
            let (rs_files, _) = try_get_changed_files_by_ext(&["rs"]).await;
            let rs_files = rs_files?;
            assert_eq!(rs_files.len(), 1);
            assert!(rs_files[0].to_string_lossy().contains("test.rs"));

            // Test: Get .rs and .toml files.
            let (config_files, _) = try_get_changed_files_by_ext(&["rs", "toml"]).await;
            let config_files = config_files?;
            assert_eq!(config_files.len(), 2);

            // Test: Get .md files.
            let (md_files, _) = try_get_changed_files_by_ext(&["md"]).await;
            let md_files = md_files?;
            assert_eq!(md_files.len(), 1);
            assert!(md_files[0].to_string_lossy().contains("README.md"));

            ok!(())
        })
    }

    async fn run_changed_files_tests() -> miette::Result<()> {
        test_try_get_changed_files_by_ext().await?;
        ok!(())
    }

    #[tokio::test]
    async fn test_changed_files_in_isolated_process() {
        if std::env::var(TEST_ENV_ISOLATED_TEST_RUNNER).is_ok() {
            if let Err(err) = run_changed_files_tests().await {
                eprintln!("Test failed with error: {err}");
                std::process::exit(1);
            }
            std::process::exit(0);
        }

        let current_exe = std::env::current_exe().unwrap();
        let mut cmd = std::process::Command::new(&current_exe);
        cmd.env(TEST_ENV_ISOLATED_TEST_RUNNER, "1")
            .env("RUST_BACKTRACE", "1")
            .args([
                "--test-threads",
                "1",
                "--nocapture",
                "test_changed_files_in_isolated_process",
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
