// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Git integration utilities.
//!
//! Find changed files in git working tree or from recent commits.

use miette::{IntoDiagnostic, Result, WrapErr};
use std::{path::PathBuf, process::Command};

/// Get list of changed Rust files from git.
///
/// Priority:
/// 1. If there are staged or unstaged changes, return those files
/// 2. If working tree is clean, return files from most recent commit
///
/// # Errors
///
/// Returns an error if git commands fail to execute or produce invalid output.
pub fn get_changed_rust_files() -> Result<Vec<PathBuf>> {
    // First check for staged and unstaged files
    let changed_files = get_working_tree_changes()?;

    if !changed_files.is_empty() {
        return Ok(changed_files);
    }

    // Working tree is clean, check most recent commit
    get_files_from_last_commit()
}

/// Get Rust files with staged or unstaged changes.
fn get_working_tree_changes() -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .output()
        .into_diagnostic()
        .wrap_err("Failed to run git diff")?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let files: Vec<PathBuf> = stdout
        .lines()
        .filter(|line| {
            std::path::Path::new(line)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
        })
        .map(PathBuf::from)
        .collect();

    Ok(files)
}

/// Get Rust files from the most recent commit.
fn get_files_from_last_commit() -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .args(["diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"])
        .output()
        .into_diagnostic()
        .wrap_err("Failed to run git diff-tree")?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let files: Vec<PathBuf> = stdout
        .lines()
        .filter(|line| {
            std::path::Path::new(line)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
        })
        .map(PathBuf::from)
        .collect();

    Ok(files)
}

/// Check if we're in a git repository.
#[must_use]
pub fn is_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_git_repo() {
        // Should be in a git repo since r3bl-build-infra is version controlled
        assert!(is_git_repo());
    }

    #[test]
    fn test_get_changed_files_does_not_panic() {
        // Should not panic even if there are no changes
        let result = get_changed_rust_files();
        assert!(result.is_ok());
    }
}
