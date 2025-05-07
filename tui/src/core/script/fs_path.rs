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

//! Note that [PathBuf] is owned and [Path] is a slice into it.
//! - So replace `&`[PathBuf] with a `&`[Path].
//! - More details [here](https://rust-lang.github.io/rust-clippy/master/index.html#ptr_arg).

use std::{env,
          fs,
          io::ErrorKind,
          path::{Path, PathBuf}};

use miette::Diagnostic;
use thiserror::Error;

use crate::ok;

/// Use this macro to make it more ergonomic to work with [PathBuf]s.
///
/// # Example - create a new path
///
/// ```
/// use r3bl_tui::fs_paths;
/// use std::path::{PathBuf, Path};
///
/// let my_path = fs_paths![with_empty_root => "usr/bin" => "bash"];
/// assert_eq!(my_path, PathBuf::from("usr/bin/bash"));
///
/// let my_path = fs_paths![with_empty_root => "usr" => "bin" => "bash"];
/// assert_eq!(my_path, PathBuf::from("usr/bin/bash"));
/// ```
///
/// # Example - join to an existing path
///
/// ```
/// use r3bl_tui::fs_paths;
/// use std::path::{PathBuf, Path};
///
/// let root = PathBuf::from("/home/user");
/// let my_path = fs_paths![with_root: root => "Downloads" => "rust"];
/// assert_eq!(my_path, PathBuf::from("/home/user/Downloads/rust"));
///
/// let root = PathBuf::from("/home/user");
/// let my_path = fs_paths![with_root: root => "Downloads" => "rust"];
/// assert_eq!(my_path, PathBuf::from("/home/user/Downloads/rust"));
/// ```
#[macro_export]
macro_rules! fs_paths {
    // Join to an existing root path.
    (with_root: $path:expr=> $($x:expr)=>*) => {{
        let mut it: std::path::PathBuf = $path.to_path_buf();
        $(
            it = it.join($x);
        )*
        it
    }};

    // Create a new path w/ no pre-existing root.
    (with_empty_root=> $($x:expr)=>*) => {{
        use std::path::{PathBuf};
        let mut it = PathBuf::new();
        $(
            it = it.join($x);
        )*
        it
    }}
}

/// Use this macro to ensure that all the paths provided exist on the filesystem, in which
/// case it will return true If any of the paths do not exist, the function will return
/// false. No error will be returned in case any of the paths are invalid or there aren't
/// enough permissions to check if the paths exist.
///
/// # Example
///
/// ```
/// use r3bl_tui::fs_paths_exist;
/// use r3bl_tui::fs_paths;
/// use r3bl_tui::create_temp_dir;
///
/// let temp_dir = create_temp_dir().unwrap();
/// let path_1 = fs_paths![with_root: temp_dir => "some_dir"];
/// let path_2 = fs_paths![with_root: temp_dir => "another_dir"];
///
/// assert!(!fs_paths_exist!(path_1, path_2));
/// ```
#[macro_export]
macro_rules! fs_paths_exist {
    ($($x:expr),*) => {'block: {
        $(
            if !std::fs::metadata($x).is_ok() {
                break 'block false;
            };
        )*
        true
    }};
}

#[derive(Debug, Error, Diagnostic)]
pub enum FsOpError {
    #[error("File does not exist: {0}")]
    FileDoesNotExist(String),

    #[error("Directory does not exist: {0}")]
    DirectoryDoesNotExist(String),

    #[error("File already exists: {0}")]
    FileAlreadyExists(String),

    #[error("Directory already exists: {0}")]
    DirectoryAlreadyExists(String),

    #[error("Insufficient permissions: {0}")]
    PermissionDenied(String),

    #[error("Invalid name: {0}")]
    InvalidName(String),

    #[error("Failed to perform fs operation directory: {0}")]
    IoError(#[from] std::io::Error),
}

pub type FsOpResult<T> = miette::Result<T, FsOpError>;

/// Checks whether the directory exist. If won't provide any errors if there are
/// permissions issues or the directory is invalid. Use [try_directory_exists] if you
/// want to handle these errors.
pub fn directory_exists(directory: impl AsRef<Path>) -> bool {
    fs::metadata(directory).is_ok_and(|metadata| metadata.is_dir())
}

/// Checks whether the file exists. If won't provide any errors if there are permissions
/// issues or the file is invalid. Use [try_file_exists] if you want to handle these
/// errors.
pub fn file_exists(file: impl AsRef<Path>) -> bool {
    fs::metadata(file).is_ok_and(|metadata| metadata.is_file())
}

/// Checks whether the directory exist. If there are issues with permissions for
/// directory access or invalid directory it will return an error. Use
/// [directory_exists] if you want to ignore these errors.
pub fn try_directory_exists(directory_path: impl AsRef<Path>) -> FsOpResult<bool> {
    match fs::metadata(directory_path) {
        Ok(metadata) => {
            // The directory_path might be found in the file system, but it might be a
            // file. This won't result in an error.
            ok!(metadata.is_dir())
        }
        Err(err) => match err.kind() {
            ErrorKind::NotFound => {
                FsOpResult::Err(FsOpError::DirectoryDoesNotExist(err.to_string()))
            }
            ErrorKind::InvalidInput => {
                FsOpResult::Err(FsOpError::InvalidName(err.to_string()))
            }
            _ => FsOpResult::Err(FsOpError::IoError(err)),
        },
    }
}

/// Checks whether the file exist. If there are issues with permissions for file access
/// or invalid file it will return an error. Use [file_exists] if you want to ignore
/// these errors.
pub fn try_file_exists(file_path: impl AsRef<Path>) -> FsOpResult<bool> {
    match fs::metadata(file_path) {
        // The file_path might be found in the file system, but it might be a
        // directory. This won't result in an error.
        Ok(metadata) => ok!(metadata.is_file()),
        Err(err) => match err.kind() {
            ErrorKind::NotFound => {
                FsOpResult::Err(FsOpError::FileDoesNotExist(err.to_string()))
            }
            ErrorKind::InvalidInput => {
                FsOpResult::Err(FsOpError::InvalidName(err.to_string()))
            }
            _ => FsOpResult::Err(FsOpError::IoError(err)),
        },
    }
}

/// Returns the current working directory of the process as a [PathBuf] (owned). If
/// there are issues with permissions for directory access or invalid directory it
/// will return an error.
///
/// - `bash` equivalent: `$(pwd)`
/// - Eg: `PathBuf("/home/user/some/path")`
pub fn try_pwd() -> FsOpResult<PathBuf> {
    match env::current_dir() {
        Ok(pwd) => FsOpResult::Ok(pwd),
        Err(err) => match err.kind() {
            ErrorKind::NotFound => {
                FsOpResult::Err(FsOpError::DirectoryDoesNotExist(err.to_string()))
            }
            _ => FsOpResult::Err(FsOpError::IoError(err)),
        },
    }
}

/// Returns the [Path] slice as a string.
/// - Eg: `"/home/user/some/path"`
pub fn path_as_string(path: &Path) -> String { path.display().to_string() }

#[cfg(test)]
mod tests_fs_path {
    use std::os::unix::fs::PermissionsExt as _;

    use fs_path::try_pwd;

    use super::*;
    use crate::{create_temp_dir, fs_path, serial_preserve_pwd_test};

    serial_preserve_pwd_test!(test_try_pwd, {
        // Create the root temp dir.
        let root = create_temp_dir().unwrap();

        let new_dir = root.join("test_pwd");
        fs::create_dir_all(&new_dir).unwrap();
        env::set_current_dir(&new_dir).unwrap();

        let pwd = try_pwd().unwrap();
        assert!(pwd.exists());
        assert_eq!(pwd, new_dir);
    });

    serial_preserve_pwd_test!(test_try_pwd_errors, {
        // Create the root temp dir.
        let root = create_temp_dir().unwrap();

        // Create a directory, change to it, remove all permissions for user.
        let no_permissions_dir = root.join("no_permissions_dir");
        fs::create_dir_all(&no_permissions_dir).unwrap();
        env::set_current_dir(&no_permissions_dir).unwrap();
        let mut permissions = fs::metadata(&no_permissions_dir).unwrap().permissions();
        permissions.set_mode(0o000);
        fs::set_permissions(&no_permissions_dir, permissions).unwrap();
        assert!(no_permissions_dir.exists());

        // Try to get the pwd with insufficient permissions. It should work!
        let result = try_pwd();
        assert!(result.is_ok());

        // Change the permissions back, so that it can be cleaned up!
        let mut permissions = fs::metadata(&no_permissions_dir).unwrap().permissions();
        permissions.set_mode(0o777);
        fs::set_permissions(&no_permissions_dir, permissions).unwrap();

        // Delete this directory, and try pwd again. It will not longer exist.
        fs::remove_dir_all(&no_permissions_dir).unwrap();
        let result = try_pwd();
        assert!(result.is_err());
        assert!(matches!(result, Err(FsOpError::DirectoryDoesNotExist(_))));
    });

    serial_preserve_pwd_test!(test_fq_path_relative_to_try_pwd, {
        // Create the root temp dir.
        let root = create_temp_dir().unwrap();

        let sub_path = "test_fq_path_relative_to_pwd";
        let new_dir = root.join(sub_path);
        fs::create_dir_all(&new_dir).unwrap();

        env::set_current_dir(&root).unwrap();

        println!("Current directory set to: {root}");
        println!("Current directory is    : {}", try_pwd().unwrap().display());

        let fq_path = fs_paths!(with_root: try_pwd().unwrap() => sub_path);

        println!("Sub directory created at: {}", fq_path.display());
        println!("Sub directory exists    : {}", fq_path.exists());

        assert!(fq_path.exists());
    });

    serial_preserve_pwd_test!(test_path_as_string, {
        // Create the root temp dir.
        let root = create_temp_dir().unwrap();

        env::set_current_dir(&root).unwrap();

        let fq_path = fs_paths!(with_root: try_pwd().unwrap() => "some_dir");
        let fq_path_str = fs_path::path_as_string(&fq_path);

        assert_eq!(fq_path_str, fq_path.display().to_string());
    });

    serial_preserve_pwd_test!(test_try_file_exists, {
        // Create the root temp dir.
        let root = create_temp_dir().unwrap();

        let new_dir = root.join("test_file_exists_dir");
        fs::create_dir_all(&new_dir).unwrap();

        let new_file = new_dir.join("test_file_exists_file.txt");
        fs::write(&new_file, "test").unwrap();

        assert!(fs_path::try_file_exists(&new_file).unwrap());
        assert!(!fs_path::try_file_exists(&new_dir).unwrap());

        fs::remove_dir_all(&new_dir).unwrap();

        // Ensure that an invalid path returns an error.
        assert!(fs_path::try_file_exists(&new_file).is_err()); // This file does not exist.
        assert!(fs_path::try_file_exists(&new_dir).is_err()); // This directory does
                                                              // not exist.
    });

    serial_preserve_pwd_test!(test_try_file_exists_not_found_error, {
        // Create the root temp dir.
        let root = create_temp_dir().unwrap();

        let new_dir = root.join("test_file_exists_not_found_error");

        // Try to check if the file exists. It should return an error.
        let result = fs_path::try_file_exists(&new_dir);
        assert!(result.is_err());
        assert!(matches!(result, Err(FsOpError::FileDoesNotExist(_))));
    });

    serial_preserve_pwd_test!(test_try_file_exists_invalid_name_error, {
        // Create the root temp dir.
        let root = create_temp_dir().unwrap();

        let new_dir = root.join("test_file_exists_invalid_name_error\0");

        // Try to check if the file exists. It should return an error.
        let result = fs_path::try_file_exists(&new_dir);
        assert!(result.is_err());
        assert!(matches!(result, Err(FsOpError::InvalidName(_))));
    });

    serial_preserve_pwd_test!(test_try_file_exists_permissions_errors, {
        // Create the root temp dir.
        let root = create_temp_dir().unwrap();

        // Create a directory, change to it, remove all permissions for user.
        let no_permissions_dir = root.join("no_permissions_dir");
        fs::create_dir_all(&no_permissions_dir).unwrap();
        let mut permissions = fs::metadata(&no_permissions_dir).unwrap().permissions();
        permissions.set_mode(0o000);
        fs::set_permissions(&no_permissions_dir, permissions).unwrap();
        assert!(no_permissions_dir.exists());

        // Try to check if the file exists with insufficient permissions. It should
        // work!
        let result = fs_path::try_file_exists(&no_permissions_dir);
        assert!(result.is_ok());

        // Change the permissions back, so that it can be cleaned up!
        let mut permissions = fs::metadata(&no_permissions_dir).unwrap().permissions();
        permissions.set_mode(0o777);
        fs::set_permissions(&no_permissions_dir, permissions).unwrap();
    });

    serial_preserve_pwd_test!(test_try_directory_exists, {
        // Create the root temp dir.
        let root = create_temp_dir().unwrap();

        let new_dir = root.join("test_dir_exists_dir");
        fs::create_dir_all(&new_dir).unwrap();

        let new_file = new_dir.join("test_dir_exists_file.txt");
        fs::write(&new_file, "test").unwrap();

        assert!(fs_path::try_directory_exists(&new_dir).unwrap());
        assert!(!fs_path::try_directory_exists(&new_file).unwrap());
    });

    serial_preserve_pwd_test!(test_try_directory_exists_not_found_error, {
        // Create the root temp dir.
        let root = create_temp_dir().unwrap();

        let new_dir = root.join("test_dir_exists_not_found_error");

        // Try to check if the directory exists. It should return an error.
        let result = fs_path::try_directory_exists(&new_dir);
        assert!(result.is_err());
        assert!(matches!(result, Err(FsOpError::DirectoryDoesNotExist(_))));
    });

    serial_preserve_pwd_test!(test_try_directory_exists_invalid_name_error, {
        // Create the root temp dir.
        let root = create_temp_dir().unwrap();

        let new_dir = root.join("test_dir_exists_invalid_name_error\0");

        // Try to check if the directory exists. It should return an error.
        let result = fs_path::try_directory_exists(&new_dir);
        assert!(result.is_err());
        assert!(matches!(result, Err(FsOpError::InvalidName(_))));
    });

    serial_preserve_pwd_test!(test_try_directory_exists_permissions_errors, {
        // Create the root temp dir.
        let root = create_temp_dir().unwrap();

        // Create a directory, change to it, remove all permissions for user.
        let no_permissions_dir = root.join("no_permissions_dir");
        fs::create_dir_all(&no_permissions_dir).unwrap();
        let mut permissions = fs::metadata(&no_permissions_dir).unwrap().permissions();
        permissions.set_mode(0o000);
        fs::set_permissions(&no_permissions_dir, permissions).unwrap();
        assert!(no_permissions_dir.exists());

        // Try to check if the directory exists with insufficient permissions. It
        // should work!
        let result = fs_path::try_directory_exists(&no_permissions_dir);
        assert!(result.is_ok());

        // Change the permissions back, so that it can be cleaned up!
        let mut permissions = fs::metadata(&no_permissions_dir).unwrap().permissions();
        permissions.set_mode(0o777);
        fs::set_permissions(&no_permissions_dir, permissions).unwrap();
    });
}
