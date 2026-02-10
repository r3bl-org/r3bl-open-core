// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Note that [`PathBuf`] is owned and [Path] is a slice into it.
//! - So replace `&`[`PathBuf`] with a `&`[Path].
//! - More details [here].
//!
//! [here]: https://rust-lang.github.io/rust-clippy/master/index.html#ptr_arg

use crate::ok;
use miette::Diagnostic;
use std::{env, fs,
          fs::File,
          io::{ErrorKind, Write},
          path::{Path, PathBuf}};
use thiserror::Error;

/// Use this macro to make it more ergonomic to work with [`PathBuf`]s.
///
/// # Example - create a new path
///
/// ```no_run
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
/// ```no_run
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
/// ```no_run
/// use r3bl_tui::fs_paths_exist;
/// use r3bl_tui::fs_paths;
/// use r3bl_tui::try_create_temp_dir;
///
/// let temp_dir = try_create_temp_dir().unwrap();
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
/// permissions issues or the directory is invalid. Use [`try_directory_exists`] if you
/// want to handle these errors.
pub fn directory_exists(directory: impl AsRef<Path>) -> bool {
    fs::metadata(directory).is_ok_and(|metadata| metadata.is_dir())
}

/// Checks whether the file exists. If won't provide any errors if there are permissions
/// issues or the file is invalid. Use [`try_file_exists`] if you want to handle these
/// errors.
pub fn file_exists(file: impl AsRef<Path>) -> bool {
    fs::metadata(file).is_ok_and(|metadata| metadata.is_file())
}

/// Checks whether the directory exist. If there are issues with permissions for
/// directory access or invalid directory it will return an error. Use
/// [`directory_exists`] if you want to ignore these errors.
///
/// # Errors
///
/// Returns an error if:
/// - The directory does not exist
/// - The directory name is invalid
/// - I/O errors occur while accessing the directory
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
/// or invalid file it will return an error. Use [`file_exists`] if you want to ignore
/// these errors.
///
/// # Errors
///
/// Returns an error if:
/// - The file does not exist
/// - The file name is invalid
/// - I/O errors occur while accessing the file
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

/// Returns the current working directory of the process as a [`PathBuf`] (owned). If
/// there are issues with permissions for directory access or invalid directory it
/// will return an error.
///
/// - `bash` equivalent: `$(pwd)`
/// - Eg: `PathBuf("/home/user/some/path")`
///
/// # Errors
///
/// Returns an error if:
/// - The current directory has been deleted
/// - I/O errors occur while accessing the current directory
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
#[must_use]
pub fn path_as_string(path: &Path) -> String { path.display().to_string() }

/// Writes the given content to the file named `file_name` in the specified `folder`.
/// - If the parent directory does not exist, returns an error.
/// - If the file cannot be written due to permissions or invalid name, returns an error.
///
/// # Errors
///
/// Returns an error if:
/// - The parent directory does not exist
/// - Insufficient permissions to write the file
/// - The file name is invalid
/// - I/O errors occur during file creation or writing
pub fn try_write_file(
    folder: impl AsRef<Path>,
    file_name: impl AsRef<str>,
    content: impl AsRef<str>,
) -> FsOpResult<()> {
    let file_path = folder.as_ref().join(file_name.as_ref());
    match File::create(file_path) {
        Ok(mut file) => {
            if let Err(err) = file.write_all(content.as_ref().as_bytes()) {
                return FsOpResult::Err(FsOpError::IoError(err));
            }
            ok!()
        }
        Err(err) => match err.kind() {
            ErrorKind::NotFound => {
                FsOpResult::Err(FsOpError::DirectoryDoesNotExist(err.to_string()))
            }
            ErrorKind::PermissionDenied => {
                FsOpResult::Err(FsOpError::PermissionDenied(err.to_string()))
            }
            ErrorKind::InvalidInput => {
                FsOpResult::Err(FsOpError::InvalidName(err.to_string()))
            }
            _ => FsOpResult::Err(FsOpError::IoError(err)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{try_create_temp_dir, with_saved_pwd};
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    /// On Windows, `canonicalize()` returns paths with the `\\?\` extended-length
    /// prefix, but `env::current_dir()` does not. Strip the prefix for comparison.
    /// On non-Windows platforms, this is a no-op.
    fn strip_extended_length_prefix(path: PathBuf) -> PathBuf {
        #[cfg(windows)]
        {
            let s = path.to_string_lossy();
            if let Some(stripped) = s.strip_prefix(r"\\?\") {
                return PathBuf::from(stripped);
            }
        }
        path
    }

    fn test_try_directory_exists_not_found_error() {
        with_saved_pwd!({
            // Create the root temp dir.
            let root = try_create_temp_dir().unwrap();

            let new_dir = root.join("test_dir_exists_not_found_error");

            // Try to check if the directory exists. It should return an error.
            let result = try_directory_exists(&new_dir);
            assert!(result.is_err());
            assert!(matches!(result, Err(FsOpError::DirectoryDoesNotExist(_))));
        });
    }

    #[cfg(unix)]
    fn test_try_directory_exists_permissions_errors() {
        with_saved_pwd!({
            // Create the root temp dir.
            let root = try_create_temp_dir().unwrap();

            // Create a directory, change to it, remove all permissions for user.
            let no_permissions_dir = root.join("no_permissions_dir");
            fs::create_dir_all(&no_permissions_dir).unwrap();
            let mut permissions =
                fs::metadata(&no_permissions_dir).unwrap().permissions();
            permissions.set_mode(0o000);
            fs::set_permissions(&no_permissions_dir, permissions).unwrap();
            assert!(no_permissions_dir.exists());

            // Try to check if the directory exists with insufficient permissions. It
            // should work!
            let result = try_directory_exists(&no_permissions_dir);
            assert!(result.is_ok());

            // Change the permissions back, so that it can be cleaned up!
            let mut permissions =
                fs::metadata(&no_permissions_dir).unwrap().permissions();
            permissions.set_mode(0o777);
            fs::set_permissions(&no_permissions_dir, permissions).unwrap();
        });
    }

    fn test_try_file_exists() {
        with_saved_pwd!({
            // Create the root temp dir.
            let root = try_create_temp_dir().unwrap();

            let new_dir = root.join("test_file_exists_dir");
            fs::create_dir_all(&new_dir).unwrap();

            let new_file = new_dir.join("test_file_exists_file.txt");
            fs::write(&new_file, "test").unwrap();

            assert!(try_file_exists(&new_file).unwrap());
            assert!(!try_file_exists(&new_dir).unwrap());

            fs::remove_dir_all(&new_dir).unwrap();

            // Ensure that an invalid path returns an error.
            assert!(try_file_exists(&new_file).is_err()); // This file does not exist.
            assert!(try_file_exists(&new_dir).is_err()); // This directory does
            // not exist.
        });
    }

    // On Windows, null bytes in paths are handled differently than on Unix.
    // Windows may truncate the path at the null byte rather than returning
    // InvalidInput, so this test only applies on Unix.
    #[cfg(unix)]
    fn test_try_file_exists_invalid_name_error() {
        with_saved_pwd!({
            // Create the root temp dir.
            let root = try_create_temp_dir().unwrap();

            let new_dir = root.join("test_file_exists_invalid_name_error\0");

            // Try to check if the file exists. It should return an error.
            let result = try_file_exists(&new_dir);
            assert!(result.is_err());
            assert!(matches!(result, Err(FsOpError::InvalidName(_))));
        });
    }

    #[cfg(unix)]
    fn test_try_file_exists_permissions_errors() {
        with_saved_pwd!({
            // Create the root temp dir.
            let root = try_create_temp_dir().unwrap();

            // Create a directory, change to it, remove all permissions for user.
            let no_permissions_dir = root.join("no_permissions_dir");
            fs::create_dir_all(&no_permissions_dir).unwrap();
            let mut permissions =
                fs::metadata(&no_permissions_dir).unwrap().permissions();
            permissions.set_mode(0o000);
            fs::set_permissions(&no_permissions_dir, permissions).unwrap();
            assert!(no_permissions_dir.exists());

            // Try to check if the file exists with insufficient permissions. It should
            // work!
            let result = try_file_exists(&no_permissions_dir);
            assert!(result.is_ok());

            // Change the permissions back, so that it can be cleaned up!
            let mut permissions =
                fs::metadata(&no_permissions_dir).unwrap().permissions();
            permissions.set_mode(0o777);
            fs::set_permissions(&no_permissions_dir, permissions).unwrap();
        });
    }

    fn test_try_pwd() {
        with_saved_pwd!({
            // Create the root temp dir.
            let root = try_create_temp_dir().unwrap();

            let new_dir = root.join("test_pwd");
            fs::create_dir_all(&new_dir).unwrap();
            env::set_current_dir(&new_dir).unwrap();

            let pwd = try_pwd().unwrap();
            assert!(pwd.exists());
            // Canonicalize new_dir for comparison because env::current_dir() returns
            // the canonical path (resolving symlinks). On macOS, /var is a symlink to
            // /private/var, so temp_dir() returns /var/... but current_dir() returns
            // /private/var/...
            // On Windows, strip the `\\?\` extended-length path prefix.
            let new_dir_canonical =
                strip_extended_length_prefix(new_dir.canonicalize().unwrap());
            assert_eq!(pwd, new_dir_canonical);
        });
    }

    #[cfg(unix)]
    fn test_try_pwd_errors() {
        with_saved_pwd!({
            // Create the root temp dir.
            let root = try_create_temp_dir().unwrap();

            // Create a directory, change to it, remove all permissions for user.
            let no_permissions_dir = root.join("no_permissions_dir");
            fs::create_dir_all(&no_permissions_dir).unwrap();
            env::set_current_dir(&no_permissions_dir).unwrap();
            let mut permissions =
                fs::metadata(&no_permissions_dir).unwrap().permissions();
            permissions.set_mode(0o000);
            fs::set_permissions(&no_permissions_dir, permissions).unwrap();
            assert!(no_permissions_dir.exists());

            // Try to get the pwd with insufficient permissions.
            // On Linux, getcwd() succeeds because it uses /proc/self/cwd.
            // On macOS, getcwd() fails with EACCES because it traverses directory
            // entries.
            let result = try_pwd();
            #[cfg(target_os = "linux")]
            assert!(result.is_ok());
            #[cfg(target_os = "macos")]
            assert!(result.is_err());

            // Change the permissions back, so that it can be cleaned up!
            let mut permissions =
                fs::metadata(&no_permissions_dir).unwrap().permissions();
            permissions.set_mode(0o777);
            fs::set_permissions(&no_permissions_dir, permissions).unwrap();

            // Delete this directory, and try pwd again. It will not longer exist.
            fs::remove_dir_all(&no_permissions_dir).unwrap();
            let result = try_pwd();
            assert!(result.is_err());
            assert!(matches!(result, Err(FsOpError::DirectoryDoesNotExist(_))));
        });
    }

    fn test_try_write() {
        with_saved_pwd!({
            // Create the root temp dir.
            let root = try_create_temp_dir().unwrap();

            // Create a new file.
            let content = "Hello, world!";
            let file_name = "test_file.txt";
            try_write_file(&root, file_name, content).unwrap();
            let file_path = root.join(file_name);

            // Check if the file exists and has the correct content.
            assert!(file_path.exists());
            assert!(file_path.is_file());
            let read_content = fs::read_to_string(&file_path).unwrap();
            assert_eq!(read_content, content);

            // root will be deleted at the end of the test when it is dropped.
        });
    }

    fn test_try_mkdir() {
        with_saved_pwd!({
            use crate::{MkdirOptions::*, try_mkdir};

            // Create the root temp dir.
            let root = try_create_temp_dir().unwrap();

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
    }

    #[cfg(unix)]
    fn test_try_change_directory_permissions_errors() {
        with_saved_pwd!({
            use crate::try_cd;

            // Create the root temp dir.
            let root = try_create_temp_dir().unwrap();

            // Create a new temporary directory.
            let new_tmp_dir =
                fs_paths!(with_root: root => "test_change_dir_permissions_errors");
            fs::create_dir_all(&new_tmp_dir).unwrap();
            assert!(new_tmp_dir.exists());

            // Create a directory with no permissions for user.
            let no_permissions_dir =
                fs_paths!(with_root: new_tmp_dir => "no_permissions_dir");
            fs::create_dir_all(&no_permissions_dir).unwrap();
            let mut permissions =
                fs::metadata(&no_permissions_dir).unwrap().permissions();
            permissions.set_mode(0o000);
            fs::set_permissions(&no_permissions_dir, permissions).unwrap();
            assert!(no_permissions_dir.exists());
            // Try to change to a directory with insufficient permissions.
            let result = try_cd(&no_permissions_dir);
            assert!(result.is_err());
            assert!(matches!(result, Err(FsOpError::PermissionDenied(_))));

            // Change the permissions back, so that it can be cleaned up!
            let mut permissions =
                fs::metadata(&no_permissions_dir).unwrap().permissions();
            permissions.set_mode(0o777);
            fs::set_permissions(&no_permissions_dir, permissions).unwrap();
        });
    }

    fn test_try_change_directory_happy_path() {
        with_saved_pwd!({
            use crate::try_cd;

            // Create the root temp dir.
            let root = try_create_temp_dir().unwrap();

            // Create a new temporary directory.
            let new_tmp_dir = fs_paths!(with_root: root => "test_change_dir_happy_path");
            fs::create_dir_all(&new_tmp_dir).unwrap();
            assert!(new_tmp_dir.exists());

            // Change to the temporary directory.
            try_cd(&new_tmp_dir).unwrap();
            // Canonicalize for comparison because env::current_dir() returns the
            // canonical path. On macOS, /var is a symlink to /private/var.
            // On Windows, strip the `\\?\` extended-length path prefix.
            assert_eq!(
                env::current_dir().unwrap(),
                strip_extended_length_prefix(new_tmp_dir.canonicalize().unwrap())
            );

            // Change back to the original directory.
            try_cd(&root).unwrap();
            assert_eq!(
                env::current_dir().unwrap(),
                strip_extended_length_prefix(root.canonicalize().unwrap())
            );
        });
    }

    fn test_try_change_directory_non_existent() {
        with_saved_pwd!({
            use crate::try_cd;

            // Create the root temp dir.
            let root = try_create_temp_dir().unwrap();

            // Create a new temporary directory.
            let new_tmp_dir =
                fs_paths!(with_root: root => "test_change_dir_non_existent");
            fs::create_dir_all(&new_tmp_dir).unwrap();
            assert!(new_tmp_dir.exists());

            // Try to change to a non-existent directory.
            let non_existent_dir =
                fs_paths!(with_root: new_tmp_dir => "non_existent_dir");
            let result = try_cd(&non_existent_dir);
            assert!(result.is_err());
            assert!(matches!(result, Err(FsOpError::DirectoryDoesNotExist(_))));

            // Change back to the original directory.
            try_cd(&root).unwrap();
            // Canonicalize for comparison (macOS /var -> /private/var symlink).
            // On Windows, strip the `\\?\` extended-length path prefix.
            assert_eq!(
                env::current_dir().unwrap(),
                strip_extended_length_prefix(root.canonicalize().unwrap())
            );
        });
    }

    // On Windows, null bytes in paths are handled differently than on Unix.
    // Windows may truncate the path at the null byte rather than returning
    // InvalidInput, so this test only applies on Unix.
    #[cfg(unix)]
    fn test_try_change_directory_invalid_name() {
        with_saved_pwd!({
            use crate::try_cd;

            // Create the root temp dir.
            let root = try_create_temp_dir().unwrap();

            // Create a new temporary directory.
            let new_tmp_dir =
                fs_paths!(with_root: root => "test_change_dir_invalid_name");
            fs::create_dir_all(&new_tmp_dir).unwrap();
            assert!(new_tmp_dir.exists());

            // Try to change to a directory with an invalid name.
            let invalid_name_dir =
                fs_paths!(with_root: new_tmp_dir => "invalid_name_dir\0");
            let result = try_cd(&invalid_name_dir);
            assert!(result.is_err());
            assert!(matches!(result, Err(FsOpError::InvalidName(_))));

            // Change back to the original directory.
            try_cd(&root).unwrap();
            // Canonicalize for comparison (macOS /var -> /private/var symlink).
            // On Windows, strip the `\\?\` extended-length path prefix.
            assert_eq!(
                env::current_dir().unwrap(),
                strip_extended_length_prefix(root.canonicalize().unwrap())
            );
        });
    }

    // XMARK: Process isolated test.

    /// This function runs all the tests that change the current working directory
    /// sequentially. This ensures that the current working directory is
    /// only changed in a controlled manner, eliminating flakiness when tests are run in
    /// parallel.
    ///
    /// This function is called by `test_all_fs_path_functions_in_isolated_process()` to
    /// run the tests in an isolated process.
    #[allow(clippy::missing_errors_doc)]
    fn run_all_fs_path_functions_sequentially_impl() {
        // Run each test in its own function with with_saved_pwd! to ensure the
        // current working directory is restored after each test.
        test_try_directory_exists_not_found_error();
        #[cfg(unix)]
        test_try_directory_exists_permissions_errors();
        test_try_file_exists();
        #[cfg(unix)]
        test_try_file_exists_invalid_name_error();
        #[cfg(unix)]
        test_try_file_exists_permissions_errors();
        test_try_pwd();
        #[cfg(unix)]
        test_try_pwd_errors();
        test_try_write();
        test_try_mkdir();
        #[cfg(unix)]
        test_try_change_directory_permissions_errors();
        test_try_change_directory_happy_path();
        test_try_change_directory_non_existent();
        #[cfg(unix)]
        test_try_change_directory_invalid_name();
    }

    /// This test function runs all the tests that change the current working directory
    /// in an isolated process. This ensures that the current working directory is
    /// only changed in a completely isolated environment, eliminating any potential
    /// side effects on other tests running in parallel.
    ///
    /// The issue is that when these tests are run by cargo test (in parallel in the SAME
    /// process), it leads to undefined behavior and flaky test failures, since the
    /// current working directory is changed per process, and all the tests are
    /// running in parallel in the same process.
    ///
    /// By running all these tests in an isolated process, we ensure that any changes to
    /// the current working directory are completely isolated and cannot affect other
    /// tests.
    #[test]
    fn test_all_fs_path_functions_in_isolated_process() {
        crate::suppress_wer_dialogs();
        if std::env::var("ISOLATED_TEST_RUNNER").is_ok() {
            // This is the actual test running in the isolated process.
            run_all_fs_path_functions_sequentially_impl();
            // If we reach here without errors, exit normally.
            std::process::exit(0);
        }

        // This is the test coordinator - spawn the actual test in a new process.
        let mut cmd = crate::new_isolated_test_command();
        cmd.env("ISOLATED_TEST_RUNNER", "1")
            .env("RUST_BACKTRACE", "1") // Get better error info
            .args([
                "--test-threads",
                "1",
                "test_all_fs_path_functions_in_isolated_process",
            ]);

        let output = cmd.output().expect("Failed to run isolated test");

        // Check if the child process exited successfully or if there's a panic message in
        // stderr
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success()
            || stderr.contains("panicked at")
            || stderr.contains("Test failed with error")
        {
            // These statements are important for IDEs providing hyperlinks to the line in
            // the test sources above which failed.
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
