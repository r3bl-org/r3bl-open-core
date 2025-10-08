// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use miette::IntoDiagnostic;
use std::{fs, path::Path};

/// This is noop on Windows because Windows does not use the same permission model as
/// Unix-like systems. It is determined by file extension and ACLs (Access Control Lists).
///
/// # Errors
///
/// This function never returns an error on Windows.
#[cfg(target_os = "windows")]
pub fn set_permission(_file: impl AsRef<Path>, _mode: u32) -> miette::Result<()> {
    Ok(())
}

/// # Errors
///
/// Returns an error if:
/// - The file does not exist
/// - Insufficient permissions to change the file mode
/// - I/O error occurs while setting permissions
#[cfg(not(target_os = "windows"))]
pub fn set_permission(file: impl AsRef<Path>, mode: u32) -> miette::Result<()> {
    use std::{fs::Permissions, os::unix::fs::PermissionsExt};
    fs::set_permissions(file, Permissions::from_mode(mode)).into_diagnostic()
}

/// Sets the file at the specified path to be executable by owner, group, and others.
/// - `bash` equivalent: `chmod +x file`
/// - Eg: `set_file_executable("some_file.sh")`
/// - The `file` must exist and be a file (not a directory).
///
/// # Errors
///
/// Returns an error if:
/// - The file does not exist
/// - The path points to a directory instead of a file
/// - Insufficient permissions to change the file mode
/// - I/O error occurs while setting permissions
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
    set_permission(file, 0o755)
}

#[cfg(test)]
mod tests_permissions {
    use super::*;
    use crate::try_create_temp_dir;

    /// This is noop on Windows because Windows does not use the same permission model as
    /// Unix-like systems. It is determined by file extension and ACLs (Access Control
    /// Lists).
    #[cfg(target_os = "windows")]
    pub fn assert_permissions(_permissions: &std::fs::Permissions, _mode: u32) {}

    #[cfg(not(target_os = "windows"))]
    /// # Panics
    ///
    /// Panics if the permissions do not match the expected mode.
    pub fn assert_permissions(permissions: &std::fs::Permissions, expected: u32) {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(permissions.mode() & 0o777, expected);
    }

    #[test]
    fn test_set_file_executable() {
        // Create the root temp dir.
        let root = try_create_temp_dir().unwrap();

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
        assert_permissions(&lhs, 0o755);
    }

    #[test]
    fn test_set_file_executable_on_non_file() {
        // Create the root temp dir.
        let root = try_create_temp_dir().unwrap();

        let new_dir = root.join("test_set_file_executable_on_non_file");
        fs::create_dir_all(&new_dir).unwrap();

        let result = try_set_file_executable(&new_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_set_file_executable_on_non_existent_file() {
        // Create the root temp dir.
        let root = try_create_temp_dir().unwrap();

        let new_dir = root.join("test_set_file_executable_on_non_existent_file");
        fs::create_dir_all(&new_dir).unwrap();

        let non_existent_file = new_dir.join("non_existent_file.sh");
        let result = try_set_file_executable(&non_existent_file);
        assert!(result.is_err());
    }
}
