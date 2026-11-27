// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Run `cargo fmt` on specified files.

use crate::cargo_rustdoc_fmt::{CARGO_ARGS_SEPARATOR, CARGO_PROGRAM, FMT_SUBCOMMAND};
use r3bl_tui::{ProcessExecutionOutcome, ok};
use std::{path::PathBuf, process::Command};

/// Run `cargo fmt` on the specified files.
///
/// # Arguments
///
/// * `files` - List of file paths to format
/// * `verbose` - Print verbose output
///
/// # Errors
///
/// Returns [`RunCargoFmtError`] if:
/// - `cargo fmt` command fails to execute
/// - `cargo fmt` exits with non-zero status
///
/// # Examples
///
/// ```no_run
/// use r3bl_build_infra::common::run_cargo_fmt::try_run_cargo_fmt;
/// use std::path::PathBuf;
///
/// let files = vec![PathBuf::from("src/lib.rs"), PathBuf::from("src/main.rs")];
/// try_run_cargo_fmt(&files, true)?;
/// # Ok::<(), miette::Report>(())
/// ```
pub fn try_run_cargo_fmt(
    files: &[PathBuf],
    verbose: bool,
) -> Result<(), RunCargoFmtError> {
    // No-op if no files to format.
    if files.is_empty() {
        return ok!();
    }

    if verbose {
        println!("Running cargo fmt on {} files...", files.len());
        for file in files {
            println!("  - {}", file.display());
        }
    }

    // Build cargo fmt command with file arguments.
    let mut command = Command::new(CARGO_PROGRAM);
    command.arg(FMT_SUBCOMMAND).arg(CARGO_ARGS_SEPARATOR);

    for file in files {
        command.arg(file);
    }

    // Execute command
    let res_output = command.output();
    match ProcessExecutionOutcome::from(res_output) {
        ProcessExecutionOutcome::Success(_) => {
            if verbose {
                println!("cargo fmt completed successfully");
            }
            ok!()
        }
        ProcessExecutionOutcome::NonZeroExit(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            Err(RunCargoFmtError::NonZeroExit {
                exit_code: output.status.code(),
                stderr,
            })
        }
        ProcessExecutionOutcome::SpawnFailed(err) => {
            Err(RunCargoFmtError::SpawnFailed(err))
        }
    }
}

/// Errors returned when invoking `cargo fmt` on workspace files.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum RunCargoFmtError {
    /// Failed to spawn the `cargo fmt` subprocess.
    #[error("Failed to execute cargo fmt command")]
    SpawnFailed(#[source] std::io::Error),

    /// The `cargo fmt` subprocess exited with a non-zero status.
    #[error("cargo fmt failed with exit code {exit_code:?}: {stderr}")]
    NonZeroExit {
        /// Process exit code, or `None` if terminated by a signal.
        exit_code: Option<i32>,

        /// Captured standard error output from `cargo fmt`.
        stderr: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_file_list() {
        // Should succeed with empty list
        let result = try_run_cargo_fmt(&[], false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_nonexistent_file() {
        // cargo fmt should handle this gracefully or error
        let files = vec![PathBuf::from("/nonexistent/file.rs")];
        let result = try_run_cargo_fmt(&files, false);
        // We expect this to potentially fail, which is correct behavior
        // The test just ensures the function handles it without panicking
        drop(result);
    }
}
