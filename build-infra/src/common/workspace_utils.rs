// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Workspace discovery utilities.
//!
//! Find workspace root and collect Rust files.

use miette::{IntoDiagnostic, Result, miette};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Find the workspace root by searching for Cargo.toml with `[workspace]`.
///
/// Searches from current directory up to the filesystem root.
///
/// # Errors
///
/// Returns an error if:
/// - Current directory cannot be determined
/// - File I/O errors occur while reading Cargo.toml
/// - No workspace or package root is found
pub fn get_workspace_root() -> Result<PathBuf> {
    let mut current = std::env::current_dir().into_diagnostic()?;

    loop {
        let cargo_toml = current.join("Cargo.toml");

        if cargo_toml.exists() {
            // Check if this is a workspace Cargo.toml
            let content = std::fs::read_to_string(&cargo_toml).into_diagnostic()?;
            if content.contains("[workspace]") {
                return Ok(current);
            }
            // For standalone crates, treat the package root as the "workspace"
            if content.contains("[package]") {
                return Ok(current);
            }
        }

        if !current.pop() {
            return Err(miette!(
                "Could not find workspace root. \
                 Make sure you're in a Cargo workspace directory."
            ));
        }
    }
}

/// Find all .rs files in the workspace, excluding target/, hidden directories, and test
/// data.
///
/// Automatically excludes common test fixture directory patterns:
/// - Directories containing `test_data` (e.g., `test_data`, `conformance_test_data`)
/// - Directories containing `testdata`
/// - Directories containing `test_fixtures`
/// - Directories containing `fixtures`
///
/// # Errors
///
/// Currently infallible, but returns `Result` for future compatibility.
pub fn find_rust_files(workspace_root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(workspace_root)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let path = entry.path();

        // Skip target, hidden directories, test data directories, and non-.rs files
        if path.starts_with(workspace_root.join("target"))
            || path.components().any(|c| {
                let component = c.as_os_str().to_string_lossy();
                component.starts_with('.')
                    || component.contains("test_data")
                    || component.contains("testdata")
                    || component.contains("test_fixtures")
                    || component.contains("fixtures")
            })
            || path.extension().is_none_or(|ext| ext != "rs")
        {
            continue;
        }

        files.push(path.to_path_buf());
    }

    files.sort();
    Ok(files)
}

/// Find all .rs files in specific paths.
///
/// # Errors
///
/// Returns an error if file system operations fail while traversing directories.
pub fn find_rust_files_in_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for path in paths {
        if path.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path.clone());
        } else if path.is_dir() {
            files.extend(find_rust_files(path)?);
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_workspace_root() {
        // Should find the workspace root without panicking
        let root = get_workspace_root();
        assert!(root.is_ok());

        let root_path = root.unwrap();
        assert!(root_path.join("Cargo.toml").exists());
    }

    #[test]
    fn test_find_rust_files_in_workspace() {
        let root = get_workspace_root().unwrap();
        let files = find_rust_files(&root).unwrap();

        // Should find at least src/lib.rs
        assert!(!files.is_empty());
        assert!(files.iter().any(|f| f.ends_with("lib.rs")));
    }

    #[test]
    fn test_find_rust_files_in_paths() {
        let root = get_workspace_root().unwrap();
        let src_dir = root.join("src");

        if src_dir.exists() {
            let files = find_rust_files_in_paths(&[src_dir]).unwrap();
            assert!(!files.is_empty());
        }
    }
}
