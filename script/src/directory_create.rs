/*
 *   Copyright (c) 2024-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use std::{fs, io::ErrorKind, path::Path};

use r3bl_core::ok;
use strum_macros::Display;

use crate::{fs_path,
            fs_path::{FsOpError, FsOpResult}};

#[derive(Debug, Display, Default)]
pub enum MkdirOptions {
    #[default]
    CreateIntermediateDirectories,
    CreateIntermediateDirectoriesOnlyIfNotExists,
    CreateIntermediateDirectoriesAndPurgeExisting,
}

/// Creates a new directory at the specified path.
/// - Depending on the [MkdirOptions] the directories can be created destructively or
///   non-destructively.
/// - Any intermediate folders that don't exist will be created.
///
/// If any permissions issues occur or the directory can't be created due to
/// inconsistent [MkdirOptions] then an error is returned.
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

fn handle_err(err: std::io::Error) -> FsOpResult<()> {
    match err.kind() {
        ErrorKind::PermissionDenied => {
            FsOpResult::Err(FsOpError::PermissionDenied(err.to_string()))
        }
        ErrorKind::InvalidInput => {
            FsOpResult::Err(FsOpError::InvalidName(err.to_string()))
        }
        ErrorKind::ReadOnlyFilesystem => {
            FsOpResult::Err(FsOpError::PermissionDenied(err.to_string()))
        }
        _ => FsOpResult::Err(FsOpError::IoError(err)),
    }
}

fn create_dir_all(new_path: &Path) -> FsOpResult<()> {
    match fs::create_dir_all(new_path) {
        Ok(_) => ok!(),
        Err(err) => handle_err(err),
    }
}

#[cfg(test)]
mod tests_directory_create {
    use r3bl_core::create_temp_dir;

    use super::*;
    use crate::{directory_create::{MkdirOptions::*, try_mkdir},
                fs_paths,
                serial_preserve_pwd_test,
                with_saved_pwd};

    serial_preserve_pwd_test!(test_try_mkdir, {
        with_saved_pwd!({
            // Create the root temp dir.
            let root = create_temp_dir().unwrap();

            // Create a temporary directory.
            let tmp_root_dir = fs_paths!(with_root: root => "test_create_clean_new_dir");
            try_mkdir(&tmp_root_dir, CreateIntermediateDirectories).unwrap();

            // Create a new directory inside the temporary directory.
            let new_dir = fs_paths!(with_root: tmp_root_dir => "new_dir");
            try_mkdir(&new_dir, CreateIntermediateDirectories).unwrap();
            assert!(new_dir.exists());

            // Try & fail to create the same directory again non destructively.
            let result =
                try_mkdir(&new_dir, CreateIntermediateDirectoriesOnlyIfNotExists);
            assert!(result.is_err());
            assert!(matches!(result, Err(FsOpError::DirectoryAlreadyExists(_))));

            // Create a file inside the new directory.
            let file_path = new_dir.join("test_file.txt");
            fs::write(&file_path, "test").unwrap();
            assert!(file_path.exists());

            // Call `mkdir` again with destructive options and ensure the directory is
            // clean.
            try_mkdir(&new_dir, CreateIntermediateDirectoriesAndPurgeExisting).unwrap();

            // Ensure the directory is clean.
            assert!(new_dir.exists());
            assert!(!file_path.exists());
        });
    });
}
