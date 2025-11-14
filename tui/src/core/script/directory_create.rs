// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! All tests have been moved to
//! `fs_path.rs::test_all_fs_path_functions_in_isolated_process()` to prevent flakiness
//! when tests are run in parallel.

use crate::{ok,
            script::fs_path::{self, FsOpError, FsOpResult}};
use std::{fs, io::ErrorKind, path::Path};
use strum_macros::Display;

#[derive(Debug, Display, Default, Copy, Clone, PartialEq, Eq)]
pub enum MkdirOptions {
    #[default]
    CreateIntermediateDirectories,
    CreateIntermediateDirectoriesOnlyIfNotExists,
    CreateIntermediateDirectoriesAndPurgeExisting,
}

/// Creates a new directory at the specified path.
/// - Depending on the [`MkdirOptions`] the directories can be created destructively or
///   non-destructively.
/// - Any intermediate folders that don't exist will be created.
///
/// If any permissions issues occur or the directory can't be created due to
/// inconsistent [`MkdirOptions`] then an error is returned.
///
/// # Errors
///
/// Returns an error if:
/// - Insufficient permissions to create the directory
/// - The directory already exists (when using
///   `CreateIntermediateDirectoriesOnlyIfNotExists`)
/// - The path name is invalid
/// - I/O errors occur during directory creation
pub fn try_mkdir(new_path: impl AsRef<Path>, options: MkdirOptions) -> FsOpResult<()> {
    let new_path = new_path.as_ref();

    // Pre-process the directory creation options.
    match options {
        // This is the default option.
        MkdirOptions::CreateIntermediateDirectories => { /* Do nothing. */ }

        // This will delete the directory if it exists and then create it.
        MkdirOptions::CreateIntermediateDirectoriesAndPurgeExisting => {
            match fs::exists(new_path) {
                // The new_path exists.
                Ok(true) => {
                    // Remove the entire new_path.
                    if let Err(err) = fs::remove_dir_all(new_path) {
                        return handle_err(err);
                    }
                }
                // Encountered problem checking if the new_path exists.
                Err(err) => return handle_err(err),
                // The new_path does not exist.
                _ => { /* Do nothing. */ }
            }
        }

        // This will error out if the directory already exists.
        MkdirOptions::CreateIntermediateDirectoriesOnlyIfNotExists => {
            if let Ok(true) = fs::exists(new_path) {
                let new_dir_display = fs_path::path_as_string(new_path);
                return FsOpResult::Err(FsOpError::DirectoryAlreadyExists(
                    new_dir_display,
                ));
            }
        }
    }

    // Create the path.
    create_dir_all(new_path)
}

#[allow(clippy::missing_errors_doc)]
fn handle_err(err: std::io::Error) -> FsOpResult<()> {
    match err.kind() {
        ErrorKind::PermissionDenied | ErrorKind::ReadOnlyFilesystem => {
            FsOpResult::Err(FsOpError::PermissionDenied(err.to_string()))
        }
        ErrorKind::InvalidInput => {
            FsOpResult::Err(FsOpError::InvalidName(err.to_string()))
        }

        _ => FsOpResult::Err(FsOpError::IoError(err)),
    }
}

#[allow(clippy::missing_errors_doc)]
fn create_dir_all(new_path: &Path) -> FsOpResult<()> {
    match fs::create_dir_all(new_path) {
        Ok(()) => ok!(),
        Err(err) => handle_err(err),
    }
}
