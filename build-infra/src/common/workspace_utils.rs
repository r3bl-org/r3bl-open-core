// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Workspace discovery utilities to report workspace root and collect Rust files.

use crate::cargo_rustdoc_fmt::{CARGO_TOML_FILE_NAME, HIDDEN_PATH_PREFIX,
                               PACKAGE_SECTION_HEADER, RUST_FILE_EXTENSION,
                               TARGET_DIR_NAME, WORKSPACE_SECTION_HEADER};
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

/// Finds the workspace root by searching upward from the current directory for a
/// `Cargo.toml` containing `[workspace]`, falling back to the closest `[package]`
/// `Cargo.toml` for standalone single-crate projects.
///
/// # Errors
///
/// Returns [`GetWorkspaceRootError`] if:
/// - Current directory cannot be determined
/// - File I/O errors occur while reading `Cargo.toml`
/// - No workspace or package root is found
pub fn try_get_workspace_root() -> Result<PathBuf, GetWorkspaceRootError> {
    let current = std::env::current_dir().map_err(GetWorkspaceRootError::CurrentDirIo)?;
    workspace_root_impl::try_find_workspace_root_from(&current)
}

/// Errors returned while discovering the Cargo workspace or package root.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum GetWorkspaceRootError {
    /// Failed to query the process's current working directory.
    #[error("Failed to determine current working directory")]
    CurrentDirIo(#[source] std::io::Error),

    /// Failed to read a `Cargo.toml` manifest while walking upward.
    #[error("Failed to read Cargo manifest '{}'", path.display())]
    ReadManifest {
        /// Path to the `Cargo.toml` file that could not be read.
        path: PathBuf,
        /// Underlying filesystem I/O error.
        #[source]
        src_err: std::io::Error,
    },

    /// Reached the filesystem root without finding a `[workspace]` or `[package]`
    /// manifest.
    #[error(
        "Could not find workspace root; make sure you are inside a Cargo workspace directory"
    )]
    NotFound,
}

mod workspace_root_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Walks upward from `start_dir` looking for a `Cargo.toml` with `[workspace]`.
    ///
    /// If `start_dir` is inside a workspace member crate (whose `Cargo.toml` only has
    /// `[package]`), records the first `[package]` root as a fallback and continues
    /// walking upward so an enclosing `[workspace]` takes precedence.
    pub fn try_find_workspace_root_from(
        start_dir: &Path,
    ) -> Result<PathBuf, GetWorkspaceRootError> {
        let mut current = start_dir.to_path_buf();
        let mut fallback_package_root: Option<PathBuf> = None;

        loop {
            let cargo_toml = current.join(CARGO_TOML_FILE_NAME);

            if cargo_toml.exists() {
                let content =
                    std::fs::read_to_string(&cargo_toml).map_err(|src_err| {
                        GetWorkspaceRootError::ReadManifest {
                            path: cargo_toml,
                            src_err,
                        }
                    })?;
                // Path 1: Found a Cargo workspace root (`[workspace]`).
                if content.contains(WORKSPACE_SECTION_HEADER) {
                    return Ok(current);
                }
                // Path 2: Found a crate manifest (`[package]`). Record the closest
                // one as a fallback in case this is a standalone single-crate repo, but
                // keep walking upward in case an enclosing `[workspace]` exists above it.
                if fallback_package_root.is_none()
                    && content.contains(PACKAGE_SECTION_HEADER)
                {
                    fallback_package_root = Some(current.clone());
                }
            }

            if !current.pop() {
                return fallback_package_root.ok_or(GetWorkspaceRootError::NotFound);
            }
        }
    }
}

/// Finds all `.rs` files under `root_dir`, excluding `target/`, hidden directories
/// (e.g., `.git`), and paths matching `skip_folder_matches`.
///
/// Only the path suffix relative to `root_dir` is inspected for hidden or skipped
/// components so that passing a relative path (like `.` or `./src`) or a workspace inside
/// a dot-prefixed parent directory never falsely excludes valid files.
#[must_use]
pub fn find_rust_files(root_dir: &Path, skip_folder_matches: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for entry in
        find_rust_files_impl::walk_filtered_entries(root_dir, skip_folder_matches)
    {
        let path = entry.into_path();
        if find_rust_files_impl::is_rust_file(&path) {
            files.push(path);
        }
    }

    files.sort();
    files
}

/// Finds all `.rs` files in specific paths, excluding paths matching
/// `skip_folder_matches`.
#[must_use]
pub fn find_rust_files_in_paths(
    paths: &[PathBuf],
    skip_folder_matches: &[String],
) -> Vec<PathBuf> {
    let mut files = Vec::new();

    for path in paths {
        if find_rust_files_impl::is_rust_file(path) {
            if !is_path_in_skip_folder_matches(path, skip_folder_matches) {
                files.push(path.clone());
            }
        } else if path.is_dir() {
            files.extend(find_rust_files(path, skip_folder_matches));
        }
    }

    files.sort();
    files.dedup();
    files
}

mod find_rust_files_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Returns an iterator over all the valid [`walkdir::DirEntry`] items under
    /// `root_dir`, pruning `target/`, hidden directories, and directories matching
    /// `skip_folder_matches` before descending into them.
    pub fn walk_filtered_entries<'a>(
        root_dir: &'a Path,
        skip_folder_matches: &'a [String],
    ) -> impl Iterator<Item = walkdir::DirEntry> + 'a {
        let target_dir = root_dir.join(TARGET_DIR_NAME);

        WalkDir::new(root_dir)
            .into_iter()
            .filter_entry(move |entry| {
                let target_dir: &Path = &target_dir;
                let path = entry.path();
                let relative_path = path.strip_prefix(root_dir).unwrap_or(path);
                let is_target_dir = path.starts_with(target_dir);
                let is_hidden = has_hidden_component(relative_path);
                let is_skipped_folder =
                    is_path_in_skip_folder_matches(relative_path, skip_folder_matches);
                !is_target_dir && !is_hidden && !is_skipped_folder
            })
            .flatten()
    }

    /// Returns `true` if `component` is a normal path segment starting with `.` (e.g.,
    /// `.git`, `.cargo`), ignoring relative navigation segments like `.`
    /// ([`Component::CurDir`]) and `..` ([`Component::ParentDir`]).
    #[must_use]
    pub fn is_hidden_component(component: Component<'_>) -> bool {
        match component {
            Component::Normal(name) => {
                name.to_string_lossy().starts_with(HIDDEN_PATH_PREFIX)
            }
            _ => false,
        }
    }

    /// Returns `true` if any normal component of `path` is a hidden file or directory
    /// (starts with `.`).
    #[must_use]
    pub fn has_hidden_component(path: &Path) -> bool {
        path.components().any(is_hidden_component)
    }

    /// Returns `true` if `path` is a regular file with a `.rs` extension.
    #[must_use]
    pub fn is_rust_file(path: &Path) -> bool {
        path.is_file()
            && path
                .extension()
                .is_some_and(|ext| ext == RUST_FILE_EXTENSION)
    }
}

/// Returns `true` if any normal component of `path` contains any non-empty pattern in
/// `skip_folder_matches` (the `--skip-folder-matches` CLI argument).
///
/// # Why Folder Skipping Is Needed (`cargo-rustdoc-fmt` vs `cargo fmt --all`)
///
/// - **`cargo fmt --all` (Module-Graph Discovery)**: Reads `Cargo.toml` entry points
///   (`src/lib.rs`, `src/bin/*.rs`) and recursively walks `mod <name>;` AST declarations.
///   Because standalone test fixtures in `test_data/` are loaded via `include_str!`
///   rather than `mod`, `cargo fmt --all` naturally ignores them.
/// - **`cargo-rustdoc-fmt` (Filesystem / Git Discovery)**: Discovers `.rs` files via
///   [`WalkDir`] or `git diff`, which sees every `.rs` file on disk regardless of whether
///   it is attached to the Rust module tree. This filter excludes `test_data` (or custom
///   `--skip-folder-matches` patterns) so golden test fixtures are never modified.
#[must_use]
pub fn is_path_in_skip_folder_matches(
    path: &Path,
    skip_folder_matches: &[String],
) -> bool {
    path.components().any(|component| {
        let Component::Normal(name) = component else {
            return false;
        };
        let name_str = name.to_string_lossy();
        skip_folder_matches
            .iter()
            .any(|pattern| !pattern.is_empty() && name_str.contains(pattern.as_str()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cargo_rustdoc_fmt::DEFAULT_SKIP_FOLDER_MATCH;
    use tempfile::TempDir;

    #[test]
    fn test_get_workspace_root_finds_enclosing_workspace_from_member_crate() {
        // When `cargo test -p r3bl-build-infra` runs, `current_dir()` is `build-infra/`
        // (which has a `[package]` Cargo.toml). `try_get_workspace_root()` must walk past
        // the member crate and return the enclosing `[workspace]` root.
        let root_path = try_get_workspace_root().unwrap();
        let cargo_toml_content =
            std::fs::read_to_string(root_path.join(CARGO_TOML_FILE_NAME)).unwrap();
        assert!(
            cargo_toml_content.contains(WORKSPACE_SECTION_HEADER),
            "Expected enclosing [workspace] Cargo.toml at {}, got member package instead",
            root_path.display()
        );
    }

    #[test]
    fn test_find_workspace_root_codepaths() {
        let temp = TempDir::new().unwrap();

        // 1. No Cargo.toml anywhere -> returns Err(GetWorkspaceRootError::NotFound).
        let empty_dir = temp.path().join("no_cargo_here/nested");
        std::fs::create_dir_all(&empty_dir).unwrap();
        assert!(matches!(
            workspace_root_impl::try_find_workspace_root_from(&empty_dir),
            Err(GetWorkspaceRootError::NotFound)
        ));

        // 2. Standalone single-crate project (`[package]` only, no `[workspace]` above)
        //    -> returns the standalone package root.
        let standalone_root = temp.path().join("standalone_crate");
        let standalone_src = standalone_root.join("src");
        std::fs::create_dir_all(&standalone_src).unwrap();
        std::fs::write(
            standalone_root.join(CARGO_TOML_FILE_NAME),
            "[package]\nname = \"standalone\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        assert_eq!(
            workspace_root_impl::try_find_workspace_root_from(&standalone_src).unwrap(),
            standalone_root
        );

        // 3. Multi-crate Cargo workspace (`[workspace]` at root + `[package]` in member)
        //    -> starting inside `member_a/src` must return `workspace_root`, NOT
        //    `member_a`.
        let workspace_root = temp.path().join("my_workspace");
        let member_root = workspace_root.join("crates/member_a");
        let member_src = member_root.join("src");
        std::fs::create_dir_all(&member_src).unwrap();
        std::fs::write(
            workspace_root.join(CARGO_TOML_FILE_NAME),
            "[workspace]\nmembers = [\"crates/member_a\"]\n",
        )
        .unwrap();
        std::fs::write(
            member_root.join(CARGO_TOML_FILE_NAME),
            "[package]\nname = \"member_a\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        assert_eq!(
            workspace_root_impl::try_find_workspace_root_from(&member_src).unwrap(),
            workspace_root
        );
    }

    #[test]
    fn test_has_hidden_component_ignores_cur_and_parent_dir() {
        // Relative navigation segments (`.` and `..`) are NOT hidden directories.
        assert!(!find_rust_files_impl::has_hidden_component(Path::new(".")));
        assert!(!find_rust_files_impl::has_hidden_component(Path::new(
            "./src/lib.rs"
        )));
        assert!(!find_rust_files_impl::has_hidden_component(Path::new(
            "../tui/src/lib.rs"
        )));

        // Actual dot-prefixed normal components ARE hidden.
        assert!(find_rust_files_impl::has_hidden_component(Path::new(
            ".git/config"
        )));
        assert!(find_rust_files_impl::has_hidden_component(Path::new(
            "src/.hidden/file.rs"
        )));
    }

    #[test]
    fn test_is_path_in_skip_folder_matches() {
        let default_skip = vec![DEFAULT_SKIP_FOLDER_MATCH.to_string()];
        assert!(is_path_in_skip_folder_matches(
            Path::new("src/validation_tests/test_data/sample.rs"),
            &default_skip
        ));
        assert!(is_path_in_skip_folder_matches(
            Path::new("src/conformance_test_data/sample.rs"),
            &default_skip
        ));
        assert!(!is_path_in_skip_folder_matches(
            Path::new("tui/src/core/test_fixtures/mod.rs"),
            &default_skip
        ));

        // Custom comma-separated patterns.
        let custom_skip = vec!["testdata".to_string(), "snapshots".to_string()];
        assert!(is_path_in_skip_folder_matches(
            Path::new("crates/foo/testdata/sample.rs"),
            &custom_skip
        ));
        assert!(is_path_in_skip_folder_matches(
            Path::new("crates/foo/snapshots/sample.rs"),
            &custom_skip
        ));
        assert!(!is_path_in_skip_folder_matches(
            Path::new("crates/foo/test_data/sample.rs"),
            &custom_skip
        ));

        // Empty pattern disables folder skipping.
        let disabled_skip = vec![String::new()];
        assert!(!is_path_in_skip_folder_matches(
            Path::new("src/test_data/sample.rs"),
            &disabled_skip
        ));
    }

    #[test]
    fn test_find_rust_files_in_crate_root() {
        let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let skip = vec![DEFAULT_SKIP_FOLDER_MATCH.to_string()];
        let files = find_rust_files(crate_root, &skip);

        // Should find at least src/lib.rs and never include test_data files.
        assert!(!files.is_empty());
        assert!(files.iter().any(|f| f.ends_with("lib.rs")));
        assert!(
            !files
                .iter()
                .any(|f| is_path_in_skip_folder_matches(f, &skip))
        );
    }

    #[test]
    fn test_find_rust_files_with_relative_dot_path() {
        // Regression test: passing `./src` or `.` must not be treated as a hidden
        // directory due to `Component::CurDir` (`.`).
        let skip = vec![DEFAULT_SKIP_FOLDER_MATCH.to_string()];
        let relative_src = PathBuf::from("./src");
        if relative_src.exists() {
            let files = find_rust_files_in_paths(&[relative_src], &skip);
            assert!(
                !files.is_empty(),
                "Relative path `./src` should discover `.rs` files"
            );
        }
    }

    #[test]
    fn test_find_rust_files_in_paths() {
        let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let src_dir = crate_root.join("src");
        let skip = vec![DEFAULT_SKIP_FOLDER_MATCH.to_string()];

        let files = find_rust_files_in_paths(&[src_dir], &skip);
        assert!(!files.is_empty());
        assert!(
            !files
                .iter()
                .any(|f| is_path_in_skip_folder_matches(f, &skip))
        );
    }
}
