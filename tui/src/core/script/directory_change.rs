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

//! All tests have been moved to
//! `fs_path.rs::test_all_fs_path_functions_in_isolated_process()` to prevent flakiness
//! when tests are run in parallel.

use std::{env, io::ErrorKind, path::Path};

use crate::{fs_path::{FsOpError, FsOpResult},
            ok};

/// This macro is used to wrap a block with code that saves the current working directory,
/// runs the block of code for the test, and then restores the original working directory.
///
/// You might need to run tests that use this function in an isolated process. See tests
/// in [mod@super::fs_path] as an example of how to do this.
#[macro_export]
macro_rules! with_saved_pwd {
    ($block:block) => {{
        let og_pwd_res = std::env::current_dir();
        let result = { $block };
        if let Ok(it) = og_pwd_res {
            _ = std::env::set_current_dir(it);
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
pub fn try_cd(new_dir: impl AsRef<Path>) -> FsOpResult<()> {
    match env::set_current_dir(new_dir.as_ref()) {
        Ok(_) => ok!(),
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
