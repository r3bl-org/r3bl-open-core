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

use std::{fs, os::unix::fs::PermissionsExt as _, path::Path};

use miette::IntoDiagnostic;

/// Sets the file at the specified path to be executable by owner, group, and others.
/// - `bash` equivalent: `chmod +x file`
/// - Eg: `set_file_executable("some_file.sh")`
/// - The `file` must exist and be a file (not a directory).
pub fn try_set_file_executable(file: impl AsRef<Path>) -> miette::Result<()> {
    let file = file.as_ref();
    let metadata = fs::metadata(file).into_diagnostic()?;

    if !metadata.is_file() {
        miette::bail!("This is not a file: '{}'", file.display());
    }

    // Set execute permissions for owner, group, and others on this file. 755 means:
    // - 7 (owner): read (4) + write (2) + execute (1) = 7 (rwx)
    // - 5 (group): read (4) + execute (1) = 5 (r-x)
    // - 5 (others): read (4) + execute (1) = 5 (r-x)
    fs::set_permissions(file, std::fs::Permissions::from_mode(0o755)).into_diagnostic()
}

#[cfg(test)]
mod tests_permissions {
    use r3bl_core::create_temp_dir;

    use super::*;

    #[test]
    fn test_set_file_executable() {
        // Create the root temp dir.
        let root = create_temp_dir().unwrap();

        let new_dir = root.join("test_set_file_executable");
        fs::create_dir_all(&new_dir).unwrap();

        let new_file = new_dir.join("test_set_file_executable.sh");
        fs::write(&new_file, "echo 'Hello, World!'").unwrap();

        try_set_file_executable(&new_file).unwrap();

        let metadata = fs::metadata(&new_file).unwrap();
        let lhs = metadata.permissions();

        // Assert that the file has executable permission for owner, group, and others:
        // - The bitwise AND operation (lhs.mode() & 0o777) ensures that only the
        //   permission bits are compared, ignoring other bits that might be present in
        //   the mode.
        // - The assertion checks if the permission bits match 0o755.
        assert_eq!(lhs.mode() & 0o777, 0o755);
    }

    #[test]
    fn test_set_file_executable_on_non_file() {
        // Create the root temp dir.
        let root = create_temp_dir().unwrap();

        let new_dir = root.join("test_set_file_executable_on_non_file");
        fs::create_dir_all(&new_dir).unwrap();

        let result = try_set_file_executable(&new_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_set_file_executable_on_non_existent_file() {
        // Create the root temp dir.
        let root = create_temp_dir().unwrap();

        let new_dir = root.join("test_set_file_executable_on_non_existent_file");
        fs::create_dir_all(&new_dir).unwrap();

        let non_existent_file = new_dir.join("non_existent_file.sh");
        let result = try_set_file_executable(&non_existent_file);
        assert!(result.is_err());
    }
}
