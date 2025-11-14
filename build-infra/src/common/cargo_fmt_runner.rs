// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Run `cargo fmt` on specified files.

use miette::{IntoDiagnostic, Result, WrapErr};
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
/// Returns an error if:
/// - The file list is empty (no-op, returns Ok)
/// - `cargo fmt` command fails to execute
/// - `cargo fmt` exits with non-zero status
///
/// # Examples
///
/// ```no_run
/// use r3bl_build_infra::common::cargo_fmt_runner::run_cargo_fmt_on_files;
/// use std::path::PathBuf;
///
/// let files = vec![PathBuf::from("src/lib.rs"), PathBuf::from("src/main.rs")];
/// run_cargo_fmt_on_files(&files, true)?;
/// # Ok::<(), miette::Report>(())
/// ```
pub fn run_cargo_fmt_on_files(files: &[PathBuf], verbose: bool) -> Result<()> {
    // No-op if no files to format
    if files.is_empty() {
        return Ok(());
    }

    if verbose {
        println!("Running cargo fmt on {} files...", files.len());
        for file in files {
            println!("  - {}", file.display());
        }
    }

    // Build cargo fmt command with file arguments
    let mut cmd = Command::new("cargo");
    cmd.arg("fmt").arg("--");

    for file in files {
        cmd.arg(file);
    }

    // Execute command
    let output = cmd
        .output()
        .into_diagnostic()
        .wrap_err("Failed to execute cargo fmt command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(miette::miette!(
            "cargo fmt failed with exit code: {:?}\n{}",
            output.status.code(),
            stderr
        ));
    }

    if verbose {
        println!("cargo fmt completed successfully");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_file_list() {
        // Should succeed with empty list
        let result = run_cargo_fmt_on_files(&[], false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_nonexistent_file() {
        // cargo fmt should handle this gracefully or error
        let files = vec![PathBuf::from("/nonexistent/file.rs")];
        let result = run_cargo_fmt_on_files(&files, false);
        // We expect this to potentially fail, which is correct behavior
        // The test just ensures the function handles it without panicking
        drop(result);
    }
}
