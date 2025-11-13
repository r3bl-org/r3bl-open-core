// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! All tests have been moved to
//! `fs_path.rs::test_all_fs_path_functions_in_isolated_process()` to prevent flakiness
//! when tests are run in parallel.

use crate::ok;
use super::fs_path::{FsOpError, FsOpResult};
use std::{env, io::ErrorKind, path::Path};

/// This macro is used to wrap a block with code that saves the current working directory,
/// runs the block of code for the test, and then restores the original working directory.
///
/// You might need to run tests that use this function in an isolated process. See tests
/// in [`mod@crate::script::fs_path`] as an example of how to do this.
///
/// # Examples
///
/// ## Sync usage - temporarily changes directory and restores it
///
/// ```no_run
/// # use std::env;
/// # use r3bl_tui::with_saved_pwd;
/// let result = with_saved_pwd!({
///     let _ = env::set_current_dir("/tmp");
///     // Operations here see /tmp as current directory
///     42 // Return value from the block
/// });
/// assert_eq!(result, 42);
/// // Original directory is restored here
/// ```
///
/// ## Async usage - works with async code
///
/// ```no_run
/// # use r3bl_tui::with_saved_pwd;
/// # async fn async_example() {
/// let result = with_saved_pwd!(async {
///     let _ = std::env::set_current_dir("/tmp");
///     // Operations here see /tmp as current directory
///     // Can use .await and async operations here
///     // Directory is restored when block completes
///     "done" // Return value from the block
/// });
/// assert_eq!(result, "done");
/// # }
/// ```
#[macro_export]
macro_rules! with_saved_pwd {
    // Async block variant
    (async $block:block) => {{
        let og_pwd_res = std::env::current_dir();
        let result = async { $block }.await; // <- This line is different
        if let Ok(it) = og_pwd_res {
            // We don't care about the result of this operation.
            std::env::set_current_dir(it).ok();
        }
        result
    }};
    // Sync block variant
    ($block:block) => {{
        let og_pwd_res = std::env::current_dir();
        let result = { $block }; // <- This line is different
        if let Ok(it) = og_pwd_res {
            // We don't care about the result of this operation.
            std::env::set_current_dir(it).ok();
        }
        result
    }};
}

/// Change cwd for current process. This is potentially dangerous, as it can
/// affect other parts of the program that rely on the current working directory.
/// Use with caution. An example of this is when running tests in parallel in `cargo
/// test`. `cargo test` runs all the tests in a single process. This means that when one
/// test changes the current working directory, it affects all other tests that run after
/// it.
///
/// # Errors
///
/// Returns an error if:
/// - The directory does not exist
/// - Insufficient permissions to access the directory
/// - The directory name is invalid
/// - I/O errors occur while changing the directory
pub fn try_cd(new_dir: impl AsRef<Path>) -> FsOpResult<()> {
    match env::set_current_dir(new_dir.as_ref()) {
        Ok(()) => ok!(),
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
