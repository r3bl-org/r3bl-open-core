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

use std::{env, io::ErrorKind, path::Path};

use crate::{fs_path::{FsOpError, FsOpResult},
            ok};

/// This macro is used to wrap a block with code that saves the current working directory,
/// runs the block of code for the test, and then restores the original working directory.
/// It also ensures that the test is run serially.
///
/// Be careful when manipulating the current working directory in tests using
/// [env::set_current_dir] as it can affect other tests that run in parallel.
#[macro_export]
macro_rules! serial_preserve_pwd_test {
    ($name:ident, $block:block) => {
        #[serial_test::serial]
        #[test]
        fn $name() {
            $crate::with_saved_pwd!($block);
        }
    };
}

/// This macro is used to wrap a block with code that saves the current working directory,
/// runs the block of code for the test, and then restores the original working directory.
///
/// Use this in conjunction with
/// [serial_test::serial](https://docs.rs/serial_test/latest/serial_test/) in order to
/// make sure that multiple threads are not changing the current working directory at the
/// same time (even with this macro). In other words, use this macro
/// [serial_preserve_pwd_test!] for tests.
#[macro_export]
macro_rules! with_saved_pwd {
    ($block:block) => {{
        let og_pwd = std::env::current_dir().unwrap();
        let result = { $block };
        std::env::set_current_dir(og_pwd).unwrap();
        result
    }};
}

/// Change cwd for current process.
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

#[cfg(test)]
mod tests_directory_change {
    use std::{fs, os::unix::fs::PermissionsExt as _};

    use super::*;
    use crate::{create_temp_dir,
                directory_change::try_cd,
                fs_path::FsOpError,
                fs_paths};

    serial_preserve_pwd_test!(test_try_change_directory_permissions_errors, {
        // Create the root temp dir.
        let root = create_temp_dir().unwrap();

        // Create a new temporary directory.
        let new_tmp_dir =
            fs_paths!(with_root: root => "test_change_dir_permissions_errors");
        fs::create_dir_all(&new_tmp_dir).unwrap();
        assert!(new_tmp_dir.exists());

        // Create a directory with no permissions for user.
        let no_permissions_dir =
            fs_paths!(with_root: new_tmp_dir => "no_permissions_dir");
        fs::create_dir_all(&no_permissions_dir).unwrap();
        let mut permissions = fs::metadata(&no_permissions_dir).unwrap().permissions();
        permissions.set_mode(0o000);
        fs::set_permissions(&no_permissions_dir, permissions).unwrap();
        assert!(no_permissions_dir.exists());
        // Try to change to a directory with insufficient permissions.
        let result = try_cd(&no_permissions_dir);
        println!("✅ err: {result:?}");
        assert!(result.is_err());
        assert!(matches!(result, Err(FsOpError::PermissionDenied(_))));

        // Change the permissions back, so that it can be cleaned up!
        let mut permissions = fs::metadata(&no_permissions_dir).unwrap().permissions();
        permissions.set_mode(0o777);
        fs::set_permissions(&no_permissions_dir, permissions).unwrap();
    });

    serial_preserve_pwd_test!(test_try_change_directory_happy_path, {
        // Create the root temp dir.
        let root = create_temp_dir().unwrap();

        // Create a new temporary directory.
        let new_tmp_dir = fs_paths!(with_root: root => "test_change_dir_happy_path");
        fs::create_dir_all(&new_tmp_dir).unwrap();
        assert!(new_tmp_dir.exists());

        // Change to the temporary directory.
        try_cd(&new_tmp_dir).unwrap();
        assert_eq!(env::current_dir().unwrap(), new_tmp_dir);

        // Change back to the original directory.
        try_cd(&root).unwrap();
        assert_eq!(env::current_dir().unwrap(), *root);
    });

    serial_preserve_pwd_test!(test_try_change_directory_non_existent, {
        // Create the root temp dir.
        let root = create_temp_dir().unwrap();

        // Create a new temporary directory.
        let new_tmp_dir = fs_paths!(with_root: root => "test_change_dir_non_existent");
        fs::create_dir_all(&new_tmp_dir).unwrap();
        assert!(new_tmp_dir.exists());

        // Try to change to a non-existent directory.
        let non_existent_dir = fs_paths!(with_root: new_tmp_dir => "non_existent_dir");
        let result = try_cd(&non_existent_dir);
        assert!(result.is_err());
        assert!(matches!(result, Err(FsOpError::DirectoryDoesNotExist(_))));

        // Change back to the original directory.
        try_cd(&root).unwrap();
        assert_eq!(env::current_dir().unwrap(), *root);
    });

    serial_preserve_pwd_test!(test_try_change_directory_invalid_name, {
        // Create the root temp dir.
        let root = create_temp_dir().unwrap();

        // Create a new temporary directory.
        let new_tmp_dir = fs_paths!(with_root: root => "test_change_dir_invalid_name");
        fs::create_dir_all(&new_tmp_dir).unwrap();
        assert!(new_tmp_dir.exists());

        // Try to change to a directory with an invalid name.
        let invalid_name_dir = fs_paths!(with_root: new_tmp_dir => "invalid_name_dir\0");
        let result = try_cd(&invalid_name_dir);
        assert!(result.is_err());
        println!("✅ err: {result:?}");
        assert!(matches!(result, Err(FsOpError::InvalidName(_))));

        // Change back to the original directory.
        try_cd(&root).unwrap();
        assert_eq!(env::current_dir().unwrap(), *root);
    });
}
